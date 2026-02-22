//! Builtin handler for shell-exec tool — runs a shell command as a subprocess.

use std::collections::HashMap;

use anyhow::Result;
use assistant_core::{ExecutionContext, ToolHandler, ToolOutput};
use async_trait::async_trait;
use tokio::time::Duration;
use tracing::debug;

const TIMEOUT_SECS: u64 = 30;

pub struct ShellExecHandler;

impl ShellExecHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShellExecHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolHandler for ShellExecHandler {
    fn name(&self) -> &str {
        "shell-exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command with user confirmation required. Blocked on remote interfaces (Signal, Slack, Mattermost)."
    }

    fn params_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "command": {"type": "string", "description": "The shell command to execute"},
            "working_dir": {"type": "string", "description": "Optional working directory for the command"}
        })
    }

    fn is_mutating(&self) -> bool {
        true
    }

    fn requires_confirmation(&self) -> bool {
        true
    }

    async fn run(
        &self,
        params: HashMap<String, serde_json::Value>,
        ctx: &ExecutionContext,
    ) -> Result<ToolOutput> {
        // Only available in interactive mode
        if !ctx.interactive {
            return Ok(ToolOutput::error(
                "shell-exec is not available in non-interactive mode",
            ));
        }

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

        debug!("shell-exec: running command: {}", command);

        let mut cmd = tokio::process::Command::new("/bin/sh");
        cmd.arg("-c").arg(&command);
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref dir) = working_dir {
            cmd.current_dir(dir);
        }

        let timeout = Duration::from_secs(TIMEOUT_SECS);

        let result = tokio::time::timeout(timeout, async {
            let child = cmd.spawn()?;
            child.wait_with_output().await.map_err(anyhow::Error::from)
        })
        .await;

        match result {
            Err(_elapsed) => Ok(ToolOutput::error(format!(
                "Command timed out after {} seconds: {}",
                TIMEOUT_SECS, command
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

                let content = parts.join("\n\n");

                if output.status.success() {
                    Ok(ToolOutput::success(content))
                } else {
                    // Non-zero exit is still a valid result — mark it but don't fail
                    Ok(ToolOutput::success(content))
                }
            }
        }
    }
}
