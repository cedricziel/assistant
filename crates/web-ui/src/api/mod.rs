//! REST JSON API for the assistant web UI.
//!
//! Sub-modules:
//! - `mod.rs` (this file): conversation management endpoints
//! - `personas.rs`: persona listing and active persona switching
//! - `traces.rs`: distributed trace retrieval
//! - `logs.rs`: log entry retrieval
//! - `skills.rs`: skill discovery per persona

pub mod agents;
pub mod analytics;
pub mod logs;
pub mod personas;
pub mod push;
pub mod skills;
pub mod traces;
pub mod webhooks;
pub mod workflows;

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

use std::convert::Infallible;
use std::sync::Arc;

use assistant_transcription::{
    TranscriptionProvider, TranscriptionRequest, TtsProvider, TtsRequest,
};

use assistant_core::{Interface, MessageRole};
use assistant_runtime::{AssistantInterface, OrchestratorEvent};
use assistant_storage::ConversationStore;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;
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
}

impl ApiState {
    pub fn new(
        pool: SqlitePool,
        orchestrator: Arc<dyn AssistantInterface>,
        agent_id: Arc<RwLock<String>>,
    ) -> Self {
        Self {
            pool,
            agent_id,
            orchestrator,
            push_dispatcher: None,
            transcription_provider: None,
            tts_provider: None,
            audio_store: Arc::new(crate::audio_store::AudioStore::new()),
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

// -- Response types ----------------------------------------------------------

/// A conversation summary (no messages).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConversationSummary {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A conversation with its full message history.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConversationDetail {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<MessageSummary>,
}

/// A single message in a conversation.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MessageSummary {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub turn: i64,
    pub created_at: DateTime<Utc>,
    /// Tool names called in this message (present when `role == "assistant"`
    /// and the message contains tool invocations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<String>>,
    /// Name of the tool or skill that produced this result (present when
    /// `role == "tool"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/conversations`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateConversationRequest {
    /// Optional title for the new conversation.
    pub title: Option<String>,
}

/// Body for `PATCH /api/conversations/{id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateConversationRequest {
    /// New title for the conversation.
    pub title: String,
}

/// Body for `POST /api/conversations/{id}/messages`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = ApiSendMessageRequest)]
pub struct SendMessageRequest {
    /// The message text to send to the assistant.
    pub message: String,
}

// -- Router ------------------------------------------------------------------

