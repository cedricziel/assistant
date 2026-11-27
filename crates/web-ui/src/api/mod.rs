//! REST JSON API for the assistant web UI.
//!
//! Sub-modules:
//! - `mod.rs` (this file): conversation management endpoints
//! - `personas.rs`: persona listing and active persona switching
//! - `traces.rs`: distributed trace retrieval
//! - `logs.rs`: log entry retrieval
//! - `skills.rs`: skill discovery per persona

pub mod account;
pub mod agents;
pub mod api_keys;
pub mod attachments;
pub mod audio;
pub mod bindings;
pub mod branch_turns;
pub mod branches;
pub mod capabilities;
pub mod catalog;
pub mod commands;
pub mod conversations;
pub mod feedback;
pub mod integrations;
pub mod interactions;
pub mod interfaces;
pub mod mcp_apps;
pub mod members;
pub mod messages;
pub mod models;
pub mod org_oidc;
pub mod orgs;
pub mod personas;
pub mod push;
pub mod skills;
pub mod spaces;
pub mod surfaces;
pub mod templates;
pub mod tool_permission_rules;
pub mod turns;
pub mod users;
pub mod webhooks;
pub mod workflows;

#[cfg(test)]
mod approval_flow_tests;
#[cfg(test)]
mod space_turn_tests;
#[cfg(test)]
mod tenancy_tests;
#[cfg(test)]
pub(crate) mod test_helpers;

// -- Shared helpers ----------------------------------------------------------

/// Convert any `Display`-able error into an Axum-compatible 500 response pair.
pub(crate) fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.to_string(),
    )
}

/// Build an SSE response with keepalive pings and `Cache-Control: no-cache`.
///
/// Contract: every SSE response from this crate MUST flow through this helper
/// (or apply the same `.keep_alive(KeepAlive::default())` directly if it
/// builds `Sse::new(...)` itself). The default emits a `:` comment line
/// every 15 seconds during periods of byte silence — this is the wire-level
/// liveness signal that prevents:
/// - Reverse-proxy idle timeouts from closing the connection (nginx default
///   60 s, AWS ALB default 60 s).
/// - The Flutter client's 90-second byte-level watchdog
///   (`withHeartbeatTimeout` in `app/lib/api/api_client.dart`) from
///   false-firing during slow tool calls.
///
/// `Cache-Control: no-cache` prevents intermediaries from buffering the
/// stream.
///
/// Contract lock: end-to-end behaviour is covered by
/// `crates/web-ui/tests/sse_keepalive_contract.rs`.
pub(crate) fn sse_response(rx: mpsc::Receiver<Result<Event, Infallible>>) -> Response {
    let mut response = Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
        .into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-cache"),
    );
    response
}

// -- Conversation API below --------------------------------------------------
//
// Designed for native/external clients (mobile apps, desktop apps, etc.)
// that need to manage conversations and send messages programmatically.
//
// All endpoints require `Authorization: Bearer <token>` (same token as the
// web UI).  Responses are `application/json`.  The streaming send endpoint
// uses `text/event-stream` (SSE).
//
// Routes:
//
// | Method   | Path                                  | Description                     |
// |----------|---------------------------------------|---------------------------------|
// | GET      | `/api/conversations`                  | List all conversations          |
// | POST     | `/api/conversations`                  | Create a new conversation       |
// | GET      | `/api/conversations/{id}`             | Get conversation + history      |
// | DELETE   | `/api/conversations/{id}`             | Delete a conversation           |
// | PATCH    | `/api/conversations/{id}`             | Update conversation title       |
// | POST     | `/api/conversations/{id}/messages`    | Send a message (SSE stream)     |

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use assistant_core::mcp_app::{McpAppServerChain, McpAppServers};
use assistant_runtime::{
    CommandRegistry, FeedbackTelemetrySink, OtelFeedbackTelemetrySink, SpaceRuntime, SpaceRuntimes,
};
use assistant_storage::{ConversationBroadcast, InMemoryConversationBroadcaster, RunBroadcaster};
use assistant_transcription::{TranscriptionProvider, TtsProvider};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, patch, post, put},
};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;

use crate::errors::json_error;
use crate::space::{SpaceCtx, SpaceResolver};

