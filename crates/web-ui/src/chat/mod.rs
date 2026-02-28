//! Chat interface: conversation list + real-time chat panel.
//!
//! All HTML is rendered via Askama templates under `templates/chat/`.
//! Dynamic interactions use htmx 2 — the server returns HTML fragments
//! for partial page updates.

use std::convert::Infallible;
use std::sync::Arc;

use askama::Template;
use assistant_core::{Message, MessageRole};
use assistant_llm::{ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse};
use assistant_storage::{ConversationRecord, ConversationStore};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, Sse},
        Html, IntoResponse, Redirect, Response,
    },
    routing::{delete, get, post},
    Form, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;
use uuid::Uuid;

use crate::common;

// -- State -------------------------------------------------------------------

/// Default system prompt for chat conversations.
const SYSTEM_PROMPT: &str = "You are a helpful assistant. Answer clearly and concisely.";

/// Shared state for chat route handlers.
#[derive(Clone)]
pub struct ChatState {
    pub pool: SqlitePool,
    pub llm: Arc<dyn LlmProvider>,
}

// -- View models -------------------------------------------------------------

/// A conversation entry shown in the sidebar list.
pub struct ConversationView {
    pub id: String,
    pub title: String,
    pub time_ago: String,
}

/// A single rendered message in the chat panel.
pub struct MessageView {
    pub role_class: &'static str,
    pub role_label: &'static str,
    pub content: String,
    pub time: String,
    pub tool_calls: Vec<ToolCallView>,
}

/// A collapsed tool-call block inside an assistant message.
pub struct ToolCallView {
    pub name: String,
    pub summary: String,
    pub is_success: bool,
}

/// The currently active conversation with its message history.
pub struct ActiveConversationView {
    pub id: String,
    pub title: String,
    pub messages: Vec<MessageView>,
}

// -- Templates ---------------------------------------------------------------

/// Full chat page (extends base.html).
#[derive(Template)]
#[template(path = "chat/page.html")]
struct ChatPageTemplate {
    active_page: &'static str,
    conversations: Vec<ConversationView>,
    active_conversation: Option<ActiveConversationView>,
    active_id: Option<String>,
}

/// htmx partial: chat panel content for a selected conversation.
#[derive(Template)]
#[template(path = "chat/panel.html")]
struct ChatPanelTemplate {
    id: String,
    title: String,
    messages: Vec<MessageView>,
}

/// htmx partial: a single message (appended after sending).
#[derive(Template)]
#[template(path = "chat/message.html")]
struct MessageTemplate {
    msg: MessageView,
}

/// htmx partial: conversation list items.
#[derive(Template)]
#[template(path = "chat/conversation_list.html")]
struct ConversationListTemplate {
    conversations: Vec<ConversationView>,
    active_id: Option<String>,
}

/// htmx partial: streaming assistant response skeleton.
///
/// Contains SSE connection attributes so the browser opens a server-sent
/// events stream that progressively fills in the assistant's reply.
#[derive(Template)]
#[template(path = "chat/streaming.html")]
struct StreamingTemplate {
    id: String,
    turn: i64,
}

// -- Router ------------------------------------------------------------------

