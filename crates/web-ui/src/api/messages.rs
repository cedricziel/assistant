//! Message endpoints: send streamed messages, quick (synchronous) messages,
//! voice messages (multipart audio → STT → assistant turn), and SSE replay
//! of run events.
//!
//! Endpoints:
//! - `POST /api/conversations/{id}/messages`         — streamed send
//! - `POST /api/conversations/{id}/voice`            — voice upload
//! - `POST /api/quick-message`                       — synchronous send
//! - `GET  /api/conversations/{id}/runs/{run_id}/events/stream` — run replay

use assistant_storage::PersonaStore as _;
use std::convert::Infallible;

use axum::Extension;
use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::Event;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::warn;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Action, ResourceKind};
use assistant_core::types::conversation::Interface;
use assistant_runtime::OrchestratorEvent;
use assistant_runtime::projection::{SseProjector, StreamProjector};
use assistant_storage::{ConversationStore, SqliteConversationStore};
use assistant_transcription::TranscriptionRequest;

use super::{ApiState, sse_response};

/// Whether the caller may post a message / start a turn — gated on the
/// `conversations:write` scope, with org admins bypassing. Trusted local callers
/// carry this via [`AuthContext::system`]; network callers resolve a real
/// `AuthContext` from the request.
fn caller_can_post(auth: &AuthContext) -> bool {
    auth.is_org_admin()
        || auth
            .scopes
            .iter()
            .any(|s| s.covers(&ResourceKind::Conversations, &Action::Write, None))
}

/// Body for `POST /api/conversations/{id}/messages`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = ApiSendMessageRequest)]
pub struct SendMessageRequest {
    /// The message text to send to the assistant.
    pub message: String,
    /// Optional attachment IDs to include with this message.
    #[serde(default)]
    pub attachment_ids: Vec<Uuid>,
}

