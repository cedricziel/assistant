//! Turn interactions API (ADR-0017): the owner of a running turn answers a
//! question the turn put to them, such as a tool approval.
//!
//! - `POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`
//!
//! The answer is validated by the kind's handler in the runtime's
//! `InteractionKinds` registry, so this module never matches on a kind except
//! to word a `decline`.
//!
//! Spec: `openspec/changes/tool-approval-prompts/specs/turn-interactions/spec.md`

use std::sync::Arc;

use axum::Extension;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::warn;
use utoipa::ToSchema;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_runtime::interaction::{
    InteractionBroker, InteractionKinds, InteractionOutcome, RespondError, Responder,
};
use assistant_storage::{InteractionRow, InteractionStatus};

use super::ApiState;
use super::conversations::{caller_store, owned_conversation};
use super::messages::caller_can_post;
use crate::errors::json_error;
use crate::openapi::ErrorBody;
use crate::space::{Access, SpaceCtx};

/// Largest accepted respond body (the route's body limit).
pub const MAX_RESPOND_BODY_BYTES: usize = 64 * 1024;

// -- Request --

/// What the owner decided about a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolDecision {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

impl ToolDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::AllowOnce => "allow_once",
            Self::AllowAlways => "allow_always",
            Self::RejectOnce => "reject_once",
            Self::RejectAlways => "reject_always",
        }
    }
}

/// The owner's answer to one question of a `question` interaction.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct QuestionAnswer {
    /// Id of the question being answered.
    pub question_id: String,
    /// Ids of the chosen options. Empty when the answer is free text only.
    #[serde(default)]
    pub selected_option_ids: Vec<String>,
    /// Free-text answer, at most 4000 characters, where the question allows it.
    #[serde(default)]
    pub text: Option<String>,
}

/// Body of the respond route, tagged by `kind`. Unknown fields are rejected,
/// so a client cannot smuggle in a field such as a permission rule: the saved
/// rule comes from the stored interaction, never from the body.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RespondToInteractionRequest {
    /// Answers a `tool_approval` interaction with one of its offered options.
    ToolApproval { decision: ToolDecision },
    /// Answers a `question` interaction.
    Question { answers: Vec<QuestionAnswer> },
    /// "I won't answer", for any kind. For an approval this means reject once.
    Decline,
}

impl RespondToInteractionRequest {
    /// The interaction kind this body answers, or `None` for [`Self::Decline`],
    /// which fits any.
    fn kind(&self) -> Option<&'static str> {
        match self {
            Self::ToolApproval { .. } => Some("tool_approval"),
            Self::Question { .. } => Some("question"),
            Self::Decline => None,
        }
    }

    /// The stored response for an interaction of `interaction_kind`.
    fn into_response(self, interaction_kind: &str) -> Value {
        match self {
            Self::ToolApproval { decision } => json!({"decision": decision.as_str()}),
            Self::Question { answers } => {
                let answers: Vec<Value> = answers
                    .into_iter()
                    .map(|a| {
                        let mut answer = json!({
                            "question_id": a.question_id,
                            "selected_option_ids": a.selected_option_ids,
                        });
                        if let (Some(text), Some(map)) = (a.text, answer.as_object_mut()) {
                            map.insert("text".into(), Value::String(text));
                        }
                        answer
                    })
                    .collect();
                json!({"answers": answers})
            }
            Self::Decline if interaction_kind == "tool_approval" => {
                json!({"decision": ToolDecision::RejectOnce.as_str()})
            }
            Self::Decline => json!({"declined": true}),
        }
    }
}

// -- Response --

/// Lifecycle state of an interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InteractionStatusView {
    Pending,
    Resolved,
    Cancelled,
    Expired,
}

impl From<InteractionStatus> for InteractionStatusView {
    fn from(status: InteractionStatus) -> Self {
        match status {
            InteractionStatus::Pending => Self::Pending,
            InteractionStatus::Resolved => Self::Resolved,
            InteractionStatus::Cancelled => Self::Cancelled,
            InteractionStatus::Expired => Self::Expired,
        }
    }
}

