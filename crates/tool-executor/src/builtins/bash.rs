//! Builtin handler for the `bash` tool — runs a bash command as a subprocess.
//!
//! Output is capped per stream (stdout / stderr) while the command runs, so a
//! chatty command never buffers more than twice the cap. When truncation is
//! necessary the **tail** of the output is kept (matching the approach used by
//! OpenClaw) because the most recent output — error messages, final status
//! lines — is almost always the most relevant part.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::permission::{WorkspaceRoots, resolve_real_path};
use assistant_core::{
    Attachment, ToolHandler, ToolOutput, runtime_workspace_dir,
    types::conversation::ExecutionContext, types::permissions::LimitsConfig,
};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::time::Duration;
use tracing::debug;

const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Keeps the last `cap` bytes of a stream without holding more than `2 * cap`.
struct TailBuffer {
    cap: usize,
    buf: Vec<u8>,
    total: usize,
}

impl TailBuffer {
    fn new(cap: usize) -> Self {
        Self {
            cap,
            buf: Vec::new(),
            total: 0,
        }
    }

    fn push(&mut self, chunk: &[u8]) {
        self.total += chunk.len();
        self.buf.extend_from_slice(chunk);
        if self.buf.len() > self.cap.saturating_mul(2) {
            self.buf.drain(..self.buf.len() - self.cap);
        }
    }

    /// The kept tail as text, the total bytes seen, and whether bytes were dropped.
    fn finish(self) -> (String, usize, bool) {
        let start = self.buf.len().saturating_sub(self.cap);
        let text = String::from_utf8_lossy(&self.buf[start..]).into_owned();
        (text, self.total, self.total > self.cap)
    }
}

async fn read_tail<R: AsyncRead + Unpin>(mut reader: R, cap: usize) -> TailBuffer {
    let mut tail = TailBuffer::new(cap);
    let mut chunk = [0u8; 8192];
    while let Ok(n) = reader.read(&mut chunk).await {
        if n == 0 {
            break;
        }
        tail.push(&chunk[..n]);
    }
    tail
}

/// Image file extensions we recognise for automatic attachment.
const IMAGE_EXTENSIONS: &[(&str, &str)] = &[
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("svg", "image/svg+xml"),
    ("bmp", "image/bmp"),
];

/// Maximum image file size we'll auto-attach (10 MB).
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

/// Scan stdout for absolute file paths that point to images on disk.
/// Returns a list of [`Attachment`]s for each file that exists, is within the
/// size limit and resolves (symlinks included) to a location inside `roots`.
async fn collect_image_attachments(text: &str, roots: &WorkspaceRoots) -> Vec<Attachment> {
    let mut attachments = Vec::new();
    for token in text.split_whitespace() {
        // Only consider tokens that look like absolute paths.
        let token = token.trim_matches(|c: char| c == '\'' || c == '"' || c == '`');
        if !token.starts_with('/') {
            continue;
        }
        let resolved = resolve_real_path(token, None);
        if !roots.contains(&resolved) {
            continue;
        }
        let path = resolved.as_path();
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };
        let mime = match IMAGE_EXTENSIONS.iter().find(|(e, _)| *e == ext) {
            Some((_, m)) => *m,
            None => continue,
        };
        // Check file existence and size asynchronously.
        let meta = match tokio::fs::metadata(path).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() || meta.len() > MAX_IMAGE_BYTES {
            continue;
        }
        if let Ok(data) = tokio::fs::read(path).await {
            let filename = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            attachments.push(Attachment::new(filename, mime, data));
        }
    }
    attachments
}

#[derive(Default)]
pub struct BashHandler {
    limits: LimitsConfig,
    extra_roots: Vec<String>,
}

impl BashHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Timeout and output limits (`[permissions.limits]`).
    pub fn with_limits(mut self, limits: LimitsConfig) -> Self {
        self.limits = limits;
        self
    }

    /// Directories, besides the persona workspace, whose images may be auto-attached.
    pub fn with_extra_roots(mut self, roots: Vec<String>) -> Self {
        self.extra_roots = roots;
        self
    }
}

