//! Conversation CRUD endpoints: list, stream, create, get, delete, update.
//!
//! Endpoints:
//! - `GET    /api/conversations`           — list conversations
//! - `GET    /api/conversations/stream`    — SSE stream of conversation list changes
//! - `POST   /api/conversations`           — create a new conversation
//! - `GET    /api/conversations/{id}`      — get a conversation with its history
//! - `DELETE /api/conversations/{id}`      — delete a conversation
//! - `PATCH  /api/conversations/{id}`      — update a conversation's title

use std::collections::HashMap;
use std::convert::Infallible;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::Event;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::warn;
use uuid::Uuid;

use assistant_core::types::conversation::MessageRole;
use assistant_storage::{ConversationEvent, ConversationStore};

use super::attachments::AttachmentMetaResponse;
use super::{ApiState, sse_response};

// -- Response types ----------------------------------------------------------

/// A conversation summary (no message history).
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

/// Summary of a single tool invocation within an assistant message.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ToolCallSummary {
    /// The tool that was called.
    pub name: String,
    /// The JSON arguments passed to the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    /// The tool's output, truncated to a reasonable display length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    /// `"ok"`, `"error"`, or `"denied"`.
    pub status: String,
}

/// Maximum length for tool result strings in history summaries.
const TOOL_RESULT_HISTORY_LIMIT: usize = 512;

/// A single message in a conversation.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MessageSummary {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub turn: i64,
    pub created_at: DateTime<Utc>,
    /// Tool calls made in this message (present when `role == "assistant"`
    /// and the message contains tool invocations).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallSummary>>,
    /// Name of the tool or skill that produced this result (present when
    /// `role == "tool"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,
    /// Whether text-to-speech audio can be synthesised for this message.
    /// `true` when a TTS provider is configured and the message is a
    /// non-empty assistant reply.
    pub tts_available: bool,
    /// Attachments linked to this message.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentMetaResponse>,
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

// -- Handlers ----------------------------------------------------------------

/// `GET /api/conversations` — list all conversations.
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

