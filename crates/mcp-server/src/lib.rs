// MCP server library — consumed by the unified `assistant-cli` crate.
pub mod protocol;
pub mod server;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_runtime::Orchestrator;
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{info, warn};

use crate::protocol::JsonRpcRequest;

/// Run the MCP stdio JSON-RPC server using the provided pre-built components.
///
/// This function takes over `stdin`/`stdout` and runs a line-oriented JSON-RPC
/// loop until EOF.  All logging is expected to go to `stderr` (configure the
/// tracing subscriber accordingly before calling this).
///
/// # Arguments
///
/// * `orchestrator` — shared orchestrator for `run_prompt` / `invoke_skill`
/// * `executor` — shared skill executor for `invoke_skill`
/// * `registry` — shared skill registry for `list_skills` / `resources/list`
/// * `_storage` — storage layer (reserved for future use)
/// * `user_skills_dir` — directory where `install_skill` writes new skills
pub async fn run(
    orchestrator: Arc<Orchestrator>,
    executor: Arc<SkillExecutor>,
    registry: Arc<SkillRegistry>,
    _storage: Arc<StorageLayer>,
    user_skills_dir: PathBuf,
) -> Result<()> {
    info!("MCP server ready — reading JSON-RPC from stdin");

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                warn!("stdin read error: {e}");
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to parse JSON-RPC request: {e}");
                let err = crate::protocol::JsonRpcResponse::err(None, -32700, "Parse error");
                let mut json = serde_json::to_vec(&err).unwrap_or_default();
                json.push(b'\n');
                stdout.write_all(&json).await.ok();
                stdout.flush().await.ok();
                continue;
            }
        };

        let response = server::handle_request(
            request,
            registry.clone(),
            executor.clone(),
            orchestrator.clone(),
            user_skills_dir.clone(),
        )
        .await;

        let mut json = serde_json::to_vec(&response).unwrap_or_default();
        json.push(b'\n');
        stdout.write_all(&json).await.ok();
        stdout.flush().await.ok();
    }

    info!("MCP server shutting down");
    Ok(())
}
