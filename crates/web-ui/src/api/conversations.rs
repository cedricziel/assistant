//! Conversation CRUD endpoints: list, stream, create, get, delete, update.
//!
//! Endpoints:
//! - `GET    /api/conversations`           — list conversations
//! - `GET    /api/conversations/stream`    — SSE stream of conversation list changes
//! - `POST   /api/conversations`           — create a new conversation
//! - `GET    /api/conversations/{id}`      — get a conversation with its history
//! - `DELETE /api/conversations/{id}`      — delete a conversation
//! - `PATCH  /api/conversations/{id}`      — update a conversation's title or pin state

use std::collections::HashMap;
use std::convert::Infallible;

use axum::Json;
use axum::extract::rejection::JsonRejection;
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
use assistant_storage::{
    AttachmentStore as _, ConversationEvent, ConversationPatch, ConversationRecord,
    ConversationStore, MessageFeedbackRow, MessageFeedbackStore as _, MetadataUpdate,
    SqliteConversationStore, SurfaceStore as _,
};

use crate::errors::json_error;
use crate::openapi::ErrorBody;
use crate::space::{Access, SpaceCtx};

use super::attachments::AttachmentMetaResponse;
use super::branches::{BranchInfo, project_branches};
use super::surfaces::{SurfaceSummary, surface_summaries};
use super::{ApiState, sse_response};

// -- Caller scoping ----------------------------------------------------------

/// The conversation store for the calling user in the caller's space: every
/// read through it sees only conversations that user owns.
pub(crate) async fn caller_store(ctx: &SpaceCtx, state: &ApiState) -> SqliteConversationStore {
    let agent_id = state.agent_id.read().await.clone();
    SqliteConversationStore::for_agent(ctx.storage.pool.clone(), agent_id)
        .with_user_id(ctx.auth.user_id.0.clone())
}

/// Load a conversation the caller owns, or the enveloped `404`/`500` response
/// to return.
///
/// A conversation owned by someone else is reported as missing, so the
/// response does not confirm that the id exists.
pub(crate) async fn owned_conversation(
    store: &SqliteConversationStore,
    id: Uuid,
) -> Result<ConversationRecord, Response> {
    match store.get_conversation(id).await {
        Ok(Some(conv)) => Ok(conv),
        Ok(None) => Err(json_error(StatusCode::NOT_FOUND, "Conversation not found")),
        Err(e) => {
            warn!("Failed to load conversation {id}: {e}");
            Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            ))
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
    /// When the conversation was pinned; absent when it is not pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<DateTime<Utc>>,
}

impl From<&ConversationRecord> for ConversationSummary {
    fn from(record: &ConversationRecord) -> Self {
        Self {
            id: record.id,
            title: display_title(record),
            created_at: record.created_at,
            updated_at: record.updated_at,
            pinned_at: record.pinned_at,
        }
    }
}

/// The title shown to users: a `NULL` title displays as `"Untitled"`.
fn display_title(record: &ConversationRecord) -> String {
    record.title.clone().unwrap_or_else(|| "Untitled".into())
}

/// A conversation with its full message history.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ConversationDetail {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<MessageSummary>,
    /// Generative UI surfaces anchored on the returned messages.
    pub surfaces: Vec<SurfaceSummary>,
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
    /// Provider-assigned id of the call, when the provider supplied one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Structured output of the tool, a JSON object with a `kind` tag. Absent
    /// when the tool returned text only, the data exceeded the size limit, or
    /// the call predates structured data. Each `kind` is defined by a JSON
    /// Schema in `schemas/tool-results/`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub data: Option<serde_json::Value>,
    /// The MCP App the tool declared for its result. Absent for tools
    /// without one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<super::mcp_apps::McpAppView>,
}

/// Maximum length for tool result strings in history summaries.
const TOOL_RESULT_HISTORY_LIMIT: usize = 512;

/// The tool-result rows of a conversation, with a record of which have been
/// joined to an assistant tool call.
struct ToolRows {
    rows: Vec<assistant_core::types::conversation::Message>,
    claimed: Vec<bool>,
}

impl ToolRows {
    fn new(rows: Vec<assistant_core::types::conversation::Message>) -> Self {
        let claimed = vec![false; rows.len()];
        Self { rows, claimed }
    }

    /// Claim the unclaimed row of `turn` that satisfies `matches`, the
    /// earliest first.
    fn claim(
        &mut self,
        turn: i64,
        matches: impl Fn(&assistant_core::types::conversation::Message) -> bool,
    ) -> Option<usize> {
        let idx = self
            .rows
            .iter()
            .enumerate()
            .position(|(i, r)| !self.claimed[i] && r.turn == turn && matches(r))?;
        self.claimed[idx] = true;
        Some(idx)
    }

    /// Summarise the `tool_calls_json` entries of one assistant row.
    ///
    /// Each call joins to its result row by `tool_call_id`. Calls without a
    /// matching id take the next unclaimed row of the same tool name in the
    /// turn, which is the dispatch order.
    fn summaries(&mut self, turn: i64, items: &[serde_json::Value]) -> Vec<ToolCallSummary> {
        let calls: Vec<(String, Option<String>, &serde_json::Value)> = items
            .iter()
            .filter_map(|v| {
                let name = v.get("name").and_then(|n| n.as_str())?.to_string();
                let id = v.get("id").and_then(|i| i.as_str()).map(str::to_string);
                Some((name, id, v))
            })
            .collect();

        let mut joined: Vec<Option<usize>> = calls
            .iter()
            .map(|(_, id, _)| {
                let id = id.as_deref()?;
                self.claim(turn, |r| r.tool_call_id.as_deref() == Some(id))
            })
            .collect();
        for (slot, (name, _, _)) in joined.iter_mut().zip(&calls) {
            if slot.is_none() {
                *slot = self.claim(turn, |r| r.skill_name.as_deref() == Some(name.as_str()));
            }
        }

        calls
            .into_iter()
            .zip(joined)
            .map(|((name, id, v), row)| {
                let row = row.map(|i| &self.rows[i]);
                let result = row.map(|r| truncate_for_history(&r.content));
                let status = match row.and_then(|r| r.tool_status.as_deref()) {
                    Some(status) => status.to_string(),
                    None => status_from_content(result.as_deref()).to_string(),
                };
                ToolCallSummary {
                    name,
                    arguments: v.get("params").cloned(),
                    result,
                    status,
                    tool_call_id: row.and_then(|r| r.tool_call_id.clone()).or(id),
                    data: row
                        .and_then(|r| r.tool_data_json.as_deref())
                        .and_then(|d| serde_json::from_str::<serde_json::Value>(d).ok())
                        .filter(serde_json::Value::is_object),
                    app: row
                        .and_then(|r| super::mcp_apps::stored_app(r.tool_app_json.as_deref()))
                        .map(Into::into),
                }
            })
            .collect()
    }
}