/// Build the chat sub-router.  Mounted under the auth-protected scope.
pub fn chat_router() -> Router<ChatState> {
    Router::new()
        .route("/chat", get(chat_page))
        .route("/chat/conversations", get(conversation_list))
        .route("/chat/new", post(new_conversation))
        .route("/chat/{id}", get(load_conversation))
        .route("/chat/{id}/send", post(send_message))
        .route("/chat/{id}/stream", get(stream_response))
        .route("/chat/{id}", delete(delete_conversation))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /chat` — full page with conversation list and empty/selected state.
async fn chat_page(State(state): State<ChatState>) -> Response {
    let store = ConversationStore::new(state.pool);
    let convs = store.list_conversations().await.unwrap_or_default();

    let conversations = convs.iter().map(conv_to_view).collect();

    let tmpl = ChatPageTemplate {
        active_page: "chat",
        conversations,
        active_conversation: None,
        active_id: None,
    };

    render_template(tmpl)
}

/// `GET /chat/conversations?q=...` — htmx partial: filtered conversation list.
#[derive(Deserialize, Default)]
struct ConvSearchQuery {
    q: Option<String>,
}

async fn conversation_list(
    State(state): State<ChatState>,
    Query(query): Query<ConvSearchQuery>,
) -> Response {
    let store = ConversationStore::new(state.pool);
    let mut convs = store.list_conversations().await.unwrap_or_default();

    // Client-side search filter
    if let Some(ref q) = query.q {
        let q_lower = q.to_lowercase();
        if !q_lower.is_empty() {
            convs.retain(|c| {
                c.title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&q_lower)
            });
        }
    }

    let conversations = convs.iter().map(conv_to_view).collect();

    let tmpl = ConversationListTemplate {
        conversations,
        active_id: None,
    };

    render_template(tmpl)
}

/// `POST /chat/new` — create a new conversation.
///
/// For htmx requests: returns an updated conversation list fragment.
/// For full-page requests: redirects to the new conversation.
async fn new_conversation(State(state): State<ChatState>, headers: HeaderMap) -> Response {
    let store = ConversationStore::new(state.pool.clone());

    let conv = match store.create_conversation(Some("New Chat")).await {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to create conversation: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create conversation",
            )
                .into_response();
        }
    };

    let is_htmx = headers.get("HX-Request").is_some();

    if is_htmx {
        // Return just the new conversation item so htmx can prepend it
        let view = conv_to_view(&conv);
        let tmpl = ConversationListTemplate {
            active_id: Some(view.id.clone()),
            conversations: {
                let all = store.list_conversations().await.unwrap_or_default();
                all.iter().map(conv_to_view).collect()
            },
        };
        let mut resp = render_template(tmpl);
        // Tell htmx to also load the new conversation into the chat panel
        resp.headers_mut().insert(
            "HX-Trigger-After-Swap",
            format!("{{\"loadChat\": \"{}\"}}", conv.id)
                .parse()
                .unwrap(),
        );
        resp
    } else {
        Redirect::to(&format!("/chat/{}", conv.id)).into_response()
    }
}

/// `GET /chat/{id}` — load a conversation.
///
/// htmx request: returns chat panel fragment.
/// Full-page request: returns the full page with that conversation selected.
async fn load_conversation(
    State(state): State<ChatState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let store = ConversationStore::new(state.pool.clone());

    let conv = match store.get_conversation(conv_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return (StatusCode::NOT_FOUND, "Conversation not found").into_response(),
        Err(e) => {
            warn!("Failed to load conversation: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let history = store.load_history(conv_id).await.unwrap_or_default();
    let messages: Vec<MessageView> = history.iter().map(msg_to_view).collect();
    let title = conv.title.clone().unwrap_or_else(|| "Untitled".into());

    let is_htmx = headers.get("HX-Request").is_some();

    if is_htmx {
        let tmpl = ChatPanelTemplate {
            id: id.clone(),
            title,
            messages,
        };
        render_template(tmpl)
    } else {
        // Full page with this conversation selected
        let convs = store.list_conversations().await.unwrap_or_default();
        let conversations = convs.iter().map(conv_to_view).collect();

        let tmpl = ChatPageTemplate {
            active_page: "chat",
            conversations,
            active_conversation: Some(ActiveConversationView {
                id: id.clone(),
                title,
                messages,
            }),
            active_id: Some(id),
        };
        render_template(tmpl)
    }
}

/// `POST /chat/{id}/send` — send a user message.
///
/// Returns the rendered message as an htmx fragment to append.
#[derive(Deserialize)]
struct SendMessageForm {
    message: String,
}

async fn send_message(
    State(state): State<ChatState>,
    Path(id): Path<String>,
    Form(form): Form<SendMessageForm>,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let content = form.message.trim().to_string();
    if content.is_empty() {
        return (StatusCode::BAD_REQUEST, "Message cannot be empty").into_response();
    }

    let store = ConversationStore::new(state.pool.clone());

    // Determine the next turn number
    let last = store.last_messages(conv_id, 1).await.unwrap_or_default();
    let next_turn = last.last().map(|m| m.turn + 1).unwrap_or(1);

    // Save user message
    let mut user_msg = Message::user(conv_id, &content);
    user_msg.turn = next_turn;
    if let Err(e) = store.save_message(&user_msg).await {
        warn!("Failed to save message: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save message").into_response();
    }

    // If this is the first message, update the conversation title
    if next_turn == 1 {
        let title = if content.len() > 60 {
            format!("{}...", &content[..57])
        } else {
            content.clone()
        };
        // Direct SQL update since ConversationStore doesn't have update_title
        let _ = sqlx::query("UPDATE conversations SET title = ?1 WHERE id = ?2")
            .bind(&title)
            .bind(conv_id.to_string())
            .execute(&state.pool)
            .await;
    }

    // Render the user message fragment
    let user_view = msg_to_view(&user_msg);
    let user_html = MessageTemplate { msg: user_view }
        .render()
        .unwrap_or_default();

    // Render the streaming skeleton — the browser will open an SSE connection
    // to progressively fill in the assistant's reply.
    let streaming_html = StreamingTemplate {
        id: id.clone(),
        turn: next_turn,
    }
    .render()
    .unwrap_or_default();

    // Return user bubble + streaming skeleton so htmx appends both
    Html(format!("{user_html}{streaming_html}")).into_response()
}

// -- SSE streaming -----------------------------------------------------------

/// Query parameters for the streaming endpoint.
#[derive(Deserialize)]
struct StreamQuery {
    turn: i64,
}

/// `GET /chat/{id}/stream?turn=N` — SSE endpoint that streams the assistant's
/// response token-by-token.
///
/// The client connects via the htmx SSE extension.  Each token is sent as an
/// `event: token` with the HTML-escaped text as data.  When generation is
/// complete, the full rendered message HTML is sent as `event: done`, which
/// replaces the streaming skeleton via `outerHTML` swap.
async fn stream_response(
    State(state): State<ChatState>,
    Path(id): Path<String>,
    Query(params): Query<StreamQuery>,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let store = ConversationStore::new(state.pool.clone());

    // Load full conversation history (includes the just-saved user message)
    let history = match store.load_history(conv_id).await {
        Ok(h) => h,
        Err(e) => {
            warn!("Failed to load history for streaming: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let chat_history = build_chat_history(&history);
    let user_turn = params.turn;

    // Channel for SSE events sent to the client.
    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);

    // Channel for LLM tokens.
    let (token_tx, mut token_rx) = mpsc::channel::<String>(64);

    // Spawn the LLM streaming call.
    let llm = state.llm.clone();
    let result_tx = {
        let (tx, rx) = tokio::sync::oneshot::channel::<anyhow::Result<LlmResponse>>();
        tokio::spawn(async move {
            let result = llm
                .chat_streaming(SYSTEM_PROMPT, &chat_history, &[], Some(token_tx))
                .await;
            let _ = tx.send(result);
        });
        rx
    };

    // Spawn a task that reads tokens from the LLM, sends SSE events, and
    // emits the final "done" event with the rendered message.
    let pool = state.pool.clone();
    tokio::spawn(async move {
        let mut full_text = String::new();

        // Forward each token as an SSE event.
        while let Some(token) = token_rx.recv().await {
            full_text.push_str(&token);
            let escaped = common::html_escape(&token);
            let event = Event::default().event("token").data(escaped);
            if sse_tx.send(Ok(event)).await.is_err() {
                // Client disconnected
                return;
            }
        }

        // Get the authoritative response text from the LLM result.
        let reply_text = match result_tx.await {
            Ok(Ok(LlmResponse::FinalAnswer(text, _))) => text,
            Ok(Ok(LlmResponse::Thinking(text, _))) => text,
            Ok(Ok(LlmResponse::ToolCalls(_, _))) => {
                "(The model tried to use tools, but none are available in chat mode.)".to_string()
            }
            Ok(Err(e)) => {
                // LLM call failed — use whatever we collected, or an error msg
                if full_text.is_empty() {
                    format!("Sorry, I couldn't generate a response: {e}")
                } else {
                    full_text.clone()
                }
            }
            Err(_) => {
                // Oneshot dropped — use collected tokens
                full_text.clone()
            }
        };

        // Save the assistant message to the database.
        let mut assistant_msg = Message::assistant(conv_id, &reply_text);
        assistant_msg.turn = user_turn + 1;
        let save_store = ConversationStore::new(pool);
        if let Err(e) = save_store.save_message(&assistant_msg).await {
            warn!("Failed to save assistant message: {}", e);
        }

        // Render the final message HTML and send as the "done" event.
        // This replaces the streaming skeleton via outerHTML swap.
        let view = msg_to_view(&assistant_msg);
        let html = MessageTemplate { msg: view }.render().unwrap_or_default();
        let done = Event::default().event("done").data(html);
        let _ = sse_tx.send(Ok(done)).await;
    });

    Sse::new(ReceiverStream::new(sse_rx)).into_response()
}

/// Convert stored [`Message`] history to [`ChatHistoryMessage`] for the LLM.
fn build_chat_history(messages: &[Message]) -> Vec<ChatHistoryMessage> {
    messages
        .iter()
        .filter_map(|m| match m.role {
            MessageRole::User => Some(ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: m.content.clone(),
            }),
            MessageRole::Assistant => Some(ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: m.content.clone(),
            }),
            MessageRole::System => Some(ChatHistoryMessage::Text {
                role: ChatRole::System,
                content: m.content.clone(),
            }),
            // Skip tool messages — not relevant for simple chat
            MessageRole::Tool => None,
        })
        .collect()
}

/// `DELETE /chat/{id}` — delete a conversation and redirect to chat.
async fn delete_conversation(
    State(state): State<ChatState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let store = ConversationStore::new(state.pool);
    if let Err(e) = store.delete_conversation(conv_id).await {
        warn!("Failed to delete conversation: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to delete conversation",
        )
            .into_response();
    }

    let is_htmx = headers.get("HX-Request").is_some();

    if is_htmx {
        // Tell htmx to redirect
        let mut resp = StatusCode::OK.into_response();
        resp.headers_mut()
            .insert("HX-Redirect", "/chat".parse().unwrap());
        resp
    } else {
        Redirect::to("/chat").into_response()
    }
}

// -- Helpers -----------------------------------------------------------------

/// Render an Askama template into an axum `Response`.
fn render_template(tmpl: impl Template) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            warn!("Template render error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// Convert a `ConversationRecord` to the view model used by templates.
fn conv_to_view(c: &ConversationRecord) -> ConversationView {
    ConversationView {
        id: c.id.to_string(),
        title: c.title.clone().unwrap_or_else(|| "Untitled".into()),
        time_ago: format_time_ago(c.updated_at),
    }
}

/// Convert a `Message` to the view model used by templates.
fn msg_to_view(m: &Message) -> MessageView {
    let (role_class, role_label) = match m.role {
        MessageRole::User => ("msg-user", "You"),
        MessageRole::Assistant => ("msg-assistant", "Assistant"),
        MessageRole::System => ("msg-system", "System"),
        MessageRole::Tool => ("msg-assistant", "Tool"),
    };

    // Parse tool calls from JSON if present
    let tool_calls = m
        .tool_calls_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<Vec<serde_json::Value>>(json).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|tc| ToolCallView {
            name: tc
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string(),
            summary: tc
                .get("result")
                .or_else(|| tc.get("arguments"))
                .map(|v| {
                    if let Some(s) = v.as_str() {
                        truncate_str(s, 500).to_string()
                    } else {
                        let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
                        truncate_str(&pretty, 500).to_string()
                    }
                })
                .unwrap_or_default(),
            is_success: tc
                .get("status")
                .and_then(|v| v.as_str())
                .map(|s| s != "error")
                .unwrap_or(true),
        })
        .collect();

    MessageView {
        role_class,
        role_label,
        content: m.content.clone(),
        time: format_time(m.created_at),
        tool_calls,
    }
}

/// Format a timestamp as a human-readable relative time.
fn format_time_ago(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(dt);

    if diff.num_seconds() < 60 {
        "just now".into()
    } else if diff.num_minutes() < 60 {
        let m = diff.num_minutes();
        if m == 1 {
            "1 min ago".into()
        } else {
            format!("{m} min ago")
        }
    } else if diff.num_hours() < 24 {
        let h = diff.num_hours();
        if h == 1 {
            "1 hr ago".into()
        } else {
            format!("{h} hr ago")
        }
    } else if diff.num_days() < 7 {
        let d = diff.num_days();
        if d == 1 {
            "Yesterday".into()
        } else {
            format!("{d} days ago")
        }
    } else {
        dt.format("%b %d").to_string()
    }
}

/// Format a timestamp as a short clock time.
fn format_time(dt: DateTime<Utc>) -> String {
    dt.format("%l:%M %p").to_string().trim().to_string()
}

/// Truncate a string to at most `max` characters, appending "..." if needed.
fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        // Find a safe char boundary
        let mut end = max;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        &s[..end]
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time_ago_just_now() {
        let now = Utc::now();
        assert_eq!(format_time_ago(now), "just now");
    }

    #[test]
    fn test_format_time_ago_minutes() {
        let dt = Utc::now() - chrono::Duration::minutes(5);
        assert_eq!(format_time_ago(dt), "5 min ago");
    }

    #[test]
    fn test_format_time_ago_yesterday() {
        let dt = Utc::now() - chrono::Duration::days(1);
        assert_eq!(format_time_ago(dt), "Yesterday");
    }

    #[test]
    fn test_truncate_str_short() {
        assert_eq!(truncate_str("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_str_long() {
        let s = "a".repeat(100);
        assert_eq!(truncate_str(&s, 10).len(), 10);
    }
}