// -- State -------------------------------------------------------------------

/// Shared state for the conversation API handlers.
#[derive(Clone)]
pub struct ApiState {
    /// Resolves each request to the caller's space and its storage.
    pub spaces: SpaceResolver,
    /// Shared live agent ID — updated when the user switches personas at runtime.
    pub agent_id: Arc<RwLock<String>>,
    /// Per-space orchestrators; a turn runs on the runtime of the caller's space.
    pub runtimes: Arc<SpaceRuntimes>,
    /// Optional Web Push dispatcher — absent when VAPID keys are not configured.
    pub push_dispatcher: Option<Arc<crate::push::PushDispatcher>>,
    /// Optional transcription provider for voice message STT.
    pub transcription_provider: Option<Arc<dyn TranscriptionProvider>>,
    /// Optional TTS provider for voice playback synthesis.
    pub tts_provider: Option<Arc<dyn TtsProvider>>,
    /// In-memory store for tool-synthesized audio blobs.
    pub audio_store: Arc<crate::audio_store::AudioStore>,
    /// In-memory broadcast registry for live-tailing active runs.
    pub run_broadcaster: RunBroadcaster,
    /// Slash-command registry (shared with all interfaces).
    pub command_registry: Arc<CommandRegistry>,
    /// Per-space conversation-list broadcasters for reactive SSE streaming.
    pub conversation_broadcasters: ConversationBroadcasters,
    /// Receives an event for every stored or cleared message vote.
    pub feedback_telemetry: Arc<dyn FeedbackTelemetrySink>,
    /// Where MCP Apps' calls go: the servers their tools came from.
    pub mcp_apps: Arc<dyn McpAppServers>,
}

impl ApiState {
    pub fn new(
        spaces: SpaceResolver,
        runtimes: Arc<SpaceRuntimes>,
        agent_id: Arc<RwLock<String>>,
    ) -> Self {
        Self {
            spaces,
            agent_id,
            runtimes,
            push_dispatcher: None,
            transcription_provider: None,
            tts_provider: None,
            audio_store: Arc::new(crate::audio_store::AudioStore::new()),
            run_broadcaster: RunBroadcaster::new(),
            command_registry: Arc::new(CommandRegistry::new()),
            conversation_broadcasters: ConversationBroadcasters::default(),
            feedback_telemetry: Arc::new(OtelFeedbackTelemetrySink::new()),
            mcp_apps: Arc::new(McpAppServerChain::default()),
        }
    }

    /// The runtime of the space `ctx` is bound to.
    pub(crate) async fn runtime(&self, ctx: &SpaceCtx) -> Result<Arc<SpaceRuntime>, Response> {
        self.runtimes
            .get(&ctx.org_slug, &ctx.space_slug)
            .await
            .map_err(|e| {
                tracing::error!("space runtime unavailable: {e:#}");
                json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "space runtime unavailable",
                )
            })
    }

    pub fn with_push_dispatcher(mut self, dispatcher: Arc<crate::push::PushDispatcher>) -> Self {
        self.push_dispatcher = Some(dispatcher);
        self
    }

    pub fn with_transcription_provider(mut self, provider: Arc<dyn TranscriptionProvider>) -> Self {
        self.transcription_provider = Some(provider);
        self
    }

    pub fn with_tts_provider(mut self, provider: Arc<dyn TtsProvider>) -> Self {
        self.tts_provider = Some(provider);
        self
    }

    pub fn with_mcp_apps(mut self, servers: Arc<dyn McpAppServers>) -> Self {
        self.mcp_apps = servers;
        self
    }
}

/// Conversation-list broadcasters, one per `(org, space)`, so a list stream
/// only ever sees events from the space it was opened in.
#[derive(Clone, Default)]
pub struct ConversationBroadcasters {
    by_space: Arc<std::sync::Mutex<HashMap<SpaceKey, Arc<dyn ConversationBroadcast>>>>,
}

/// `(org slug, space slug)`.
type SpaceKey = (String, String);

impl ConversationBroadcasters {
    /// The broadcaster for one space, created on first use.
    pub fn for_space(&self, org_slug: &str, space_slug: &str) -> Arc<dyn ConversationBroadcast> {
        let mut by_space = self
            .by_space
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        by_space
            .entry((org_slug.to_owned(), space_slug.to_owned()))
            .or_insert_with(|| Arc::new(InMemoryConversationBroadcaster::new()))
            .clone()
    }
}

