//! Message endpoints: send streamed messages, quick (synchronous) messages,
//! voice messages (multipart audio → STT → assistant turn), and SSE replay
//! of run events.
//!
//! Endpoints:
//! - `POST /api/conversations/{id}/messages`         — streamed send
//! - `POST /api/conversations/{id}/voice`            — voice upload
//! - `POST /api/quick-message`                       — synchronous send
//! - `GET  /api/conversations/{id}/runs/{run_id}/events/stream` — run replay

use assistant_storage::{AttachmentStore as _, PersonaStore as _};
use std::convert::Infallible;
use std::time::Duration;

use axum::Extension;
use axum::Json;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::sse::Event;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, warn};
use uuid::Uuid;

use assistant_core::VOICE_PLACEHOLDER;
use assistant_core::auth::AuthContext;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::identity::{Action, ResourceKind};
use assistant_core::llm::provider::LlmProvider;
use assistant_core::types::conversation::Interface;
use assistant_core::types::features::SuggestionsConfig;
use assistant_runtime::OrchestratorEvent;
use assistant_runtime::SubmitTurn;
use assistant_runtime::projection::{SseProjector, StreamProjector};
use assistant_storage::{
    ConversationEventStore as _, ConversationStore, PushSubscriptionStore, SqliteConversationStore,
};
use assistant_transcription::TranscriptionRequest;

use crate::openapi::ErrorBody;
use crate::space::{Access, SpaceCtx};

use super::conversations::{caller_store, owned_conversation};
use super::{ApiState, sse_response};