/// A turn interaction: the request the turn put to its owner and, once it
/// ended, how.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InteractionDetail {
    pub interaction_id: String,
    pub conversation_id: String,
    /// The run (turn) that asked.
    pub run_id: String,
    /// `tool_approval`, `question`, or a kind a newer server added.
    pub kind: String,
    pub status: InteractionStatusView,
    /// Whether the kind renders a card in the timeline.
    pub renders_card: bool,
    /// Provider id of the tool call this interaction belongs to, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// The kind-specific request (for `tool_approval`: `tool_name`,
    /// `arguments`, `arguments_truncated`, `reason`, `options`,
    /// `suggested_rule`; for `question`: `questions`).
    #[schema(value_type = Object)]
    pub request: Value,
    /// The owner's answer. Present once `status` is `resolved`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub response: Option<Value>,
    /// Id of the permission rule an "always" answer saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,
}

impl InteractionDetail {
    /// `renders_card` comes from the kind's handler. A kind nobody registered
    /// is reported as a card, so clients show their "needs a newer app" notice.
    pub fn from_row(row: InteractionRow, kinds: &InteractionKinds) -> Self {
        Self {
            renders_card: kinds.get(&row.kind).is_none_or(|h| h.renders_card()),
            interaction_id: row.id,
            conversation_id: row.conversation_id,
            run_id: row.run_id,
            kind: row.kind,
            status: row.status.into(),
            tool_call_id: row.tool_call_id,
            request: row.request,
            response: row.response,
            rule_id: row.rule_id,
            created_at: row.created_at,
            expires_at: row.expires_at,
            resolved_at: row.resolved_at,
        }
    }
}

// -- SSE event schemas --

/// SSE `interaction_requested` event: a running turn asks its owner something.
/// The kind-specific request fields sit next to the common ones, as in
/// [`InteractionDetail::request`].
#[derive(Serialize, ToSchema)]
pub struct SseInteractionRequestedEvent {
    pub interaction_id: String,
    /// `tool_approval`, `question`, or a kind a newer server added.
    pub kind: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Whether the kind renders a card in the timeline.
    pub renders_card: bool,
    /// Provider id of the tool call this belongs to, when there is one.
    pub tool_call_id: Option<String>,
    /// The kind-specific request fields, flattened into the event object.
    #[serde(flatten)]
    #[schema(value_type = Object)]
    pub request: Value,
}

/// How an interaction ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InteractionOutcomeView {
    Resolved,
    Cancelled,
    Expired,
}

/// SSE `interaction_resolved` event: exactly one per interaction of a live run.
#[derive(Serialize, ToSchema)]
pub struct SseInteractionResolvedEvent {
    pub interaction_id: String,
    pub kind: String,
    pub outcome: InteractionOutcomeView,
    pub resolved_at: DateTime<Utc>,
    /// The owner's answer. Only when `outcome` is `resolved`.
    #[schema(value_type = Option<Object>)]
    pub response: Option<Value>,
    /// Id of the permission rule an "always" answer saved.
    pub rule_id: Option<String>,
}

impl SseInteractionResolvedEvent {
    pub fn new(
        interaction_id: String,
        kind: String,
        resolved_at: DateTime<Utc>,
        outcome: &InteractionOutcome,
    ) -> Self {
        let (view, response, rule_id) = match outcome {
            InteractionOutcome::Resolved { response, rule_id } => (
                InteractionOutcomeView::Resolved,
                Some(response.clone()),
                rule_id.clone(),
            ),
            InteractionOutcome::Cancelled => (InteractionOutcomeView::Cancelled, None, None),
            InteractionOutcome::Expired => (InteractionOutcomeView::Expired, None, None),
        };
        Self {
            interaction_id,
            kind,
            outcome: view,
            resolved_at,
            response,
            rule_id,
        }
    }
}

// -- Handler --

/// The runtime's broker, or one over the space's store when the runtime has
/// none. Without a broker no turn waits in this process, but a turn in another
/// process still reads answers from the shared store, and the end-of-run
/// cleanup still has rows to settle.
pub(crate) fn broker_or_store_only(
    broker: Option<Arc<InteractionBroker>>,
    ctx: &SpaceCtx,
) -> Arc<InteractionBroker> {
    broker.unwrap_or_else(|| {
        Arc::new(InteractionBroker::new(
            Arc::new(ctx.storage.interaction_store()),
            Arc::new(InteractionKinds::with_builtin()),
        ))
    })
}