/// Body for `POST /api/quick-message`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct QuickMessageRequest {
    /// The message text to send to the assistant.
    pub message: String,
    /// Optional persona ID to route the message to a specific persona.
    /// When absent, the server's active persona is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    /// Optional context text to include with the message (e.g. clipboard
    /// contents, file text).  Prepended to the user message for the LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Response for `POST /api/quick-message`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct QuickMessageResponse {
    /// The ID of the newly created conversation.
    pub conversation_id: Uuid,
    /// The ID of the persisted assistant message, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Uuid>,
    /// The full assistant response text.
    pub answer: String,
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
        (status = 403, description = "Missing conversations:write scope"),
        (status = 404, description = "Conversation not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn send_message(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Response {
    if !caller_can_post(&auth) {
        return (StatusCode::FORBIDDEN, "Missing conversations:write scope").into_response();
    }
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
    let store = SqliteConversationStore::for_agent(state.pool.clone(), &agent_id)
        .with_broadcaster(state.conversation_broadcaster.clone());

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

    // Title generation is owned by the title-generator worker (consumer of
    // `turn.result`), not this handler. See `crates/runtime/src/title_generator.rs`.

    // Generate a unique ID for this orchestrator run.
    let run_id = Uuid::new_v4();

    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    let (event_tx, mut event_rx) = mpsc::channel::<OrchestratorEvent>(64);

    state
        .orchestrator
        .register_token_sink(conv_id, event_tx)
        .await;

    let orchestrator = state.orchestrator.clone();
    let user_attachment_ids = body.attachment_ids;
    // Use `run_id` as the orchestrator's `request_id` so external cancel
    // calls — `POST .../turns/{run_id}/cancel` — can find the matching
    // CancellationToken via `Orchestrator::cancel_turn`.
    let turn_result_rx = {
        let (tx, rx) =
            tokio::sync::oneshot::channel::<anyhow::Result<assistant_runtime::TurnResult>>();
        let submit_request_id = run_id;
        tokio::spawn(async move {
            let result = orchestrator
                .submit_turn_with_request_id(
                    &auth,
                    submit_request_id,
                    &content,
                    conv_id,
                    Interface::Web,
                    None,
                    user_attachment_ids,
                )
                .await;
            let _ = tx.send(result);
        });
        rx
    };

    let push_dispatcher_for_sse = state.push_dispatcher.clone();
    let conv_id_for_push = conv_id;
    let event_store = state.event_store.clone();
    let run_broadcaster = state.run_broadcaster.clone();

    // Emit run_started event (sequence 0) to the durable log.
    let run_id_str = run_id.to_string();
    let run_started_payload = serde_json::json!({"run_id": run_id_str});
    if let Err(e) = event_store
        .append_event(
            &run_id_str,
            &conv_id.to_string(),
            0,
            "run_started",
            &run_started_payload,
        )
        .await
    {
        warn!("Failed to persist run_started event for run {run_id}: {e}");
    }

    // Register broadcast channel for live tailing.
    let broadcast_tx = run_broadcaster.start_run(run_id).await;

    // Emit run_started as first SSE event so the client learns the run_id.
    let run_started_sse = Event::default()
        .event("run_started")
        .data(run_started_payload.to_string())
        .id("0");
    if sse_tx.send(Ok(run_started_sse)).await.is_err() {
        run_broadcaster.finish_run(&run_id).await;
        return sse_response(sse_rx);
    }

    tokio::spawn(async move {
        let mut full_text = String::new();
        let mut seq: i64 = 1; // sequence 0 was run_started
        let conv_id_str = conv_id_for_push.to_string();

        // Throttle thinking persistence: accumulate tokens, flush every 20 tokens
        // or when a non-thinking event arrives. SSE is still sent immediately.
        let mut thinking_buf = String::new();
        let mut thinking_token_count: usize = 0;
        const THINKING_BATCH_SIZE: usize = 20;

        while let Some(orch_event) = event_rx.recv().await {
            // Serialization is owned by the shared projection layer; this loop
            // keeps only transport concerns (batching, persistence, push, seq).
            // A single OrchestratorEvent yields exactly one SSE frame today.
            let Some(frame) = SseProjector.project(&orch_event).into_iter().next() else {
                continue;
            };
            let is_thinking = frame.event == "thinking";

            // Flush accumulated thinking buffer before non-thinking events.
            if !is_thinking && !thinking_buf.is_empty() {
                let p = serde_json::json!({"content": thinking_buf});
                if let Err(e) = event_store
                    .append_event(&run_id_str, &conv_id_str, seq, "thinking", &p)
                    .await
                {
                    warn!("Failed to persist thinking batch seq={seq} for run {run_id}: {e}");
                }
                let live = assistant_storage::LiveEvent {
                    sequence: seq,
                    event_type: "thinking".to_string(),
                    payload: p,
                };
                let _ = broadcast_tx.send(live);
                seq += 1;
                thinking_buf.clear();
                thinking_token_count = 0;
            }

            // -- Side effects keyed off the projected frame (no enum match) --

            // Accumulate the assistant's visible text from token frames.
            if frame.event == "token" {
                full_text.push_str(&frame.sse_data);
            }

            // Fire push notifications for skill / error frames.
            if frame.event == "skill_complete"
                && let Some(ref dispatcher) = push_dispatcher_for_sse
            {
                let success = frame.log_payload["success"].as_bool().unwrap_or(false);
                let title = if success {
                    "Skill complete"
                } else {
                    "Skill failed"
                };
                let skill_name = frame.log_payload["skill_name"].as_str().unwrap_or_default();
                let summary = frame.log_payload["summary"].as_str().unwrap_or_default();
                let body = format!("{skill_name}: {summary}");
                let cid = conv_id_str.clone();
                let d = dispatcher.clone();
                tokio::spawn(async move {
                    if let Err(e) = d.send_to_all(title, &body, Some(&cid)).await {
                        warn!("Push (skill) failed: {e}");
                    }
                });
            } else if frame.event == "agent_error"
                && let Some(ref dispatcher) = push_dispatcher_for_sse
            {
                // `sse_data` is the raw error text for agent_error frames.
                let body = frame.sse_data.chars().take(80).collect::<String>();
                let cid = conv_id_str.clone();
                let d = dispatcher.clone();
                tokio::spawn(async move {
                    if let Err(e) = d.send_to_all("Assistant error", &body, Some(&cid)).await {
                        warn!("Push (agent error) failed: {e}");
                    }
                });
            }

            // Thinking frames stream immediately and persist in batches.
            if is_thinking {
                let e = Event::default()
                    .event("thinking")
                    .data(frame.sse_data.clone());
                if sse_tx.send(Ok(e)).await.is_err() {
                    break;
                }
                if let Some(content) = frame.log_payload["content"].as_str() {
                    thinking_buf.push_str(content);
                }
                thinking_token_count += 1;
                if thinking_token_count >= THINKING_BATCH_SIZE {
                    let batch_p = serde_json::json!({"content": thinking_buf});
                    if let Err(e) = event_store
                        .append_event(&run_id_str, &conv_id_str, seq, "thinking", &batch_p)
                        .await
                    {
                        warn!("Failed to persist thinking batch seq={seq} for run {run_id}: {e}");
                    }
                    let live = assistant_storage::LiveEvent {
                        sequence: seq,
                        event_type: "thinking".to_string(),
                        payload: batch_p,
                    };
                    let _ = broadcast_tx.send(live);
                    seq += 1;
                    thinking_buf.clear();
                    thinking_token_count = 0;
                }
                continue;
            }

            // Persist to durable log.
            if let Err(e) = event_store
                .append_event(
                    &run_id_str,
                    &conv_id_str,
                    seq,
                    &frame.event,
                    &frame.log_payload,
                )
                .await
            {
                warn!("Failed to persist event seq={seq} for run {run_id}: {e}");
            }
            // Broadcast to live tail subscribers.
            let live = assistant_storage::LiveEvent {
                sequence: seq,
                event_type: frame.event.clone(),
                payload: frame.log_payload.clone(),
            };
            let _ = broadcast_tx.send(live);
            let sse_event = Event::default()
                .event(frame.event.clone())
                .data(frame.sse_data)
                .id(seq.to_string());
            seq += 1;

            if sse_tx.send(Ok(sse_event)).await.is_err() {
                break;
            }
        }

        // Flush any remaining thinking tokens that didn't reach the batch threshold.
        if !thinking_buf.is_empty() {
            let p = serde_json::json!({"content": thinking_buf});
            if let Err(e) = event_store
                .append_event(&run_id_str, &conv_id_str, seq, "thinking", &p)
                .await
            {
                warn!("Failed to persist final thinking batch seq={seq} for run {run_id}: {e}");
            }
            let live = assistant_storage::LiveEvent {
                sequence: seq,
                event_type: "thinking".to_string(),
                payload: p,
            };
            let _ = broadcast_tx.send(live);
            seq += 1;
        }

        // Decide the terminal event: `done` for a successful turn, or
        // `agent_error` with a `cancelled` reason when the turn was aborted
        // via `Orchestrator::cancel_turn` (e.g. POST .../turns/{id}/cancel).
        // Any other Err is also surfaced as `agent_error` so SSE clients see
        // a structured terminal rather than a `done` with empty content.
        enum TerminalKind {
            Done,
            Cancelled,
            Errored(String),
        }
        let (reply_text, reply_message_id, reply_attachment_ids, terminal) =
            match turn_result_rx.await {
                Ok(Ok(result)) => (
                    result.answer,
                    result.message_id,
                    result.attachment_ids,
                    TerminalKind::Done,
                ),
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    let kind = if msg.contains(assistant_runtime::TURN_CANCELLED_MARKER) {
                        TerminalKind::Cancelled
                    } else {
                        TerminalKind::Errored(msg)
                    };
                    (full_text, None, vec![], kind)
                }
                Err(_) => (
                    full_text,
                    None,
                    vec![],
                    TerminalKind::Errored("submit_turn channel dropped".to_string()),
                ),
            };

        let (event_type, payload) = match &terminal {
            TerminalKind::Done => {
                let mut done_data = serde_json::json!({
                    "role": "assistant",
                    "content": reply_text,
                });
                if let Some(mid) = reply_message_id {
                    done_data["message_id"] = serde_json::Value::String(mid.to_string());
                }
                if !reply_attachment_ids.is_empty() {
                    done_data["attachment_ids"] = serde_json::json!(
                        reply_attachment_ids
                            .iter()
                            .map(|id| id.to_string())
                            .collect::<Vec<_>>()
                    );
                }
                ("done", done_data)
            }
            TerminalKind::Cancelled => (
                "agent_error",
                serde_json::json!({
                    "reason": "cancelled",
                    "message": "Turn cancelled by user",
                    "partial_content": reply_text,
                }),
            ),
            TerminalKind::Errored(msg) => (
                "agent_error",
                serde_json::json!({
                    "reason": "error",
                    "message": msg,
                    "partial_content": reply_text,
                }),
            ),
        };

        // Persist terminal event.
        if let Err(e) = event_store
            .append_event(&run_id_str, &conv_id_str, seq, event_type, &payload)
            .await
        {
            warn!("Failed to persist {event_type} event for run {run_id}: {e}");
        }
        let _ = broadcast_tx.send(assistant_storage::LiveEvent {
            sequence: seq,
            event_type: event_type.to_string(),
            payload: payload.clone(),
        });

        let terminal_sse = Event::default()
            .event(event_type)
            .data(payload.to_string())
            .id(seq.to_string());
        let _ = sse_tx.send(Ok(terminal_sse)).await;

        // Signal run completion — drops broadcast channel, subscribers observe close.
        run_broadcaster.finish_run(&run_id).await;

        // Fire Web Push notification after emitting the SSE done event.
        if let Some(dispatcher) = push_dispatcher_for_sse {
            let body = reply_text.chars().take(80).collect::<String>();
            if let Err(e) = dispatcher
                .send_to_all("New message", &body, Some(&conv_id_str))
                .await
            {
                warn!("Push dispatch failed: {e}");
            }
        }
    });

    // Include run_id in response headers as a fallback for clients that crash
    // before receiving the run_started SSE event.
    let mut response = sse_response(sse_rx);
    if let Ok(hv) = run_id.to_string().parse::<header::HeaderValue>() {
        response.headers_mut().insert("X-Run-Id", hv);
    }
    response
}