/// Cut `s` to [`TOOL_RESULT_HISTORY_LIMIT`] bytes on a character boundary.
fn truncate_for_history(s: &str) -> String {
    if s.len() <= TOOL_RESULT_HISTORY_LIMIT {
        return s.to_string();
    }
    let cut = s.floor_char_boundary(TOOL_RESULT_HISTORY_LIMIT);
    format!("{}…", &s[..cut])
}

/// Status of a row stored before `tool_status` existed.
fn status_from_content(result: Option<&str>) -> &'static str {
    match result {
        Some(r) if r.starts_with("Error executing") => "error",
        Some(r) if r.starts_with("User denied") => "denied",
        _ => "ok",
    }
}

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
    /// Why the reply ended early; `"cancelled"` for a reply the user stopped.
    /// Omitted for every other message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// The message this one follows in the conversation tree. Omitted for a
    /// root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    /// This message's place among its versions, present only when it has
    /// siblings. On a user message: the edited versions. On an assistant
    /// message: the versions of the reply it ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<BranchInfo>,
    /// On a user message whose reply has no visible assistant row: the
    /// versions of that reply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_branch: Option<BranchInfo>,
    /// The calling user's verdict on this message. Omitted when the caller has
    /// not rated it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<MessageFeedbackSummary>,
}

