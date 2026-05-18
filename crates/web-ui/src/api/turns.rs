//! Turn-status read + cancel API.
//!
//! - `GET /api/conversations/{conversation_id}/turns/{turn_id}/status`
//!   returns the authoritative server-side state of a single turn
//!   (running, completed, errored, or unknown). The client polls this
//!   when it suspects a stream has stalled — replacing the byte-watchdog
//!   driven cancellation heuristic with an explicit health probe.
//!
//! - `POST /api/conversations/{conversation_id}/turns/{turn_id}/cancel`
//!   triggers the runtime's per-turn cancellation token. The worker
//!   aborts in-flight LLM/tool work on its next yield point, `submit_turn`
//!   bails with the `turn_cancelled` marker, and the SSE handler emits a
//!   final `agent_error` event with `{"reason": "cancelled"}`. The
//!   endpoint is idempotent: cancelling a completed / errored / unknown
//!   turn is a no-op that returns the current status.
//!
//! Spec: `openspec/changes/turn-status-endpoint/specs/turn-status-api/spec.md`

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use super::ApiState;
use crate::openapi::ErrorBody;

/// Closed enum of turn states surfaced by the status endpoint.
///
/// Anything not in this set must round-trip to one of these four when
/// serialised — the client's switch is exhaustive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TurnState {
    /// The turn is being processed by the runtime (LLM in flight, tool
    /// invocation, subagent task — anything that hasn't terminated).
    Running,
    /// The turn finished cleanly; a `done` event was persisted.
    Completed,
    /// The turn terminated with an `agent_error` event; a partial reply
    /// may or may not be in the conversation history.
    Errored,
    /// The turn ID is not recorded — either it never existed, has been
    /// garbage-collected, or belongs to a different conversation than
    /// the path's `{conversation_id}` says it does. Distinct from a
    /// 404 because the client can react meaningfully (refetch the
    /// conversation; the turn may have been replaced).
    Unknown,
}

/// Response body for `GET .../turns/{turn_id}/status`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TurnStatusResponse {
    pub turn_id: String,
    pub conversation_id: String,
    pub state: TurnState,
    /// Wall-clock timestamp of the most recent SSE event recorded for
    /// the turn, in RFC 3339 format. `null` when [`state`] is `unknown`.
    pub last_event_at: Option<String>,
    /// Kind of the most recent SSE event (`run_started`, `token`,
    /// `status`, `thinking`, `tool_result`, `agent_error`, `done`, etc.).
    /// `null` when [`state`] is `unknown`.
    pub last_event_kind: Option<String>,
}

/// `GET /api/conversations/{conversation_id}/turns/{turn_id}/status`
///
/// Returns the authoritative state of the named turn.
///
/// Authorisation: the existing conversation-scoped auth middleware applies.
#[utoipa::path(
    get,
    path = "/api/conversations/{conversation_id}/turns/{turn_id}/status",
    params(
        ("conversation_id" = Uuid, Path, description = "Conversation UUID"),
        ("turn_id" = Uuid, Path, description = "Turn (run) UUID"),
    ),
    responses(
        (status = 200, description = "Turn status", body = TurnStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorBody),
    ),
    operation_id = "get_turn_status",
    tag = "conversations",
    security(("bearer_token" = [])),
)]
pub async fn get_turn_status(
    State(state): State<ApiState>,
    Path((conversation_id, turn_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TurnStatusResponse>, (StatusCode, Json<ErrorBody>)> {
    let turn_id_str = turn_id.to_string();
    let conv_id_str = conversation_id.to_string();

    // Walk the event log for this turn. We use list_events_since(_, 0) and
    // take the last event so we get both the recency timestamp and the
    // kind in one query. For typical turns this is O(events_per_turn);
    // for very long-running turns we could add a "tail" query later.
    let events = state
        .event_store
        .list_events_since(&turn_id_str, 0)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: format!("Failed to read turn events: {e}"),
                }),
            )
        })?;

    // No events at all (or events belong to a different conversation) →
    // `unknown`. We additionally guard against turn-id reuse across
    // conversations by checking conversation_id on the first row.
    if events.is_empty() {
        return Ok(Json(TurnStatusResponse {
            turn_id: turn_id_str,
            conversation_id: conv_id_str,
            state: TurnState::Unknown,
            last_event_at: None,
            last_event_kind: None,
        }));
    }
    if events
        .first()
        .map(|e| e.conversation_id != conv_id_str)
        .unwrap_or(false)
    {
        return Ok(Json(TurnStatusResponse {
            turn_id: turn_id_str,
            conversation_id: conv_id_str,
            state: TurnState::Unknown,
            last_event_at: None,
            last_event_kind: None,
        }));
    }

    // The empty-events branch returned above, so `last` is always Some.
    let Some(last) = events.last() else {
        return Ok(Json(TurnStatusResponse {
            turn_id: turn_id_str,
            conversation_id: conv_id_str,
            state: TurnState::Unknown,
            last_event_at: None,
            last_event_kind: None,
        }));
    };
    let last_kind = last.event_type.clone();
    let last_at = last.created_at.to_rfc3339();

    let derived_state = match last_kind.as_str() {
        "done" => TurnState::Completed,
        "agent_error" => TurnState::Errored,
        _ => TurnState::Running,
    };

    Ok(Json(TurnStatusResponse {
        turn_id: turn_id_str,
        conversation_id: conv_id_str,
        state: derived_state,
        last_event_at: Some(last_at),
        last_event_kind: Some(last_kind),
    }))
}