#[async_trait]
impl ToolHandler for BashHandler {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Run a bash command and return its stdout/stderr. Use for automation tasks that do not require user confirmation."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "The bash command to execute"},
                "working_dir": {"type": "string", "description": "Optional working directory for the command"},
                "timeout_secs": {"type": "integer", "minimum": 1, "description": "Timeout in seconds (default: 120)"}
            },
            "required": ["command"]
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn output_schema(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "exit_code": {"type": "integer", "description": "Process exit code (0 = success)"},
                "stdout": {"type": "string", "description": "Standard output"},
                "stderr": {"type": "string", "description": "Standard error"}
            },
            "required": ["exit_code", "stdout", "stderr"]
        }))
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let command = match params.get("command").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => {
                return Ok(ToolOutput::error("Missing required parameter 'command'"));
            }
        };

        let working_dir = params
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| runtime_workspace_dir().map(|path| path.to_string_lossy().to_string()));

        let timeout_secs = params
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .min(self.limits.bash_max_timeout_secs);

        debug!(
            "bash: running command: {} (timeout: {}s)",
            command, timeout_secs
        );

        let mut cmd = tokio::process::Command::new("bash");
        cmd.kill_on_drop(true);
        cmd.arg("-c").arg(&command);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref dir) = working_dir {
            cmd.current_dir(dir);
        }

        let timeout = Duration::from_secs(timeout_secs);
        let cap = self.limits.bash_max_output_chars;

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolOutput::error(format!(
                    "Failed to spawn command '{}': {}",
                    command, e
                )));
            }
        };
        let (Some(stdout_pipe), Some(stderr_pipe)) = (child.stdout.take(), child.stderr.take())
        else {
            return Ok(ToolOutput::error("spawned command has no piped output"));
        };

        let finished = tokio::time::timeout(timeout, async {
            let (status, stdout, stderr) = tokio::join!(
                child.wait(),
                read_tail(stdout_pipe, cap),
                read_tail(stderr_pipe, cap)
            );
            (status, stdout, stderr)
        })
        .await;

        match finished {
            Err(_elapsed) => {
                let _ = child.kill().await;
                Ok(ToolOutput::error(format!(
                    "Command timed out after {} seconds: {}",
                    timeout_secs, command
                )))
            }
            Ok((Err(e), _, _)) => Ok(ToolOutput::error(format!(
                "Failed to run command '{}': {}",
                command, e
            ))),
            Ok((Ok(status), stdout_tail, stderr_tail)) => {
                let exit_code = status.code().unwrap_or(-1);
                let (stdout, stdout_total, stdout_truncated) = stdout_tail.finish();
                let (stderr, stderr_total, stderr_truncated) = stderr_tail.finish();
                let mut parts: Vec<String> = Vec::new();
                parts.push(format!("Exit code: {}", exit_code));

                if !stdout.is_empty() {
                    if stdout_truncated {
                        parts.push(format!(
                            "[stdout truncated — showing last {} of {} chars]",
                            cap, stdout_total,
                        ));
                    }
                    parts.push(format!("stdout:\n{}", stdout.trim_end()));
                }
                if !stderr.is_empty() {
                    if stderr_truncated {
                        parts.push(format!(
                            "[stderr truncated — showing last {} of {} chars]",
                            cap, stderr_total,
                        ));
                    }
                    parts.push(format!("stderr:\n{}", stderr.trim_end()));
                }
                if stdout.is_empty() && stderr.is_empty() {
                    parts.push("(no output)".to_string());
                }

                let data = serde_json::json!({
                    "exit_code": exit_code,
                    "stdout": stdout,
                    "stderr": stderr
                });

                // Auto-attach images referenced by path in stdout so they
                // flow to the user via the interface (Slack upload, etc.)
                // without polluting the conversation context.
                let roots = WorkspaceRoots::for_agent(&ctx.agent_id, &self.extra_roots);
                let images = collect_image_attachments(&stdout, &roots).await;

                let mut result = ToolOutput::success(parts.join("\n\n")).with_data(data);
                if !images.is_empty() {
                    debug!(
                        count = images.len(),
                        "bash: auto-attaching image files from output"
                    );
                    result = result.with_attachments(images);
                }
                Ok(result)
            }
        }
    }
}
