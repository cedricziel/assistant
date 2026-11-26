//! MCP server library — consumed by the unified `assistant-cli` crate.
//!
//! The protocol itself (framing, lifecycle, version negotiation) comes from
//! `rmcp`; this crate supplies the [`AssistantMcpServer`] handler.

mod http;
mod identity;
mod listeners;
mod prompts;
mod resources;
mod server;
mod tasks;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::auth::AuthContext;
use assistant_runtime::Orchestrator;
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;
use rmcp::service::ServerInitializeError;
use rmcp::transport::IntoTransport;
use rmcp::{RoleServer, ServiceExt};
use tracing::info;

pub use http::{RequestBoundServer, streamable_http_service};
pub use identity::{Caller, with_identity};
pub use listeners::ListChangeListeners;
pub use server::AssistantMcpServer;
pub use tasks::TaskStore;

/// Serve MCP over the process's stdin and stdout until the client disconnects.
///
/// stdout carries only protocol messages; configure the tracing subscriber to
/// write to stderr before calling this.
///
/// # Arguments
///
/// * `orchestrator` — shared orchestrator for the `run-prompt` tool
/// * `executor` — shared tool executor for `tools/call`
/// * `registry` — shared skill registry for resources and `install-skill`
/// * `_storage` — storage layer (reserved for future use)
/// * `user_skills_dir` — directory where `install-skill` writes new skills
pub async fn run(
    orchestrator: Arc<Orchestrator>,
    executor: Arc<ToolExecutor>,
    registry: Arc<SkillRegistry>,
    _storage: Arc<StorageLayer>,
    user_skills_dir: PathBuf,
) -> Result<()> {
    let policy = orchestrator.permission_policy().clone();
    let server = AssistantMcpServer::new(
        orchestrator,
        executor,
        registry.clone(),
        policy,
        user_skills_dir,
        ListChangeListeners::default(),
        TaskStore::default(),
    )
    .with_install_registry(registry);

    info!("MCP server ready — serving on stdio");
    serve_stdio(server).await
}

/// Serve `server` over stdin and stdout until the client disconnects.
///
/// Stdio callers act as the local system identity.
pub async fn serve_stdio(server: AssistantMcpServer) -> Result<()> {
    serve_transport(
        server,
        with_identity(rmcp::transport::stdio(), AuthContext::system()),
    )
    .await
}

/// Serve `server` over `transport` until the peer disconnects.
///
/// A peer that disconnects before it starts a session is a clean shutdown, not
/// an error.
pub async fn serve_transport<T, E, A>(server: AssistantMcpServer, transport: T) -> Result<()>
where
    T: IntoTransport<RoleServer, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    match server.serve(transport).await {
        Ok(running) => {
            running.waiting().await?;
        }
        Err(ServerInitializeError::ConnectionClosed(_)) => {}
        Err(error) => return Err(error.into()),
    }
    info!("MCP server shutting down");
    Ok(())
}