/// `POST /api/conversations/{conversation_id}/turns/{turn_id}/cancel`
///
/// Idempotent: triggers `Orchestrator::cancel_turn` if a matching token is
/// registered, otherwise no-op. Always returns 200 with the current turn
/// status — the cancellation propagates asynchronously through the worker's
/// `tokio::select!`, the SSE stream's terminal `agent_error` event, and the
/// `event_store`. Clients should poll `GET .../status` (or wait for the
/// stream's `agent_error`) to observe the transition.
#[utoipa::path(
    post,
    path = "/api/conversations/{conversation_id}/turns/{turn_id}/cancel",
    params(
        ("conversation_id" = Uuid, Path, description = "Conversation UUID"),
        ("turn_id" = Uuid, Path, description = "Turn (run) UUID — matches the SSE run_id"),
    ),
    responses(
        (status = 200, description = "Current turn status after cancel signal", body = TurnStatusResponse),
        (status = 401, description = "Unauthorized", body = ErrorBody),
    ),
    operation_id = "cancel_turn",
    tag = "conversations",
    security(("bearer_token" = [])),
)]
pub async fn cancel_turn(
    State(state): State<ApiState>,
    Path((conversation_id, turn_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TurnStatusResponse>, (StatusCode, Json<ErrorBody>)> {
    // Fire the cancellation signal. Outcome is intentionally not surfaced —
    // the endpoint is idempotent and the authoritative reply is whatever
    // the event store shows after the worker reacts.
    let _outcome = state.orchestrator.cancel_turn(turn_id).await;

    // Return the current status. The transition from `running` →
    // `errored` (via the synthetic `agent_error` event from the SSE
    // handler) may not be visible yet — that's fine. The client polls.
    get_turn_status(State(state), Path((conversation_id, turn_id))).await
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::super::test_helpers::{app, body_json, event_log_state};

    /// Helper: build the status URL.
    fn url(conv: &Uuid, turn: &Uuid) -> String {
        format!("/conversations/{conv}/turns/{turn}/status")
    }

    /// A run with only intermediate events (run_started + a token) → `running`.
    #[tokio::test]
    async fn returns_running_for_in_flight_turn() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();
        let turn_id = Uuid::new_v4();

        state
            .event_store
            .append_event(
                &turn_id.to_string(),
                &conv_id.to_string(),
                0,
                "run_started",
                &serde_json::json!({"run_id": turn_id.to_string()}),
            )
            .await
            .unwrap();
        state
            .event_store
            .append_event(
                &turn_id.to_string(),
                &conv_id.to_string(),
                1,
                "token",
                &serde_json::json!({"token": "hi"}),
            )
            .await
            .unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(url(&conv_id, &turn_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "running");
        assert_eq!(body["turn_id"], turn_id.to_string());
        assert_eq!(body["conversation_id"], conv_id.to_string());
        assert_eq!(body["last_event_kind"], "token");
        assert!(body["last_event_at"].is_string());
    }

    /// A run terminated by `done` → `completed`.
    #[tokio::test]
    async fn returns_completed_when_done_event_present() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();
        let turn_id = Uuid::new_v4();

        for (seq, kind) in [(0, "run_started"), (1, "token"), (2, "done")] {
            state
                .event_store
                .append_event(
                    &turn_id.to_string(),
                    &conv_id.to_string(),
                    seq,
                    kind,
                    &serde_json::json!({}),
                )
                .await
                .unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(url(&conv_id, &turn_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "completed");
        assert_eq!(body["last_event_kind"], "done");
    }

    /// A run terminated by `agent_error` → `errored`.
    #[tokio::test]
    async fn returns_errored_when_agent_error_event_present() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();
        let turn_id = Uuid::new_v4();

        for (seq, kind) in [(0, "run_started"), (1, "agent_error")] {
            state
                .event_store
                .append_event(
                    &turn_id.to_string(),
                    &conv_id.to_string(),
                    seq,
                    kind,
                    &serde_json::json!({"error": "boom"}),
                )
                .await
                .unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(url(&conv_id, &turn_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "errored");
        assert_eq!(body["last_event_kind"], "agent_error");
    }

    /// A turn id that was never seeded → `unknown`. The endpoint returns 200,
    /// not 404, so the Flutter client can react meaningfully (the run may
    /// have been replaced or GC'd) without parsing error bodies.
    #[tokio::test]
    async fn returns_unknown_for_missing_turn() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();
        let turn_id = Uuid::new_v4(); // never seeded

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(url(&conv_id, &turn_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "unknown");
        assert!(body["last_event_at"].is_null());
        assert!(body["last_event_kind"].is_null());
    }

    // -- POST .../cancel ------------------------------------------------------

    /// Cancelling a turn the server doesn't know about is a no-op that
    /// returns `unknown` (same shape as the status endpoint). The endpoint
    /// is idempotent.
    #[tokio::test]
    async fn cancel_unknown_turn_returns_unknown() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();
        let turn_id = Uuid::new_v4(); // never seeded, no registered token

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conv_id}/turns/{turn_id}/cancel"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "unknown");
    }

    /// Cancelling a turn that already finished cleanly returns its current
    /// `completed` state without modification. The cancel signal fires
    /// (no-op since no token registered), then the status read returns
    /// what the event store has.
    #[tokio::test]
    async fn cancel_completed_turn_returns_completed() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();
        let turn_id = Uuid::new_v4();

        for (seq, kind) in [(0, "run_started"), (1, "done")] {
            state
                .event_store
                .append_event(
                    &turn_id.to_string(),
                    &conv_id.to_string(),
                    seq,
                    kind,
                    &serde_json::json!({}),
                )
                .await
                .unwrap();
        }

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conv_id}/turns/{turn_id}/cancel"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "completed");
    }

    /// Cancelling a turn whose `(conversation_id, turn_id)` pair doesn't
    /// match returns `unknown` (same guard as the status endpoint).
    #[tokio::test]
    async fn cancel_mismatched_conversation_returns_unknown() {
        let (state, _storage) = event_log_state().await;
        let real_conv = Uuid::new_v4();
        let other_conv = Uuid::new_v4();
        let turn_id = Uuid::new_v4();

        state
            .event_store
            .append_event(
                &turn_id.to_string(),
                &real_conv.to_string(),
                0,
                "run_started",
                &serde_json::json!({}),
            )
            .await
            .unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/conversations/{other_conv}/turns/{turn_id}/cancel"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "unknown");
    }

    // -- existing status tests below --

    /// Guard against turn-id reuse across conversations: if a path conversation
    /// id does not match the conversation recorded against the turn's events,
    /// the response is `unknown` (the client should refetch the conversation).
    #[tokio::test]
    async fn returns_unknown_when_conversation_id_mismatches() {
        let (state, _storage) = event_log_state().await;
        let real_conv = Uuid::new_v4();
        let other_conv = Uuid::new_v4();
        let turn_id = Uuid::new_v4();

        state
            .event_store
            .append_event(
                &turn_id.to_string(),
                &real_conv.to_string(),
                0,
                "run_started",
                &serde_json::json!({}),
            )
            .await
            .unwrap();

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(url(&other_conv, &turn_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp.into_body()).await;
        assert_eq!(body["state"], "unknown");
        assert_eq!(body["conversation_id"], other_conv.to_string());
        assert!(body["last_event_kind"].is_null());
    }
}