/// `POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`
///
/// The turn's owner answers a pending interaction. The answer is validated by
/// the interaction kind's handler. Answering is a compare-and-set on `pending`:
/// repeating the same answer returns `200` with the stored resolution, a
/// different answer after resolution returns `409`, and an expired or
/// cancelled interaction (or one whose turn process stopped waiting) returns
/// `410`.
#[utoipa::path(
    post,
    path = "/api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond",
    params(
        ("conversation_id" = Uuid, Path, description = "Conversation UUID"),
        ("turn_id" = Uuid, Path, description = "Turn (run) UUID — matches the SSE run_id"),
        ("interaction_id" = String, Path, description = "Interaction id from `interaction_requested`"),
    ),
    request_body = RespondToInteractionRequest,
    responses(
        (status = 200, description = "The interaction as resolved", body = InteractionDetail),
        (status = 400, description = "The answer does not fit the interaction", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Viewer access, or missing conversations:write scope", body = ErrorBody),
        (status = 404, description = "Conversation, run or interaction not found, or not the caller's", body = ErrorBody),
        (status = 409, description = "Already resolved with a different answer", body = ErrorBody),
        (status = 410, description = "Expired, cancelled, or no turn is waiting", body = ErrorBody),
        (status = 413, description = "Body larger than 64 KiB", body = ErrorBody),
    ),
    operation_id = "respond_to_interaction",
    tag = "conversations",
    security(("bearer_token" = [])),
)]
pub async fn respond_to_interaction(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path((conversation_id, turn_id, interaction_id)): Path<(String, String, String)>,
    body: Result<Json<RespondToInteractionRequest>, JsonRejection>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
    if !caller_can_post(&auth) {
        return json_error(StatusCode::FORBIDDEN, "Missing conversations:write scope");
    }
    let (Ok(conversation_id), Ok(turn_id)) =
        (Uuid::parse_str(&conversation_id), Uuid::parse_str(&turn_id))
    else {
        return json_error(StatusCode::BAD_REQUEST, "Invalid conversation or turn ID");
    };
    if let Err(response) =
        owned_conversation(&caller_store(&ctx, &state).await, conversation_id).await
    {
        return response;
    }
    let request = match body {
        Ok(Json(request)) => request,
        Err(rejection) => return json_error(rejection_status(&rejection), rejection.body_text()),
    };

    let broker = match state.runtime(&ctx).await {
        Ok(runtime) => broker_or_store_only(runtime.interactions.clone(), &ctx),
        Err(response) => return response,
    };

    let (conversation_id, run_id) = (conversation_id.to_string(), turn_id.to_string());
    let who = Responder {
        user_id: &ctx.auth.user_id.0,
        conversation_id: &conversation_id,
        run_id: &run_id,
    };
    let row = match broker.store().get(&interaction_id).await {
        Ok(Some(row)) if who.owns(&row) => row,
        Ok(_) => return json_error(StatusCode::NOT_FOUND, "Interaction not found"),
        Err(e) => return internal(&e),
    };
    if request.kind().is_some_and(|kind| kind != row.kind) {
        return json_error(
            StatusCode::BAD_REQUEST,
            format!("This interaction is a '{}' request", row.kind),
        );
    }

    match broker
        .respond(&who, &interaction_id, request.into_response(&row.kind))
        .await
    {
        Ok(resolved) => Json(InteractionDetail::from_row(resolved, broker.kinds())).into_response(),
        Err(RespondError::NotFound) => json_error(StatusCode::NOT_FOUND, "Interaction not found"),
        Err(RespondError::Conflict(_)) => json_error(
            StatusCode::CONFLICT,
            "The interaction was already answered differently",
        ),
        Err(RespondError::Gone) => json_error(
            StatusCode::GONE,
            "The interaction expired, was cancelled, or its turn is no longer waiting",
        ),
        Err(RespondError::Invalid(reason)) => json_error(StatusCode::BAD_REQUEST, reason),
        Err(RespondError::Internal(e)) => internal(&e),
    }
}

/// A body the JSON extractor refused. Shape errors are `400`, not axum's `422`.
pub(crate) fn rejection_status(rejection: &JsonRejection) -> StatusCode {
    match rejection.status() {
        StatusCode::UNPROCESSABLE_ENTITY => StatusCode::BAD_REQUEST,
        other => other,
    }
}