/// Whether the caller may post a message / start a turn — gated on the
/// `conversations:write` scope, with org admins bypassing. Trusted local callers
/// carry this via [`AuthContext::system`]; network callers resolve a real
/// `AuthContext` from the request. Shared with the A2A handlers.
pub(crate) fn caller_can_post(auth: &AuthContext) -> bool {
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
    /// Optional persona ID to run the message as. Must name the persona this
    /// instance serves; any other existing persona is rejected with 422.
    /// Unknown IDs and an absent value fall back to the server's active persona.
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
        (status = 404, description = "Conversation not found", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn send_message(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    Json(body): Json<SendMessageRequest>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
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

    // Verify the caller owns the conversation before streaming.
    if let Err(response) = owned_conversation(&caller_store(&ctx, &state).await, conv_id).await {
        return response;
    }

    // Title generation is owned by the title-generator worker (consumer of
    // `turn.completed`), not this handler. See `crates/runtime/src/title_generator.rs`.

    // Generate a unique ID for this orchestrator run.
    let run_id = Uuid::new_v4();

    let (event_tx, event_rx) = mpsc::channel::<OrchestratorEvent>(64);

    let orchestrator = match state.runtime(&ctx).await {
        Ok(runtime) => runtime.orchestrator.clone(),
        Err(response) => return response,
    };
    let user_attachment_ids = body.attachment_ids;
    let user_message = content.clone();
    // Use `run_id` as the orchestrator's `request_id` so external cancel
    // calls — `POST .../turns/{run_id}/cancel` — can find the matching
    // CancellationToken via `Orchestrator::cancel_turn`.
    let turn_result_rx = {
        let (tx, rx) =
            tokio::sync::oneshot::channel::<anyhow::Result<assistant_runtime::TurnResult>>();
        let submit_request_id = run_id;
        tokio::spawn(async move {
            let result = orchestrator
                .submit_streaming(
                    SubmitTurn::new(content, conv_id, Interface::Web)
                        .with_auth(&auth)
                        .with_request_id(submit_request_id)
                        .with_attachments(user_attachment_ids)
                        // The turn may run in another process, which relays
                        // its events over the bus instead of into `event_tx`.
                        .with_relay_events(true),
                    event_tx,
                )
                .await;
            let _ = tx.send(result);
        });
        rx
    };

    stream_turn_to_sse(
        &ctx,
        &state,
        run_id,
        conv_id,
        RunOptions {
            push: true,
            suggestions: true,
        },
        user_message,
        event_rx,
        turn_result_rx,
    )
    .await
}

/// Per-entry-point policy for [`stream_turn_to_sse`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct RunOptions {
    /// Send Web Push notifications for skill, error and completion events.
    pub push: bool,
    /// Offer follow-up suggestions after a completed turn (`[suggestions]`
    /// still has to be enabled for the org).
    pub suggestions: bool,
}

/// Drive one turn to its SSE response: emit `run_started` (sequence 0), forward
/// orchestrator events as SSE frames while persisting them to the durable run
/// log and the live broadcaster, then emit the terminal frame (`done`, or
/// `agent_error` for a cancelled or failed turn) and send the completion push.
/// The POST stream closes right after the terminal frame. When
/// `opts.suggestions` is set and the turn ended in `done`, the run then stays
/// open for the follow-up suggestion phase (see [`follow_up_prompts`]), whose
/// result reaches clients on the replay tail as a `suggestions` frame.
///
/// `user_message` is the text the turn answered; the suggestion call reads it.
///
/// The caller has already submitted the turn with
/// [`submit_streaming`](assistant_runtime::Orchestrator::submit_streaming),
/// streaming into `event_rx`; its outcome arrives on `turn_result_rx`. The
/// returned response carries the run id in `X-Run-Id` for clients that drop the
/// stream before `run_started` arrives.
// Every argument is a distinct per-turn input; the sibling entry points
// (edit, regenerate, voice) each pass all of them.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn stream_turn_to_sse(
    ctx: &SpaceCtx,
    state: &ApiState,
    run_id: Uuid,
    conv_id: Uuid,
    opts: RunOptions,
    user_message: String,
    event_rx: mpsc::Receiver<OrchestratorEvent>,
    turn_result_rx: tokio::sync::oneshot::Receiver<anyhow::Result<assistant_runtime::TurnResult>>,
) -> Response {
    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);

    let push_dispatcher_for_sse = state.push_dispatcher.clone().filter(|_| opts.push);
    let push_store: std::sync::Arc<dyn PushSubscriptionStore> =
        std::sync::Arc::new(ctx.storage.push_subscription_store());
    let event_store = ctx.storage.conversation_event_store();
    let run_broadcaster = state.run_broadcaster.clone();
    // Events of a turn another process runs arrive over the bus; the stream
    // ends with the turn either way. The same runtime supplies the org's
    // suggestion settings and LLM; without it the turn still streams, only
    // the relay and the chips are lost.
    let runtime = match state.runtime(ctx).await {
        Ok(runtime) => Some(runtime),
        Err(_) => {
            warn!("No runtime to relay events for run {run_id}; streaming local events only");
            None
        }
    };
    let relay = runtime
        .as_ref()
        .map(|runtime| runtime.orchestrator.run_event_relay(conv_id, run_id));
    let suggestion_setup = runtime
        .filter(|runtime| opts.suggestions && runtime.config.suggestions.enabled)
        .map(|runtime| (runtime.config.suggestions.clone(), runtime.llm.clone()));
    let mut run_events = RunEvents::new(event_rx, relay, turn_result_rx);

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
        let conv_id_str = conv_id.to_string();

        // Throttle thinking persistence: accumulate tokens, flush every 20 tokens
        // or when a non-thinking event arrives. SSE is still sent immediately.
        let mut thinking_buf = String::new();
        let mut thinking_token_count: usize = 0;
        const THINKING_BATCH_SIZE: usize = 20;

        // A closed SSE channel only stops forwarding. The loop keeps draining
        // `run_events` so the durable log and live tail see the whole turn, and
        // a reconnecting client can resume from `events/stream?since=N`.
        let mut client_gone = false;

        while let Some(orch_event) = run_events.next().await {
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
                let store = push_store.clone();
                tokio::spawn(async move {
                    if let Err(e) = d.send_to_all(&*store, title, &body, Some(&cid)).await {
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
                let store = push_store.clone();
                tokio::spawn(async move {
                    if let Err(e) = d
                        .send_to_all(&*store, "Assistant error", &body, Some(&cid))
                        .await
                    {
                        warn!("Push (agent error) failed: {e}");
                    }
                });
            }

            // Thinking frames stream immediately and persist in batches.
            if is_thinking {
                let e = Event::default()
                    .event("thinking")
                    .data(frame.sse_data.clone());
                if !client_gone && sse_tx.send(Ok(e)).await.is_err() {
                    client_gone = true;
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

            if !client_gone && sse_tx.send(Ok(sse_event)).await.is_err() {
                client_gone = true;
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
            Cancelled {
                /// The turn never started, so its user message was not saved.
                before_start: bool,
            },
            Errored(String),
        }
        let (reply_text, reply_message_id, reply_attachment_ids, terminal) =
            match run_events.into_turn_result().await {
                Ok(Ok(result)) => (
                    result.answer,
                    result.message_id,
                    result.attachment_ids,
                    TerminalKind::Done,
                ),
                Ok(Err(e)) => {
                    let msg = e.to_string();
                    let kind = if msg.contains(assistant_runtime::TURN_CANCELLED_MARKER) {
                        TerminalKind::Cancelled {
                            before_start: msg
                                .contains(assistant_runtime::TURN_CANCELLED_BEFORE_START_MARKER),
                        }
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

        let done_message_id = match terminal {
            TerminalKind::Done => reply_message_id,
            TerminalKind::Cancelled { .. } | TerminalKind::Errored(_) => None,
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
            TerminalKind::Cancelled { before_start } => {
                let mut cancelled = serde_json::json!({
                    "reason": "cancelled",
                    "message": "Turn cancelled by user",
                    "partial_content": reply_text,
                });
                if *before_start {
                    cancelled["user_message_persisted"] = serde_json::Value::Bool(false);
                }
                ("agent_error", cancelled)
            }
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
        let client_present_at_done = !client_gone && sse_tx.send(Ok(terminal_sse)).await.is_ok();

        // Fire Web Push notification after emitting the SSE done event, ahead
        // of the suggestion phase so its latency never delays the push.
        if let Some(dispatcher) = push_dispatcher_for_sse {
            let body = reply_text.chars().take(80).collect::<String>();
            if let Err(e) = dispatcher
                .send_to_all(&*push_store, "New message", &body, Some(&conv_id_str))
                .await
            {
                warn!("Push dispatch failed: {e}");
            }
        }

        // The POST stream ends at the terminal frame. Suggestions travel on
        // the run's replay tail, so a client that buffers the body until close
        // still gets the whole answer without waiting for them.
        drop(sse_tx);

        if let (Some((cfg, llm)), Some(message_id)) = (suggestion_setup, done_message_id)
            && (client_present_at_done || broadcast_tx.receiver_count() > 0)
        {
            let prompts =
                follow_up_prompts(&cfg, &*llm, &user_message, &reply_text, &broadcast_tx).await;
            if !prompts.is_empty() {
                let payload = serde_json::json!({
                    "message_id": message_id.to_string(),
                    "suggestions": prompts
                        .iter()
                        .map(|prompt| serde_json::json!({"prompt": prompt}))
                        .collect::<Vec<_>>(),
                });
                let seq = seq + 1;
                if let Err(e) = event_store
                    .append_event(&run_id_str, &conv_id_str, seq, "suggestions", &payload)
                    .await
                {
                    warn!("Failed to persist suggestions event for run {run_id}: {e}");
                }
                let _ = broadcast_tx.send(assistant_storage::LiveEvent {
                    sequence: seq,
                    event_type: "suggestions".to_string(),
                    payload,
                });
            }
        }

        // Signal run completion — drops broadcast channel, subscribers observe close.
        run_broadcaster.finish_run(&run_id).await;
    });

    // Include run_id in response headers as a fallback for clients that crash
    // before receiving the run_started SSE event.
    let mut response = sse_response(sse_rx);
    if let Ok(hv) = run_id.to_string().parse::<header::HeaderValue>() {
        response.headers_mut().insert("X-Run-Id", hv);
    }
    response
}

// -- Follow-up suggestion phase ----------------------------------------------

/// How long after `done` a replay subscriber may take to attach before the
/// suggestion call is judged to have no audience.
const SUGGESTION_ATTACH_GRACE: Duration = Duration::from_secs(2);

/// How often the departed-listener watchdog re-reads the receiver count.
const SUGGESTION_LISTENER_POLL: Duration = Duration::from_millis(500);

/// Resolve once no replay subscriber is connected, after the attach grace.
async fn listener_departed(tx: &tokio::sync::broadcast::Sender<assistant_storage::LiveEvent>) {
    tokio::time::sleep(SUGGESTION_ATTACH_GRACE).await;
    while tx.receiver_count() > 0 {
        tokio::time::sleep(SUGGESTION_LISTENER_POLL).await;
    }
}

/// Ask the org's LLM for follow-up prompts, bounded by the configured timeout
/// and abandoned (dropping the in-flight call) when the listener leaves.
///
/// Errors, timeouts and aborts only log; they yield no prompts.
async fn follow_up_prompts(
    cfg: &SuggestionsConfig,
    llm: &dyn LlmProvider,
    user_message: &str,
    answer: &str,
    tx: &tokio::sync::broadcast::Sender<assistant_storage::LiveEvent>,
) -> Vec<String> {
    let call = tokio::time::timeout(
        cfg.effective_timeout(),
        assistant_runtime::follow_ups::generate(user_message, answer, cfg.effective_max(), llm),
    );
    tokio::select! {
        result = call => match result {
            Ok(Ok(generated)) => generated.prompts,
            Ok(Err(e)) => {
                warn!("Follow-up suggestion call failed: {e:#}");
                Vec::new()
            }
            Err(_) => {
                debug!("Follow-up suggestion call timed out");
                Vec::new()
            }
        },
        () = listener_departed(tx) => {
            debug!("No replay listener for follow-up suggestions; call aborted");
            Vec::new()
        }
    }
}

/// How often the runner looks for relayed events while the turn is quiet.
/// Events that arrive back to back are read without waiting.
const RELAY_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

type TurnOutcome =
    Result<anyhow::Result<assistant_runtime::TurnResult>, tokio::sync::oneshot::error::RecvError>;

/// The events of one turn, from whichever side delivers them: the local sink
/// when a worker in this process runs the turn, the bus relay when a worker in
/// another process does. It ends with the turn's outcome, because the relay
/// has no end of its own.
struct RunEvents {
    local: mpsc::Receiver<OrchestratorEvent>,
    local_open: bool,
    relay: Option<assistant_runtime::RunEventRelay>,
    /// The last poll of the relay returned an event, so the next one goes
    /// straight back without waiting.
    relay_busy: bool,
    turn_result: tokio::sync::oneshot::Receiver<anyhow::Result<assistant_runtime::TurnResult>>,
    outcome: Option<TurnOutcome>,
}

impl RunEvents {
    fn new(
        local: mpsc::Receiver<OrchestratorEvent>,
        relay: Option<assistant_runtime::RunEventRelay>,
        turn_result: tokio::sync::oneshot::Receiver<anyhow::Result<assistant_runtime::TurnResult>>,
    ) -> Self {
        Self {
            local,
            local_open: true,
            relay,
            relay_busy: false,
            turn_result,
            outcome: None,
        }
    }

    /// The next event, or `None` once the turn has ended and every event it
    /// emitted has been handed out.
    async fn next(&mut self) -> Option<OrchestratorEvent> {
        while self.outcome.is_none() {
            if self.relay_busy {
                match self.poll_relay().await {
                    Some(event) => return Some(event),
                    None => self.relay_busy = false,
                }
            }
            tokio::select! {
                event = self.local.recv(), if self.local_open => match event {
                    Some(event) => return Some(event),
                    None => self.local_open = false,
                },
                outcome = &mut self.turn_result => self.outcome = Some(outcome),
                _ = tokio::time::sleep(RELAY_POLL_INTERVAL), if self.relay.is_some() => {
                    if let Some(event) = self.poll_relay().await {
                        return Some(event);
                    }
                }
            }
        }
        // The turn is over. Everything it emitted was queued before its
        // outcome was sent, so draining both sides now loses nothing.
        if let Ok(event) = self.local.try_recv() {
            return Some(event);
        }
        self.poll_relay().await
    }

    async fn poll_relay(&mut self) -> Option<OrchestratorEvent> {
        let event = self.relay.as_ref()?.next().await;
        self.relay_busy = event.is_some();
        event
    }

    /// The turn's outcome, waiting for it if the events ended first.
    async fn into_turn_result(self) -> TurnOutcome {
        match self.outcome {
            Some(outcome) => outcome,
            None => self.turn_result.await,
        }
    }
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
    /// The arguments passed to the tool, as a JSON object.
    #[schema(value_type = Option<Object>)]
    pub arguments: Option<serde_json::Value>,
    /// The tool's output or error message.
    pub result: Option<String>,
    /// Identifier of the tool call this result answers, when the provider
    /// supplied one.
    pub tool_call_id: Option<String>,
    /// Structured output of the tool, a JSON object with a `kind` tag. Absent
    /// when the tool returned text only or the data exceeded the size limit.
    /// Each `kind` is defined by a JSON Schema in `schemas/tool-results/`.
    #[schema(value_type = Option<Object>)]
    pub data: Option<serde_json::Value>,
    /// The MCP App the tool declared for its result. Absent for tools
    /// without one, and on subagent tool results.
    pub app: Option<super::mcp_apps::McpAppView>,
}

/// SSE `suggestions` event: tappable follow-up prompts for a finished turn.
///
/// Sent only on the run's replay stream, after `done`, when
/// `[suggestions].enabled` is on for the org.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSuggestionsEvent {
    /// The assistant message the suggestions follow (`done.message_id`).
    pub message_id: Uuid,
    /// Prompts the user could send next, best first.
    pub suggestions: Vec<SseSuggestion>,
}

/// One follow-up prompt in an [`SseSuggestionsEvent`].
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SseSuggestion {
    /// Text to send as the user's next message.
    pub prompt: String,
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
/// if the run is still active. Closes when the run finishes, not at the
/// `done` or `agent_error` frame: events such as `suggestions` follow `done`.
/// A client that disconnects stops the tail at once.
///
/// Returns `404` if no events exist for `run_id` and the run is not active
/// (never started, unknown, or pruned after the TTL elapsed).
///
/// After `done` the stream may carry one `suggestions` frame
/// (`SseSuggestionsEvent`) with follow-up prompts for the finished turn; it
/// then closes when the run finishes.
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
        (status = 404, description = "Conversation or run not found", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn stream_run_events(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Path((conv_id_str, run_id_str)): Path<(String, String)>,
    axum::extract::Query(query): axum::extract::Query<StreamRunEventsQuery>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Read) {
        return denied.into_response();
    }
    let run_id = match Uuid::parse_str(&run_id_str) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid run ID").into_response(),
    };
    let since = query.since.unwrap_or(0);

    // The run is readable only through a conversation the caller owns.
    match Uuid::parse_str(&conv_id_str) {
        Ok(conv_id) => {
            if let Err(response) =
                owned_conversation(&caller_store(&ctx, &state).await, conv_id).await
            {
                return response;
            }
        }
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    }

    // Check whether any events exist for this run.
    let event_store = ctx.storage.conversation_event_store();
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

        // A run that is no longer registered has nothing more to deliver.
        if !is_active {
            return;
        }

        // Tail live events until the run finishes. `done` does not end the
        // run: later frames such as `suggestions` still follow it.
        let mut live_rx = match live_rx {
            Some(r) => r,
            None => return,
        };

        loop {
            tokio::select! {
                // A departed client releases its receiver without waiting
                // for the next event.
                _ = sse_tx.closed() => return,
                received = live_rx.recv() => match received {
                    Ok(live) => {
                        let ev = Event::default()
                            .event(live.event_type.as_str())
                            .data(live.payload.to_string())
                            .id(live.sequence.to_string());
                        if sse_tx.send(Ok(ev)).await.is_err() {
                            return;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Run finished and the sender was dropped.
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
                },
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
        (status = 422, description = "persona_id names a persona this instance does not serve"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn quick_message(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Json(body): Json<QuickMessageRequest>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
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

    let orchestrator = match state.runtime(&ctx).await {
        Ok(runtime) => runtime.orchestrator.clone(),
        Err(response) => return response,
    };

    // Resolve persona: use explicit persona_id if valid, otherwise active.
    let agent_id = if let Some(ref pid) = body.persona_id {
        let persona_store =
            assistant_storage::personas::SqlitePersonaStore::new(ctx.storage.pool.clone());
        if persona_store.get(pid).await.ok().flatten().is_some() {
            // The shared orchestrator and its bus worker only process
            // conversations owned by their own persona, so a turn for any
            // other persona would fail after the conversation was created.
            if pid != orchestrator.agent_id() {
                return (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({
                        "error": format!(
                            "Persona '{pid}' is not served by this instance (serving '{}')",
                            orchestrator.agent_id()
                        )
                    })),
                )
                    .into_response();
            }
            pid.clone()
        } else {
            state.agent_id.read().await.clone()
        }
    } else {
        state.agent_id.read().await.clone()
    };
    let store = SqliteConversationStore::for_agent(ctx.storage.pool.clone(), &agent_id)
        .with_user_id(ctx.auth.user_id.0.clone())
        .with_broadcaster(
            state
                .conversation_broadcasters
                .for_space(&ctx.org_slug, &ctx.space_slug),
        );

    // Build the prompt: prepend context if provided.
    let prompt = match body.context.as_deref() {
        Some(extra) if !extra.trim().is_empty() => {
            format!("<context>\n{}\n</context>\n\n{}", extra.trim(), content)
        }
        _ => content.clone(),
    };

    // Create a new conversation with no title; the title-generator worker
    // (consumer of `turn.completed`) will produce one once the conversation has
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
    let result = orchestrator
        .submit(SubmitTurn::new(prompt, conv.id, Interface::Web).with_auth(&auth))
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

/// `POST /api/conversations/{id}/voice` — upload audio, keep it as the user
/// message's attachment, transcribe it, run through the orchestrator, and
/// stream the response as SSE.
///
/// The message text is the transcript, or `[Voice message]` when no
/// transcription provider is configured or transcription fails.
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
        (status = 404, description = "Conversation not found", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn send_voice_message(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
    if !caller_can_post(&auth) {
        return (StatusCode::FORBIDDEN, "Missing conversations:write scope").into_response();
    }
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
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

    // Verify the caller owns the conversation.
    if let Err(response) = owned_conversation(&caller_store(&ctx, &state).await, conv_id).await {
        return response;
    }

    // Keep the recording as an attachment so the turn can carry the audio.
    let meta = assistant_core::AttachmentMeta {
        id: Uuid::new_v4(),
        message_id: None,
        conversation_id: conv_id,
        agent_id: state.agent_id.read().await.clone(),
        filename: "voice".to_string(),
        mime_type: mime_type.clone(),
        size_bytes: audio_bytes.len() as u64,
        created_at: SystemClock.now(),
    };
    if let Err(e) = ctx
        .storage
        .attachment_store()
        .store(&meta, &audio_bytes)
        .await
    {
        warn!("Failed to store voice recording: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to store voice recording",
        )
            .into_response();
    }

    // The transcript stands in for the recording; without one the turn still
    // runs on the recording alone.
    let transcript = match &state.transcription_provider {
        Some(provider) => match provider
            .transcribe(TranscriptionRequest {
                audio_data: audio_bytes,
                mime_type,
                filename: None,
                language: None,
            })
            .await
        {
            Ok(r) if !r.text.trim().is_empty() => r.text,
            Ok(_) => {
                warn!("Transcription returned empty text");
                VOICE_PLACEHOLDER.to_string()
            }
            Err(e) => {
                warn!("Transcription failed: {e}");
                VOICE_PLACEHOLDER.to_string()
            }
        },
        None => VOICE_PLACEHOLDER.to_string(),
    };

    // Run through orchestrator (same flow as send_message).
    let (sse_tx, sse_rx) = mpsc::channel::<Result<Event, Infallible>>(64);
    let (event_tx, mut event_rx) = mpsc::channel::<OrchestratorEvent>(64);

    let orchestrator = match state.runtime(&ctx).await {
        Ok(runtime) => runtime.orchestrator.clone(),
        Err(response) => return response,
    };
    let content = transcript.clone();
    let recording_id = meta.id;
    let turn_result_rx = {
        let (tx, rx) =
            tokio::sync::oneshot::channel::<anyhow::Result<assistant_runtime::TurnResult>>();
        tokio::spawn(async move {
            let result = orchestrator
                .submit_streaming(
                    SubmitTurn::new(content, conv_id, Interface::Web)
                        .with_auth(&auth)
                        .with_attachments(vec![recording_id]),
                    event_tx,
                )
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
    let push_store: std::sync::Arc<dyn PushSubscriptionStore> =
        std::sync::Arc::new(ctx.storage.push_subscription_store());
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
                .send_to_all(&*push_store, "New message", &body, Some(&conv_id_str))
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
    use tower::ServiceExt;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_storage::{
        ConversationEventStore, ConversationStore, SqliteConversationStore, StorageLayer,
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
            space_roles: HashMap::from([(
                assistant_core::identity::SpaceId::from("default"),
                assistant_core::identity::Role::Member,
            )]),
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

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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
    async fn send_message_unarchives_an_archived_conversation() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Hello again").await;
        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let id = store
            .create_conversation(Some("Old chat"))
            .await
            .unwrap()
            .id;
        store
            .update_metadata(
                id,
                assistant_storage::ConversationPatch {
                    title: None,
                    pinned: None,
                    archived: Some(true),
                },
            )
            .await
            .unwrap();
        assert!(
            store
                .get_conversation(id)
                .await
                .unwrap()
                .unwrap()
                .archived_at
                .is_some(),
            "precondition: archived"
        );

        let resp = post_message(state, id, "Are you still there?").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let _ = body_bytes(resp.into_body()).await;

        let loaded = store.get_conversation(id).await.unwrap().unwrap();
        assert!(
            loaded.archived_at.is_none(),
            "a new user message must unarchive the conversation"
        );
        let listed = store.list_conversations().await.unwrap();
        assert!(listed.iter().any(|c| c.id == id), "back in the main list");
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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

    // -- send_message frame sequence (characterisation of the streamed runner) --

    /// One parsed SSE frame: `(event, data, id)`.
    type Frame = (String, String, Option<i64>);

    fn parse_sse(text: &str) -> Vec<Frame> {
        text.split("\n\n")
            .filter(|block| !block.trim().is_empty())
            .map(|block| {
                let (mut event, mut data, mut id) = (String::new(), String::new(), None);
                for line in block.lines() {
                    if let Some(v) = line.strip_prefix("event:") {
                        event = v.trim().to_string();
                    } else if let Some(v) = line.strip_prefix("data:") {
                        data = v.trim().to_string();
                    } else if let Some(v) = line.strip_prefix("id:") {
                        id = v.trim().parse().ok();
                    }
                }
                (event, data, id)
            })
            .collect()
    }

    /// Mount an Ollama-shaped NDJSON stream: `thinking_chunks` thinking
    /// chunks of one character each, then `answer` as a single content chunk.
    async fn mount_llm_stream(server: &MockServer, thinking_chunks: usize, answer: &str) {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        let mut ndjson = String::new();
        for _ in 0..thinking_chunks {
            ndjson.push_str(
                &serde_json::json!({
                    "model": "test",
                    "message": {"role": "assistant", "content": "", "thinking": "t"},
                    "done": false
                })
                .to_string(),
            );
            ndjson.push('\n');
        }
        ndjson.push_str(
            &serde_json::json!({
                "model": "test",
                "message": {"role": "assistant", "content": answer},
                "done": true
            })
            .to_string(),
        );
        ndjson.push('\n');
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_string(ndjson))
            .mount(server)
            .await;
    }

    /// An LLM that never answers, so a turn stays in flight until cancelled.
    async fn mount_llm_hang(server: &MockServer) {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(30)))
            .mount(server)
            .await;
    }

    async fn post_message(state: ApiState, conv_id: Uuid, text: &str) -> axum::response::Response {
        app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conv_id}/messages"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "message": text }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn new_conversation(storage: &Arc<StorageLayer>) -> Uuid {
        SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system")
            .create_conversation(Some("Frames"))
            .await
            .unwrap()
            .id
    }

    async fn default_orchestrator(state: &ApiState) -> Arc<assistant_runtime::Orchestrator> {
        state
            .runtimes
            .get("default", "default")
            .await
            .unwrap()
            .orchestrator
            .clone()
    }

    /// Wait until a worker has claimed the turn and called the LLM at `llm`.
    async fn wait_until_running(
        orchestrator: &assistant_runtime::Orchestrator,
        llm: &MockServer,
        run_id: Uuid,
    ) {
        for _ in 0..200 {
            if !llm.received_requests().await.unwrap_or_default().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        assert!(
            !llm.received_requests().await.unwrap_or_default().is_empty(),
            "precondition: the turn reached the LLM"
        );
        assert!(
            orchestrator.is_turn_in_flight(run_id).await,
            "precondition: the turn is in flight"
        );
    }

    /// Wait until a worker has claimed the turn, then cancel it.
    async fn cancel_when_running(
        orchestrator: &assistant_runtime::Orchestrator,
        llm: &MockServer,
        run_id: Uuid,
    ) {
        wait_until_running(orchestrator, llm, run_id).await;
        orchestrator.cancel_turn(run_id).await;
    }

    /// The durable event log of one run, in sequence order, as `(seq, type, payload)`.
    async fn event_log(
        storage: &Arc<StorageLayer>,
        run_id: &str,
    ) -> Vec<(i64, String, serde_json::Value)> {
        storage
            .conversation_event_store()
            .list_events_since(run_id, 0)
            .await
            .unwrap()
            .into_iter()
            .map(|row| (row.sequence, row.event_type, row.payload))
            .collect()
    }

    fn run_id_of(resp: &axum::response::Response) -> String {
        resp.headers()
            .get("x-run-id")
            .and_then(|v| v.to_str().ok())
            .expect("X-Run-Id header")
            .to_string()
    }

    async fn sse_frames(resp: axum::response::Response) -> Vec<Frame> {
        parse_sse(&String::from_utf8_lossy(
            &body_bytes(resp.into_body()).await,
        ))
    }

    #[tokio::test]
    async fn send_message_streams_run_started_first_at_sequence_zero() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Hello world").await;
        let (state, storage) = test_state(&server.uri()).await;
        let conv_id = new_conversation(&storage).await;

        let resp = post_message(state, conv_id, "hi").await;
        let run_id = run_id_of(&resp);
        let frames = sse_frames(resp).await;

        let (event, data, id) = &frames[0];
        assert_eq!(event, "run_started", "first SSE frame is run_started");
        assert_eq!(*id, Some(0), "run_started carries sequence 0");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["run_id"], run_id, "run_started names the run");

        let log = event_log(&storage, &run_id).await;
        assert_eq!(log[0].1, "run_started", "the durable log starts with it");
        let sequences: Vec<i64> = log.iter().map(|(seq, _, _)| *seq).collect();
        let expected: Vec<i64> = (0..log.len() as i64).collect();
        assert_eq!(sequences, expected, "log sequences are contiguous from 0");
        assert_eq!(
            log.last().map(|(_, ty, _)| ty.as_str()),
            Some("done"),
            "a completed turn ends the log with done"
        );
        let sse_ids: Vec<Option<i64>> = frames.iter().map(|(_, _, id)| *id).collect();
        let log_ids: Vec<Option<i64>> = log.iter().map(|(seq, _, _)| Some(*seq)).collect();
        assert_eq!(sse_ids, log_ids, "SSE ids mirror the durable log");
    }

    #[tokio::test]
    async fn send_message_persists_thinking_in_batches_of_twenty() {
        let server = MockServer::start().await;
        mount_llm_stream(&server, 45, "Done thinking").await;
        let (state, storage) = test_state(&server.uri()).await;
        let conv_id = new_conversation(&storage).await;

        let resp = post_message(state, conv_id, "think hard").await;
        let run_id = run_id_of(&resp);
        let frames = sse_frames(resp).await;

        let live_thinking = frames.iter().filter(|(ev, _, _)| ev == "thinking").count();
        assert_eq!(live_thinking, 45, "every thinking token streams live");

        let log = event_log(&storage, &run_id).await;
        let batches: Vec<usize> = log
            .iter()
            .filter(|(_, ty, _)| ty == "thinking")
            .map(|(_, _, p)| p["content"].as_str().unwrap_or_default().len())
            .collect();
        assert_eq!(
            batches,
            vec![20, 20, 5],
            "thinking persists every 20 tokens; the remainder flushes before the next event"
        );
        let first_token = log.iter().position(|(_, ty, _)| ty == "token").unwrap();
        let last_thinking = log.iter().rposition(|(_, ty, _)| ty == "thinking").unwrap();
        assert!(
            last_thinking < first_token,
            "the remainder is flushed before the first non-thinking event"
        );
    }

    #[tokio::test]
    async fn send_message_ends_with_cancelled_agent_error_when_turn_is_cancelled() {
        let server = MockServer::start().await;
        mount_llm_hang(&server).await;
        let (state, storage) = test_state(&server.uri()).await;
        let orchestrator = default_orchestrator(&state).await;
        let conv_id = new_conversation(&storage).await;

        let resp = post_message(state, conv_id, "slow").await;
        let run_id = run_id_of(&resp);
        cancel_when_running(&orchestrator, &server, Uuid::parse_str(&run_id).unwrap()).await;
        let frames = sse_frames(resp).await;

        let (event, data, _) = frames.last().unwrap();
        assert_eq!(event, "agent_error", "a cancelled turn ends in agent_error");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["reason"], "cancelled");
        assert_eq!(payload["message"], "Turn cancelled by user");
        assert!(
            frames.iter().all(|(ev, _, _)| ev != "done"),
            "a cancelled turn never emits done"
        );

        let log = event_log(&storage, &run_id).await;
        let (_, last_type, last_payload) = log.last().unwrap();
        assert_eq!(last_type, "agent_error", "the terminal is persisted too");
        assert_eq!(last_payload["reason"], "cancelled");
    }

    /// Wait until `run_id` is registered with the orchestrator, claimed or not.
    async fn wait_until_in_flight(orchestrator: &assistant_runtime::Orchestrator, run_id: Uuid) {
        for _ in 0..200 {
            if orchestrator.is_turn_in_flight(run_id).await {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        panic!("run {run_id} was never registered with the orchestrator");
    }

    #[tokio::test]
    async fn send_message_cancelled_before_a_worker_claims_it_ends_the_stream_at_once() {
        let server = MockServer::start().await;
        mount_llm_hang(&server).await;
        let (state, storage) = test_state(&server.uri()).await;
        let orchestrator = default_orchestrator(&state).await;
        let busy_conv = new_conversation(&storage).await;
        let queued_conv = new_conversation(&storage).await;

        // The only worker is stuck on a turn in another conversation.
        let busy = post_message(state.clone(), busy_conv, "occupy the worker").await;
        let busy_run = Uuid::parse_str(&run_id_of(&busy)).unwrap();
        wait_until_running(&orchestrator, &server, busy_run).await;

        // This turn waits for a worker. Cancel it before one claims it.
        let queued = post_message(state.clone(), queued_conv, "never starts").await;
        let queued_run = Uuid::parse_str(&run_id_of(&queued)).unwrap();
        wait_until_in_flight(&orchestrator, queued_run).await;
        orchestrator.cancel_turn(queued_run).await;

        let frames = tokio::time::timeout(std::time::Duration::from_secs(3), sse_frames(queued))
            .await
            .expect("the stream ends without waiting for a worker");

        let (event, data, _) = frames.last().unwrap();
        assert_eq!(event, "agent_error");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["reason"], "cancelled");
        assert_eq!(
            payload["user_message_persisted"], false,
            "nothing of the turn ran, so the user message was not saved"
        );
        assert!(
            payload.get("message_id").is_none(),
            "no assistant message exists for a turn that never started"
        );

        let history = app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{queued_conv}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let history = body_json(history.into_body()).await;
        assert_eq!(
            history["messages"].as_array().map(Vec::len),
            Some(0),
            "the cancelled turn left no user row: {history}"
        );

        orchestrator.cancel_turn(busy_run).await;
    }

    #[tokio::test]
    async fn runner_streams_every_buffered_token_before_done_when_the_result_is_ready() {
        let (state, storage) = event_log_state().await;
        let tokens: Vec<_> = (0..20)
            .map(|i| assistant_runtime::OrchestratorEvent::Token(format!("t{i} ")))
            .collect();

        let (_, frames) = run_runner(
            &state,
            &storage,
            RunOptions {
                push: false,
                suggestions: false,
            },
            tokens,
            Some(Ok(turn_result("done text"))),
        )
        .await;

        let streamed = frames.iter().filter(|(ev, _, _)| ev == "token").count();
        assert_eq!(streamed, 20, "every buffered token precedes the terminal");
        assert_eq!(frames.last().unwrap().0, "done");
    }

    #[tokio::test]
    async fn send_message_ends_with_error_agent_error_when_turn_fails() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;
        let (state, storage) = test_state(&server.uri()).await;
        let conv_id = new_conversation(&storage).await;

        let frames = sse_frames(post_message(state, conv_id, "fail").await).await;

        let (event, data, _) = frames.last().unwrap();
        assert_eq!(event, "agent_error", "a failed turn ends in agent_error");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["reason"], "error");
        assert!(
            payload["message"].as_str().is_some_and(|m| !m.is_empty()),
            "the terminal carries the failure message: {payload}"
        );
    }

    // -- Turns run by an orchestrator in another process --

    /// Bound on a stream that should end on its own; a hung stream fails the
    /// test instead of running into the submit timeout.
    async fn frames_within_10s(resp: axum::response::Response) -> Vec<Frame> {
        tokio::time::timeout(std::time::Duration::from_secs(10), sse_frames(resp))
            .await
            .expect("the SSE stream must end once the turn completes")
    }

    #[tokio::test]
    async fn send_message_ends_with_cancelled_agent_error_when_cancelled_before_any_worker_claims_it()
     {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let (state, _) = state_over("http://127.0.0.1:1", &[default_space(&storage)], false).await;
        let orchestrator = default_orchestrator(&state).await;
        let conv_id = new_conversation(&storage).await;

        let resp = post_message(state, conv_id, "never claimed").await;
        let run_id = Uuid::parse_str(&run_id_of(&resp)).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !orchestrator.is_turn_in_flight(run_id).await {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("precondition: the turn is submitted");
        orchestrator.cancel_turn(run_id).await;
        let frames = frames_within_10s(resp).await;

        let (event, data, _) = frames.last().unwrap();
        assert_eq!(event, "agent_error");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["reason"], "cancelled");
    }

    /// Give `state` a dispatcher that delivers to `push_server`, and subscribe
    /// one browser (with valid RFC 8291 key material) in `storage`.
    async fn with_push_subscriber(
        mut state: ApiState,
        storage: &Arc<StorageLayer>,
        push_server: &MockServer,
    ) -> ApiState {
        use base64::Engine as _;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};

        Mock::given(method("POST"))
            .and(path("/push"))
            .respond_with(ResponseTemplate::new(201))
            .mount(push_server)
            .await;

        let (vapid_private, _) = crate::push::generate_vapid_keypair().unwrap();
        state.push_dispatcher = Some(Arc::new(crate::push::PushDispatcher::new(vapid_private)));

        let subscriber =
            p256::ecdsa::SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let p256dh = URL_SAFE_NO_PAD.encode(subscriber.verifying_key().to_encoded_point(false));
        let auth = URL_SAFE_NO_PAD.encode([7u8; 16]);
        assistant_storage::PushSubscriptionStore::upsert(
            &storage.push_subscription_store(),
            &format!("{}/push", push_server.uri()),
            &p256dh,
            &auth,
        )
        .await
        .unwrap();
        state
    }

    async fn push_count(push_server: &MockServer) -> usize {
        push_server
            .received_requests()
            .await
            .unwrap_or_default()
            .len()
    }

    #[tokio::test]
    async fn send_message_dispatches_one_push_when_the_turn_completes() {
        let llm = MockServer::start().await;
        mount_llm_reply(&llm, "Hello world").await;
        let push_server = MockServer::start().await;
        let (state, storage) = test_state(&llm.uri()).await;
        let state = with_push_subscriber(state, &storage, &push_server).await;
        let conv_id = new_conversation(&storage).await;

        let _ = sse_frames(post_message(state, conv_id, "hi").await).await;

        assert_eq!(
            push_count(&push_server).await,
            1,
            "a completed turn sends one Web Push after the done frame"
        );
    }

    #[tokio::test]
    async fn send_message_dispatches_push_when_the_turn_is_cancelled() {
        let llm = MockServer::start().await;
        mount_llm_hang(&llm).await;
        let push_server = MockServer::start().await;
        let (state, storage) = test_state(&llm.uri()).await;
        let orchestrator = default_orchestrator(&state).await;
        let state = with_push_subscriber(state, &storage, &push_server).await;
        let conv_id = new_conversation(&storage).await;

        let resp = post_message(state, conv_id, "slow").await;
        cancel_when_running(
            &orchestrator,
            &llm,
            Uuid::parse_str(&run_id_of(&resp)).unwrap(),
        )
        .await;
        let _ = sse_frames(resp).await;

        assert_eq!(
            push_count(&push_server).await,
            1,
            "the terminal push fires for a cancelled turn too"
        );
    }

    // -- stream_turn_to_sse driven directly -----------------------------------

    use super::{RunOptions, stream_turn_to_sse};

    fn system_ctx(storage: &Arc<StorageLayer>) -> crate::space::SpaceCtx {
        crate::space::SpaceCtx {
            auth: assistant_core::auth::AuthContext::system(),
            space_id: assistant_core::identity::SpaceId::from("default"),
            org_slug: "default".into(),
            space_slug: "default".into(),
            role: assistant_core::identity::Role::OrgAdmin,
            storage: storage.clone(),
        }
    }

    /// Runner options with the suggestion phase off.
    const NO_SUGGESTIONS: RunOptions = RunOptions {
        push: true,
        suggestions: false,
    };

    /// Run the shared runner over hand-fed channels; returns the run id and SSE frames.
    async fn run_runner(
        state: &ApiState,
        storage: &Arc<StorageLayer>,
        opts: RunOptions,
        events: Vec<assistant_runtime::OrchestratorEvent>,
        outcome: Option<anyhow::Result<assistant_runtime::TurnResult>>,
    ) -> (String, Vec<Frame>) {
        let conv_id = new_conversation(storage).await;
        run_runner_in(state, storage, conv_id, opts, events, outcome).await
    }

    /// [`run_runner`] in an existing conversation.
    async fn run_runner_in(
        state: &ApiState,
        storage: &Arc<StorageLayer>,
        conv_id: Uuid,
        opts: RunOptions,
        events: Vec<assistant_runtime::OrchestratorEvent>,
        outcome: Option<anyhow::Result<assistant_runtime::TurnResult>>,
    ) -> (String, Vec<Frame>) {
        let run_id = Uuid::new_v4();
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(64);
        for event in events {
            event_tx.send(event).await.unwrap();
        }
        drop(event_tx);
        let (turn_tx, turn_rx) = tokio::sync::oneshot::channel();
        match outcome {
            Some(outcome) => {
                let _ = turn_tx.send(outcome);
            }
            // No outcome: the submit task died, so the sender is dropped.
            None => drop(turn_tx),
        }

        let resp = stream_turn_to_sse(
            &system_ctx(storage),
            state,
            run_id,
            conv_id,
            opts,
            "the user message".to_string(),
            event_rx,
            turn_rx,
        )
        .await;
        assert_eq!(run_id_of(&resp), run_id.to_string());
        (run_id.to_string(), sse_frames(resp).await)
    }

    fn turn_result(answer: &str) -> assistant_runtime::TurnResult {
        assistant_runtime::TurnResult {
            answer: answer.to_string(),
            attachments: vec![],
            attachment_ids: vec![],
            message_id: None,
            had_errors: false,
            timing: None,
        }
    }

    #[tokio::test]
    async fn runner_done_frame_carries_message_and_attachment_ids() {
        let (state, storage) = event_log_state().await;
        let message_id = Uuid::new_v4();
        let attachment_id = Uuid::new_v4();
        let result = assistant_runtime::TurnResult {
            message_id: Some(message_id),
            attachment_ids: vec![attachment_id],
            ..turn_result("final answer")
        };

        let (run_id, frames) = run_runner(
            &state,
            &storage,
            NO_SUGGESTIONS,
            vec![assistant_runtime::OrchestratorEvent::Token("part".into())],
            Some(Ok(result)),
        )
        .await;

        let (event, data, id) = frames.last().unwrap();
        assert_eq!(event, "done");
        assert_eq!(*id, Some(2), "run_started=0, token=1, done=2");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["role"], "assistant");
        assert_eq!(payload["content"], "final answer");
        assert_eq!(payload["message_id"], message_id.to_string());
        assert_eq!(
            payload["attachment_ids"],
            serde_json::json!([attachment_id.to_string()])
        );
        assert_eq!(event_log(&storage, &run_id).await.len(), 3);
    }

    #[tokio::test]
    async fn runner_keeps_persisting_and_broadcasting_after_the_client_disconnects() {
        use assistant_runtime::OrchestratorEvent::Token;

        let (state, storage) = event_log_state().await;
        let conv_id = new_conversation(&storage).await;
        let run_id = Uuid::new_v4();
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(64);
        let (turn_tx, turn_rx) = tokio::sync::oneshot::channel();

        let resp = stream_turn_to_sse(
            &system_ctx(&storage),
            &state,
            run_id,
            conv_id,
            RunOptions {
                push: false,
                suggestions: false,
            },
            "hi".to_string(),
            event_rx,
            turn_rx,
        )
        .await;
        // The client goes away after run_started; subscribe a live tail first.
        let mut live = state
            .run_broadcaster
            .subscribe(&run_id)
            .await
            .expect("run is registered once the runner returns");
        drop(resp);

        for part in ["a", "b", "c"] {
            // The runner must still be draining; a closed channel is the bug.
            event_tx
                .send(Token(part.into()))
                .await
                .unwrap_or_else(|_| panic!("runner stopped draining after the disconnect"));
        }
        drop(event_tx);
        let _ = turn_tx.send(Ok(turn_result("abc")));

        let mut tailed = vec![];
        while let Some(event) = tokio::time::timeout(std::time::Duration::from_secs(5), live.recv())
            .await
            .expect("live tail stalled")
            .ok()
        {
            tailed.push((event.sequence, event.event_type));
        }
        let expected: Vec<(i64, String)> = [(1, "token"), (2, "token"), (3, "token"), (4, "done")]
            .into_iter()
            .map(|(seq, kind)| (seq, kind.to_string()))
            .collect();
        assert_eq!(
            tailed, expected,
            "live tail sees every frame and the terminal"
        );

        let logged: Vec<(i64, String)> = event_log(&storage, &run_id.to_string())
            .await
            .into_iter()
            .map(|(seq, kind, _)| (seq, kind))
            .collect();
        let mut all = vec![(0, "run_started".to_string())];
        all.extend(expected);
        assert_eq!(
            logged, all,
            "the event log has consecutive seq ids to the terminal"
        );
    }

    #[tokio::test]
    async fn runner_cancelled_terminal_reports_the_partial_text() {
        let (state, storage) = event_log_state().await;

        let (_, frames) = run_runner(
            &state,
            &storage,
            NO_SUGGESTIONS,
            vec![assistant_runtime::OrchestratorEvent::Token("par".into())],
            Some(Err(anyhow::anyhow!(
                assistant_runtime::TURN_CANCELLED_MARKER
            ))),
        )
        .await;

        let (event, data, _) = frames.last().unwrap();
        assert_eq!(event, "agent_error");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["reason"], "cancelled");
        assert_eq!(payload["partial_content"], "par");
    }

    #[tokio::test]
    async fn runner_reports_a_dropped_turn_channel_as_an_error() {
        let (state, storage) = event_log_state().await;

        let (_, frames) = run_runner(&state, &storage, NO_SUGGESTIONS, vec![], None).await;

        let (event, data, _) = frames.last().unwrap();
        assert_eq!(event, "agent_error");
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["reason"], "error");
        assert_eq!(payload["message"], "submit_turn channel dropped");
    }

    #[tokio::test]
    async fn runner_with_push_disabled_sends_no_push_even_when_configured() {
        let (state, storage) = event_log_state().await;
        let push_server = MockServer::start().await;
        let state = with_push_subscriber(state, &storage, &push_server).await;

        let (_, frames) = run_runner(
            &state,
            &storage,
            RunOptions {
                push: false,
                suggestions: false,
            },
            vec![],
            Some(Ok(turn_result("quiet"))),
        )
        .await;

        assert_eq!(frames.last().unwrap().0, "done");
        assert_eq!(
            push_count(&push_server).await,
            0,
            "push: false suppresses the completion push"
        );
    }

    // -- suggestion phase ------------------------------------------------------

    const SUGGEST: RunOptions = RunOptions {
        push: true,
        suggestions: true,
    };

    /// State over `llm` whose org has `[suggestions]` set as given.
    async fn suggestions_state(
        llm: &MockServer,
        workers: bool,
        enabled: bool,
        timeout_secs: u64,
    ) -> (ApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let mut config = assistant_core::types::agent::OrgConfig::default();
        config.memory.enabled = false;
        config.suggestions.enabled = enabled;
        config.suggestions.timeout_secs = timeout_secs;
        let (state, _) = state_over_config(
            std::path::Path::new("/nonexistent/assistant-test-root"),
            &llm.uri(),
            &[default_space(&storage)],
            workers,
            config,
        )
        .await;
        (state, storage)
    }

    fn done_with_message(answer: &str) -> assistant_runtime::TurnResult {
        assistant_runtime::TurnResult {
            message_id: Some(Uuid::new_v4()),
            ..turn_result(answer)
        }
    }

    async fn chat_calls(llm: &MockServer) -> usize {
        llm.received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| r.url.path() == "/api/chat")
            .count()
    }

    /// Wait until the run has been unregistered from the broadcaster.
    async fn wait_run_finished(state: &ApiState, run_id: &str, within: std::time::Duration) {
        let id = Uuid::parse_str(run_id).unwrap();
        tokio::time::timeout(within, async {
            while state.run_broadcaster.is_active(&id).await {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("run must finish");
    }

    async fn replay_frames(state: ApiState, conv_id: Uuid, run_id: &str, since: i64) -> Vec<Frame> {
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/conversations/{conv_id}/runs/{run_id}/events/stream?since={since}"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        sse_frames(resp).await
    }

    fn suggestion_prompts(frame: &Frame) -> Vec<String> {
        let payload: serde_json::Value = serde_json::from_str(&frame.1).unwrap();
        payload["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["prompt"].as_str().unwrap().to_string())
            .collect()
    }

    #[tokio::test]
    async fn suggestions_arrive_on_the_replay_tail_after_the_post_stream_closes() {
        let llm = MockServer::start().await;
        mount_llm_replies(&llm, &["The answer", "1. Show an example\n2. Explain more"]).await;
        let (state, storage) = suggestions_state(&llm, true, true, 10).await;
        let conv_id = new_conversation(&storage).await;

        let resp = post_message(state.clone(), conv_id, "hi").await;
        let run_id = run_id_of(&resp);
        let frames = sse_frames(resp).await;

        let (event, data, done_seq) = frames.last().unwrap();
        assert_eq!(event, "done", "the POST stream ends with done");
        assert!(
            frames.iter().all(|f| f.0 != "suggestions"),
            "the POST stream carries no suggestions: {frames:?}"
        );
        let done_seq = done_seq.unwrap();
        let message_id = serde_json::from_str::<serde_json::Value>(data).unwrap()["message_id"]
            .as_str()
            .unwrap()
            .to_string();

        let replayed = replay_frames(state, conv_id, &run_id, done_seq + 1).await;
        assert_eq!(
            replayed.len(),
            1,
            "only suggestions follow done: {replayed:?}"
        );
        assert_eq!(replayed[0].0, "suggestions");
        assert_eq!(replayed[0].2, Some(done_seq + 1));
        let payload: serde_json::Value = serde_json::from_str(&replayed[0].1).unwrap();
        assert_eq!(payload["message_id"], message_id);
        assert_eq!(
            suggestion_prompts(&replayed[0]),
            ["Show an example", "Explain more"]
        );

        let log = event_log(&storage, &run_id).await;
        let (seq, kind, _) = log.last().unwrap();
        assert_eq!((*seq, kind.as_str()), (done_seq + 1, "suggestions"));
    }

    #[tokio::test]
    async fn disabled_suggestions_make_no_extra_llm_call() {
        let llm = MockServer::start().await;
        mount_llm_replies(&llm, &["The answer", "1. Never asked"]).await;
        let (state, storage) = suggestions_state(&llm, true, false, 10).await;
        let conv_id = new_conversation(&storage).await;

        let resp = post_message(state.clone(), conv_id, "hi").await;
        let run_id = run_id_of(&resp);
        let _ = sse_frames(resp).await;
        wait_run_finished(&state, &run_id, std::time::Duration::from_secs(5)).await;

        assert_eq!(chat_calls(&llm).await, 1, "only the turn's own call");
        assert!(
            event_log(&storage, &run_id)
                .await
                .iter()
                .all(|e| e.1 != "suggestions")
        );
    }

    /// Run a hand-fed turn with the suggestion phase on and an LLM that would
    /// answer a suggestion call; returns how many `/api/chat` calls were made.
    async fn suggestion_calls_for(
        outcome: Option<anyhow::Result<assistant_runtime::TurnResult>>,
    ) -> usize {
        let llm = MockServer::start().await;
        mount_llm_reply(&llm, "1. A suggestion").await;
        let (state, storage) = suggestions_state(&llm, false, true, 10).await;
        let (run_id, _) = run_runner(&state, &storage, SUGGEST, vec![], outcome).await;
        wait_run_finished(&state, &run_id, std::time::Duration::from_secs(5)).await;
        assert!(
            event_log(&storage, &run_id)
                .await
                .iter()
                .all(|e| e.1 != "suggestions")
        );
        chat_calls(&llm).await
    }

    #[tokio::test]
    async fn cancelled_turn_makes_no_suggestion_call() {
        let outcome = Err(anyhow::anyhow!(assistant_runtime::TURN_CANCELLED_MARKER));
        assert_eq!(suggestion_calls_for(Some(outcome)).await, 0);
    }

    #[tokio::test]
    async fn errored_turn_makes_no_suggestion_call() {
        assert_eq!(
            suggestion_calls_for(Some(Err(anyhow::anyhow!("boom")))).await,
            0
        );
        assert_eq!(suggestion_calls_for(None).await, 0, "dropped turn channel");
    }

    #[tokio::test]
    async fn done_without_message_id_makes_no_suggestion_call() {
        assert_eq!(
            suggestion_calls_for(Some(Ok(turn_result("answer")))).await,
            0
        );
    }

    #[tokio::test]
    async fn receiver_dropped_before_done_makes_no_suggestion_call() {
        let llm = MockServer::start().await;
        mount_llm_reply(&llm, "1. A suggestion").await;
        let (state, storage) = suggestions_state(&llm, false, true, 10).await;
        let conv_id = new_conversation(&storage).await;
        let run_id = Uuid::new_v4();
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(64);
        let (turn_tx, turn_rx) = tokio::sync::oneshot::channel();

        let resp = stream_turn_to_sse(
            &system_ctx(&storage),
            &state,
            run_id,
            conv_id,
            SUGGEST,
            "hi".to_string(),
            event_rx,
            turn_rx,
        )
        .await;
        drop(resp); // the client leaves before the turn ends
        drop(event_tx);
        let _ = turn_tx.send(Ok(done_with_message("answer")));

        wait_run_finished(
            &state,
            &run_id.to_string(),
            std::time::Duration::from_secs(5),
        )
        .await;
        assert_eq!(chat_calls(&llm).await, 0, "nobody is listening");
    }

    #[tokio::test]
    async fn suggestion_call_that_outlasts_the_timeout_persists_nothing() {
        let llm = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "model": "test",
                        "message": {"role": "assistant", "content": "1. Too late"},
                        "done": true
                    }))
                    .set_delay(std::time::Duration::from_secs(3)),
            )
            .mount(&llm)
            .await;
        let (state, storage) = suggestions_state(&llm, false, true, 1).await;

        let (run_id, _) = run_runner(
            &state,
            &storage,
            SUGGEST,
            vec![],
            Some(Ok(done_with_message("answer"))),
        )
        .await;
        let started = std::time::Instant::now();
        wait_run_finished(&state, &run_id, std::time::Duration::from_secs(5)).await;

        assert!(
            started.elapsed() < std::time::Duration::from_millis(1800),
            "the run finishes about one timeout after done, took {:?}",
            started.elapsed()
        );
        assert!(
            event_log(&storage, &run_id)
                .await
                .iter()
                .all(|e| e.1 != "suggestions")
        );
    }

    #[tokio::test]
    async fn client_that_never_opens_a_replay_has_its_call_aborted_after_the_grace() {
        let llm = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "model": "test",
                        "message": {"role": "assistant", "content": "1. Nobody reads this"},
                        "done": true
                    }))
                    .set_delay(std::time::Duration::from_secs(8)),
            )
            .mount(&llm)
            .await;
        let (state, storage) = suggestions_state(&llm, false, true, 15).await;

        // The client reads the POST stream to `done` but never replays.
        let (run_id, _) = run_runner(
            &state,
            &storage,
            SUGGEST,
            vec![],
            Some(Ok(done_with_message("answer"))),
        )
        .await;
        let started = std::time::Instant::now();
        wait_run_finished(&state, &run_id, std::time::Duration::from_secs(6)).await;

        let elapsed = started.elapsed();
        assert!(
            elapsed >= std::time::Duration::from_millis(1500)
                && elapsed < std::time::Duration::from_secs(4),
            "aborted after the 2 s attach grace, took {elapsed:?}"
        );
        assert!(
            event_log(&storage, &run_id)
                .await
                .iter()
                .all(|e| e.1 != "suggestions")
        );
    }

    #[tokio::test]
    async fn push_is_sent_before_the_suggestion_call_starts() {
        // One server answers both the LLM and the push endpoint, so the order
        // wiremock saw the requests in is the order they were made.
        let server = MockServer::start().await;
        mount_llm_reply(&server, "1. A suggestion").await;
        let (state, storage) = suggestions_state(&server, false, true, 10).await;
        let state = with_push_subscriber(state, &storage, &server).await;

        let (run_id, _) = run_runner(
            &state,
            &storage,
            SUGGEST,
            vec![],
            Some(Ok(done_with_message("answer"))),
        )
        .await;
        wait_run_finished(&state, &run_id, std::time::Duration::from_secs(5)).await;

        let order: Vec<String> = server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|r| r.url.path().to_string())
            .collect();
        assert_eq!(
            order,
            ["/push", "/api/chat"],
            "push first, then suggestions"
        );
    }

    #[tokio::test]
    async fn a_runner_call_without_the_flag_makes_no_suggestion_call() {
        let llm = MockServer::start().await;
        mount_llm_reply(&llm, "1. A suggestion").await;
        let (state, storage) = suggestions_state(&llm, false, true, 10).await;

        let (run_id, _) = run_runner(
            &state,
            &storage,
            NO_SUGGESTIONS,
            vec![],
            Some(Ok(done_with_message("answer"))),
        )
        .await;
        wait_run_finished(&state, &run_id, std::time::Duration::from_secs(5)).await;

        assert_eq!(chat_calls(&llm).await, 0);
    }

    #[tokio::test]
    async fn quick_message_makes_no_suggestion_call() {
        let llm = MockServer::start().await;
        mount_llm_replies(&llm, &["Quick answer", "1. Never asked"]).await;
        let (state, _) = suggestions_state(&llm, true, true, 10).await;

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
        let _ = body_bytes(resp.into_body()).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        assert_eq!(chat_calls(&llm).await, 1, "only the turn's own call");
    }

    #[tokio::test]
    async fn send_voice_message_done_event_contains_message_id() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Voice reply").await;

        let (state, storage) = test_state(&server.uri()).await;
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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

    #[tokio::test]
    async fn voice_message_ends_with_done_when_another_process_runs_the_turn() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Voice from afar").await;
        let (state, storage) = split_process_state(&server.uri()).await;
        let conv_id = seed_conversation(&storage).await;
        let state = ApiState {
            transcription_provider: Some(Arc::new(StubTranscriptionProvider {
                transcript: "hello from voice".to_string(),
            })),
            ..state
        };
        let boundary = "testboundary";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"audio.webm\"\r\nContent-Type: audio/webm\r\n\r\nfakeaudio\r\n--{boundary}--\r\n",
        );

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conv_id}/voice"))
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let text = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            body_bytes(resp.into_body()),
        )
        .await
        .expect("the voice stream must end once the turn completes");

        let text = String::from_utf8_lossy(&text);
        assert!(
            text.contains("Voice from afar"),
            "the reply reaches the stream: {text}"
        );
    }

    // -- POST /conversations/{id}/voice — MIME validation & size limit ---------

    /// Post a `fakeaudio` webm recording to the voice endpoint and return the
    /// SSE body.
    async fn post_voice(state: ApiState, conv_id: uuid::Uuid) -> String {
        let boundary = "testboundary";
        let body = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"audio.webm\"\r\nContent-Type: audio/webm\r\n\r\nfakeaudio\r\n--{boundary}--\r\n",
        );
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conv_id}/voice"))
                    .header(
                        "content-type",
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        String::from_utf8_lossy(&body_bytes(resp.into_body()).await).into_owned()
    }

    /// The text of the SSE `transcript` event.
    fn transcript_text(sse: &str) -> String {
        let payloads = frame_payloads(sse, "transcript");
        assert_eq!(payloads.len(), 1, "one transcript event: {sse}");
        let json: serde_json::Value = serde_json::from_str(&payloads[0]).unwrap();
        json["content"].as_str().unwrap().to_string()
    }

    /// The conversation's only attachment must be the recording, linked to
    /// the user message so the orchestrator turned it into an audio block.
    async fn assert_recording_attached(storage: &StorageLayer, conv_id: uuid::Uuid) {
        use assistant_storage::AttachmentStore as _;
        let store = storage.attachment_store();
        let attachments = store.list_for_conversation(conv_id).await.unwrap();
        assert_eq!(attachments.len(), 1, "one stored recording");
        let meta = &attachments[0];
        assert_eq!(meta.mime_type, "audio/webm");
        assert!(meta.message_id.is_some(), "linked to the user message");
        assert_eq!(store.load_bytes(meta.id).await.unwrap(), b"fakeaudio");
    }

    #[tokio::test]
    async fn voice_with_transcription_stores_the_recording() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Voice reply").await;
        let (state, storage) = test_state(&server.uri()).await;
        let conv_id = seed_conversation(&storage).await;
        let state = ApiState {
            transcription_provider: Some(Arc::new(StubTranscriptionProvider {
                transcript: "hello from voice".to_string(),
            })),
            ..state
        };

        let sse = post_voice(state, conv_id).await;

        assert_eq!(transcript_text(&sse), "hello from voice");
        assert_recording_attached(&storage, conv_id).await;
    }

    #[tokio::test]
    async fn voice_upload_without_transcription_provider_runs_turn_with_placeholder() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "Voice reply").await;
        let (state, storage) = test_state(&server.uri()).await;
        let conv_id = seed_conversation(&storage).await;
        assert!(state.transcription_provider.is_none());

        let sse = post_voice(state, conv_id).await;

        assert_eq!(transcript_text(&sse), "[Voice message]");
        assert!(sse.contains("Voice reply"), "the turn ran: {sse}");
        assert_recording_attached(&storage, conv_id).await;
    }

    #[tokio::test]
    async fn voice_upload_keeps_the_turn_when_transcription_fails_or_is_empty() {
        let providers: [Arc<dyn assistant_transcription::TranscriptionProvider>; 2] = [
            Arc::new(FailingTranscriptionProvider),
            Arc::new(StubTranscriptionProvider {
                transcript: "  ".to_string(),
            }),
        ];
        for provider in providers {
            let server = MockServer::start().await;
            mount_llm_reply(&server, "Voice reply").await;
            let (state, storage) = test_state(&server.uri()).await;
            let conv_id = seed_conversation(&storage).await;
            let state = ApiState {
                transcription_provider: Some(provider),
                ..state
            };

            let sse = post_voice(state, conv_id).await;

            assert_eq!(transcript_text(&sse), "[Voice message]");
            assert_recording_attached(&storage, conv_id).await;
        }
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

    // -- tool data replay ------------------------------------------------------

    /// A tool that returns structured `data` next to its text content.
    struct DataTool;

    #[async_trait::async_trait]
    impl assistant_core::ToolHandler for DataTool {
        fn name(&self) -> &str {
            "data-stub"
        }
        fn description(&self) -> &str {
            "Returns structured data."
        }
        fn params_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}, "required": []})
        }
        async fn run(
            &self,
            _params: HashMap<String, serde_json::Value>,
            _ctx: &assistant_core::types::conversation::ExecutionContext,
        ) -> anyhow::Result<assistant_core::ToolOutput> {
            Ok(assistant_core::ToolOutput::success("plain text")
                .with_data(serde_json::json!({"kind": "terminal", "exit_code": 0})))
        }
    }

    /// The raw `data:` payload of every `event` frame in an SSE body.
    fn frame_payloads(sse: &str, event: &str) -> Vec<String> {
        let marker = format!("event: {event}");
        sse.split("\n\n")
            .filter(|frame| frame.lines().any(|l| l.trim() == marker))
            .map(|frame| {
                frame
                    .lines()
                    .filter_map(|l| l.strip_prefix("data:"))
                    .map(str::trim_start)
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect()
    }

    /// The `tool_result` frames in an SSE body, parsed from their `data:` lines.
    fn tool_result_frames(sse: &str) -> Vec<serde_json::Value> {
        frame_payloads(sse, "tool_result")
            .iter()
            .filter_map(|data| serde_json::from_str(data).ok())
            .collect()
    }

    /// A turn whose tool returns `data` streams it live, and replaying the
    /// persisted run log yields the identical `tool_result` frame.
    #[tokio::test]
    async fn replayed_tool_result_frame_carries_the_same_data() {
        let server = MockServer::start().await;
        // First LLM call asks for the tool; every later call answers in text.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "model": "test",
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{"function": {"name": "data-stub", "arguments": {}}}]
                },
                "done": true
            })))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        mount_llm_reply(&server, "all done").await;

        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let mut config = assistant_core::types::agent::OrgConfig::default();
        config.memory.enabled = false;
        // The stub is not a built-in, so no rule would allow it otherwise.
        config.permissions.allow = vec!["*".parse().unwrap()];
        let (state, _) = state_over_config(
            std::path::Path::new("/nonexistent/assistant-test-root"),
            &server.uri(),
            &[default_space(&storage)],
            true,
            config,
        )
        .await;
        state.runtimes.register_ambient_tool(Arc::new(DataTool));
        let conv_id = seed_conversation(&storage).await;

        let resp = app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conv_id}/messages"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"message":"run the stub"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let run_id = resp
            .headers()
            .get("X-Run-Id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .expect("send_message sets X-Run-Id");
        let live = String::from_utf8_lossy(&body_bytes(resp.into_body()).await).into_owned();

        let replay = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/conversations/{conv_id}/runs/{run_id}/events/stream?since=0"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::OK);
        let replayed = String::from_utf8_lossy(&body_bytes(replay.into_body()).await).into_owned();

        let expected = serde_json::json!({"kind": "terminal", "exit_code": 0});
        let live_frames = tool_result_frames(&live);
        let replayed_frames = tool_result_frames(&replayed);
        assert_eq!(live_frames.len(), 1, "one live tool_result: {live}");
        assert_eq!(live_frames[0]["data"], expected, "live frame data");
        assert_eq!(
            replayed_frames, live_frames,
            "replayed tool_result must equal the live frame; replay: {replayed}"
        );
    }

    // -- split topology: the worker runs in another runtime ---------------------

    /// Two runtimes over one `space.db` and one bus: the first serves HTTP and
    /// runs no worker, the second runs the turn worker, as in the packaged
    /// install where the web process and the orchestrator are separate.
    async fn split_topology(llm_url: &str) -> (ApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let mut config = assistant_core::types::agent::OrgConfig::default();
        config.memory.enabled = false;
        config.permissions.allow = vec!["*".parse().unwrap()];
        let root = std::path::Path::new("/nonexistent/assistant-test-root");
        let (web, _) = state_over_config(
            root,
            llm_url,
            &[default_space(&storage)],
            false,
            config.clone(),
        )
        .await;
        let (worker, _) =
            state_over_config(root, llm_url, &[default_space(&storage)], true, config).await;
        worker.runtimes.register_ambient_tool(Arc::new(DataTool));
        // Keep the worker's runtimes alive for as long as the test runs.
        std::mem::forget(worker);
        (web, storage)
    }

    #[tokio::test]
    async fn a_turn_run_by_another_runtime_streams_to_the_sse_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "model": "test",
                "message": {
                    "role": "assistant",
                    "content": "",
                    "tool_calls": [{"function": {"name": "data-stub", "arguments": {}}}]
                },
                "done": true
            })))
            .up_to_n_times(1)
            .with_priority(1)
            .mount(&server)
            .await;
        mount_llm_reply(&server, "all done").await;
        let (state, storage) = split_topology(&server.uri()).await;
        let conv_id = seed_conversation(&storage).await;

        let resp = post_message(state, conv_id, "run the stub").await;
        let frames = tokio::time::timeout(std::time::Duration::from_secs(15), sse_frames(resp))
            .await
            .expect("the stream ends with the turn");

        let events: Vec<&str> = frames.iter().map(|(event, _, _)| event.as_str()).collect();
        assert!(
            events.contains(&"tool_result"),
            "tool result relayed: {events:?}"
        );
        let tokens: String = frames
            .iter()
            .filter(|(event, _, _)| event == "token")
            .map(|(_, data, _)| data.as_str())
            .collect();
        assert_eq!(tokens, "all done", "tokens relayed: {events:?}");
        let (last, data, _) = frames.last().unwrap();
        assert_eq!(
            last, "done",
            "the turn's own terminal frame closes the stream"
        );
        let payload: serde_json::Value = serde_json::from_str(data).unwrap();
        assert_eq!(payload["content"], "all done");
        let sequences: Vec<_> = frames.iter().filter_map(|(_, _, id)| *id).collect();
        let expected: Vec<i64> = (0..frames.len() as i64).collect();
        assert_eq!(sequences, expected, "one writer numbers the frames");
    }

    /// A persisted `surface_update` frame replays with the exact `data:` bytes
    /// the live projection sent, so a reload sees the surface unchanged.
    #[tokio::test]
    async fn stream_run_events_replays_surface_update_byte_identically() {
        use assistant_runtime::orchestrator::OrchestratorEvent;
        use assistant_runtime::projection::{SseProjector, StreamProjector};

        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4().to_string();

        let live = SseProjector.project(&OrchestratorEvent::SurfaceUpdate {
            surface_id: "s-1".into(),
            tool_call_id: Some("call_9".into()),
            anchor_message_id: "m-1".into(),
            anchor_call_index: 2,
            title: "Trip form".into(),
            fallback_text: "Trip form: choose a date".into(),
            status: "active".into(),
            messages: vec![serde_json::json!({
                "version": "1.0",
                "createSurface": {"surfaceId": "s-1", "components": [{"id": "root", "component": "Text", "text": "héllo"}]}
            })],
        });
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].event, "surface_update");
        storage
            .conversation_event_store()
            .append_event(&run_id, &conv_id, 0, &live[0].event, &live[0].log_payload)
            .await
            .unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/conversations/{conv_id}/runs/{run_id}/events/stream"
                    ))
                    .header("Authorization", "Bearer test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let replayed = String::from_utf8_lossy(&body_bytes(resp.into_body()).await).into_owned();
        assert_eq!(
            frame_payloads(&replayed, "surface_update"),
            vec![live[0].sse_data.clone()],
            "replay: {replayed}"
        );
    }

    // -- stream_run_events tests -----------------------------------------------

    /// Seed the event store directly and replay via the endpoint.
    #[tokio::test]
    async fn stream_run_events_replays_completed_run() {
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4().to_string();

        // Seed events into the store.
        storage
            .conversation_event_store()
            .append_event(
                &run_id,
                &conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();
        storage
            .conversation_event_store()
            .append_event(
                &run_id,
                &conv_id,
                1,
                "token",
                &serde_json::json!({"token": "hello"}),
            )
            .await
            .unwrap();
        storage
            .conversation_event_store()
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
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
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
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4().to_string();

        for i in 0i64..5 {
            storage
                .conversation_event_store()
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
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4();
        let run_id_str = run_id.to_string();

        // Seed one already-persisted event (the run_started from before client connects).
        storage
            .conversation_event_store()
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

        // Spawn a task that delivers live events once the HTTP handler has
        // subscribed; broadcast drops events sent before any receiver exists.
        let run_id_for_task = run_id;
        tokio::spawn(async move {
            tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
                while broadcast_tx.receiver_count() == 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                }
            })
            .await
            .expect("handler never subscribed to the run broadcaster");
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

    /// A finished run replays every stored row, including those after `done`.
    #[tokio::test]
    async fn stream_run_events_replays_rows_after_done_and_closes() {
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4().to_string();

        for (seq, kind) in [(0, "run_started"), (1, "done"), (2, "suggestions")] {
            storage
                .conversation_event_store()
                .append_event(&run_id, &conv_id, seq, kind, &serde_json::json!({}))
                .await
                .unwrap();
        }

        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id}/events/stream"
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp.into_body()).await;
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("event: done"), "done missing: {text}");
        assert!(
            text.contains("event: suggestions"),
            "suggestions after done missing: {text}"
        );
    }

    /// An active run whose `done` row is already stored keeps tailing, and
    /// delivers an event broadcast after `done`, until the run finishes.
    #[tokio::test]
    async fn stream_run_events_tails_past_done_until_run_finishes() {
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4();
        let run_id_str = run_id.to_string();

        for (seq, kind) in [(0, "run_started"), (1, "done")] {
            storage
                .conversation_event_store()
                .append_event(&run_id_str, &conv_id, seq, kind, &serde_json::json!({}))
                .await
                .unwrap();
        }

        let broadcast_tx = state.run_broadcaster.start_run(run_id).await;
        let broadcaster = state.run_broadcaster.clone();
        tokio::spawn(async move {
            tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
                while broadcast_tx.receiver_count() == 0 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                }
            })
            .await
            .expect("handler never subscribed to the run broadcaster");
            let _ = broadcast_tx.send(assistant_storage::LiveEvent {
                sequence: 2,
                event_type: "suggestions".to_string(),
                payload: serde_json::json!({"suggestions": [{"prompt": "later-prompt"}]}),
            });
            broadcaster.finish_run(&run_id).await;
        });

        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id_str}/events/stream"
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = tokio::time::timeout(
            tokio::time::Duration::from_secs(10),
            body_bytes(resp.into_body()),
        )
        .await
        .expect("stream must close once the run finishes");
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("event: done"),
            "replayed done missing: {text}"
        );
        assert!(
            text.contains("later-prompt"),
            "event broadcast after done missing: {text}"
        );
    }

    /// A replay client that goes away releases its broadcast receiver
    /// without any further event being broadcast.
    #[tokio::test]
    async fn stream_run_events_releases_receiver_when_client_drops() {
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4();
        let run_id_str = run_id.to_string();

        // No stored rows: the handler has nothing to replay, so it can only
        // notice the departed client while it waits on the live channel.
        let broadcast_tx = state.run_broadcaster.start_run(run_id).await;

        let req = Request::builder()
            .uri(format!(
                "/conversations/{conv_id}/runs/{run_id_str}/events/stream"
            ))
            .body(Body::empty())
            .unwrap();
        let resp = app(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body();

        tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
            while broadcast_tx.receiver_count() == 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("handler never subscribed to the run broadcaster");

        drop(body);

        tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
            while broadcast_tx.receiver_count() != 0 {
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("departed client must release its broadcast receiver");
    }

    /// Race-condition path: broadcaster is active but no DB events have been
    /// persisted yet (run_started event is still in-flight). The handler should
    /// recognise the run as active and switch to live-tail mode.
    #[tokio::test]
    async fn stream_run_events_live_only_when_no_db_events_yet() {
        let (state, storage) = event_log_state().await;
        let conv_id = seed_conversation(&storage).await.to_string();
        let run_id = Uuid::new_v4();
        let run_id_str = run_id.to_string();

        // Register as active — no DB events at all.
        let broadcast_tx = state.run_broadcaster.start_run(run_id).await;
        let broadcaster = state.run_broadcaster.clone();

        let run_id_for_task = run_id;
        tokio::spawn(async move {
            // Publish only once the SSE handler has subscribed; a broadcast
            // event sent earlier is lost and `finish_run` would hide the run.
            while broadcast_tx.receiver_count() == 0 {
                tokio::task::yield_now().await;
            }
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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
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

    /// After events are pruned (TTL), the endpoint returns 404, the same as
    /// for a run that never existed.
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
        let conv_id = SqliteConversationStore::for_agent(sl.pool.clone(), "default")
            .with_user_id("system")
            .create_conversation(None)
            .await
            .unwrap()
            .id
            .to_string();

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
        let (state, _) = super::super::test_helpers::state_over(
            "http://127.0.0.1:1",
            &[super::super::test_helpers::TestSpace {
                id: "default",
                org_id: "default",
                org_slug: "default",
                slug: "default",
                storage: sl.clone(),
            }],
            false,
        )
        .await;

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
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.get_conversation(conv_id).await.unwrap().unwrap();
        assert!(
            conv.title.is_none(),
            "quick-message must not auto-truncate the title; got {:?}",
            conv.title
        );
        assert!(!conv.title_locked);
    }

    #[tokio::test]
    async fn quick_message_with_serving_persona_id_runs_turn_as_that_persona() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "persona answer").await;

        let (state, storage) = test_state(&server.uri()).await;
        let serving = state
            .runtimes
            .get("default", "default")
            .await
            .unwrap()
            .orchestrator
            .agent_id()
            .to_string();

        let persona_store =
            assistant_storage::personas::SqlitePersonaStore::new(storage.pool.clone());
        if persona_store.get(&serving).await.unwrap().is_none() {
            persona_store.create(&serving, "Serving").await.unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quick-message")
                    .header("Content-Type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"message": "hello", "persona_id": serving}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let json = body_json(resp.into_body()).await;
        assert_eq!(
            json["answer"].as_str().unwrap(),
            "persona answer",
            "turn must complete rather than report a failure as the answer"
        );
        let conv_id: Uuid = json["conversation_id"].as_str().unwrap().parse().unwrap();

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), &serving);
        assert!(
            store.get_conversation(conv_id).await.unwrap().is_some(),
            "conversation should exist under the serving persona"
        );
    }

    #[tokio::test]
    async fn quick_message_with_persona_not_served_by_orchestrator_is_rejected() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "must not run").await;

        let (state, storage) = test_state(&server.uri()).await;

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

        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let json = body_json(resp.into_body()).await;
        assert!(
            json["error"].as_str().unwrap().contains("reviewer"),
            "error should name the unserved persona: {json}"
        );

        let (conversations,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM conversations")
            .fetch_one(&storage.pool)
            .await
            .unwrap();
        assert_eq!(conversations, 0, "no conversation should be created");
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
