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
pub mod analytics;
pub mod api_keys;
pub mod attachments;
pub mod audio;
pub mod bindings;
pub mod capabilities;
pub mod catalog;
pub mod commands;
pub mod conversations;
pub mod interfaces;
pub mod logs;
pub mod members;
pub mod messages;
pub mod orgs;
pub mod personas;
pub mod push;
pub mod skills;
pub mod spaces;
pub mod templates;
pub mod traces;
pub mod turns;
pub mod users;
pub mod webhooks;
pub mod workflows;

#[cfg(test)]
mod test_helpers;

// -- Shared helpers ----------------------------------------------------------

/// Convert any `Display`-able error into an Axum-compatible 500 response pair.
pub(crate) fn internal_error<E: std::fmt::Display>(err: E) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.to_string(),
    )
}

/// Serde deserializer that maps an absent **or** empty-string query parameter
/// to `None`, and parses non-empty strings via `FromStr`.
///
/// The generated Dart/Dio client sends `""` for every `null` optional query
/// parameter (e.g. `?limit=50&since=&until=`).  Without this helper, axum's
/// `Query` extractor rejects `since=` because `""` is not a valid
/// `DateTime<Utc>`, returning 400.  Applying this to all optional query-param
/// fields makes the server tolerate the generated client's serialisation.
pub(crate) fn empty_string_as_none<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let opt: Option<String> = serde::Deserialize::deserialize(d)?;
    match opt {
        None => Ok(None),
        Some(ref s) if s.is_empty() => Ok(None),
        Some(s) => s.parse::<T>().map(Some).map_err(serde::de::Error::custom),
    }
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

use assistant_core::ConversationConfig;
use assistant_runtime::{AssistantInterface, CommandRegistry, Orchestrator};
use assistant_storage::{
    AttachmentStore, CommandEventStore, ConversationBroadcast, ConversationEventStore,
    InMemoryConversationBroadcaster, RunBroadcaster, SqliteAttachmentStore,
    SqliteCommandEventStore, SqliteConversationEventStore,
};
use assistant_transcription::{TranscriptionProvider, TtsProvider};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::header,
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{delete, get, patch, post},
};
use sqlx::SqlitePool;
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

// -- State -------------------------------------------------------------------

/// Shared state for the conversation API handlers.
#[derive(Clone)]
pub struct ApiState {
    pub pool: SqlitePool,
    /// Shared live agent ID — updated when the user switches personas at runtime.
    pub agent_id: Arc<RwLock<String>>,
    pub orchestrator: Arc<dyn AssistantInterface>,
    /// Optional Web Push dispatcher — absent when VAPID keys are not configured.
    pub push_dispatcher: Option<Arc<crate::push::PushDispatcher>>,
    /// Optional transcription provider for voice message STT.
    pub transcription_provider: Option<Arc<dyn TranscriptionProvider>>,
    /// Optional TTS provider for voice playback synthesis.
    pub tts_provider: Option<Arc<dyn TtsProvider>>,
    /// In-memory store for tool-synthesized audio blobs.
    pub audio_store: Arc<crate::audio_store::AudioStore>,
    /// Durable event log store for conversation streaming runs.
    pub event_store: Arc<dyn ConversationEventStore>,
    /// In-memory broadcast registry for live-tailing active runs.
    pub run_broadcaster: RunBroadcaster,
    /// Persistent attachment storage (metadata in SQLite, bytes on disk).
    pub attachment_store: Arc<dyn AttachmentStore>,
    /// Slash-command registry (shared with all interfaces).
    pub command_registry: Arc<CommandRegistry>,
    /// Durable store for slash-command events.
    pub command_event_store: Arc<dyn CommandEventStore>,
    /// Conversation-list broadcaster for reactive SSE streaming.
    pub conversation_broadcaster: Arc<dyn ConversationBroadcast>,
    /// Per-conversation config overrides (model selection, etc.).
    pub conversation_configs: Arc<RwLock<HashMap<Uuid, ConversationConfig>>>,
    /// Concrete orchestrator reference for command execution.
    pub orchestrator_ref: Arc<Orchestrator>,
    /// Active turns map: conv_id → request_id (for `/stop`).
    pub active_turns: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    /// Default model name from global config.
    pub default_model: String,
}

impl ApiState {
    pub fn new(
        pool: SqlitePool,
        orchestrator: Arc<dyn AssistantInterface>,
        agent_id: Arc<RwLock<String>>,
        orchestrator_ref: Arc<Orchestrator>,
    ) -> Self {
        let event_store: Arc<dyn ConversationEventStore> =
            Arc::new(SqliteConversationEventStore::new(pool.clone()));
        let attachment_store: Arc<dyn AttachmentStore> =
            Arc::new(SqliteAttachmentStore::new(pool.clone()));
        let command_event_store: Arc<dyn CommandEventStore> =
            Arc::new(SqliteCommandEventStore::new(pool.clone()));
        let default_model = orchestrator_ref.llm.model_name().to_string();
        Self {
            pool,
            agent_id,
            orchestrator,
            push_dispatcher: None,
            transcription_provider: None,
            tts_provider: None,
            audio_store: Arc::new(crate::audio_store::AudioStore::new()),
            event_store,
            run_broadcaster: RunBroadcaster::new(),
            attachment_store,
            command_registry: Arc::new(CommandRegistry::new()),
            command_event_store,
            conversation_broadcaster: Arc::new(InMemoryConversationBroadcaster::new()),
            conversation_configs: Arc::new(RwLock::new(HashMap::new())),
            orchestrator_ref,
            active_turns: Arc::new(RwLock::new(HashMap::new())),
            default_model,
        }
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
}

// -- Router ------------------------------------------------------------------

/// Build the conversations API sub-router.  Mounted under `/api`.
pub fn api_router() -> Router<ApiState> {
    Router::new()
        .route("/capabilities", get(capabilities::get_capabilities))
        .route("/conversations", get(conversations::list_conversations))
        .route(
            "/conversations/stream",
            get(conversations::stream_conversations),
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

// -- Handlers ----------------------------------------------------------------