/// Build the conversations API sub-router.  Mounted under `/api`.
pub fn api_router() -> Router<ApiState> {
    Router::new()
        .route("/capabilities", get(get_capabilities))
        .route("/conversations", get(list_conversations))
        .route("/conversations", post(create_conversation))
        .route("/conversations/{id}", get(get_conversation))
        .route("/conversations/{id}", delete(delete_conversation))
        .route("/conversations/{id}", patch(update_conversation))
        .route("/conversations/{id}/messages", post(send_message))
        .route("/conversations/{id}/voice", post(send_voice_message))
        .route("/messages/{id}/audio", get(get_message_audio))
        .route("/audio/{id}", get(get_audio))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/conversations` — list all conversations, newest first.
#[utoipa::path(
    get,
    path = "/api/conversations",
    tag = "conversations",
    responses(
        (status = 200, description = "List of conversations", body = Vec<ConversationSummary>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_conversations(State(state): State<ApiState>) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool, &agent_id);
    match store.list_conversations().await {
        Ok(convs) => {
            let summaries: Vec<ConversationSummary> = convs
                .into_iter()
                .map(|c| ConversationSummary {
                    id: c.id,
                    title: c.title.unwrap_or_else(|| "Untitled".into()),
                    created_at: c.created_at,
                    updated_at: c.updated_at,
                })
                .collect();
            Json(summaries).into_response()
        }
        Err(e) => {
            warn!("Failed to list conversations: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list conversations",
            )
                .into_response()
        }
    }
}

/// `POST /api/conversations` — create a new conversation.
#[utoipa::path(
    post,
    path = "/api/conversations",
    tag = "conversations",
    request_body = CreateConversationRequest,
    responses(
        (status = 201, description = "Created conversation", body = ConversationSummary),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn create_conversation(
    State(state): State<ApiState>,
    Json(body): Json<CreateConversationRequest>,
) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool, &agent_id);
    let title = body.title.as_deref().unwrap_or("New Chat");
    match store.create_conversation(Some(title)).await {
        Ok(c) => (
            StatusCode::CREATED,
            Json(ConversationSummary {
                id: c.id,
                title: c.title.unwrap_or_else(|| "Untitled".into()),
                created_at: c.created_at,
                updated_at: c.updated_at,
            }),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to create conversation: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create conversation",
            )
                .into_response()
        }
    }
}

/// `GET /api/conversations/{id}` — get a conversation and its message history.
#[utoipa::path(
    get,
    path = "/api/conversations/{id}",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    responses(
        (status = 200, description = "Conversation with messages", body = ConversationDetail),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_conversation(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool, &agent_id);

    let conv = match store.get_conversation(conv_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return (StatusCode::NOT_FOUND, "Conversation not found").into_response(),
        Err(e) => {
            warn!("Failed to get conversation: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let history = store.load_history(conv_id).await.unwrap_or_default();
    let messages = history
        .into_iter()
        .filter(|m| !matches!(m.role, MessageRole::System | MessageRole::Tool))
        .map(|m| {
            let tool_calls = m.tool_calls_json.as_deref().and_then(|json| {
                serde_json::from_str::<Vec<serde_json::Value>>(json)
                    .ok()
                    .map(|items| {
                        items
                            .into_iter()
                            .filter_map(|v| {
                                v.get("name").and_then(|n| n.as_str()).map(str::to_string)
                            })
                            .collect::<Vec<_>>()
                    })
                    .filter(|v: &Vec<_>| !v.is_empty())
            });
            MessageSummary {
                id: m.id,
                role: m.role.to_string(),
                content: m.content,
                turn: m.turn,
                created_at: m.created_at,
                tool_calls,
                skill_name: m.skill_name,
            }
        })
        .collect();

    Json(ConversationDetail {
        id: conv.id,
        title: conv.title.unwrap_or_else(|| "Untitled".into()),
        created_at: conv.created_at,
        updated_at: conv.updated_at,
        messages,
    })
    .into_response()
}

/// `DELETE /api/conversations/{id}` — delete a conversation and all its messages.
#[utoipa::path(
    delete,
    path = "/api/conversations/{id}",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn delete_conversation(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool, &agent_id);
    match store.delete_conversation(conv_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            warn!("Failed to delete conversation: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete conversation",
            )
                .into_response()
        }
    }
}

/// `PATCH /api/conversations/{id}` — update a conversation's title.
#[utoipa::path(
    patch,
    path = "/api/conversations/{id}",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    request_body = UpdateConversationRequest,
    responses(
        (status = 200, description = "Updated conversation", body = ConversationSummary),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn update_conversation(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateConversationRequest>,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool, &agent_id);
    match store.update_title(conv_id, &body.title).await {
        Ok(()) => {}
        Err(e) if e.to_string().contains("not found") => {
            return (StatusCode::NOT_FOUND, "Conversation not found").into_response();
        }
        Err(e) => {
            warn!("Failed to update conversation title: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update conversation",
            )
                .into_response();
        }
    }

    match store.get_conversation(conv_id).await {
        Ok(Some(c)) => Json(ConversationSummary {
            id: c.id,
            title: c.title.unwrap_or_else(|| "Untitled".into()),
            created_at: c.created_at,
            updated_at: c.updated_at,
        })
        .into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Conversation not found").into_response(),
        Err(e) => {
            warn!("Failed to fetch updated conversation: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// `POST /api/conversations/{id}/messages` — send a message and stream the response.
///
/// The response is a `text/event-stream` (SSE) with two event types:
/// - `event: token` — incremental assistant token (data is plain text)
/// - `event: done`  — final JSON object: `{"role":"assistant","content":"..."}`
#[utoipa::path(
    post,
    path = "/api/conversations/{id}/messages",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "SSE stream of assistant tokens and final message",
         content_type = "text/event-stream"),
        (status = 400, description = "Invalid ID or empty message"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Conversation not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn send_message(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let content = body.message.trim().to_string();
    if content.is_empty() {
        return (StatusCode::BAD_REQUEST, "Message cannot be empty").into_response();
    }

    // Verify the conversation exists before streaming.
    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool.clone(), &agent_id);

    match store.get_conversation(conv_id).await {
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Conversation not found").into_response();
        }
        Err(e) => {
            warn!("Failed to check conversation {conv_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
        _ => {}
    }

    // Auto-title on the first user message (empty history).
    match store.load_history(conv_id).await {
        Ok(prior) if prior.is_empty() => {
            let title = if content.chars().count() > 60 {
                format!("{}...", content.chars().take(57).collect::<String>())
            } else {
                content.clone()
            };
            if let Err(e) = store.update_title(conv_id, &title).await {
                warn!("Auto-title failed for {conv_id}: {e}");
            }
        }
        Err(e) => {
            warn!("Failed to load history for {conv_id}: {e}");
        }
        _ => {}
    }

    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    let (event_tx, mut event_rx) = mpsc::channel::<OrchestratorEvent>(64);

    state
        .orchestrator
        .register_token_sink(conv_id, event_tx)
        .await;

    let orchestrator = state.orchestrator.clone();
    let turn_result_rx = {
        let (tx, rx) =
            tokio::sync::oneshot::channel::<anyhow::Result<assistant_runtime::TurnResult>>();
        tokio::spawn(async move {
            let result = orchestrator
                .submit_turn(&content, conv_id, Interface::Web, None)
                .await;
            let _ = tx.send(result);
        });
        rx
    };

    let push_dispatcher_for_sse = state.push_dispatcher.clone();
    let conv_id_for_push = conv_id;
    tokio::spawn(async move {
        let mut full_text = String::new();

        while let Some(orch_event) = event_rx.recv().await {
            let sse_event = match orch_event {
                OrchestratorEvent::Token(token) => {
                    full_text.push_str(&token);
                    Event::default().event("token").data(token)
                }
                OrchestratorEvent::Status(msg) => Event::default().event("status").data(msg),
                OrchestratorEvent::ToolResult { tool_name, status } => {
                    let data = serde_json::json!({
                        "tool_name": tool_name,
                        "status": status,
                    });
                    Event::default().event("tool_result").data(data.to_string())
                }
                OrchestratorEvent::SkillComplete {
                    skill_name,
                    success,
                    summary,
                } => {
                    // Fire push notification for skill completion (task 4.4).
                    if let Some(ref dispatcher) = push_dispatcher_for_sse {
                        let title = if success {
                            "Skill complete".to_string()
                        } else {
                            "Skill failed".to_string()
                        };
                        let body = format!("{skill_name}: {summary}");
                        let conv_id_str = conv_id_for_push.to_string();
                        let d = dispatcher.clone();
                        tokio::spawn(async move {
                            if let Err(e) = d.send_to_all(&title, &body, Some(&conv_id_str)).await {
                                warn!("Push (skill) failed: {e}");
                            }
                        });
                    }
                    let data = serde_json::json!({
                        "skill_name": skill_name,
                        "success": success,
                        "summary": summary,
                    });
                    Event::default()
                        .event("skill_complete")
                        .data(data.to_string())
                }
                OrchestratorEvent::AgentError { message } => {
                    // Fire push notification for agent critical error (task 4.5).
                    if let Some(ref dispatcher) = push_dispatcher_for_sse {
                        let body = message.chars().take(80).collect::<String>();
                        let conv_id_str = conv_id_for_push.to_string();
                        let d = dispatcher.clone();
                        tokio::spawn(async move {
                            if let Err(e) = d
                                .send_to_all("Assistant error", &body, Some(&conv_id_str))
                                .await
                            {
                                warn!("Push (agent error) failed: {e}");
                            }
                        });
                    }
                    Event::default().event("agent_error").data(message)
                }
                OrchestratorEvent::AudioReady { audio_id } => {
                    let data = serde_json::json!({
                        "audio_id": audio_id,
                        "auto_play": true,
                    });
                    Event::default().event("audio_ready").data(data.to_string())
                }
            };
            if sse_tx.send(Ok(sse_event)).await.is_err() {
                return;
            }
        }

        let (reply_text, reply_message_id) = match turn_result_rx.await {
            Ok(Ok(result)) => (result.answer, result.message_id),
            Ok(Err(_)) | Err(_) => (full_text, None),
        };

        let mut done_data = serde_json::json!({
            "role": "assistant",
            "content": reply_text,
        });
        if let Some(mid) = reply_message_id {
            done_data["message_id"] = serde_json::Value::String(mid.to_string());
        }
        let done = Event::default().event("done").data(done_data.to_string());
        let _ = sse_tx.send(Ok(done)).await;

        // Fire Web Push notification after emitting the SSE done event.
        if let Some(dispatcher) = push_dispatcher_for_sse {
            let body = reply_text.chars().take(80).collect::<String>();
            let conv_id_str = conv_id_for_push.to_string();
            if let Err(e) = dispatcher
                .send_to_all("New message", &body, Some(&conv_id_str))
                .await
            {
                warn!("Push dispatch failed: {e}");
            }
        }
    });

    Sse::new(ReceiverStream::new(sse_rx)).into_response()
}

// -- Voice / capabilities handlers ------------------------------------------

/// Server capability flags.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ServerCapabilities {
    /// Whether the server can accept voice messages (STT configured).
    pub voice_send: bool,
    /// Whether the server can serve TTS audio for messages (TTS configured).
    pub voice_receive: bool,
}

/// Multipart form body for `POST /api/conversations/{id}/voice`.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
struct VoiceUploadForm {
    /// Raw audio bytes (opus/aac/webm/wav …).
    #[schema(format = Binary, content_encoding = "binary")]
    audio: Vec<u8>,
}

/// `GET /api/capabilities` — return server capability flags.
#[utoipa::path(
    get,
    path = "/api/capabilities",
    tag = "capabilities",
    responses(
        (status = 200, description = "Server capabilities", body = ServerCapabilities),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_capabilities(State(state): State<ApiState>) -> Response {
    Json(ServerCapabilities {
        voice_send: state.transcription_provider.is_some(),
        voice_receive: state.tts_provider.is_some(),
    })
    .into_response()
}

/// `POST /api/conversations/{id}/voice` — upload audio, transcribe it, run
/// through the orchestrator, and stream the response as SSE.
#[utoipa::path(
    post,
    path = "/api/conversations/{id}/voice",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    request_body(
        content_type = "multipart/form-data",
        content = inline(VoiceUploadForm),
    ),
    responses(
        (status = 200, description = "SSE stream of assistant response", content_type = "text/event-stream"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "Voice not configured"),
    ),
    security(("bearer_token" = []))
)]
pub async fn send_voice_message(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let transcription_provider = match &state.transcription_provider {
        Some(p) => p.clone(),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Voice transcription not configured",
            )
                .into_response();
        }
    };

    // Parse the multipart body — expect a single "audio" field.
    let mut audio_bytes: Option<Vec<u8>> = None;
    let mut mime_type = "audio/webm".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("audio") {
            if let Some(ct) = field.content_type() {
                mime_type = ct.to_string();
            }
            match field.bytes().await {
                Ok(b) => {
                    // Enforce 25 MB limit.
                    if b.len() > 25 * 1024 * 1024 {
                        return (StatusCode::BAD_REQUEST, "Audio too large (max 25 MB)")
                            .into_response();
                    }
                    audio_bytes = Some(b.to_vec());
                }
                Err(e) => {
                    warn!("Failed to read audio field: {e}");
                    return (StatusCode::BAD_REQUEST, "Failed to read audio data").into_response();
                }
            }
        }
    }

    let audio_bytes = match audio_bytes {
        Some(b) if !b.is_empty() => b,
        _ => return (StatusCode::BAD_REQUEST, "Missing audio field").into_response(),
    };

    // Validate MIME type — must be audio/*.
    if !mime_type.starts_with("audio/") {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid MIME type — expected audio/*",
        )
            .into_response();
    }

    // Verify the conversation exists.
    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool.clone(), &agent_id);
    match store.get_conversation(conv_id).await {
        Ok(None) => return (StatusCode::NOT_FOUND, "Conversation not found").into_response(),
        Err(e) => {
            warn!("Failed to check conversation {conv_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
        _ => {}
    }

    // Transcribe.
    let transcript = match transcription_provider
        .transcribe(TranscriptionRequest {
            audio_data: audio_bytes,
            mime_type: mime_type.clone(),
            filename: None,
            language: None,
        })
        .await
    {
        Ok(r) => r.text,
        Err(e) => {
            warn!("Transcription failed: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Transcription failed").into_response();
        }
    };

    if transcript.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Transcription returned empty text").into_response();
    }

    // Run through orchestrator (same flow as send_message).
    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    let (event_tx, mut event_rx) = mpsc::channel::<OrchestratorEvent>(64);

    state
        .orchestrator
        .register_token_sink(conv_id, event_tx)
        .await;

    let orchestrator = state.orchestrator.clone();
    let content = transcript.clone();
    let turn_result_rx = {
        let (tx, rx) =
            tokio::sync::oneshot::channel::<anyhow::Result<assistant_runtime::TurnResult>>();
        tokio::spawn(async move {
            let result = orchestrator
                .submit_turn(&content, conv_id, Interface::Web, None)
                .await;
            let _ = tx.send(result);
        });
        rx
    };

    // Emit the user transcript as the first SSE event so the UI can display it.
    let transcript_event = serde_json::json!({ "role": "user", "content": transcript });
    let _ = sse_tx
        .send(Ok(Event::default()
            .event("transcript")
            .data(transcript_event.to_string())))
        .await;

    let push_dispatcher_for_sse = state.push_dispatcher.clone();
    let conv_id_for_push = conv_id;
    tokio::spawn(async move {
        let mut full_text = String::new();

        while let Some(orch_event) = event_rx.recv().await {
            let sse_event = match orch_event {
                OrchestratorEvent::Token(token) => {
                    full_text.push_str(&token);
                    Event::default().event("token").data(token)
                }
                OrchestratorEvent::Status(msg) => Event::default().event("status").data(msg),
                OrchestratorEvent::ToolResult { tool_name, status } => {
                    let data = serde_json::json!({
                        "tool_name": tool_name,
                        "status": status,
                    });
                    Event::default().event("tool_result").data(data.to_string())
                }
                OrchestratorEvent::SkillComplete {
                    skill_name,
                    success,
                    summary,
                } => {
                    let data = serde_json::json!({
                        "skill_name": skill_name,
                        "success": success,
                        "summary": summary,
                    });
                    Event::default()
                        .event("skill_complete")
                        .data(data.to_string())
                }
                OrchestratorEvent::AgentError { message } => {
                    Event::default().event("agent_error").data(message)
                }
                OrchestratorEvent::AudioReady { audio_id } => {
                    let data = serde_json::json!({
                        "audio_id": audio_id,
                        "auto_play": true,
                    });
                    Event::default().event("audio_ready").data(data.to_string())
                }
            };
            if sse_tx.send(Ok(sse_event)).await.is_err() {
                return;
            }
        }

        let (reply_text, reply_message_id) = match turn_result_rx.await {
            Ok(Ok(result)) => (result.answer, result.message_id),
            Ok(Err(_)) | Err(_) => (full_text, None),
        };

        let mut done_data = serde_json::json!({
            "role": "assistant",
            "content": reply_text,
        });
        if let Some(mid) = reply_message_id {
            done_data["message_id"] = serde_json::Value::String(mid.to_string());
        }
        let done = Event::default().event("done").data(done_data.to_string());
        let _ = sse_tx.send(Ok(done)).await;

        if let Some(dispatcher) = push_dispatcher_for_sse {
            let body = reply_text.chars().take(80).collect::<String>();
            let conv_id_str = conv_id_for_push.to_string();
            if let Err(e) = dispatcher
                .send_to_all("New message", &body, Some(&conv_id_str))
                .await
            {
                warn!("Push dispatch failed: {e}");
            }
        }
    });

    Sse::new(ReceiverStream::new(sse_rx)).into_response()
}

/// `GET /api/messages/{id}/audio` — synthesize TTS audio for an assistant
/// message and return it as `audio/mpeg`.
#[utoipa::path(
    get,
    path = "/api/messages/{id}/audio",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Message ID")),
    responses(
        (status = 200, description = "MP3 audio bytes", content_type = "audio/mpeg"),
        (status = 400, description = "Invalid ID or non-assistant message"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Message not found"),
        (status = 503, description = "TTS not configured"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_message_audio(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    let tts_provider = match &state.tts_provider {
        Some(p) => p.clone(),
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "TTS not configured").into_response();
        }
    };

    let msg_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid message ID").into_response(),
    };

    let agent_id = state.agent_id.read().await.clone();
    let store = ConversationStore::for_agent(state.pool.clone(), &agent_id);

    let msg = match store.get_message(msg_id).await {
        Ok(Some(m)) => m,
        Ok(None) => return (StatusCode::NOT_FOUND, "Message not found").into_response(),
        Err(e) => {
            warn!("Failed to fetch message {msg_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    if !matches!(msg.role, MessageRole::Assistant) {
        return (
            StatusCode::BAD_REQUEST,
            "Only assistant messages can be synthesized",
        )
            .into_response();
    }

    match tts_provider
        .synthesize(TtsRequest {
            text: msg.content,
            voice: None,
            format: None,
            speed: None,
        })
        .await
    {
        Ok(result) => (
            [(header::CONTENT_TYPE, result.mime_type)],
            Body::from(result.audio_data),
        )
            .into_response(),
        Err(e) => {
            warn!("TTS synthesis failed for message {msg_id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "TTS synthesis failed").into_response()
        }
    }
}

/// `GET /api/audio/{id}` — serve a synthesized audio blob from the in-memory store.
#[utoipa::path(
    get,
    path = "/api/audio/{id}",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Audio blob ID")),
    responses(
        (status = 200, description = "MP3 audio bytes", content_type = "audio/mpeg"),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Audio not found or expired"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_audio(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    let audio_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid audio ID").into_response(),
    };

    match state.audio_store.get(audio_id).await {
        Some((bytes, mime)) => ([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response(),
        None => (StatusCode::NOT_FOUND, "Audio not found or expired").into_response(),
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tokio::sync::RwLock;
    use tower::ServiceExt;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::AssistantConfig;
    use assistant_llm::{LlmClient, LlmClientConfig, RetryConfig};
    use assistant_runtime::Orchestrator;
    use assistant_storage::{ConversationStore, SkillRegistry, StorageLayer};
    use assistant_tool_executor::ToolExecutor;
    use assistant_transcription::{
        TranscriptionProvider, TranscriptionRequest, TranscriptionResult,
    };
    use async_trait::async_trait;

    use super::{api_router, ApiState};

    // -- Stubs -----------------------------------------------------------------

    /// Stub transcription provider that returns a fixed transcript.
    struct StubTranscriptionProvider {
        transcript: String,
    }

    #[async_trait]
    impl TranscriptionProvider for StubTranscriptionProvider {
        fn name(&self) -> &str {
            "stub"
        }

        async fn transcribe(
            &self,
            _request: TranscriptionRequest,
        ) -> anyhow::Result<TranscriptionResult> {
            Ok(TranscriptionResult {
                text: self.transcript.clone(),
                language: None,
                duration_secs: None,
            })
        }
    }

    // -- Helpers ---------------------------------------------------------------

    /// Minimal LLM mock: returns a static assistant reply.
    async fn mount_llm_reply(server: &MockServer, reply: &str) {
        let body = serde_json::json!({
            "model": "test",
            "message": { "role": "assistant", "content": reply },
            "done": true
        });
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(server)
            .await;
    }

    /// Build an `ApiState` wired to an in-memory DB and a mock LLM server.
    async fn test_state(llm_url: &str) -> (ApiState, Arc<StorageLayer>) {
        let mut config = AssistantConfig::default();
        config.memory.enabled = false;

        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: llm_url.to_string(),
                timeout_secs: 5,
                retry_config: RetryConfig::disabled(),
            })
            .unwrap(),
        );
        let executor = Arc::new(ToolExecutor::new(
            storage.clone(),
            llm.clone(),
            registry.clone(),
            Arc::new(config.clone()),
        ));
        let bus = Arc::new(storage.message_bus());
        let orchestrator = Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            executor,
            registry,
            bus,
            &config,
        ));

        // Spawn the turn-processing worker so submit_turn requests are handled.
        let worker_orch = orchestrator.clone();
        tokio::spawn(async move {
            worker_orch.run_worker("test-worker").await;
        });

        let state = ApiState {
            pool: storage.pool.clone(),
            agent_id: Arc::new(RwLock::new("default".to_string())),
            orchestrator,
            push_dispatcher: None,
            transcription_provider: None,
            tts_provider: None,
            audio_store: Arc::new(crate::audio_store::AudioStore::new()),
        };
        (state, storage)
    }

    fn app(state: ApiState) -> axum::Router {
        api_router().with_state(state)
    }

    async fn body_bytes(body: Body) -> Vec<u8> {
        body.collect().await.unwrap().to_bytes().to_vec()
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body_bytes(body).await;
        serde_json::from_slice(&b).unwrap()
    }

    // -- GET /conversations ----------------------------------------------------

    #[tokio::test]
    async fn list_conversations_empty() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/conversations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn list_conversations_returns_created_items() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        store.create_conversation(Some("Alpha")).await.unwrap();
        store.create_conversation(Some("Beta")).await.unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/conversations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        let titles: Vec<&str> = arr.iter().map(|c| c["title"].as_str().unwrap()).collect();
        assert!(titles.contains(&"Alpha"));
        assert!(titles.contains(&"Beta"));
    }

    // -- POST /conversations ---------------------------------------------------

    #[tokio::test]
    async fn create_conversation_returns_201_with_id() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"title":"My Chat"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["title"], "My Chat");
        assert!(json["id"].as_str().is_some(), "should have an id");
    }

    #[tokio::test]
    async fn create_conversation_without_title_uses_default() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["title"], "New Chat");
    }

    // -- GET /conversations/{id} -----------------------------------------------

    #[tokio::test]
    async fn get_conversation_returns_detail_with_empty_messages() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("Test")).await.unwrap();
        let id = conv.id;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["id"], id.to_string());
        assert_eq!(json["title"], "Test");
        assert_eq!(json["messages"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn get_conversation_unknown_id_returns_404() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let id = uuid::Uuid::new_v4();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_conversation_bad_uuid_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/conversations/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // -- DELETE /conversations/{id} --------------------------------------------

    #[tokio::test]
    async fn delete_conversation_returns_204() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("Bye")).await.unwrap();
        let id = conv.id;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/conversations/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        // Verify it is gone.
        let gone = store.get_conversation(id).await.unwrap();
        assert!(gone.is_none(), "conversation should be deleted");
    }

    // -- PATCH /conversations/{id} ---------------------------------------------

    #[tokio::test]
    async fn patch_conversation_renames_it() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("Old Name")).await.unwrap();
        let id = conv.id;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/conversations/{id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"title":"New Name"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["title"], "New Name");

        let updated = store.get_conversation(id).await.unwrap().unwrap();
        assert_eq!(updated.title.as_deref(), Some("New Name"));
    }

    // -- POST /conversations/{id}/messages — error paths ----------------------

    #[tokio::test]
    async fn send_message_bad_uuid_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations/not-a-uuid/messages")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn send_message_empty_body_returns_400() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("T")).await.unwrap();
        let id = conv.id;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{id}/messages"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn send_message_unknown_conversation_returns_404() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let id = uuid::Uuid::new_v4();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{id}/messages"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn send_message_streams_sse_tokens() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Hello world").await;

        let (state, storage) = test_state(&server.uri()).await;
        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("Stream")).await.unwrap();
        let id = conv.id;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{id}/messages"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"ping"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("text/event-stream")
        );
        // Consume enough of the body to confirm SSE framing is present.
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("event:") || text.contains("data:"),
            "expected SSE framing, got: {text}"
        );
    }

    #[tokio::test]
    async fn send_message_done_event_contains_message_id() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Hello world").await;

        let (state, storage) = test_state(&server.uri()).await;
        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("MsgId")).await.unwrap();
        let id = conv.id;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{id}/messages"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);
        // Find the done event data line (role == "assistant").
        let done_line = text
            .lines()
            .find(|l| l.starts_with("data:") && l.contains("\"assistant\""))
            .expect("expected a done data line with assistant role");
        let data = done_line.trim_start_matches("data:").trim();
        let json: serde_json::Value = serde_json::from_str(data).expect("valid JSON");
        assert_eq!(json["role"], "assistant", "done role should be assistant");
        assert!(
            json.get("message_id").and_then(|v| v.as_str()).is_some(),
            "done event should contain message_id, got: {json}"
        );
        // message_id must be a valid UUID.
        let mid = json["message_id"].as_str().unwrap();
        assert!(
            uuid::Uuid::parse_str(mid).is_ok(),
            "message_id should be a valid UUID, got: {mid}"
        );
    }

    #[tokio::test]
    async fn send_voice_message_done_event_contains_message_id() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Voice reply").await;

        let (state, storage) = test_state(&server.uri()).await;
        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("VoiceMsgId")).await.unwrap();

        // Wire up the stub transcription provider.
        let state = ApiState {
            transcription_provider: Some(Arc::new(StubTranscriptionProvider {
                transcript: "hello from voice".to_string(),
            })),
            ..state
        };

        let boundary = "testboundary";
        let body_str = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"audio.webm\"\r\nContent-Type: audio/webm\r\n\r\nfakeaudio\r\n--{boundary}--\r\n",
        );

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{}/voice", conv.id))
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body_str))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);

        // The stream should contain a transcript event with the stub text.
        assert!(
            text.contains("transcript"),
            "expected transcript event in SSE stream, got: {text}"
        );

        // Find the done event data line (role == "assistant").
        let done_line = text
            .lines()
            .find(|l| l.starts_with("data:") && l.contains("\"assistant\""))
            .expect("expected a done data line with assistant role");
        let data = done_line.trim_start_matches("data:").trim();
        let json: serde_json::Value = serde_json::from_str(data).expect("valid JSON");
        assert_eq!(json["role"], "assistant");
        assert!(
            json.get("message_id").and_then(|v| v.as_str()).is_some(),
            "voice done event should contain message_id, got: {json}"
        );
        let mid = json["message_id"].as_str().unwrap();
        assert!(
            uuid::Uuid::parse_str(mid).is_ok(),
            "message_id should be a valid UUID, got: {mid}"
        );
    }

    // -- Runtime agent_id switch -----------------------------------------------

    #[tokio::test]
    async fn list_reflects_agent_id_change_at_runtime() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        // Seed a conversation for "alice".
        let store_alice = ConversationStore::for_agent(storage.pool.clone(), "alice");
        store_alice
            .create_conversation(Some("Alice conv"))
            .await
            .unwrap();

        // Switch the shared agent ID to "alice" at runtime.
        *state.agent_id.write().await = "alice".to_string();

        let resp = api_router()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .uri("/conversations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "Alice conv");
    }

    // -- GET /capabilities -----------------------------------------------------

    #[tokio::test]
    async fn capabilities_no_providers_returns_false() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        // Both providers absent — both flags must be false.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["voice_send"], false);
        assert_eq!(json["voice_receive"], false);
    }

    // -- POST /conversations/{id}/voice — MIME validation & size limit ---------

    #[tokio::test]
    async fn voice_upload_without_transcription_provider_returns_503() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;
        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(Some("test")).await.unwrap();

        // Build a minimal multipart body
        let boundary = "testboundary";
        let body_bytes = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"audio.webm\"\r\nContent-Type: audio/webm\r\n\r\nfakeaudio\r\n--{boundary}--\r\n",
            boundary = boundary
        );

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{}/voice", conv.id))
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body_bytes))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn voice_upload_invalid_conversation_id_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let boundary = "testboundary";
        let body_bytes = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"\r\nContent-Type: audio/webm\r\n\r\nfake\r\n--{boundary}--\r\n",
            boundary = boundary
        );

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations/not-a-uuid/voice")
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body_bytes))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // -- GET /audio/{id} -------------------------------------------------------

    #[tokio::test]
    async fn get_audio_unknown_id_returns_404() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/audio/{}", uuid::Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_audio_invalid_id_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/audio/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_audio_returns_stored_bytes() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let fake_audio = b"mp3-bytes".to_vec();
        let audio_id = state
            .audio_store
            .insert(fake_audio.clone(), "audio/mpeg".to_string())
            .await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/audio/{audio_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("audio/mpeg")
        );
        let bytes = body_bytes(resp.into_body()).await;
        assert_eq!(bytes, fake_audio);
    }
}
