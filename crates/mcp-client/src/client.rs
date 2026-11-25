//! MCP client session — thin wrapper around the `rmcp` SDK.
//!
//! Stores the `Peer<RoleClient>` for sending requests and the
//! `RunningService` for lifecycle management.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use rmcp::ClientServiceExt;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, CancelTaskParams, ContentBlock,
    GetTaskParams, ProtocolVersion, ReadResourceRequestParams, ResourceContents, ServerPeerInfo,
    Skill, SubscriptionFilter, Task, TaskPayload, Tool,
};
use rmcp::service::{ClientLifecycleMode, Peer, RoleClient, RunningService, ServiceError};
use rmcp::transport::IntoTransport;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::debug;

use crate::handler::{AssistantClientHandler, StaleTools};

/// How long to wait before listening again after a subscription ends.
const RELISTEN_DELAY: Duration = Duration::from_secs(30);

/// How long a tool call may take unless [`McpClient::with_tool_timeout`] says
/// otherwise; the same as the `bash` tool's default.
const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(120);

/// How often to poll a task whose server suggests no interval.
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// The longest wait between polls, whatever the server suggests.
const MAX_POLL_INTERVAL: Duration = Duration::from_secs(10);

/// The shortest wait between polls, so that a suggestion of zero does not
/// turn into a busy loop.
const MIN_POLL_INTERVAL: Duration = Duration::from_millis(100);

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
    /// Longest a single tool call may take, task polling included.
    tool_timeout: Duration,
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
            tool_timeout: DEFAULT_TOOL_TIMEOUT,
        })
    }

    /// Bound every tool call, task polling included, to `timeout`.
    pub fn with_tool_timeout(mut self, timeout: Duration) -> Self {
        self.tool_timeout = timeout;
        self
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
    ///
    /// A server may answer with a task instead of a result. The task is polled
    /// until it ends and its outcome comes back like any other result: a task
    /// that failed, was cancelled or needs input yields an error result. The
    /// whole call, polling included, is bounded by the tool timeout; when it
    /// expires, or when this future is dropped, the task is cancelled.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult> {
        let mut params = CallToolRequestParams::new(name.to_string());
        if let Some(args) = arguments {
            params = params.with_arguments(args);
        }

        let call = async {
            match self.peer.call_tool_once(params).await? {
                CallToolResponse::Complete(result) => Ok(result),
                CallToolResponse::Task(created) => poll_task(&self.peer, created.task).await,
                _ => Ok(CallToolResult::error(vec![ContentBlock::text(
                    "the server asked for input, which this client cannot give",
                )])),
            }
        };
        match tokio::time::timeout(self.tool_timeout, call).await {
            Ok(result) => result.with_context(|| format!("tools/call '{name}' request failed")),
            Err(_) => anyhow::bail!(
                "tools/call '{name}' timed out after {}s",
                self.tool_timeout.as_secs()
            ),
        }
    }

    /// Whether the server advertises the Skills extension.
    pub fn supports_skills(&self) -> bool {
        self.peer
            .peer_info()
            .is_some_and(|info| info.capabilities.supports_skills())
    }

    /// Every skill the server lists (handles pagination).
    pub async fn list_skills(&self) -> Result<Vec<Skill>> {
        self.peer
            .list_all_skills()
            .await
            .context("skills/list request failed")
    }

    /// The bytes of the resource at `uri`: a text resource's UTF-8 or a
    /// binary one's decoded content.
    pub async fn read_resource_bytes(&self, uri: &str) -> Result<Vec<u8>> {
        let result = self
            .peer
            .read_resource(ReadResourceRequestParams::new(uri))
            .await
            .with_context(|| format!("resources/read '{uri}' request failed"))?;
        match result.contents.into_iter().next() {
            Some(ResourceContents::TextResourceContents { text, .. }) => Ok(text.into_bytes()),
            Some(ResourceContents::BlobResourceContents { blob, .. }) => {
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, blob)
                    .with_context(|| format!("'{uri}' is not valid base64"))
            }
            _ => anyhow::bail!("resources/read '{uri}' returned no content"),
        }
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

/// Poll `task` until it ends and turn its outcome into a tool result.
///
/// Dropping this future, which is how a cancelled turn or an expired timeout
/// stops it, cancels the task on the server.
async fn poll_task(peer: &Peer<RoleClient>, task: Task) -> Result<CallToolResult, ServiceError> {
    let mut guard = CancelOnDrop::new(peer.clone(), task.task_id.clone());
    if !task.status.is_terminal() {
        tokio::time::sleep(poll_interval(&task)).await;
    }
    loop {
        let detailed = peer
            .get_task(GetTaskParams::new(task.task_id.clone()))
            .await?
            .task;
        let failure = match detailed.payload {
            TaskPayload::Completed { result } => {
                guard.disarm();
                return serde_json::from_value(serde_json::Value::Object(result))
                    .map_err(|_| ServiceError::UnexpectedResponse);
            }
            TaskPayload::Failed { error } => {
                guard.disarm();
                error
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .map_or_else(|| "the task failed".to_string(), str::to_string)
            }
            TaskPayload::Cancelled => {
                guard.disarm();
                "the task was cancelled".to_string()
            }
            TaskPayload::InputRequired { .. } => {
                "the task needs input, which this client cannot give".to_string()
            }
            _ => {
                tokio::time::sleep(poll_interval(&detailed.task)).await;
                continue;
            }
        };
        return Ok(CallToolResult::error(vec![ContentBlock::text(failure)]));
    }
}

/// How long to wait before polling `task` again.
fn poll_interval(task: &Task) -> Duration {
    task.poll_interval_ms
        .map_or(DEFAULT_POLL_INTERVAL, Duration::from_millis)
        .clamp(MIN_POLL_INTERVAL, MAX_POLL_INTERVAL)
}

/// Sends `tasks/cancel` for a task when dropped, unless disarmed.
struct CancelOnDrop {
    peer: Peer<RoleClient>,
    task_id: String,
    armed: bool,
}

impl CancelOnDrop {
    fn new(peer: Peer<RoleClient>, task_id: String) -> Self {
        Self {
            peer,
            task_id,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let peer = self.peer.clone();
        let task_id = std::mem::take(&mut self.task_id);
        runtime.spawn(async move {
            if let Err(error) = peer.cancel_task(CancelTaskParams::new(&task_id)).await {
                debug!(%error, task_id, "could not cancel the task");
            }
        });
    }
}