// -- Run event replay / tail -------------------------------------------------

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct StreamRunEventsQuery {
    /// Replay from this sequence number (inclusive). Defaults to 0.
    pub since: Option<i64>,
}

// -- SSE event payload schemas ------------------------------------------------

/// SSE `token` event — a text chunk from the assistant's response.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseTokenEvent {
    /// The text content of this token chunk.
    pub content: String,
}

/// SSE `thinking` event — a thinking/reasoning token from the model.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseThinkingEvent {
    /// The thinking text content.
    pub content: String,
}

/// SSE `status` event — a status message (e.g. "Calling tool X").
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseStatusEvent {
    /// Human-readable status message.
    pub message: String,
}

/// SSE `tool_result` event — result of a tool invocation.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseToolResultEvent {
    /// Name of the tool that was called.
    pub tool_name: String,
    /// Whether the tool call succeeded or failed.
    pub status: String,
    /// The arguments passed to the tool (JSON string).
    pub arguments: Option<String>,
    /// The tool's output or error message.
    pub result: Option<String>,
}

/// SSE `subagent_started` event — a subagent has been spawned.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSubagentStartedEvent {
    /// Unique identifier of the subagent.
    pub agent_id: String,
    /// The task description given to the subagent.
    pub task: String,
}

/// SSE `subagent_completed` event — a subagent has finished.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSubagentCompletedEvent {
    /// Unique identifier of the subagent.
    pub agent_id: String,
    /// Completion status (e.g. "success", "error").
    pub status: String,
    /// Summary of the subagent's work.
    pub summary: Option<String>,
}