fn internal(e: &anyhow::Error) -> Response {
    warn!("Interaction store error: {e:#}");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::{Duration, Utc};
    use serde_json::{Value, json};
    use tower::ServiceExt;
    use uuid::Uuid;

    use assistant_core::auth::AuthContext;
    use assistant_core::clock::FakeClock;
    use assistant_core::identity::{Action, ResourceKind, Role, Scope, SpaceId};
    use assistant_storage::{
        ConversationStore as _, InteractionStatus, InteractionStore as _, NewInteraction,
        SqliteConversationStore, SqliteInteractionStore, StorageLayer,
    };

    use super::super::test_helpers::{
        app, app_with_auth, body_json, event_log_state, seed_conversation,
    };
    use crate::api::ApiState;

    struct Fx {
        state: ApiState,
        storage: std::sync::Arc<StorageLayer>,
        conv: Uuid,
        run: Uuid,
    }

    async fn fx() -> Fx {
        let (state, storage) = event_log_state().await;
        let conv = seed_conversation(&storage).await;
        Fx {
            state,
            storage,
            conv,
            run: Uuid::new_v4(),
        }
    }

    fn approval_request() -> Value {
        json!({
            "tool_name": "bash",
            "arguments": {"command": "git status"},
            "arguments_truncated": false,
            "reason": "no rule allows 'bash'",
            "options": ["allow_once", "reject_once"],
        })
    }

    fn question_request() -> Value {
        json!({"questions": [
            {"id": "q1", "prompt": "Which branch?", "multiple": false, "free_text": true,
             "options": [{"id": "o1", "label": "main"}, {"id": "o2", "label": "dev"}]},
        ]})
    }

    impl Fx {
        fn new_interaction(&self, kind: &str, request: Value) -> NewInteraction {
            NewInteraction {
                id: Uuid::new_v4().to_string(),
                conversation_id: self.conv.to_string(),
                run_id: self.run.to_string(),
                user_id: "system".into(),
                kind: kind.into(),
                tool_call_id: Some("call-1".into()),
                request,
                expires_at: Utc::now() + Duration::minutes(10),
            }
        }

        async fn seed(&self, kind: &str, request: Value) -> String {
            self.seed_row(self.new_interaction(kind, request)).await
        }

        async fn seed_row(&self, new: NewInteraction) -> String {
            let id = new.id.clone();
            self.storage.interaction_store().create(new).await.unwrap();
            id
        }

        async fn status(&self, id: &str) -> InteractionStatus {
            self.storage
                .interaction_store()
                .get(id)
                .await
                .unwrap()
                .unwrap()
                .status
        }

        async fn post(&self, id: &str, body: Value) -> (StatusCode, Value) {
            self.post_as(app(self.state.clone()), self.conv, self.run, id, body)
                .await
        }

        async fn post_as(
            &self,
            router: axum::Router,
            conv: Uuid,
            run: Uuid,
            id: &str,
            body: Value,
        ) -> (StatusCode, Value) {
            self.post_raw(router, conv, run, id, body.to_string()).await
        }

        async fn post_raw(
            &self,
            router: axum::Router,
            conv: Uuid,
            run: Uuid,
            id: &str,
            body: String,
        ) -> (StatusCode, Value) {
            let resp = router
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(format!(
                            "/conversations/{conv}/turns/{run}/interactions/{id}/respond"
                        ))
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = resp.status();
            (status, body_json(resp.into_body()).await)
        }
    }

    fn assert_error_body(body: &Value) {
        assert!(
            body["error"].as_str().is_some_and(|e| !e.is_empty()),
            "errors carry an {{\"error\": …}} envelope, got {body}"
        );
    }

    fn auth(user: &str, role: Role, scopes: Vec<Scope>) -> AuthContext {
        AuthContext {
            user_id: user.into(),
            org_id: "default".into(),
            email: format!("{user}@example.com"),
            space_roles: HashMap::from([(SpaceId::from("default"), role)]),
            scopes,
            client_id: "test".into(),
        }
    }

    fn write_scope() -> Vec<Scope> {
        vec![Scope::new(ResourceKind::Conversations, Action::Write)]
    }

    // -- 200 --

    #[tokio::test]
    async fn owner_answers_a_tool_approval() {
        let f = fx().await;
        let id = f.seed("tool_approval", approval_request()).await;

        let (status, body) = f
            .post(
                &id,
                json!({"kind": "tool_approval", "decision": "allow_once"}),
            )
            .await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["interaction_id"], id);
        assert_eq!(body["kind"], "tool_approval");
        assert_eq!(body["status"], "resolved");
        assert_eq!(body["response"], json!({"decision": "allow_once"}));
        assert_eq!(body["renders_card"], true);
        assert_eq!(body["run_id"], f.run.to_string());
        assert_eq!(body["request"]["tool_name"], "bash");
        assert!(body["resolved_at"].is_string(), "{body}");
        assert_eq!(f.status(&id).await, InteractionStatus::Resolved);
    }

    #[tokio::test]
    async fn owner_answers_a_question() {
        let f = fx().await;
        let id = f.seed("question", question_request()).await;
        let answers = json!([{
            "question_id": "q1", "selected_option_ids": ["o2"], "text": "the dev one",
        }]);

        let (status, body) = f
            .post(&id, json!({"kind": "question", "answers": answers}))
            .await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["response"], json!({"answers": answers}));
    }

    #[tokio::test]
    async fn decline_rejects_an_approval_and_declines_a_question() {
        let f = fx().await;
        let approval = f.seed("tool_approval", approval_request()).await;
        let question = f.seed("question", question_request()).await;

        let (status, body) = f.post(&approval, json!({"kind": "decline"})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["response"], json!({"decision": "reject_once"}));

        let (status, body) = f.post(&question, json!({"kind": "decline"})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["response"], json!({"declined": true}));
    }

    // -- 400 --

    #[tokio::test]
    async fn answers_that_do_not_fit_are_refused() {
        let f = fx().await;
        let approval = f.seed("tool_approval", approval_request()).await;
        let question = f.seed("question", question_request()).await;
        let unknown_kind = f.seed("future_thing", json!({})).await;
        let answer = |extra: Value| {
            let mut a = json!({"question_id": "q1", "selected_option_ids": ["o1"]});
            a.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            json!({"kind": "question", "answers": [a]})
        };
        let cases = [
            (
                "question answer for an approval",
                &approval,
                json!({"kind": "question", "answers": []}),
            ),
            (
                "approval decision for a question",
                &question,
                json!({"kind": "tool_approval", "decision": "allow_once"}),
            ),
            (
                "decision that was not offered",
                &approval,
                json!({"kind": "tool_approval", "decision": "allow_always"}),
            ),
            (
                "unknown option",
                &question,
                json!({"kind": "question", "answers": [
                    {"question_id": "q1", "selected_option_ids": ["nope"]}]}),
            ),
            (
                "two selections for a single choice",
                &question,
                json!({"kind": "question", "answers": [
                    {"question_id": "q1", "selected_option_ids": ["o1", "o2"]}]}),
            ),
            (
                "4001-character text",
                &question,
                answer(json!({"text": "x".repeat(4001)})),
            ),
            (
                "tampered body carrying a rule",
                &approval,
                json!({"kind": "tool_approval", "decision": "allow_once", "rule": "bash(*)"}),
            ),
            ("unknown body kind", &approval, json!({"kind": "surprise"})),
            (
                "interaction kind nobody registered",
                &unknown_kind,
                json!({"kind": "decline"}),
            ),
        ];
        for (name, id, body) in cases {
            let (status, resp) = f.post(id, body).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{name}: {resp}");
            assert_error_body(&resp);
        }
        for id in [&approval, &question, &unknown_kind] {
            assert_eq!(
                f.status(id).await,
                InteractionStatus::Pending,
                "a refused answer leaves the interaction pending"
            );
        }
    }

    // -- 403 --

    #[tokio::test]
    async fn viewers_and_callers_without_the_scope_are_refused() {
        let f = fx().await;
        let id = f.seed("tool_approval", approval_request()).await;
        let body = json!({"kind": "tool_approval", "decision": "allow_once"});
        let refused = [
            ("viewer", auth("system", Role::Viewer, write_scope())),
            (
                "missing scope",
                auth(
                    "system",
                    Role::Member,
                    vec![Scope::new(ResourceKind::Personas, Action::Read)],
                ),
            ),
        ];
        for (name, who) in refused {
            let (status, resp) = f
                .post_as(
                    app_with_auth(f.state.clone(), who),
                    f.conv,
                    f.run,
                    &id,
                    body.clone(),
                )
                .await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{name}: {resp}");
            assert_error_body(&resp);
        }
        assert_eq!(f.status(&id).await, InteractionStatus::Pending);
    }

    // -- 404 --

    #[tokio::test]
    async fn unknown_or_foreign_things_are_not_found() {
        let f = fx().await;
        let id = f.seed("tool_approval", approval_request()).await;
        let body = json!({"kind": "tool_approval", "decision": "allow_once"});

        // Another member of the space, who owns a conversation of their own.
        let other = auth("other", Role::Member, write_scope());
        let other_conv = SqliteConversationStore::for_agent(f.storage.pool.clone(), "default")
            .with_user_id("other")
            .create_conversation(None)
            .await
            .unwrap()
            .id;

        let cases = [
            (
                "unknown conversation",
                app(f.state.clone()),
                Uuid::new_v4(),
                f.run,
                id.clone(),
            ),
            (
                "another user's conversation",
                app_with_auth(f.state.clone(), other.clone()),
                f.conv,
                f.run,
                id.clone(),
            ),
            (
                "wrong run in the path",
                app(f.state.clone()),
                f.conv,
                Uuid::new_v4(),
                id.clone(),
            ),
            (
                "unknown interaction",
                app(f.state.clone()),
                f.conv,
                f.run,
                Uuid::new_v4().to_string(),
            ),
            (
                "own conversation, someone else's interaction",
                app_with_auth(f.state.clone(), other),
                other_conv,
                f.run,
                id.clone(),
            ),
        ];
        for (name, router, conv, run, id) in cases {
            let (status, resp) = f.post_as(router, conv, run, &id, body.clone()).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{name}: {resp}");
            assert_error_body(&resp);
        }
        assert_eq!(f.status(&id).await, InteractionStatus::Pending);
    }

    // -- idempotency (200 / 409) --

    #[tokio::test]
    async fn the_same_answer_twice_returns_the_same_resolution() {
        let f = fx().await;
        let id = f.seed("tool_approval", approval_request()).await;
        let body = json!({"kind": "tool_approval", "decision": "allow_once"});

        let (s1, first) = f.post(&id, body.clone()).await;
        let (s2, second) = f.post(&id, body).await;

        assert_eq!((s1, s2), (StatusCode::OK, StatusCode::OK));
        assert_eq!(first, second, "a double tap gets identical bodies");
    }

    #[tokio::test]
    async fn a_different_answer_after_resolution_conflicts() {
        let f = fx().await;
        let id = f.seed("tool_approval", approval_request()).await;
        f.post(
            &id,
            json!({"kind": "tool_approval", "decision": "allow_once"}),
        )
        .await;

        let (status, resp) = f
            .post(
                &id,
                json!({"kind": "tool_approval", "decision": "reject_once"}),
            )
            .await;

        assert_eq!(status, StatusCode::CONFLICT, "{resp}");
        assert_error_body(&resp);
        let stored = f
            .storage
            .interaction_store()
            .get(&id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            stored.response,
            Some(json!({"decision": "allow_once"})),
            "first writer wins"
        );
    }

    // -- 410 --

    #[tokio::test]
    async fn expired_cancelled_and_waiterless_interactions_are_gone() {
        let f = fx().await;
        let body = json!({"kind": "tool_approval", "decision": "allow_once"});

        let cancelled = f.seed("tool_approval", approval_request()).await;
        f.storage
            .interaction_store()
            .cancel_pending_for_run(&f.run.to_string())
            .await
            .unwrap();

        let mut late = f.new_interaction("tool_approval", approval_request());
        late.expires_at = Utc::now() - Duration::seconds(1);
        let expired = f.seed_row(late).await;

        // The turn process stopped refreshing `waiter_seen_at` two minutes ago.
        let two_minutes_ago = FakeClock::new(Utc::now() - Duration::minutes(2));
        let silent = SqliteInteractionStore::new(f.storage.pool.clone())
            .with_clock(std::sync::Arc::new(two_minutes_ago));
        let orphan = f.new_interaction("tool_approval", approval_request());
        let orphan_id = orphan.id.clone();
        silent.create(orphan).await.unwrap();

        for (name, id) in [
            ("expired", &expired),
            ("cancelled", &cancelled),
            ("no live waiter", &orphan_id),
        ] {
            let (status, resp) = f.post(id, body.clone()).await;
            assert_eq!(status, StatusCode::GONE, "{name}: {resp}");
            assert_error_body(&resp);
        }
        assert_eq!(f.status(&orphan_id).await, InteractionStatus::Expired);
        assert_eq!(f.status(&expired).await, InteractionStatus::Expired);
    }

    // -- 413 --

    #[tokio::test]
    async fn bodies_over_64_kib_are_refused() {
        let f = fx().await;
        let id = f.seed("question", question_request()).await;
        let body = json!({"kind": "question", "answers": [
            {"question_id": "q1", "selected_option_ids": [], "text": "x".repeat(65 * 1024)}]});

        let (status, resp) = f.post(&id, body).await;

        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE, "{resp}");
        assert_error_body(&resp);
        assert_eq!(f.status(&id).await, InteractionStatus::Pending);
    }
}
