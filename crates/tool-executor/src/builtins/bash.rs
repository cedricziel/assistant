//! Builtin handler for the `bash` tool — runs a bash command as a subprocess.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use tokio::time::Duration;
use tracing::debug;

const DEFAULT_TIMEOUT_SECS: u64 = 120;

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
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                let mut parts: Vec<String> = Vec::new();
                parts.push(format!("Exit code: {}", exit_code));

                if !stdout.is_empty() {
                    parts.push(format!("stdout:\n{}", stdout.trim_end()));
                }
                if !stderr.is_empty() {
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
                Ok(ToolOutput::success(parts.join("\n\n")).with_data(data))
            }
        }
    }
}
