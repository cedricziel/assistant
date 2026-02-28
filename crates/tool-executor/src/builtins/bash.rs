//! Builtin handler for the `bash` tool — runs a bash command as a subprocess.
//!
//! Output is capped at [`MAX_OUTPUT_CHARS`] per stream (stdout / stderr).
//! When truncation is necessary the **tail** of the output is kept (matching
//! the approach used by OpenClaw) because the most recent output — error
//! messages, final status lines — is almost always the most relevant part.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use assistant_core::{Attachment, ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use tokio::time::Duration;
use tracing::debug;

const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Maximum number of characters to keep from stdout/stderr.
/// Output beyond this limit is truncated so it does not blow up the
/// conversation context window.  Value chosen to stay well within the
/// LLM context limit (~200 K tokens ≈ 800 K chars).
const MAX_OUTPUT_CHARS: usize = 200_000;

/// Truncate `s` to at most `max` characters, keeping the **tail**.
/// Returns the (possibly shortened) string, its original length, and
/// whether truncation occurred.
fn truncate_tail(s: &str, max: usize) -> (String, usize, bool) {
    let original_len = s.len();
    if original_len <= max {
        return (s.to_string(), original_len, false);
    }
    // Find a char boundary at or after (len - max).
    let start = original_len - max;
    let start = s.ceil_char_boundary(start);
    (s[start..].to_string(), original_len, true)
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
/// Returns a list of [`Attachment`]s for each file that exists and is
/// within the size limit.
async fn collect_image_attachments(text: &str) -> Vec<Attachment> {
    let mut attachments = Vec::new();
    for token in text.split_whitespace() {
        // Only consider tokens that look like absolute paths.
        let token = token.trim_matches(|c: char| c == '\'' || c == '"' || c == '`');
        if !token.starts_with('/') {
            continue;
        }
        let path = Path::new(token);
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

pub struct BashHandler;

impl BashHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BashHandler {
    fn default() -> Self {
        Self::new()
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
        _ctx: &ExecutionContext,
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
            .map(|s| s.to_string());

        let timeout_secs = params
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

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

        let result = tokio::time::timeout(timeout, async {
            let child = cmd.spawn()?;
            child.wait_with_output().await.map_err(anyhow::Error::from)
        })
        .await;

        match result {
            Err(_elapsed) => Ok(ToolOutput::error(format!(
                "Command timed out after {} seconds: {}",
                timeout_secs, command
            ))),
            Ok(Err(e)) => Ok(ToolOutput::error(format!(
                "Failed to spawn command '{}': {}",
                command, e
            ))),
            Ok(Ok(output)) => {
                let stdout_raw = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr_raw = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                let (stdout, stdout_total, stdout_truncated) =
                    truncate_tail(&stdout_raw, MAX_OUTPUT_CHARS);
                let (stderr, stderr_total, stderr_truncated) =
                    truncate_tail(&stderr_raw, MAX_OUTPUT_CHARS);

                let mut parts: Vec<String> = Vec::new();
                parts.push(format!("Exit code: {}", exit_code));

                if !stdout.is_empty() {
                    if stdout_truncated {
                        parts.push(format!(
                            "[stdout truncated — showing last {} of {} chars]",
                            MAX_OUTPUT_CHARS, stdout_total,
                        ));
                    }
                    parts.push(format!("stdout:\n{}", stdout.trim_end()));
                }
                if !stderr.is_empty() {
                    if stderr_truncated {
                        parts.push(format!(
                            "[stderr truncated — showing last {} of {} chars]",
                            MAX_OUTPUT_CHARS, stderr_total,
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
                let images = collect_image_attachments(&stdout_raw).await;

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