/// SSE `subagent_token` event — a text token from a subagent's response.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSubagentTokenEvent {
    /// The subagent that produced this token.
    pub agent_id: String,
    /// Inner event payload.
    pub data: SseTokenEvent,
}

/// SSE `subagent_thinking` event — a thinking token from a subagent.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSubagentThinkingEvent {
    /// The subagent that produced this thinking token.
    pub agent_id: String,
    /// Inner event payload.
    pub data: SseThinkingEvent,
}

/// SSE `subagent_tool_result` event — a tool result from a subagent.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSubagentToolResultEvent {
    /// The subagent that executed the tool.
    pub agent_id: String,
    /// Inner event payload.
    pub data: SseToolResultEvent,
}

/// SSE `subagent_status` event — a status update from a subagent.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSubagentStatusEvent {
    /// The subagent that emitted this status.
    pub agent_id: String,
    /// Inner event payload.
    pub data: SseStatusEvent,
}

/// `GET /api/conversations/{id}/runs/{run_id}/events/stream`
///
/// Replays stored events from `?since` (default 0), then tails live events
/// if the run is still active.  Closes automatically when the `done` or
/// `error` event is reached.
///
/// Returns:
/// - `404` if no events exist for `run_id` (run never started or unknown)
/// - `410` if the run existed but all events have been pruned (TTL elapsed)
#[utoipa::path(
    get,
    path = "/api/conversations/{id}/runs/{run_id}/events/stream",
    tag = "conversations",
    params(
        ("id" = String, Path, description = "Conversation UUID"),
        ("run_id" = String, Path, description = "Run UUID from run_started event"),
        ("since" = Option<i64>, Query, description = "Replay from this sequence number (default 0)"),
    ),
    responses(
        (status = 200, description = "SSE stream of run events", content_type = "text/event-stream"),
        (status = 404, description = "Run not found"),
        (status = 410, description = "Run events expired"),
    ),
    security(("bearer_token" = []))
)]
pub async fn stream_run_events(
    State(state): State<ApiState>,
    Path((conv_id_str, run_id_str)): Path<(String, String)>,
    axum::extract::Query(query): axum::extract::Query<StreamRunEventsQuery>,
) -> Response {
    let run_id = match Uuid::parse_str(&run_id_str) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid run ID").into_response(),
    };
    let since = query.since.unwrap_or(0);

    // Check whether any events exist for this run.
    let event_store = &state.event_store;
    match event_store.has_events(&run_id_str).await {
        Ok(false) => {
            // Could be: run never existed, or all events were pruned.
            // Distinguish by checking if the run is active in the broadcaster.
            if state.run_broadcaster.subscribe(&run_id).await.is_none() {
                // Not in the live registry either — 404.
                return (
                    StatusCode::NOT_FOUND,
                    axum::Json(serde_json::json!({"error": "run not found"})),
                )
                    .into_response();
            }
            // It's active but has no DB events yet (race condition on startup).
            // Fall through to live-tail only.
        }
        Ok(true) => {
            // Check if it was pruned (no events AND no live registration).
            // has_events returned true, so there are still rows — continue.
        }
        Err(e) => {
            warn!("event store error for run {run_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    }

    // Check if events were pruned (run existed, events gone).
    // A run_id with zero rows that is NOT in the broadcaster means it's 410.
    // (Handled above — if has_events is false and no live entry → 404.)
    // For simplicity we treat unknown old runs as 404.

    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);

    // Subscribe to live broadcast channel before replaying, so we don't miss
    // any events emitted between the DB read and the subscription.
    let live_rx = state.run_broadcaster.subscribe(&run_id).await;
    let is_active = live_rx.is_some();

    // Replay stored events.
    let past_events = match event_store.list_events_since(&run_id_str, since).await {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to list events for run {run_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // Check if the run completed before we even arrived.
    let already_complete = past_events
        .iter()
        .any(|e| e.event_type == "done" || e.event_type == "agent_error");

    let conv_id_str_clone = conv_id_str.clone();
    tokio::spawn(async move {
        // Send all replayed events.
        for row in past_events {
            let ev = Event::default()
                .event(row.event_type.as_str())
                .data(row.payload.to_string())
                .id(row.sequence.to_string());
            if sse_tx.send(Ok(ev)).await.is_err() {
                return;
            }
        }

        // If the run was already complete, close the stream.
        if already_complete || !is_active {
            return;
        }

        // Tail live events.
        let mut live_rx = match live_rx {
            Some(r) => r,
            None => return,
        };

        loop {
            match live_rx.recv().await {
                Ok(live) => {
                    let ev = Event::default()
                        .event(live.event_type.as_str())
                        .data(live.payload.to_string())
                        .id(live.sequence.to_string());
                    let is_terminal = live.event_type == "done" || live.event_type == "agent_error";
                    if sse_tx.send(Ok(ev)).await.is_err() {
                        return;
                    }
                    if is_terminal {
                        return;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Run completed and sender was dropped.
                    return;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        "stream_run_events lagged by {n} events for run {run_id} conv {conv_id_str_clone}"
                    );
                    // Continue — we already replayed from DB; lagged live events
                    // are a minor gap in the tail (tokens may be duplicated on
                    // reconnect from DB, which is acceptable).
                }
            }
        }
    });

    sse_response(sse_rx)
}

// -- Quick-message handler --------------------------------------------------

/// `POST /api/quick-message` — create a new conversation, send a message, and
/// return the complete assistant response as a synchronous JSON reply.
///
/// This endpoint is designed for machine clients (Apple Shortcuts, Siri App
/// Intents, curl, webhooks) that need a single request/response round-trip.
#[utoipa::path(
    post,
    path = "/api/quick-message",
    operation_id = "create_quick_message",
    tag = "conversations",
    request_body = QuickMessageRequest,
    responses(
        (status = 201, description = "Message processed, conversation created", body = QuickMessageResponse),
        (status = 400, description = "Empty message"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Missing conversations:write scope"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn quick_message(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<QuickMessageRequest>,
) -> Response {
    if !caller_can_post(&auth) {
        return (StatusCode::FORBIDDEN, "Missing conversations:write scope").into_response();
    }
    let content = body.message.trim().to_string();
    if content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Message cannot be empty"})),
        )
            .into_response();
    }

    // Resolve persona: use explicit persona_id if valid, otherwise active.
    let agent_id = if let Some(ref pid) = body.persona_id {
        let persona_store =
            assistant_storage::personas::SqlitePersonaStore::new(state.pool.clone());
        if persona_store.get(pid).await.ok().flatten().is_some() {
            pid.clone()
        } else {
            state.agent_id.read().await.clone()
        }
    } else {
        state.agent_id.read().await.clone()
    };
    let store = SqliteConversationStore::for_agent(state.pool.clone(), &agent_id)
        .with_broadcaster(state.conversation_broadcaster.clone());

    // Build the prompt: prepend context if provided.
    let prompt = match body.context.as_deref() {
        Some(ctx) if !ctx.trim().is_empty() => {
            format!("<context>\n{}\n</context>\n\n{}", ctx.trim(), content)
        }
        _ => content.clone(),
    };

    // Create a new conversation with no title; the title-generator worker
    // (consumer of `turn.result`) will produce one once the conversation has
    // accumulated enough material.
    let conv = match store.create_conversation(None).await {
        Ok(c) => c,
        Err(e) => {
            warn!("quick-message: failed to create conversation: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to create conversation"})),
            )
                .into_response();
        }
    };

    // Submit the turn and await the full result (synchronous).
    let result = state
        .orchestrator
        .submit_turn(&auth, &prompt, conv.id, Interface::Web, None)
        .await;

    match result {
        Ok(turn_result) => (
            StatusCode::CREATED,
            Json(QuickMessageResponse {
                conversation_id: conv.id,
                message_id: turn_result.message_id,
                answer: turn_result.answer,
            }),
        )
            .into_response(),
        Err(e) => {
            warn!("quick-message: orchestrator error for {}: {e}", conv.id);
            // Clean up the orphan conversation.
            if let Err(del_err) = store.delete_conversation(conv.id).await {
                warn!(
                    "quick-message: failed to delete orphan conversation {}: {del_err}",
                    conv.id
                );
            }
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to process message"})),
            )
                .into_response()
        }
    }
}