/// The calling user's feedback on a message, as embedded in history.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MessageFeedbackSummary {
    /// `"positive"` or `"negative"`.
    pub rating: String,
    /// Optional free-text comment. Omitted when none was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl From<MessageFeedbackRow> for MessageFeedbackSummary {
    fn from(row: MessageFeedbackRow) -> Self {
        Self {
            rating: row.rating.as_str().to_string(),
            comment: row.comment,
            updated_at: row.updated_at,
        }
    }
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/conversations`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateConversationRequest {
    /// Optional title for the new conversation.
    pub title: Option<String>,
}

/// Longest title a user may set, in Unicode scalar values.
const MAX_TITLE_CHARS: usize = 200;

/// Body for `PATCH /api/conversations/{id}`. At least one field is required.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateConversationRequest {
    /// New title. Trimmed; must be 1 to 200 characters after trimming.
    pub title: Option<String>,
    /// `true` pins the conversation, `false` unpins it.
    pub pinned: Option<bool>,
}

impl UpdateConversationRequest {
    /// Validate the request and turn it into a storage patch, or the message
    /// for the `400` response.
    fn into_patch(self) -> Result<ConversationPatch, &'static str> {
        if self.title.is_none() && self.pinned.is_none() {
            return Err("Nothing to update: provide title or pinned");
        }
        let title = match self.title {
            Some(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Err("Title must not be blank");
                }
                if trimmed.chars().count() > MAX_TITLE_CHARS {
                    return Err("Title must be at most 200 characters");
                }
                Some(trimmed.to_owned())
            }
            None => None,
        };
        Ok(ConversationPatch {
            title,
            pinned: self.pinned,
        })
    }
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
pub async fn list_conversations(ctx: SpaceCtx, State(state): State<ApiState>) -> Response {
    if let Err(denied) = ctx.require(Access::Read) {
        return denied.into_response();
    }
    let store = caller_store(&ctx, &state).await;
    match store.list_conversations().await {
        Ok(convs) => {
            let summaries: Vec<ConversationSummary> =
                convs.iter().map(ConversationSummary::from).collect();
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
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    axum::extract::Query(query): axum::extract::Query<StreamConversationsQuery>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Read) {
        return denied.into_response();
    }
    let broadcaster = state
        .conversation_broadcasters
        .for_space(&ctx.org_slug, &ctx.space_slug);
    let pool = ctx.storage.pool.clone();
    let caller = ctx.auth.user_id.0.clone();

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
    let store = SqliteConversationStore::for_agent(pool.clone(), &snapshot_agent_id)
        .with_user_id(caller.clone());
    let snapshot = match store.list_conversations().await {
        Ok(convs) => convs,
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
        // Deleted events carry no owner, so remember which ids belong to the
        // caller and forward only those.
        let mut owned: std::collections::HashSet<Uuid> = snapshot.iter().map(|c| c.id).collect();
        let snapshot: Vec<ConversationSummary> =
            snapshot.iter().map(ConversationSummary::from).collect();

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
                            if record.user_id.as_deref() != Some(caller.as_str()) {
                                continue;
                            }
                            owned.insert(record.id);
                            // Apply agent filter.
                            if let Some(ref filter) = filter_agent_id
                                && record.agent_id != *filter
                            {
                                continue;
                            }
                            let summary = ConversationSummary::from(record);
                            let json = serde_json::to_string(&summary).unwrap_or_default();
                            Event::default().event("upserted").data(json)
                        }
                        ConversationEvent::Deleted {
                            conversation_id,
                            agent_id,
                        } => {
                            if !owned.remove(conversation_id) {
                                continue;
                            }
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
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Json(body): Json<CreateConversationRequest>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
    let store = caller_store(&ctx, &state).await.with_broadcaster(
        state
            .conversation_broadcasters
            .for_space(&ctx.org_slug, &ctx.space_slug),
    );
    // Pass the title through as-is. Without an explicit title, the row stays
    // NULL-titled and the title-generator worker will fill it in once the
    // conversation has enough material. Display layers coerce NULL → "Untitled".
    match store.create_conversation(body.title.as_deref()).await {
        Ok(c) => (StatusCode::CREATED, Json(ConversationSummary::from(&c))).into_response(),
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
        (status = 404, description = "Not found", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_conversation(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Read) {
        return denied.into_response();
    }
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let store = caller_store(&ctx, &state).await;
    let conv = match owned_conversation(&store, conv_id).await {
        Ok(c) => c,
        Err(response) => return response,
    };

    Json(conversation_detail(&ctx, &state, &store, &conv).await).into_response()
}

/// The conversation `conv` with its active path, as `GET` returns it.
pub(crate) async fn conversation_detail(
    ctx: &SpaceCtx,
    state: &ApiState,
    store: &SqliteConversationStore,
    conv: &ConversationRecord,
) -> ConversationDetail {
    let conv_id = conv.id;
    let history = store.load_history(conv_id).await.unwrap_or_default();
    let tts_configured = state.tts_provider.is_some();

    // Pre-load all attachments for this conversation, grouped by message ID.
    let mut attachments_by_msg: HashMap<Uuid, Vec<AttachmentMetaResponse>> = HashMap::new();
    if let Ok(all_atts) = ctx
        .storage
        .attachment_store()
        .list_for_conversation(conv_id)
        .await
    {
        for att in all_atts {
            if let Some(msg_id) = att.message_id {
                attachments_by_msg
                    .entry(msg_id)
                    .or_default()
                    .push(AttachmentMetaResponse::from_meta(&att));
            }
        }
    }

    // Surfaces whose anchor is on the returned (active) path.
    let surfaces = match ctx
        .storage
        .surface_store()
        .list_for_conversation(conv_id)
        .await
    {
        Ok(rows) => surface_summaries(&history, rows),
        Err(e) => {
            warn!(error = %e, "Failed to load surfaces for conversation");
            Vec::new()
        }
    };

    // The caller's feedback for the whole conversation, in one query.
    let mut feedback_by_msg: HashMap<Uuid, MessageFeedbackSummary> = ctx
        .storage
        .message_feedback_store()
        .list_for_conversation(conv_id, &ctx.auth.user_id.0)
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to load message feedback: {e}");
            Vec::new()
        })
        .into_iter()
        .map(|row| (row.message_id, MessageFeedbackSummary::from(row)))
        .collect();

    // Tool-result rows (role=tool), joined to assistant tool calls below.
    let mut tool_rows = ToolRows::new(
        history
            .iter()
            .filter(|m| matches!(m.role, MessageRole::Tool) && m.skill_name.is_some())
            .cloned()
            .collect(),
    );

    let mut branches = project_branches(store, conv_id, &history, |m| {
        !m.content.is_empty() || attachments_by_msg.contains_key(&m.id)
    })
    .await;

    let messages = history
        .into_iter()
        .filter(|m| !matches!(m.role, MessageRole::System | MessageRole::Tool))
        .map(|m| {
            let tool_calls = m.tool_calls_json.as_deref().and_then(|json| {
                serde_json::from_str::<Vec<serde_json::Value>>(json)
                    .ok()
                    .map(|items| tool_rows.summaries(m.turn, &items))
                    .filter(|v: &Vec<_>| !v.is_empty())
            });
            let tts_available =
                tts_configured && matches!(m.role, MessageRole::Assistant) && !m.content.is_empty();
            let attachments = attachments_by_msg.remove(&m.id).unwrap_or_default();
            let feedback = feedback_by_msg.remove(&m.id);
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
                finish_reason: m.finish_reason.map(|r| r.as_str().to_string()),
                parent_id: m.parent_id,
                branch: branches.branch.remove(&m.id),
                reply_branch: branches.reply_branch.remove(&m.id),
                feedback,
            }
        })
        .collect();

    ConversationDetail {
        id: conv.id,
        title: display_title(conv),
        created_at: conv.created_at,
        updated_at: conv.updated_at,
        messages,
        surfaces,
    }
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
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let store = caller_store(&ctx, &state).await.with_broadcaster(
        state
            .conversation_broadcasters
            .for_space(&ctx.org_slug, &ctx.space_slug),
    );
    if let Err(response) = owned_conversation(&store, conv_id).await {
        return response;
    }
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

/// `PATCH /api/conversations/{id}` — update a conversation's title and/or pin state.
///
/// Neither change moves the conversation in the list: only new messages bump
/// `updated_at`.
#[utoipa::path(
    patch,
    path = "/api/conversations/{id}",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    request_body = UpdateConversationRequest,
    responses(
        (status = 200, description = "Updated conversation", body = ConversationSummary),
        (status = 400, description = "Invalid ID or body", body = ErrorBody),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Read-only access", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn update_conversation(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Path(id): Path<String>,
    body: Result<Json<UpdateConversationRequest>, JsonRejection>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
    let Ok(conv_id) = Uuid::parse_str(&id) else {
        return json_error(StatusCode::BAD_REQUEST, "Invalid conversation ID");
    };
    let Json(body) = match body {
        Ok(body) => body,
        Err(rejection) => return json_error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    let patch = match body.into_patch() {
        Ok(patch) => patch,
        Err(message) => return json_error(StatusCode::BAD_REQUEST, message),
    };

    let store = caller_store(&ctx, &state).await.with_broadcaster(
        state
            .conversation_broadcasters
            .for_space(&ctx.org_slug, &ctx.space_slug),
    );
    match store.update_metadata(conv_id, patch).await {
        Ok(MetadataUpdate::Updated(record)) => {
            Json(ConversationSummary::from(&record)).into_response()
        }
        Ok(MetadataUpdate::NotFound) => json_error(StatusCode::NOT_FOUND, "Conversation not found"),
        Err(e) => {
            warn!("Failed to update conversation {conv_id}: {e}");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update conversation",
            )
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

    use chrono::Utc;

    use assistant_storage::{ConversationStore, SqliteConversationStore};

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

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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

    /// Seed an assistant tool-call row followed by tool rows, then load the
    /// conversation through the handler and return the assistant's
    /// `tool_calls` array.
    async fn history_tool_calls(
        calls: serde_json::Value,
        tool_rows: Vec<assistant_core::types::conversation::Message>,
    ) -> Vec<serde_json::Value> {
        use assistant_core::types::conversation::Message;

        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(Some("Tools")).await.unwrap();

        let mut user = Message::user(conv.id, "go");
        user.turn = 1;
        store.save_message(&user).await.unwrap();
        let mut assistant = Message::assistant(conv.id, "");
        assistant.turn = 1;
        assistant.tool_calls_json = Some(calls.to_string());
        store.save_message(&assistant).await.unwrap();
        for mut row in tool_rows {
            row.conversation_id = conv.id;
            row.turn = 1;
            store.save_message(&row).await.unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{}", conv.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        json["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|m| m["tool_calls"].as_array().cloned())
            .expect("assistant message carries tool_calls")
    }

    fn tool_row(
        name: &str,
        content: &str,
        id: Option<&str>,
        status: Option<&str>,
        data: Option<serde_json::Value>,
    ) -> assistant_core::types::conversation::Message {
        use assistant_core::types::conversation::{Message, MessageRole};

        let mut m = Message::new(Uuid::nil(), MessageRole::Tool, content);
        m.skill_name = Some(name.into());
        m.tool_call_id = id.map(str::to_string);
        m.tool_status = status.map(str::to_string);
        m.tool_data_json = data.map(|d| d.to_string());
        m
    }

    /// Seed `user, assistant(tool call)` plus a surface anchored on the
    /// assistant row, optionally followed by more user messages, and return
    /// the loaded conversation JSON.
    async fn conversation_with_surface(follow_up: bool) -> (serde_json::Value, Uuid) {
        use assistant_core::types::conversation::Message;
        use assistant_storage::{NewSurface, SurfaceStore as _};

        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(Some("Surfaces")).await.unwrap();
        let mut user = Message::user(conv.id, "plan a trip");
        user.turn = 1;
        store.save_message(&user).await.unwrap();
        let mut assistant = Message::assistant(conv.id, "");
        assistant.turn = 1;
        assistant.tool_calls_json =
            Some(serde_json::json!([{"name": "render-surface", "params": {}}]).to_string());
        store.save_message(&assistant).await.unwrap();
        let surface = storage
            .surface_store()
            .create(&NewSurface {
                id: Uuid::new_v4(),
                conversation_id: conv.id,
                turn: 1,
                anchor_message_id: assistant.id,
                anchor_call_index: 0,
                tool_call_id: Some("call_1".into()),
                title: "Trip".into(),
                fallback_text: "Pick a date".into(),
                catalog_id: "cat".into(),
                protocol_version: "1.0".into(),
                components: serde_json::json!([{"id": "root", "component": "Text", "text": "hi"}]),
                data_model: serde_json::json!({"when": "soon"}),
                send_data_model: true,
            })
            .await
            .unwrap();
        if follow_up {
            let mut next = Message::user(conv.id, "never mind");
            next.turn = 2;
            store.save_message(&next).await.unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{}", conv.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        (body_json(resp.into_body()).await, surface.id)
    }

    #[tokio::test]
    async fn get_conversation_includes_surfaces_with_anchor_and_answerable() {
        let (json, surface_id) = conversation_with_surface(false).await;
        let surfaces = json["surfaces"].as_array().expect("surfaces preload");
        assert_eq!(surfaces.len(), 1);
        let s = &surfaces[0];
        assert_eq!(s["surface_id"], surface_id.to_string());
        assert_eq!(s["tool_call_id"], "call_1");
        assert_eq!(s["status"], "active");
        assert_eq!(s["answerable"], true);
        assert_eq!(s["anchor_call_index"], 0);
        assert_eq!(
            s["anchor_message_id"], json["messages"][1]["id"],
            "anchored on the assistant tool-call message"
        );
        assert_eq!(
            s["messages"][0]["createSurface"]["surfaceId"],
            surface_id.to_string()
        );
        assert!(s.get("submitted_action").is_none());
    }

    #[tokio::test]
    async fn get_conversation_surface_closes_after_a_later_user_message() {
        let (json, _) = conversation_with_surface(true).await;
        assert_eq!(json["surfaces"][0]["answerable"], false);
        assert_eq!(json["surfaces"][0]["status"], "active");
    }

    #[tokio::test]
    async fn history_joins_same_named_calls_by_ordinal_without_ids() {
        let calls = serde_json::json!([
            {"name": "bash", "params": {"command": "echo A"}},
            {"name": "bash", "params": {"command": "echo B"}},
        ]);
        let rows = vec![
            tool_row("bash", "A", None, None, None),
            tool_row("bash", "B", None, None, None),
        ];
        let tc = history_tool_calls(calls, rows).await;
        assert_eq!(tc[0]["result"], "A");
        assert_eq!(tc[1]["result"], "B");
    }

    #[tokio::test]
    async fn history_joins_by_tool_call_id_with_own_data() {
        let calls = serde_json::json!([
            {"name": "bash", "params": {}, "id": "c1"},
            {"name": "bash", "params": {}, "id": "c2"},
        ]);
        // Rows stored in the opposite order: the id wins over position.
        let rows = vec![
            tool_row(
                "bash",
                "B",
                Some("c2"),
                Some("ok"),
                Some(serde_json::json!({"kind": "terminal", "stdout": "B"})),
            ),
            tool_row(
                "bash",
                "A",
                Some("c1"),
                Some("ok"),
                Some(serde_json::json!({"kind": "terminal", "stdout": "A"})),
            ),
        ];
        let tc = history_tool_calls(calls, rows).await;
        assert_eq!(tc[0]["result"], "A");
        assert_eq!(tc[0]["data"]["stdout"], "A");
        assert_eq!(tc[0]["tool_call_id"], "c1");
        assert_eq!(tc[1]["result"], "B");
        assert_eq!(tc[1]["data"]["stdout"], "B");
        assert_eq!(tc[1]["tool_call_id"], "c2");
    }

    #[tokio::test]
    async fn history_carries_the_mcp_app_of_the_call() {
        let calls =
            serde_json::json!([{"name": "mcp__weather__forecast", "params": {}, "id": "c1"}]);
        let mut row = tool_row(
            "mcp__weather__forecast",
            "21 degrees",
            Some("c1"),
            Some("ok"),
            None,
        );
        row.tool_app_json = Some(
            serde_json::json!({
                "server": "weather",
                "resource_uri": "ui://weather/forecast",
                "html": "<p>sunny</p>",
                "csp": {"connect_domains": ["https://api.weather.test"]},
                "permissions": {"geolocation": true},
                "prefers_border": true,
            })
            .to_string(),
        );
        let tc = history_tool_calls(calls, vec![row]).await;
        assert_eq!(tc[0]["app"]["server"], "weather");
        assert_eq!(tc[0]["app"]["resource_uri"], "ui://weather/forecast");
        assert_eq!(tc[0]["app"]["html"], "<p>sunny</p>");
        assert_eq!(
            tc[0]["app"]["csp"]["connect_domains"][0],
            "https://api.weather.test"
        );
        assert_eq!(tc[0]["app"]["permissions"]["geolocation"], true);
        assert_eq!(tc[0]["app"]["prefers_border"], true);
    }

    #[tokio::test]
    async fn history_without_an_app_omits_the_field() {
        let calls = serde_json::json!([{"name": "bash", "params": {}}]);
        let rows = vec![tool_row("bash", "ok", None, Some("ok"), None)];
        let tc = history_tool_calls(calls, rows).await;
        assert!(tc[0].get("app").is_none(), "no app, no field");
    }

    #[tokio::test]
    async fn history_status_comes_from_the_column() {
        let calls = serde_json::json!([{"name": "bash", "params": {}}]);
        let rows = vec![tool_row("bash", "exit 1", None, Some("error"), None)];
        let tc = history_tool_calls(calls, rows).await;
        assert_eq!(tc[0]["status"], "error");
    }

    #[tokio::test]
    async fn history_null_metadata_omits_data_and_uses_prefix_rule() {
        let calls = serde_json::json!([
            {"name": "a", "params": {}},
            {"name": "b", "params": {}},
            {"name": "c", "params": {}},
        ]);
        let rows = vec![
            tool_row("a", "Error executing a: boom", None, None, None),
            tool_row("b", "User denied b", None, None, None),
            tool_row("c", "fine", None, None, None),
        ];
        let tc = history_tool_calls(calls, rows).await;
        assert_eq!(tc[0]["status"], "error");
        assert_eq!(tc[1]["status"], "denied");
        assert_eq!(tc[2]["status"], "ok");
        for call in &tc {
            assert!(call.get("data").is_none(), "NULL data must be omitted");
        }
    }

    #[tokio::test]
    async fn history_truncates_multibyte_result_without_panicking() {
        // 'é' is two bytes, so byte 512 falls inside a character.
        let content = format!("x{}", "é".repeat(400));
        let calls = serde_json::json!([{"name": "bash", "params": {}}]);
        let rows = vec![tool_row("bash", &content, None, None, None)];
        let tc = history_tool_calls(calls, rows).await;
        let result = tc[0]["result"].as_str().unwrap();
        assert!(result.ends_with('…'));
        assert!(content.starts_with(result.trim_end_matches('…')));
    }

    #[tokio::test]
    async fn history_surfaces_tool_call_id_when_row_has_it() {
        let calls = serde_json::json!([{"name": "bash", "params": {}, "id": "call_7"}]);
        let rows = vec![tool_row("bash", "ok", Some("call_7"), Some("ok"), None)];
        let tc = history_tool_calls(calls, rows).await;
        assert_eq!(tc[0]["tool_call_id"], "call_7");
    }

    /// The rows a compaction leaves behind: a summary row below the kept
    /// turn, and the kept rows with their original turn and tool metadata.
    #[tokio::test]
    async fn history_after_compaction_joins_kept_calls_with_their_own_data() {
        use assistant_core::types::conversation::{Message, MessageRole};

        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(Some("Compacted")).await.unwrap();

        let mut summary = Message::assistant(conv.id, "[Conversation summary]");
        summary.turn = 4;
        let mut user = Message::user(conv.id, "run both");
        user.turn = 5;
        let mut assistant = Message::assistant(conv.id, "");
        assistant.turn = 6;
        assistant.tool_calls_json = Some(
            serde_json::json!([
                {"name": "bash", "params": {}, "id": "c1"},
                {"name": "bash", "params": {}, "id": "c2"},
            ])
            .to_string(),
        );
        let mut rows = vec![summary, user, assistant];
        for (id, out) in [("c1", "A"), ("c2", "B")] {
            let mut m = Message::new(conv.id, MessageRole::Tool, out);
            m.turn = 6;
            m.skill_name = Some("bash".into());
            m.tool_call_id = Some(id.into());
            m.tool_status = Some("ok".into());
            m.tool_data_json =
                Some(serde_json::json!({"kind": "terminal", "stdout": out}).to_string());
            rows.push(m);
        }
        for m in &rows {
            store.save_message(m).await.unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{}", conv.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        let tc = json["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|m| m["tool_calls"].as_array().cloned())
            .expect("kept assistant message carries tool_calls");
        for (call, (id, out)) in tc.iter().zip([("c1", "A"), ("c2", "B")]) {
            assert_eq!(call["tool_call_id"], id);
            assert_eq!(call["result"], out);
            assert_eq!(call["data"]["stdout"], out);
        }
    }

    #[tokio::test]
    async fn get_conversation_exposes_finish_reason_only_when_set() {
        use assistant_core::types::conversation::{Message, MessageFinishReason};

        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(Some("Test")).await.unwrap();
        let user = Message::user(conv.id, "hi");
        let mut stopped = Message::assistant(conv.id, "Partial answer");
        stopped.turn = 1;
        stopped.finish_reason = Some(MessageFinishReason::Cancelled);
        let mut plain = Message::assistant(conv.id, "Full answer");
        plain.turn = 2;
        for m in [&user, &stopped, &plain] {
            store.save_message(m).await.unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{}", conv.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert!(
            messages[0].get("finish_reason").is_none(),
            "user message omits finish_reason"
        );
        assert_eq!(messages[1]["finish_reason"], "cancelled");
        assert!(
            messages[2].get("finish_reason").is_none(),
            "ordinary assistant message omits finish_reason"
        );
    }

    #[tokio::test]
    async fn get_conversation_projects_only_the_callers_feedback() {
        use assistant_core::types::conversation::Message;
        use assistant_storage::{MessageFeedbackRating, MessageFeedbackStore as _};

        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(Some("Test")).await.unwrap();
        let user = Message::user(conv.id, "hi");
        let mut rated = Message::assistant(conv.id, "Rated answer");
        rated.turn = 1;
        let mut unrated = Message::assistant(conv.id, "Unrated answer");
        unrated.turn = 2;
        let mut other_only = Message::assistant(conv.id, "Rated by someone else");
        other_only.turn = 3;
        for m in [&user, &rated, &unrated, &other_only] {
            store.save_message(m).await.unwrap();
        }

        let feedback = storage.message_feedback_store();
        feedback
            .set(
                conv.id,
                rated.id,
                "system",
                MessageFeedbackRating::Negative,
                Some("too long"),
            )
            .await
            .unwrap();
        feedback
            .set(
                conv.id,
                other_only.id,
                "someone-else",
                MessageFeedbackRating::Positive,
                None,
            )
            .await
            .unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{}", conv.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 4);
        assert!(
            messages[0].get("feedback").is_none(),
            "user message omits feedback"
        );
        assert_eq!(messages[1]["feedback"]["rating"], "negative");
        assert_eq!(messages[1]["feedback"]["comment"], "too long");
        assert!(
            messages[1]["feedback"]["updated_at"].as_str().is_some(),
            "feedback carries updated_at"
        );
        assert!(
            messages[2].get("feedback").is_none(),
            "unrated message omits feedback"
        );
        assert!(
            messages[3].get("feedback").is_none(),
            "another user's vote is not visible"
        );
    }

    #[tokio::test]
    async fn get_conversation_feedback_omits_absent_comment() {
        use assistant_core::types::conversation::Message;
        use assistant_storage::{MessageFeedbackRating, MessageFeedbackStore as _};

        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(Some("Test")).await.unwrap();
        let reply = Message::assistant(conv.id, "Answer");
        store.save_message(&reply).await.unwrap();
        storage
            .message_feedback_store()
            .set(
                conv.id,
                reply.id,
                "system",
                MessageFeedbackRating::Positive,
                None,
            )
            .await
            .unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{}", conv.id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let json = body_json(resp.into_body()).await;
        let feedback = &json["messages"][0]["feedback"];
        assert_eq!(feedback["rating"], "positive");
        assert!(feedback.get("comment").is_none(), "no comment key");
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

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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

    #[tokio::test]
    async fn patch_conversation_bad_uuid_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/conversations/not-a-uuid")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"title":"x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn patch_conversation_unknown_id_returns_404() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let id = uuid::Uuid::new_v4();
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/conversations/{id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"title":"x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- PATCH validation, pin and error envelopes -----------------------------

    /// Send `PATCH /conversations/{id}` as the default system caller.
    async fn patch(
        state: super::super::ApiState,
        id: &str,
        body: &str,
    ) -> (StatusCode, serde_json::Value) {
        patch_as(state, assistant_core::auth::AuthContext::system(), id, body).await
    }

    async fn patch_as(
        state: super::super::ApiState,
        auth: assistant_core::auth::AuthContext,
        id: &str,
        body: &str,
    ) -> (StatusCode, serde_json::Value) {
        let resp = app_with_auth(state, auth)
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/conversations/{id}"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = body_bytes(resp.into_body()).await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }

    fn caller(
        user: &str,
        role: assistant_core::identity::Role,
    ) -> assistant_core::auth::AuthContext {
        let mut ctx = assistant_core::auth::AuthContext::system();
        ctx.user_id = assistant_core::identity::UserId::from(user);
        ctx.space_roles = std::collections::HashMap::from([(
            assistant_core::identity::SpaceId::from("default"),
            role,
        )]);
        ctx
    }

    #[tokio::test]
    async fn patch_trims_the_title() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await;
        let (status, json) = patch(state, &id.to_string(), r#"{"title":"  Trip plan  "}"#).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["title"], "Trip plan", "title must be trimmed");
    }

    #[tokio::test]
    async fn patch_rejects_invalid_bodies_with_400_and_error_key() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await.to_string();
        let too_long = format!(r#"{{"title":"{}"}}"#, "é".repeat(201));
        let cases = [
            ("blank title", r#"{"title":"   "}"#.to_owned()),
            ("title over 200 chars", too_long),
            ("empty object", "{}".to_owned()),
            ("wrong-typed pinned", r#"{"pinned":"yes"}"#.to_owned()),
            ("malformed JSON", "{not json".to_owned()),
        ];
        for (name, body) in cases {
            let (status, json) = patch(state.clone(), &id, &body).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{name}");
            assert!(
                json["error"].is_string(),
                "{name}: missing error key: {json}"
            );
        }
        let (status, json) = patch(state, "not-a-uuid", r#"{"title":"x"}"#).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "invalid uuid");
        assert!(json["error"].is_string(), "invalid uuid: missing error key");
    }

    #[tokio::test]
    async fn patch_accepts_a_title_of_exactly_200_chars() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await.to_string();
        let body = format!(r#"{{"title":"{}"}}"#, "é".repeat(200));
        let (status, _) = patch(state, &id, &body).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "200 two-byte chars are within the limit"
        );
    }

    #[tokio::test]
    async fn patch_pinned_returns_pinned_at_and_unpinned_omits_it() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await.to_string();
        let (_, before) = patch(state.clone(), &id, r#"{"title":"x"}"#).await;
        assert!(
            before.get("pinned_at").is_none(),
            "unset pinned_at must be absent: {before}"
        );

        let (status, pinned) = patch(state.clone(), &id, r#"{"pinned":true}"#).await;
        assert_eq!(status, StatusCode::OK);
        assert!(
            pinned["pinned_at"].is_string(),
            "pinned_at missing: {pinned}"
        );

        let (_, unpinned) = patch(state, &id, r#"{"pinned":false}"#).await;
        assert!(
            unpinned.get("pinned_at").is_none(),
            "unpin must drop pinned_at: {unpinned}"
        );
    }

    #[tokio::test]
    async fn patch_title_and_pinned_together_emit_one_upserted() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await;
        let broadcaster = state
            .conversation_broadcasters
            .for_space("default", "default");
        let mut rx = broadcaster.subscribe();
        let (status, _) = patch(state, &id.to_string(), r#"{"title":"Both","pinned":true}"#).await;
        assert_eq!(status, StatusCode::OK);

        let first = rx.try_recv().expect("one upserted event");
        match first {
            assistant_storage::ConversationEvent::Upserted(rec) => {
                assert_eq!(rec.title.as_deref(), Some("Both"));
                assert!(rec.pinned_at.is_some());
            }
            other => panic!("expected Upserted, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "title + pinned must emit exactly one event"
        );
    }

    #[tokio::test]
    async fn patch_another_users_conversation_returns_404_with_error_key() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await.to_string();
        let other = caller("mallory", assistant_core::identity::Role::Member);
        let (status, json) = patch_as(state, other, &id, r#"{"title":"x"}"#).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json["error"].is_string(), "missing error key: {json}");
    }

    #[tokio::test]
    async fn patch_by_read_only_member_returns_403_with_error_key() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await.to_string();
        let viewer = caller("system", assistant_core::identity::Role::Viewer);
        let (status, json) = patch_as(state, viewer, &id, r#"{"title":"x"}"#).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(json["error"].is_string(), "missing error key: {json}");
    }

    #[tokio::test]
    async fn rename_does_not_reorder_the_list() {
        let (state, storage) = event_log_state().await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let older = store.create_conversation(Some("Older")).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        store.create_conversation(Some("Newer")).await.unwrap();

        let (status, _) = patch(
            state.clone(),
            &older.id.to_string(),
            r#"{"title":"Renamed"}"#,
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/conversations")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let list = body_json(resp.into_body()).await;
        let titles: Vec<&str> = list
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["title"].as_str().unwrap())
            .collect();
        assert_eq!(
            titles,
            ["Newer", "Renamed"],
            "a rename must not move the row"
        );
    }

    struct StubTts;

    #[async_trait::async_trait]
    impl assistant_transcription::TtsProvider for StubTts {
        fn name(&self) -> &str {
            "stub"
        }

        async fn synthesize(
            &self,
            _request: assistant_transcription::TtsRequest,
        ) -> anyhow::Result<assistant_transcription::TtsResult> {
            Ok(assistant_transcription::TtsResult {
                audio_data: b"audio".to_vec(),
                mime_type: "audio/mpeg".into(),
            })
        }
    }

    #[tokio::test]
    async fn missing_conversation_404_is_a_json_envelope_on_every_route() {
        let (state, _) = event_log_state().await;
        let state = state.with_tts_provider(std::sync::Arc::new(StubTts));
        let missing = Uuid::new_v4();
        let (multipart_type, multipart) = multipart_body("file", "a.png", "image/png", &tiny_png());
        let json_type = "application/json".to_owned();
        let json_body = br#"{"command":"help","message":"hi"}"#.to_vec();
        let routes = [
            (
                "GET",
                format!("/conversations/{missing}"),
                &json_type,
                &json_body,
            ),
            (
                "POST",
                format!("/conversations/{missing}/command"),
                &json_type,
                &json_body,
            ),
            (
                "POST",
                format!("/conversations/{missing}/attachments"),
                &multipart_type,
                &multipart,
            ),
            (
                "GET",
                format!("/conversations/{missing}/turns/{}/status", Uuid::new_v4()),
                &json_type,
                &json_body,
            ),
            (
                "POST",
                format!("/conversations/{missing}/messages"),
                &json_type,
                &json_body,
            ),
            (
                "GET",
                format!("/messages/{missing}/audio"),
                &json_type,
                &json_body,
            ),
        ];
        for (method, uri, content_type, body) in routes {
            let resp = app(state.clone())
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(&uri)
                        .header("Content-Type", content_type.as_str())
                        .body(Body::from(body.clone()))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = resp.status();
            let bytes = body_bytes(resp.into_body()).await;
            let json: serde_json::Value = serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| panic!("{method} {uri}: body is not JSON: {bytes:?}"));
            assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
            assert!(
                json["error"].is_string(),
                "{method} {uri}: missing error key"
            );
        }
    }

    #[tokio::test]
    async fn delete_conversation_bad_uuid_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/conversations/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_conversation_with_empty_body_returns_201_with_untitled() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations")
                    .header("Content-Type", "application/json")
                    // Completely absent body field — serde defaults kick in.
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["title"], "Untitled");
    }

    #[tokio::test]
    async fn create_conversation_malformed_json_returns_400_or_422() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from("{not valid"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.status().is_client_error(),
            "expected 4xx; got {}",
            resp.status()
        );
    }

    // -- Runtime agent_id switch -----------------------------------------------

    #[tokio::test]
    async fn list_reflects_agent_id_change_at_runtime() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;

        // Seed a conversation for "alice".
        let store_alice = SqliteConversationStore::for_agent(storage.pool.clone(), "alice")
            .with_user_id("system");
        store_alice
            .create_conversation(Some("Alice conv"))
            .await
            .unwrap();

        // Switch the shared agent ID to "alice" at runtime.
        *state.agent_id.write().await = "alice".to_string();

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
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "Alice conv");
    }
    // -- stream_conversations tests --------------------------------------------

    #[tokio::test]
    async fn stream_conversations_returns_snapshot() {
        let (state, storage) = event_log_state().await;

        // Create a conversation so the snapshot is non-empty.
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        store.create_conversation(Some("Hello")).await.unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/conversations/stream")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);

        // Should contain a snapshot event.
        assert!(
            text.contains("event: snapshot"),
            "response should contain snapshot event, got: {text}"
        );
        assert!(
            text.contains("Hello"),
            "snapshot should contain the conversation title"
        );
    }

    #[tokio::test]
    async fn stream_conversations_forwards_upserted_delta() {
        let (state, storage) = event_log_state().await;
        let broadcaster = state
            .conversation_broadcasters
            .for_space("default", "default");
        let pool = storage.pool.clone();

        // Spawn the SSE stream reader in a task.
        let app = app(state);
        let handle = tokio::spawn(async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .uri("/conversations/stream")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            body_bytes(resp.into_body()).await
        });

        // Give the stream a moment to subscribe.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Create a conversation through a broadcaster-wired store.
        let store = SqliteConversationStore::for_agent(pool, "default")
            .with_user_id("system")
            .with_broadcaster(broadcaster);
        store.create_conversation(Some("Delta Test")).await.unwrap();

        // Give the stream a moment to forward the event, then drop broadcaster
        // to close the channel so the SSE task finishes.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        drop(store);

        let body = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("stream should finish within timeout")
            .expect("stream task should not panic");
        let text = String::from_utf8_lossy(&body);

        assert!(
            text.contains("event: upserted"),
            "stream should contain upserted event, got: {text}"
        );
        assert!(
            text.contains("Delta Test"),
            "upserted event should contain the conversation title"
        );
    }

    #[tokio::test]
    async fn stream_snapshot_and_upserted_carry_pinned_at() {
        let (state, storage) = event_log_state().await;
        let id = seed_conversation(&storage).await;
        let broadcaster = state
            .conversation_broadcasters
            .for_space("default", "default");
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system")
            .with_broadcaster(broadcaster);
        store
            .update_metadata(
                id,
                assistant_storage::ConversationPatch {
                    title: None,
                    pinned: Some(true),
                },
            )
            .await
            .unwrap();

        // A second pin-state change arrives as a delta after the snapshot.
        let app = app(state);
        let handle = tokio::spawn(async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .uri("/conversations/stream")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            body_bytes(resp.into_body()).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        store
            .update_metadata(
                id,
                assistant_storage::ConversationPatch {
                    title: Some("Renamed".into()),
                    pinned: None,
                },
            )
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        drop(store);

        let body = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("stream should finish")
            .expect("stream task should not panic");
        let text = String::from_utf8_lossy(&body);
        let events: Vec<&str> = text.split("\n\n").filter(|e| !e.is_empty()).collect();
        assert!(
            events.len() >= 2,
            "expected snapshot and upserted, got: {text}"
        );
        for event in &events[..2] {
            assert!(
                event.contains("\"pinned_at\""),
                "event lacks pinned_at: {event}"
            );
        }
    }

    #[tokio::test]
    async fn test_update_title_broadcasts_upserted_with_new_title() {
        // Closes the loop the title-generator worker depends on: when the
        // worker calls update_title on an existing conversation, an Upserted
        // event MUST be broadcast carrying the new title so any subscribed
        // UI re-renders without a refresh.  The wire-level forward to SSE
        // is covered by `stream_conversations_forwards_upserted_delta`.
        let (state, storage) = event_log_state().await;
        let broadcaster = state
            .conversation_broadcasters
            .for_space("default", "default");
        let pool = storage.pool.clone();

        let store = SqliteConversationStore::for_agent(pool, "default")
            .with_user_id("system")
            .with_broadcaster(broadcaster.clone());
        let conv = store.create_conversation(None).await.unwrap();
        let conv_id = conv.id;

        // Subscribe AFTER create so we only capture the update.
        let mut rx = broadcaster.subscribe();
        store
            .update_title(conv_id, "Generated By Worker")
            .await
            .unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("broadcast must arrive within 2s")
            .expect("recv must not error");

        match event {
            assistant_storage::ConversationEvent::Upserted(record) => {
                assert_eq!(record.id, conv_id);
                assert_eq!(record.title.as_deref(), Some("Generated By Worker"));
                assert!(
                    record.title_locked,
                    "broadcast after update_title must carry title_locked = true",
                );
            }
            other => panic!("expected Upserted, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn stream_conversations_filters_by_agent_id() {
        use assistant_storage::ConversationEvent;

        let (state, _storage) = event_log_state().await;
        let broadcaster = state
            .conversation_broadcasters
            .for_space("default", "default");

        // Stream filtered to agent "work".
        let app = app(state);
        let handle = tokio::spawn(async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .uri("/conversations/stream?agent_id=work")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            body_bytes(resp.into_body()).await
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Emit an event for agent "default" — should be filtered out.
        broadcaster.emit(ConversationEvent::Upserted(
            assistant_storage::ConversationRecord {
                id: Uuid::new_v4(),
                agent_id: "default".to_string(),
                title: Some("DefaultConv".to_string()),
                title_locked: true,
                user_id: Some("system".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                pinned_at: None,
            },
        ));

        // Emit an event for agent "work" — should pass through.
        broadcaster.emit(ConversationEvent::Upserted(
            assistant_storage::ConversationRecord {
                id: Uuid::new_v4(),
                agent_id: "work".to_string(),
                title: Some("WorkConv".to_string()),
                title_locked: true,
                user_id: Some("system".to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                pinned_at: None,
            },
        ));

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        drop(broadcaster);

        let body = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("timeout")
            .expect("no panic");
        let text = String::from_utf8_lossy(&body);

        assert!(
            !text.contains("DefaultConv"),
            "should NOT contain event for wrong agent, got: {text}"
        );
        assert!(
            text.contains("WorkConv"),
            "should contain event for filtered agent, got: {text}"
        );
    }

    #[tokio::test]
    async fn stream_conversations_e2e_create_and_delete() {
        let (state, _storage) = event_log_state().await;

        // Clone app router so we can issue multiple requests.
        let router = app(state);

        // Spawn the SSE stream reader in a background task.
        let stream_router = router.clone();
        let handle = tokio::spawn(async move {
            let resp = stream_router
                .oneshot(
                    Request::builder()
                        .uri("/conversations/stream")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::OK);
            body_bytes(resp.into_body()).await
        });

        // Give the stream time to subscribe.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Create a conversation via REST.
        let create_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/conversations")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"title":"E2E Conv"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let json = body_json(create_resp.into_body()).await;
        let conv_id = json["id"].as_str().expect("should have an id").to_string();

        // Give the stream time to forward the upserted event.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Delete the conversation via REST.
        let delete_resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/conversations/{conv_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(delete_resp.status(), StatusCode::NO_CONTENT);

        // Give the stream time to forward the deleted event, then drop
        // broadcaster indirectly by dropping the router.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        drop(router);

        let body = tokio::time::timeout(std::time::Duration::from_secs(5), handle)
            .await
            .expect("stream should finish within timeout")
            .expect("stream task should not panic");
        let text = String::from_utf8_lossy(&body);

        assert!(
            text.contains("event: upserted"),
            "stream should contain upserted event from REST create, got: {text}"
        );
        assert!(
            text.contains("E2E Conv"),
            "upserted event should contain conversation title"
        );
        assert!(
            text.contains("event: deleted"),
            "stream should contain deleted event from REST delete, got: {text}"
        );
        assert!(
            text.contains(&conv_id),
            "deleted event should contain the conversation ID"
        );
    }

    // -- ConversationSummary projection ----------------------------------------

    fn record_with_title(title: Option<&str>) -> assistant_storage::ConversationRecord {
        let created_at = Utc::now() - chrono::Duration::hours(2);
        assistant_storage::ConversationRecord {
            id: Uuid::new_v4(),
            agent_id: "default".to_string(),
            title: title.map(str::to_string),
            title_locked: false,
            user_id: Some("system".to_string()),
            created_at,
            updated_at: created_at + chrono::Duration::hours(1),
            pinned_at: None,
        }
    }

    #[test]
    fn summary_from_record_maps_null_title_to_untitled() {
        let record = record_with_title(None);

        let summary = super::ConversationSummary::from(&record);

        assert_eq!(
            summary.title, "Untitled",
            "NULL title must display as Untitled"
        );
        assert_eq!(summary.id, record.id, "id must be copied");
        assert_eq!(
            summary.created_at, record.created_at,
            "created_at must be copied"
        );
        assert_eq!(
            summary.updated_at, record.updated_at,
            "updated_at must be copied"
        );
    }

    #[test]
    fn summary_from_record_keeps_explicit_title() {
        let record = record_with_title(Some("Trip planning"));

        let summary = super::ConversationSummary::from(&record);

        assert_eq!(
            summary.title, "Trip planning",
            "explicit title must be kept"
        );
    }
}
