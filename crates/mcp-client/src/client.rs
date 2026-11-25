//! MCP client session — thin wrapper around the `rmcp` SDK.
//!
//! Stores the `Peer<RoleClient>` for sending requests and the
//! `RunningService` for lifecycle management.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use rmcp::ClientServiceExt;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ProtocolVersion, ServerPeerInfo, SubscriptionFilter,
    Tool,
};
use rmcp::service::{ClientLifecycleMode, Peer, RoleClient, RunningService};
use rmcp::transport::IntoTransport;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::handler::{AssistantClientHandler, StaleTools};

/// How long to wait before listening again after a subscription ends.
const RELISTEN_DELAY: Duration = Duration::from_secs(30);

/// An active session with a single MCP server.
///
/// The `Peer` is cheaply clonable (Arc internally) and used for all
/// request/response interactions.  The `RunningService` owns the
/// background I/O task and is kept alive for the session duration.
pub struct McpClient {
    /// User-assigned name for this server connection.
    name: String,
    /// The rmcp peer used for sending requests.
    peer: Peer<RoleClient>,
    /// The running service that owns the background I/O task.
    /// Wrapped in `Mutex<Option<...>>` so we can `close()` it on shutdown.
    _service: Mutex<Option<RunningService<RoleClient, AssistantClientHandler>>>,
    /// Whether the server announced a tool list change since the last listing.
    stale: StaleTools,
    /// Watches list-change notifications on a `2026-07-28` connection.
    listener: Option<JoinHandle<()>>,
}

impl McpClient {
    /// Open a session over `transport`.
    ///
    /// Tries `2026-07-28` (`server/discover`) first and falls back to
    /// `initialize` on `2025-11-25` for servers that do not know it. The
    /// handler's tool-staleness flag is `stale`.
    pub async fn connect<T, E, A>(
        name: impl Into<String>,
        transport: T,
        stale: StaleTools,
    ) -> Result<Self>
    where
        T: IntoTransport<RoleClient, E, A>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let service = AssistantClientHandler::new(stale.clone())
            .serve_with_lifecycle(
                transport,
                ClientLifecycleMode::Auto {
                    preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                    legacy_version: Some(ProtocolVersion::LATEST_WITH_INITIALIZE),
                },
            )
            .await
            .map_err(anyhow::Error::new)?;
        let peer = service.peer().clone();
        // A server on `2026-07-28` sends list changes only to a subscriber;
        // earlier ones send them unsolicited, to the handler.
        let listener = peer
            .peer_info()
            .is_some_and(|info| !info.protocol_version.has_initialize())
            .then(|| tokio::spawn(watch_list_changes(peer.clone(), stale.clone())));
        Ok(Self {
            name: name.into(),
            peer,
            _service: Mutex::new(Some(service)),
            stale,
            listener,
        })
    }

    /// Whether the server signalled a tool list change since the last call.
    /// Clears the mark.
    pub fn take_tools_stale(&self) -> bool {
        self.stale.take()
    }

    /// Whether a tool list change is waiting to be picked up.
    pub fn tools_stale(&self) -> bool {
        self.stale.is_marked()
    }

    /// Flag the tool list stale again, for a listing that failed.
    pub fn mark_tools_stale(&self) {
        self.stale.mark();
    }

    /// Fetch the list of tools from the remote server (handles pagination).
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let tools = self
            .peer
            .list_all_tools()
            .await
            .context("tools/list request failed")?;

        debug!(
            server = %self.name,
            count = tools.len(),
            "discovered remote tools"
        );

        Ok(tools)
    }

    /// Call a tool on the remote server.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult> {
        let mut params = CallToolRequestParams::new(name.to_string());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }

        let result = self
            .peer
            .call_tool(params)
            .await
            .with_context(|| format!("tools/call '{name}' request failed"))?;

        Ok(result)
    }

    /// The user-assigned name for this server.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the underlying transport is still connected.
    pub fn is_connected(&self) -> bool {
        !self.peer.is_transport_closed()
    }

    /// Snapshot of the server's handshake info (available after initialization).
    ///
    /// rmcp hands out an owned `Arc` snapshot rather than a borrow, so the
    /// caller keeps the info alive independently of this session.
    pub fn server_info(&self) -> Option<Arc<ServerPeerInfo>> {
        self.peer.peer_info()
    }

    /// Gracefully shut down the MCP session.
    pub async fn shutdown(&self) -> Result<()> {
        debug!(server = %self.name, "shutting down MCP client");
        let mut service = self._service.lock().await;
        if let Some(mut svc) = service.take() {
            // RunningService::close() waits for cleanup.  Ignore its result —
            // it only fails if the transport is already gone, which is fine
            // during shutdown.
            let _ = svc.close().await;
            debug!(server = %self.name, "MCP client session closed");
        }
        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        if let Some(listener) = &self.listener {
            listener.abort();
        }
    }
}

/// Keep a subscription for tool and prompt list changes open for as long as
/// the connection lives, marking `stale` on each one.
///
/// A server that refuses the subscription gets no more attempts. One that
/// ends it is asked again after a pause, and the tools count as stale in the
/// meantime, since changes may have gone unannounced.
async fn watch_list_changes(peer: Peer<RoleClient>, stale: StaleTools) {
    let filter = SubscriptionFilter::builder()
        .tools_list_changed()
        .prompts_list_changed()
        .build();
    while !peer.is_transport_closed() {
        let mut subscription = match peer.listen(filter.clone()).await {
            Ok(subscription) => subscription,
            Err(error) => {
                debug!(%error, "server does not accept a list-changed subscription");
                return;
            }
        };
        while let Ok(Some(_)) = subscription.next().await {
            stale.mark();
        }
        stale.mark();
        tokio::time::sleep(RELISTEN_DELAY).await;
    }
}