// -- Voice handler ----------------------------------------------------------

/// Multipart form body for `POST /api/conversations/{id}/voice`.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
struct VoiceUploadForm {
    /// Raw audio bytes (opus/aac/webm/wav …).
    #[schema(format = Binary, content_encoding = "binary")]
    audio: Vec<u8>,
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
        (status = 403, description = "Missing conversations:write scope"),
        (status = 503, description = "Voice not configured"),
    ),
    security(("bearer_token" = []))
)]
pub async fn send_voice_message(
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    if !caller_can_post(&auth) {
        return (StatusCode::FORBIDDEN, "Missing conversations:write scope").into_response();
    }
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
    let store = SqliteConversationStore::for_agent(state.pool.clone(), &agent_id);
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
                .submit_turn(&auth, &content, conv_id, Interface::Web, None)
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
            // Shared projection layer owns serialization — unified with the main
            // streaming handler. The voice path neither persists nor batches, so
            // every frame (including thinking) streams straight through.
            let Some(frame) = SseProjector.project(&orch_event).into_iter().next() else {
                continue;
            };
            if frame.event == "token" {
                full_text.push_str(&frame.sse_data);
            }
            let sse_event = Event::default().event(frame.event).data(frame.sse_data);
            if sse_tx.send(Ok(sse_event)).await.is_err() {
                return;
            }
        }

        let (reply_text, reply_message_id, reply_attachment_ids) = match turn_result_rx.await {
            Ok(Ok(result)) => (result.answer, result.message_id, result.attachment_ids),
            Ok(Err(_)) | Err(_) => (full_text, None, vec![]),
        };

        let mut done_data = serde_json::json!({
            "role": "assistant",
            "content": reply_text,
        });
        if let Some(mid) = reply_message_id {
            done_data["message_id"] = serde_json::Value::String(mid.to_string());
        }
        if !reply_attachment_ids.is_empty() {
            done_data["attachment_ids"] = serde_json::json!(
                reply_attachment_ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
            );
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

    sse_response(sse_rx)
}