impl axum::extract::FromRef<ApiState> for SpaceResolver {
    fn from_ref(state: &ApiState) -> Self {
        state.spaces.clone()
    }
}

// -- Router ------------------------------------------------------------------

/// Build the conversations API sub-router.  Mounted under `/api`.
pub fn api_router() -> Router<ApiState> {
    Router::new()
        .route("/capabilities", get(capabilities::get_capabilities))
        .route("/models", get(models::list_models))
        .route("/conversations", get(conversations::list_conversations))
        .route(
            "/conversations/stream",
            get(conversations::stream_conversations),
        )
        .route(
            "/conversations/search",
            get(conversations::search_conversations),
        )
        .route("/conversations", post(conversations::create_conversation))
        .route("/conversations/{id}", get(conversations::get_conversation))
        .route(
            "/conversations/{id}",
            delete(conversations::delete_conversation),
        )
        .route(
            "/conversations/{id}",
            patch(conversations::update_conversation),
        )
        .route("/conversations/{id}/messages", post(messages::send_message))
        .route(
            "/conversations/{id}/messages/{message_id}/edit",
            post(branch_turns::edit_message),
        )
        .route(
            "/conversations/{id}/messages/{message_id}/regenerate",
            post(branch_turns::regenerate_message),
        )
        .route(
            "/conversations/{id}/switch-branch",
            post(branches::switch_conversation_branch),
        )
        .route(
            "/conversations/{id}/messages/{message_id}/feedback",
            put(feedback::set_message_feedback).delete(feedback::delete_message_feedback),
        )
        .route(
            "/conversations/{id}/voice",
            post(messages::send_voice_message),
        )
        .route("/quick-message", post(messages::quick_message))
        .route(
            "/conversations/{id}/attachments",
            post(attachments::upload_attachment).layer(DefaultBodyLimit::max(27 * 1024 * 1024)), // 27 MB
        )
        .route("/attachments/{id}", get(attachments::serve_attachment))
        .route(
            "/conversations/{id}/runs/{run_id}/events/stream",
            get(messages::stream_run_events),
        )
        .route(
            "/conversations/{conversation_id}/turns/{turn_id}/status",
            get(turns::get_turn_status),
        )
        .route(
            "/conversations/{conversation_id}/turns/{turn_id}/cancel",
            post(turns::cancel_turn),
        )
        .merge(tool_permission_rules::tool_permission_rules_router())
        .route(
            "/conversations/{conversation_id}/interactions",
            get(interactions::list_conversation_interactions),
        )
        .route(
            "/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond",
            post(interactions::respond_to_interaction)
                .layer(DefaultBodyLimit::max(interactions::MAX_RESPOND_BODY_BYTES)),
        )
        .route(
            "/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call",
            post(mcp_apps::call_mcp_app_tool)
                .layer(DefaultBodyLimit::max(mcp_apps::MAX_APP_REQUEST_BODY_BYTES)),
        )
        .route(
            "/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read",
            post(mcp_apps::read_mcp_app_resource)
                .layer(DefaultBodyLimit::max(mcp_apps::MAX_APP_REQUEST_BODY_BYTES)),
        )
        .route("/messages/{id}/audio", get(audio::get_message_audio))
        .route("/audio/{id}", get(audio::get_audio))
        .route("/commands", get(commands::list_commands))
        .route(
            "/conversations/{id}/command",
            post(commands::execute_command),
        )
        .route(
            "/conversations/{id}/events",
            get(commands::list_command_events),
        )
}

#[cfg(test)]
mod helper_tests {
    use super::internal_error;
    use axum::http::StatusCode;

    #[test]
    fn internal_error_returns_500_with_display_string() {
        let (status, body) = internal_error("boom");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, "boom");
    }

    #[test]
    fn internal_error_works_with_anyhow_error() {
        let err = anyhow::anyhow!("disk full");
        let (status, body) = internal_error(&err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(body.contains("disk full"));
    }
}

// -- Handlers ----------------------------------------------------------------
