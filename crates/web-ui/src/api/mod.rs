//! REST JSON API for conversation management.
//!
//! Designed for native/external clients (mobile apps, desktop apps, etc.)
//! that need to manage conversations and send messages programmatically.
//!
//! All endpoints require `Authorization: Bearer <token>` (same token as the
//! web UI).  Responses are `application/json`.  The streaming send endpoint
//! uses `text/event-stream` (SSE).
//!
//! ## Routes
//!
//! | Method   | Path                                  | Description                     |
//! |----------|---------------------------------------|---------------------------------|
//! | GET      | `/api/conversations`                  | List all conversations          |
//! | POST     | `/api/conversations`                  | Create a new conversation       |
//! | GET      | `/api/conversations/{id}`             | Get conversation + history      |
//! | DELETE   | `/api/conversations/{id}`             | Delete a conversation           |
//! | PATCH    | `/api/conversations/{id}`             | Update conversation title       |
//! | POST     | `/api/conversations/{id}/messages`    | Send a message (SSE stream)     |

use std::convert::Infallible;
use std::sync::Arc;

use assistant_core::{Interface, MessageRole};
use assistant_runtime::Orchestrator;
use assistant_storage::ConversationStore;
use axum::{
    extract::{Path, State},
    http::StatusCode,
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
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;
use uuid::Uuid;

// -- State -------------------------------------------------------------------

/// Shared state for the conversation API handlers.
#[derive(Clone)]
pub struct ApiState {
    pub pool: SqlitePool,
    pub agent_id: String,
    pub orchestrator: Arc<Orchestrator>,
}

impl ApiState {
    pub fn new(pool: SqlitePool, orchestrator: Arc<Orchestrator>, agent_id: String) -> Self {
        Self {
            pool,
            agent_id,
            orchestrator,
        }
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
pub struct SendMessageRequest {
    /// The message text to send to the assistant.
    pub message: String,
}

// -- Router ------------------------------------------------------------------

/// Build the conversations API sub-router.  Mounted under `/api`.
pub fn api_router() -> Router<ApiState> {
    Router::new()
        .route("/conversations", get(list_conversations))
        .route("/conversations", post(create_conversation))
        .route("/conversations/{id}", get(get_conversation))
        .route("/conversations/{id}", delete(delete_conversation))
        .route("/conversations/{id}", patch(update_conversation))
        .route("/conversations/{id}/messages", post(send_message))
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
    let store = ConversationStore::for_agent(state.pool, &state.agent_id);
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
    let store = ConversationStore::for_agent(state.pool, &state.agent_id);
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

    let store = ConversationStore::for_agent(state.pool, &state.agent_id);

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
        .map(|m| MessageSummary {
            id: m.id,
            role: m.role.to_string(),
            content: m.content,
            turn: m.turn,
            created_at: m.created_at,
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

    let store = ConversationStore::for_agent(state.pool, &state.agent_id);
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

    let store = ConversationStore::for_agent(state.pool, &state.agent_id);
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
    let store = ConversationStore::for_agent(state.pool.clone(), &state.agent_id);

    // Auto-title the conversation on the first user message.
    match store.load_history(conv_id).await {
        Ok(prior) if prior.is_empty() => {
            let title = if content.len() > 60 {
                format!("{}...", &content[..57])
            } else {
                content.clone()
            };
            if let Err(e) = store.update_title(conv_id, &title).await {
                // Non-fatal: conversation might not exist yet — the orchestrator will create it.
                warn!("Auto-title failed for {conv_id}: {e}");
            }
        }
        Err(e) => {
            warn!("Failed to load history for {conv_id}: {e}");
            return (StatusCode::NOT_FOUND, "Conversation not found").into_response();
        }
        _ => {}
    }

    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    let (token_tx, mut token_rx) = mpsc::channel::<String>(64);

    state
        .orchestrator
        .register_token_sink(conv_id, token_tx)
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

    tokio::spawn(async move {
        let mut full_text = String::new();

        while let Some(token) = token_rx.recv().await {
            full_text.push_str(&token);
            let event = Event::default().event("token").data(token);
            if sse_tx.send(Ok(event)).await.is_err() {
                return;
            }
        }

        let reply_text = match turn_result_rx.await {
            Ok(Ok(result)) => result.answer,
            Ok(Err(_)) | Err(_) => full_text,
        };

        let done_data = serde_json::json!({
            "role": "assistant",
            "content": reply_text,
        });
        let done = Event::default().event("done").data(done_data.to_string());
        let _ = sse_tx.send(Ok(done)).await;
    });

    Sse::new(ReceiverStream::new(sse_rx)).into_response()
}