#[cfg(test)]
mod tests {
    use assistant_storage::PersonaStore as _;
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tokio::sync::RwLock;
    use tower::ServiceExt;
    use uuid::Uuid;
    use wiremock::MockServer;

    use assistant_runtime::CommandRegistry;
    use assistant_storage::{
        ConversationEventStore, ConversationStore, InMemoryConversationBroadcaster, RunBroadcaster,
        SkillRegistry, SqliteAttachmentStore, SqliteCommandEventStore, SqliteConversationStore,
        StorageLayer,
    };

    use super::super::ApiState;
    use super::super::test_helpers::*;

    // -- POST /conversations/{id}/messages — error paths ----------------------

    #[tokio::test]
    async fn send_message_without_conversations_write_scope_returns_403() {
        use assistant_core::auth::AuthContext;
        use assistant_core::identity::{Action, ResourceKind, Scope};

        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        // A caller with neither an admin role nor the conversations:write scope.
        let restricted = AuthContext {
            user_id: "u1".into(),
            org_id: "default".into(),
            email: "u@example.com".into(),
            space_roles: HashMap::new(),
            scopes: vec![Scope::new(ResourceKind::Personas, Action::Read)],
            client_id: "test".into(),
        };

        let resp = app_with_auth(state, restricted)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{}/messages", Uuid::new_v4()))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

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

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
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
    async fn test_send_message_first_turn_leaves_title_null() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Hello world").await;

        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.create_conversation(None).await.unwrap();
        assert!(conv.title.is_none(), "precondition: conversation untitled");
        assert!(!conv.title_locked, "precondition: conversation unlocked");
        let id = conv.id;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{id}/messages"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"message":"What should we have for dinner tonight?"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        // Drain the SSE stream so the turn completes before we re-read.
        let _ = body_bytes(resp.into_body()).await;

        let loaded = store.get_conversation(id).await.unwrap().unwrap();
        assert!(
            loaded.title.is_none(),
            "title must remain NULL after first message — got {:?}",
            loaded.title
        );
        assert!(
            !loaded.title_locked,
            "title_locked must remain false; the worker (not send_message) is responsible for titling"
        );
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
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

    // -- POST /conversations/{id}/voice — MIME validation & size limit ---------

    #[tokio::test]
    async fn voice_upload_without_transcription_provider_returns_503() {
        let server = MockServer::start().await;
        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
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

    // -- stream_run_events tests -----------------------------------------------

    /// Seed the event store directly and replay via the endpoint.
    #[tokio::test]
    async fn stream_run_events_replays_completed_run() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4().to_string();

        // Seed events into the store.
        state
            .event_store
            .append_event(
                &run_id,
                &conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();
        state
            .event_store
            .append_event(
                &run_id,
                &conv_id,
                1,
                "token",
                &serde_json::json!({"token": "hello"}),
            )
            .await
            .unwrap();
        state
            .event_store
            .append_event(
                &run_id,
                &conv_id,
                2,
                "done",
                &serde_json::json!({"content": "hello"}),
            )
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id}/events/stream"
            ))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("text/event-stream")
        );
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("run_started"),
            "should include run_started: {text}"
        );
        assert!(text.contains("token"), "should include token event: {text}");
        assert!(text.contains("done"), "should include done event: {text}");
    }

    #[tokio::test]
    async fn stream_run_events_returns_404_for_unknown_run() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4().to_string(); // never seeded

        let app = app(state);
        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id}/events/stream"
            ))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn stream_run_events_since_skips_earlier_events() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4().to_string();

        for i in 0i64..5 {
            state
                .event_store
                .append_event(
                    &run_id,
                    &conv_id,
                    i,
                    if i == 4 { "done" } else { "token" },
                    &serde_json::json!({"token": format!("t{i}")}),
                )
                .await
                .unwrap();
        }

        let app = app(state);
        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id}/events/stream?since=3"
            ))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);
        // Only events 3 and 4 should appear.
        assert!(text.contains("t3"), "should contain seq 3: {text}");
        assert!(text.contains("done"), "should contain done: {text}");
        assert!(!text.contains("t0"), "seq 0 should be skipped: {text}");
    }

    /// Client connects while the run is still active in the broadcaster.
    /// It should receive the replayed DB event first, then live events as they
    /// arrive, and close when "done" is broadcast.
    #[tokio::test]
    async fn stream_run_events_tails_live_broadcaster() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4();
        let run_id_str = run_id.to_string();

        // Seed one already-persisted event (the run_started from before client connects).
        state
            .event_store
            .append_event(
                &run_id_str,
                &conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id_str}),
            )
            .await
            .unwrap();

        // Register the run as active in the broadcaster.
        let broadcast_tx = state.run_broadcaster.start_run(run_id).await;
        // Clone the broadcaster before state is moved into `app()`.
        let broadcaster = state.run_broadcaster.clone();

        // Spawn a task that delivers live events after a brief delay so the
        // HTTP handler has time to subscribe before events are sent.
        let run_id_for_task = run_id;
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
            let _ = broadcast_tx.send(assistant_storage::LiveEvent {
                sequence: 1,
                event_type: "token".to_string(),
                payload: serde_json::json!({"token": "live-word"}),
            });
            let _ = broadcast_tx.send(assistant_storage::LiveEvent {
                sequence: 2,
                event_type: "done".to_string(),
                payload: serde_json::json!({"content": "live-word", "role": "assistant"}),
            });
            // Drop sender and remove from registry so the stream closes.
            broadcaster.finish_run(&run_id_for_task).await;
        });

        let app = app(state);
        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id_str}/events/stream"
            ))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("text/event-stream"),
        );
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("run_started"),
            "replayed DB event missing: {text}"
        );
        assert!(
            text.contains("live-word"),
            "live token from broadcaster missing: {text}"
        );
        assert!(text.contains("done"), "done event missing: {text}");
    }

    /// Race-condition path: broadcaster is active but no DB events have been
    /// persisted yet (run_started event is still in-flight). The handler should
    /// recognise the run as active and switch to live-tail mode.
    #[tokio::test]
    async fn stream_run_events_live_only_when_no_db_events_yet() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4();
        let run_id_str = run_id.to_string();

        // Register as active — no DB events at all.
        let broadcast_tx = state.run_broadcaster.start_run(run_id).await;
        let broadcaster = state.run_broadcaster.clone();

        let run_id_for_task = run_id;
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
            let _ = broadcast_tx.send(assistant_storage::LiveEvent {
                sequence: 0,
                event_type: "token".to_string(),
                payload: serde_json::json!({"token": "first-token"}),
            });
            let _ = broadcast_tx.send(assistant_storage::LiveEvent {
                sequence: 1,
                event_type: "done".to_string(),
                payload: serde_json::json!({"content": "first-token", "role": "assistant"}),
            });
            broadcaster.finish_run(&run_id_for_task).await;
        });

        let app = app(state);
        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id_str}/events/stream"
            ))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("first-token"),
            "live token missing for early-connect case: {text}"
        );
        assert!(text.contains("done"), "done event missing: {text}");
    }

    /// Verify that the send_message response carries the X-Run-Id header so
    /// clients can capture the run identifier even if the SSE stream is dropped
    /// before the run_started event arrives.
    #[tokio::test]
    async fn send_message_sets_x_run_id_header() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Test reply").await;

        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store
            .create_conversation(Some("RunId header"))
            .await
            .unwrap();
        let id = conv.id;

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

        assert_eq!(resp.status(), StatusCode::OK);
        let run_id_header = resp
            .headers()
            .get("x-run-id")
            .and_then(|v| v.to_str().ok())
            .expect("X-Run-Id header must be present");
        assert!(
            Uuid::parse_str(run_id_header).is_ok(),
            "X-Run-Id should be a valid UUID, got: {run_id_header}"
        );
    }

    /// After events are pruned (TTL), the endpoint returns 404.
    /// (We do not distinguish 404 vs 410 in the current implementation —
    /// both expired and never-existed runs return 404.)
    #[tokio::test]
    async fn stream_run_events_returns_404_for_pruned_run() {
        let sl = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        // Use a very short TTL (already negative = instantly expired).
        let short_ttl_store = assistant_storage::SqliteConversationEventStore::with_ttl(
            sl.pool.clone(),
            chrono::Duration::seconds(-1),
        );

        let run_id = Uuid::new_v4();
        let run_id_str = run_id.to_string();
        let conv_id = Uuid::new_v4().to_string();

        // Append an event; it is already expired on insert.
        short_ttl_store
            .append_event(
                &run_id_str,
                &conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id_str}),
            )
            .await
            .unwrap();

        // Prune — removes the expired row.
        short_ttl_store.prune_expired().await.unwrap();

        // Build a state that uses the same pool (events are now gone).
        use assistant_core::types::agent::AssistantConfig;
        use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
        use assistant_llm_provider::retry::RetryConfig;
        use assistant_runtime::Orchestrator;
        use assistant_tool_executor::ToolExecutor;
        let mut config = AssistantConfig::default();
        config.memory.enabled = false;
        let registry = Arc::new(SkillRegistry::new(sl.pool.clone()).await.unwrap());
        let llm = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: "http://127.0.0.1:1".to_string(),
                timeout_secs: 1,
                retry_config: RetryConfig::disabled(),
            })
            .unwrap(),
        );
        let executor = Arc::new(ToolExecutor::new(
            sl.clone(),
            llm.clone(),
            registry.clone(),
            Arc::new(config.clone()),
        ));
        let bus = Arc::new(sl.message_bus());
        let orchestrator = Arc::new(Orchestrator::new(
            llm,
            sl.clone(),
            executor,
            registry,
            bus,
            &config,
        ));
        let orchestrator_ref = orchestrator.clone();
        let default_model = orchestrator_ref.llm.model_name().to_string();
        let state = ApiState {
            pool: sl.pool.clone(),
            agent_id: Arc::new(RwLock::new("default".to_string())),
            orchestrator,
            push_dispatcher: None,
            transcription_provider: None,
            tts_provider: None,
            audio_store: Arc::new(crate::audio_store::AudioStore::new()),
            event_store: Arc::new(short_ttl_store),
            run_broadcaster: RunBroadcaster::new(),
            attachment_store: Arc::new(SqliteAttachmentStore::new(sl.pool.clone())),
            command_registry: Arc::new(CommandRegistry::new()),
            command_event_store: Arc::new(SqliteCommandEventStore::new(sl.pool.clone())),
            conversation_broadcaster: Arc::new(InMemoryConversationBroadcaster::new()),
            conversation_configs: Arc::new(RwLock::new(HashMap::new())),
            orchestrator_ref,
            active_turns: Arc::new(RwLock::new(HashMap::new())),
            default_model,
        };

        let app = app(state);
        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id_str}/events/stream"
            ))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // The run existed but events were pruned → currently 404.
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // -- POST /quick-message ---------------------------------------------------

    #[tokio::test]
    async fn quick_message_returns_201_with_answer() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Hello from quick message").await;

        let (state, _storage) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"ping"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert!(
            json["conversation_id"].as_str().is_some(),
            "should have a conversation_id"
        );
        assert_eq!(
            json["answer"].as_str().unwrap(),
            "Hello from quick message",
            "answer should match the LLM reply"
        );
    }

    #[tokio::test]
    async fn quick_message_empty_body_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["error"], "Message cannot be empty");
    }

    #[tokio::test]
    async fn test_quick_message_creates_conversation_with_null_title() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "response").await;

        let (state, storage) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"message":"What should I cook for dinner tonight?"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        let conv_id: Uuid = json["conversation_id"].as_str().unwrap().parse().unwrap();

        // The legacy auto-truncation is gone — the conversation starts NULL
        // and the title-generator worker is solely responsible for titling.
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert!(
            conv.title.is_none(),
            "quick-message must not auto-truncate the title; got {:?}",
            conv.title
        );
        assert!(!conv.title_locked);
    }

    #[tokio::test]
    async fn quick_message_with_persona_id_routes_to_persona() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "persona answer").await;

        let (state, storage) = test_state(&server.uri()).await;

        // Create a persona so it can be resolved.
        let persona_store =
            assistant_storage::personas::SqlitePersonaStore::new(storage.pool.clone());
        persona_store.create("reviewer", "Reviewer").await.unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"hello","persona_id":"reviewer"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        let conv_id: Uuid = json["conversation_id"].as_str().unwrap().parse().unwrap();

        // Conversation should be scoped to the "reviewer" persona.
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "reviewer");
        let conv = store.get_conversation(conv_id).await.unwrap();
        assert!(
            conv.is_some(),
            "conversation should exist under 'reviewer' persona"
        );
    }

    #[tokio::test]
    async fn quick_message_with_invalid_persona_falls_back() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "fallback answer").await;

        let (state, _storage) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"message":"hello","persona_id":"nonexistent"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should succeed using fallback persona, not error.
        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["answer"].as_str().unwrap(), "fallback answer");
    }

    #[tokio::test]
    async fn quick_message_with_context_prepends_to_prompt() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "context answer").await;

        let (state, _storage) = test_state(&server.uri()).await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        r#"{"message":"summarize this","context":"Lorem ipsum dolor sit amet"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["answer"].as_str().unwrap(), "context answer");
    }

    #[tokio::test]
    async fn quick_message_backward_compatible() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "compat answer").await;

        let (state, _storage) = test_state(&server.uri()).await;

        // Send only `message` — no persona_id, no context.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"hi"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["answer"].as_str().unwrap(), "compat answer");
    }
}
