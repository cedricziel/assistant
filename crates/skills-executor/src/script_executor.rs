//! Script-tier skill executor — runs an external script as a subprocess.
//!
//! Parameters are passed to the script as JSON via the `SKILL_PARAMS` environment
//! variable, and also via stdin as a JSON object. The script's stdout is captured
//! and returned as the skill output.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Result};
use assistant_core::{ExecutionContext, SkillOutput};
use tokio::time::Duration;
use tracing::debug;

const TIMEOUT_SECS: u64 = 60;

pub async fn run_script(
    entrypoint: &Path,
    params: &HashMap<String, serde_json::Value>,
    _ctx: &ExecutionContext,
) -> Result<SkillOutput> {
    // Verify the script exists
    if !entrypoint.exists() {
        return Ok(SkillOutput::error(format!(
            "Script entrypoint does not exist: {}",
            entrypoint.display()
        )));
    }

    // Check execute permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = tokio::fs::metadata(entrypoint).await?;
        let mode = meta.permissions().mode();
        // At least one of owner/group/other execute bits must be set
        if mode & 0o111 == 0 {
            return Ok(SkillOutput::error(format!(
                "Script '{}' is not executable (chmod +x it first)",
                entrypoint.display()
            )));
        }
    }

    let params_json = serde_json::to_string(params)?;
    debug!(
        "script_executor: running {:?} with params: {}",
        entrypoint, params_json
    );

    let mut cmd = tokio::process::Command::new(entrypoint);
    cmd.env("SKILL_PARAMS", &params_json);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let timeout = Duration::from_secs(TIMEOUT_SECS);

    let result = tokio::time::timeout(timeout, async move {
        let mut child = cmd.spawn()?;

        // Write params JSON to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(params_json.as_bytes()).await?;
            // stdin is dropped here, signalling EOF to the child
        }

        child.wait_with_output().await.map_err(anyhow::Error::from)
    })
    .await;

    match result {
        Err(_elapsed) => Ok(SkillOutput::error(format!(
            "Script '{}' timed out after {} seconds",
            entrypoint.display(),
            TIMEOUT_SECS
        ))),
        Ok(Err(e)) => bail!("Failed to run script '{}': {}", entrypoint.display(), e),
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                let content = if stdout.is_empty() {
                    if stderr.is_empty() {
                        "(script produced no output)".to_string()
                    } else {
                        format!("stderr:\n{}", stderr.trim_end())
                    }
                } else {
                    stdout.trim_end().to_string()
                };
                Ok(SkillOutput::success(content))
            } else {
                let exit_code = output.status.code().unwrap_or(-1);
                let mut parts: Vec<String> = Vec::new();
                parts.push(format!(
                    "Script '{}' exited with code {}",
                    entrypoint.display(),
                    exit_code
                ));
                if !stdout.is_empty() {
                    parts.push(format!("stdout:\n{}", stdout.trim_end()));
                }
                if !stderr.is_empty() {
                    parts.push(format!("stderr:\n{}", stderr.trim_end()));
                }
                Ok(SkillOutput::error(parts.join("\n\n")))
            }
        }
    }
}
