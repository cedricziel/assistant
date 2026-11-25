//! [`AssistantClientHandler`]: what this client tells every MCP server about
//! itself, and how it reacts to the server's notifications.
//!
//! It declares the Tasks, Skills and MCP Apps extensions, and nothing
//! interactive: no sampling, roots or elicitation. When a server announces
//! that its tools or prompts changed, it marks the connection's tool list
//! stale so the next turn lists again.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rmcp::ClientHandler;
use rmcp::model::{
    ClientCapabilities, ClientConfig, ExtensionCapabilities, Implementation, JsonObject,
    ProtocolVersion, SKILLS_EXTENSION_ID, TASKS_EXTENSION_ID,
};
use rmcp::service::{NotificationContext, RoleClient};
use serde_json::json;
use tokio::sync::Notify;

/// The MCP Apps extension.
pub const APPS_EXTENSION_ID: &str = "io.modelcontextprotocol/ui";

/// The MIME type of an MCP App's HTML.
pub const APPS_MIME_TYPE: &str = "text/html;profile=mcp-app";

/// Whether a connection's tool list may have changed since it was last read.
///
/// Clones share one flag. An optional [`Notify`] lets an owner of many
/// connections wake up when any of them is marked.
#[derive(Clone, Default)]
pub struct StaleTools {
    stale: Arc<AtomicBool>,
    wake: Option<Arc<Notify>>,
}

impl StaleTools {
    /// A flag that also wakes `wake` whenever it is marked.
    pub fn waking(wake: Arc<Notify>) -> Self {
        Self {
            stale: Arc::default(),
            wake: Some(wake),
        }
    }

    /// Mark the list stale and wake whoever waits for that.
    pub fn mark(&self) {
        self.stale.store(true, Ordering::SeqCst);
        if let Some(wake) = &self.wake {
            wake.notify_one();
        }
    }

    /// Whether the list is marked, without clearing the mark.
    pub fn is_marked(&self) -> bool {
        self.stale.load(Ordering::SeqCst)
    }

    /// Whether the list was marked since the last call; clears the mark.
    pub fn take(&self) -> bool {
        self.stale.swap(false, Ordering::SeqCst)
    }
}

/// The [`ClientHandler`] used for every MCP server connection.
#[derive(Clone, Default)]
pub struct AssistantClientHandler {
    stale: StaleTools,
}

impl AssistantClientHandler {
    pub fn new(stale: StaleTools) -> Self {
        Self { stale }
    }
}

/// The capabilities this client declares.
fn capabilities() -> ClientCapabilities {
    let mut apps = JsonObject::new();
    apps.insert("mimeTypes".into(), json!([APPS_MIME_TYPE]));
    let mut extensions = ExtensionCapabilities::new();
    extensions.insert(TASKS_EXTENSION_ID.into(), JsonObject::new());
    extensions.insert(SKILLS_EXTENSION_ID.into(), JsonObject::new());
    extensions.insert(APPS_EXTENSION_ID.into(), apps);
    let mut capabilities = ClientCapabilities::default();
    capabilities.extensions = Some(extensions);
    capabilities
}

impl ClientHandler for AssistantClientHandler {
    /// The `initialize` version is the newest one that has an `initialize`;
    /// servers on `2026-07-28` are reached through `server/discover` instead.
    fn get_info(&self) -> ClientConfig {
        ClientConfig::new(
            capabilities(),
            Implementation::new("assistant", env!("CARGO_PKG_VERSION")),
        )
        .with_protocol_version(ProtocolVersion::LATEST_WITH_INITIALIZE)
    }

    async fn on_tool_list_changed(&self, _context: NotificationContext<RoleClient>) {
        self.stale.mark();
    }

    async fn on_prompt_list_changed(&self, _context: NotificationContext<RoleClient>) {
        self.stale.mark();
    }
}