/// Query parameters for `GET /api/conversations/stream`.
#[derive(Debug, Deserialize)]
pub struct StreamConversationsQuery {
    /// Filter events to a single agent. If omitted, events for all agents are streamed.
    pub agent_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/conversations/stream",
    tag = "conversations",
    params(
        ("agent_id" = Option<String>, Query, description = "Filter events to a single agent. If omitted, events for all agents are streamed."),
    ),
    responses(
        (status = 200, description = "SSE event stream. Events: `snapshot` (full list), `upserted` (single ConversationSummary), `deleted` ({conversation_id})", content_type = "text/event-stream"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to fetch conversations"),
    ),
    security(("bearer_token" = []))
)]
/// `GET /api/conversations/stream` — SSE stream of conversation list changes.
///
/// Sends an initial `snapshot` event with the full conversation list, then
/// pushes `upserted` and `deleted` delta events as conversations change.
pub async fn stream_conversations(
    State(state): State<ApiState>,
    axum::extract::Query(query): axum::extract::Query<StreamConversationsQuery>,
) -> Response {
    let broadcaster = state.conversation_broadcaster.clone();
    let pool = state.pool.clone();

    // D4: subscribe *before* snapshot to avoid race window.
    let mut rx = broadcaster.subscribe();

    // Resolve the agent scope for the snapshot.
    let default_agent_id = state.agent_id.read().await.clone();
    let filter_agent_id = query.agent_id.clone();
    let snapshot_agent_id = filter_agent_id
        .as_deref()
        .unwrap_or(&default_agent_id)
        .to_string();

    // Fetch snapshot from DB.
    let store = ConversationStore::for_agent(pool.clone(), &snapshot_agent_id);
    let snapshot = match store.list_conversations().await {
        Ok(convs) => convs
            .into_iter()
            .map(|c| ConversationSummary {
                id: c.id,
                title: c.title.unwrap_or_else(|| "Untitled".into()),
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            warn!("Failed to list conversations for stream snapshot: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch conversations",
            )
                .into_response();
        }
    };

    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);

    tokio::spawn(async move {
        // Send snapshot.
        let snapshot_json = serde_json::to_string(&snapshot).unwrap_or_default();
        let event = Event::default().event("snapshot").data(snapshot_json);
        if sse_tx.send(Ok(event)).await.is_err() {
            return;
        }

        // Forward delta events.
        loop {
            match rx.recv().await {
                Ok(conv_event) => {
                    let sse_event = match &conv_event {
                        ConversationEvent::Upserted(record) => {
                            // Apply agent filter.
                            if let Some(ref filter) = filter_agent_id
                                && record.agent_id != *filter
                            {
                                continue;
                            }
                            let summary = ConversationSummary {
                                id: record.id,
                                title: record.title.clone().unwrap_or_else(|| "Untitled".into()),
                                created_at: record.created_at,
                                updated_at: record.updated_at,
                            };
                            let json = serde_json::to_string(&summary).unwrap_or_default();
                            Event::default().event("upserted").data(json)
                        }
                        ConversationEvent::Deleted {
                            conversation_id,
                            agent_id,
                        } => {
                            if let Some(ref filter) = filter_agent_id
                                && agent_id != filter
                            {
                                continue;
                            }
                            let json = serde_json::json!({
                                "conversation_id": conversation_id,
                            })
                            .to_string();
                            Event::default().event("deleted").data(json)
                        }
                    };
                    if sse_tx.send(Ok(sse_event)).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!("conversation stream lagged by {n} events");
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    sse_response(sse_rx)
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
    let store = ConversationStore::for_agent(state.pool, &agent_id)
        .with_broadcaster(state.conversation_broadcaster.clone());
    // Pass the title through as-is. Without an explicit title, the row stays
    // NULL-titled and the title-generator worker will fill it in once the
    // conversation has enough material. Display layers coerce NULL → "Untitled".
    match store.create_conversation(body.title.as_deref()).await {
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
    let tts_configured = state.tts_provider.is_some();

    // Pre-load all attachments for this conversation, grouped by message ID.
    let mut attachments_by_msg: HashMap<Uuid, Vec<AttachmentMetaResponse>> = HashMap::new();
    if let Ok(all_atts) = state.attachment_store.list_for_conversation(conv_id).await {
        for att in all_atts {
            if let Some(msg_id) = att.message_id {
                attachments_by_msg
                    .entry(msg_id)
                    .or_default()
                    .push(AttachmentMetaResponse::from_meta(&att));
            }
        }
    }

    // Collect tool-result messages (role=tool) keyed by (turn, tool_name)
    // so we can join them with the corresponding assistant tool-call messages.
    let mut tool_results: HashMap<(i64, String), String> = HashMap::new();
    for m in &history {
        if matches!(m.role, MessageRole::Tool)
            && let Some(ref name) = m.skill_name
        {
            tool_results.insert((m.turn, name.clone()), m.content.clone());
        }
    }

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
                                let name =
                                    v.get("name").and_then(|n| n.as_str()).map(str::to_string)?;
                                let arguments = v.get("params").cloned();
                                let raw_result = tool_results.get(&(m.turn, name.clone())).cloned();
                                let result = raw_result.map(|r| {
                                    if r.len() > TOOL_RESULT_HISTORY_LIMIT {
                                        let mut s = r[..TOOL_RESULT_HISTORY_LIMIT].to_string();
                                        s.push('…');
                                        s
                                    } else {
                                        r
                                    }
                                });
                                let status = if result
                                    .as_ref()
                                    .is_some_and(|r| r.starts_with("Error executing"))
                                {
                                    "error"
                                } else if result
                                    .as_ref()
                                    .is_some_and(|r| r.starts_with("User denied"))
                                {
                                    "denied"
                                } else {
                                    "ok"
                                };
                                Some(ToolCallSummary {
                                    name,
                                    arguments,
                                    result,
                                    status: status.to_string(),
                                })
                            })
                            .collect::<Vec<_>>()
                    })
                    .filter(|v: &Vec<_>| !v.is_empty())
            });
            let tts_available =
                tts_configured && matches!(m.role, MessageRole::Assistant) && !m.content.is_empty();
            let attachments = attachments_by_msg.remove(&m.id).unwrap_or_default();
            MessageSummary {
                id: m.id,
                role: m.role.to_string(),
                content: m.content,
                turn: m.turn,
                created_at: m.created_at,
                tool_calls,
                skill_name: m.skill_name,
                tts_available,
                attachments,
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
    let store = ConversationStore::for_agent(state.pool, &agent_id)
        .with_broadcaster(state.conversation_broadcaster.clone());
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
    let store = ConversationStore::for_agent(state.pool, &agent_id)
        .with_broadcaster(state.conversation_broadcaster.clone());
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

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use uuid::Uuid;
    use wiremock::MockServer;

    use assistant_storage::ConversationStore;

    use super::super::api_router;
    use super::super::test_helpers::*;

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
    async fn test_create_conversation_without_title_leaves_title_null() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

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
        // Response surfaces "Untitled" (NULL coerced for display).
        assert_eq!(json["title"], "Untitled");

        // DB row holds NULL with the lock cleared so the worker can title later.
        let conv_id: Uuid = json["id"].as_str().unwrap().parse().unwrap();
        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert!(conv.title.is_none(), "DB title must be NULL");
        assert!(!conv.title_locked, "title_locked must be false");
    }

    #[tokio::test]
    async fn test_create_conversation_with_explicit_title_locks() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"title":"Pinned Thread"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["title"], "Pinned Thread");

        let conv_id: Uuid = json["id"].as_str().unwrap().parse().unwrap();
        let store = ConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert_eq!(conv.title.as_deref(), Some("Pinned Thread"));
        assert!(
            conv.title_locked,
            "explicit title at create must lock the conversation"
        );
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
}
