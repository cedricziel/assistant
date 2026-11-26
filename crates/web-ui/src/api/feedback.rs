//! Message feedback endpoints: the caller's thumbs up/down on an assistant reply.
//!
//! Endpoints:
//! - `PUT    /api/conversations/{id}/messages/{message_id}/feedback` — set or replace
//! - `DELETE /api/conversations/{id}/messages/{message_id}/feedback` — clear

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::types::conversation::{Message, MessageRole};
use assistant_runtime::{FeedbackLabel, FeedbackTelemetry};
use assistant_storage::{
    ConversationStore as _, MessageFeedbackRating, MessageFeedbackRow, MessageFeedbackStore as _,
};

use crate::errors::json_error;
use crate::openapi::ErrorBody;
use crate::space::{Access, SpaceCtx};

use super::ApiState;
use super::conversations::{caller_store, owned_conversation};
use super::messages::caller_can_post;

/// Longest accepted comment, in Unicode scalar values.
const MAX_COMMENT_SCALARS: usize = 2000;

/// The caller's verdict on an assistant message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackRating {
    Positive,
    Negative,
}

impl From<FeedbackRating> for MessageFeedbackRating {
    fn from(rating: FeedbackRating) -> Self {
        match rating {
            FeedbackRating::Positive => Self::Positive,
            FeedbackRating::Negative => Self::Negative,
        }
    }
}

impl From<MessageFeedbackRating> for FeedbackRating {
    fn from(rating: MessageFeedbackRating) -> Self {
        match rating {
            MessageFeedbackRating::Positive => Self::Positive,
            MessageFeedbackRating::Negative => Self::Negative,
        }
    }
}

/// Body for `PUT /api/conversations/{id}/messages/{message_id}/feedback`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetMessageFeedbackRequest {
    /// Thumbs up or down.
    pub rating: FeedbackRating,
    /// Optional free-text reason. Trimmed; an empty result means no comment.
    /// At most 2000 Unicode scalar values. Omitting it clears a stored comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// The caller's stored feedback on a message.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MessageFeedbackResponse {
    pub message_id: Uuid,
    pub rating: FeedbackRating,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<MessageFeedbackRow> for MessageFeedbackResponse {
    fn from(row: MessageFeedbackRow) -> Self {
        Self {
            message_id: row.message_id,
            rating: row.rating.into(),
            comment: row.comment,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Everything both handlers check before touching the feedback store.
async fn authorize_target(
    ctx: &SpaceCtx,
    state: &ApiState,
    auth: &AuthContext,
    id: &str,
    message_id: &str,
) -> Result<(Uuid, Message), Response> {
    ctx.require(Access::Write)
        .map_err(IntoResponse::into_response)?;
    if !caller_can_post(auth) {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "Missing conversations:write scope",
        ));
    }
    let conv_id = Uuid::parse_str(id)
        .map_err(|_| json_error(StatusCode::BAD_REQUEST, "Invalid conversation ID"))?;
    let message_id = Uuid::parse_str(message_id)
        .map_err(|_| json_error(StatusCode::BAD_REQUEST, "Invalid message ID"))?;

    let store = caller_store(ctx, state).await;
    owned_conversation(&store, conv_id).await?;
    let message = match store.get_message(message_id).await {
        Ok(Some(m)) if m.conversation_id == conv_id => m,
        Ok(_) => return Err(json_error(StatusCode::NOT_FOUND, "Message not found")),
        Err(e) => {
            warn!("Failed to load message {message_id}: {e}");
            return Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            ));
        }
    };
    Ok((conv_id, message))
}

/// Report a stored or cleared vote to the telemetry sink.
///
/// Carries only whether a comment exists, never its text. A sink failure is
/// logged and otherwise ignored: telemetry must not fail the request.
async fn emit_feedback(
    state: &ApiState,
    label: FeedbackLabel,
    conv_id: Uuid,
    message: &Message,
    has_comment: bool,
) {
    let event = FeedbackTelemetry {
        label,
        conversation_id: conv_id,
        agent_id: state.agent_id.read().await.clone(),
        message_id: message.id,
        has_comment,
        traceparent: message.traceparent.clone(),
    };
    if let Err(e) = state.feedback_telemetry.record(&event) {
        warn!(
            "Failed to record feedback telemetry for message {}: {e}",
            message.id
        );
    }
}

/// Whether a message is a final answer the user can rate.
fn is_rateable(message: &Message) -> bool {
    let content = message.content.trim();
    message.role == MessageRole::Assistant
        && !content.is_empty()
        && message.tool_calls_json.is_none()
        && !content.starts_with("<think>")
}

/// `PUT /api/conversations/{id}/messages/{message_id}/feedback` — rate a reply.
///
/// Replaces the caller's previous feedback on the message in full.
#[utoipa::path(
    put,
    path = "/api/conversations/{id}/messages/{message_id}/feedback",
    tag = "conversations",
    operation_id = "set_message_feedback",
    params(
        ("id" = Uuid, Path, description = "Conversation ID"),
        ("message_id" = Uuid, Path, description = "Assistant message ID"),
    ),
    request_body = SetMessageFeedbackRequest,
    responses(
        (status = 200, description = "Feedback stored", body = MessageFeedbackResponse),
        (status = 400, description = "Invalid ID, body or comment, or the message cannot be rated", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Viewer role or missing conversations:write scope", body = ErrorBody),
        (status = 404, description = "Conversation or message not found", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn set_message_feedback(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Path((id, message_id)): Path<(String, String)>,
    body: Result<Json<SetMessageFeedbackRequest>, JsonRejection>,
) -> Response {
    let (conv_id, message) = match authorize_target(&ctx, &state, &auth, &id, &message_id).await {
        Ok(target) => target,
        Err(response) => return response,
    };
    let Json(body) = match body {
        Ok(body) => body,
        Err(rejection) => return json_error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    if !is_rateable(&message) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Feedback is only accepted on assistant replies",
        );
    }
    let comment = body
        .comment
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty());
    if comment.is_some_and(|c| c.chars().count() > MAX_COMMENT_SCALARS) {
        return json_error(
            StatusCode::BAD_REQUEST,
            format!("comment must be at most {MAX_COMMENT_SCALARS} characters"),
        );
    }

    let result = ctx
        .storage
        .message_feedback_store()
        .set(
            conv_id,
            message.id,
            &auth.user_id.0,
            body.rating.into(),
            comment,
        )
        .await;
    match result {
        Ok(row) => {
            let label = match row.rating {
                MessageFeedbackRating::Positive => FeedbackLabel::Positive,
                MessageFeedbackRating::Negative => FeedbackLabel::Negative,
            };
            emit_feedback(&state, label, conv_id, &message, row.comment.is_some()).await;
            Json(MessageFeedbackResponse::from(row)).into_response()
        }
        Err(e) => {
            warn!("Failed to store feedback on message {}: {e}", message.id);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        }
    }
}

/// `DELETE /api/conversations/{id}/messages/{message_id}/feedback` — clear feedback.
///
/// Returns 204 whether or not feedback existed.
#[utoipa::path(
    delete,
    path = "/api/conversations/{id}/messages/{message_id}/feedback",
    tag = "conversations",
    operation_id = "delete_message_feedback",
    params(
        ("id" = Uuid, Path, description = "Conversation ID"),
        ("message_id" = Uuid, Path, description = "Assistant message ID"),
    ),
    responses(
        (status = 204, description = "Feedback cleared (or none existed)"),
        (status = 400, description = "Invalid ID", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Viewer role or missing conversations:write scope", body = ErrorBody),
        (status = 404, description = "Conversation or message not found", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn delete_message_feedback(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    axum::Extension(auth): axum::Extension<AuthContext>,
    Path((id, message_id)): Path<(String, String)>,
) -> Response {
    let (conv_id, message) = match authorize_target(&ctx, &state, &auth, &id, &message_id).await {
        Ok(target) => target,
        Err(response) => return response,
    };
    let store = ctx.storage.message_feedback_store();
    // Read before clearing so the event can say whether the withdrawn vote had
    // a comment. `clear` decides whether anything is emitted.
    let had_comment = match store.get(message.id, &auth.user_id.0).await {
        Ok(row) => row.is_some_and(|r| r.comment.is_some()),
        Err(e) => {
            warn!("Failed to read feedback on message {}: {e}", message.id);
            false
        }
    };
    match store.clear(message.id, &auth.user_id.0).await {
        Ok(removed) => {
            if removed {
                emit_feedback(
                    &state,
                    FeedbackLabel::Cleared,
                    conv_id,
                    &message,
                    had_comment,
                )
                .await;
            }
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => {
            warn!("Failed to clear feedback on message {}: {e}", message.id);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;
    use uuid::Uuid;

    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{Role, SpaceId};
    use assistant_core::types::conversation::Message;
    use assistant_runtime::{FeedbackLabel, FeedbackTelemetry, FeedbackTelemetrySink};
    use assistant_storage::{
        ConversationStore as _, MessageFeedbackStore as _, SqliteConversationStore, StorageLayer,
    };

    use super::super::test_helpers::*;

    /// Records every event; fails each call when `fail` is set.
    #[derive(Default)]
    struct RecordingSink {
        events: std::sync::Mutex<Vec<FeedbackTelemetry>>,
        fail: bool,
    }

    impl FeedbackTelemetrySink for RecordingSink {
        fn record(&self, feedback: &FeedbackTelemetry) -> anyhow::Result<()> {
            self.events.lock().unwrap().push(feedback.clone());
            if self.fail {
                anyhow::bail!("exporter down");
            }
            Ok(())
        }
    }

    const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    struct Fixture {
        sink: std::sync::Arc<RecordingSink>,
        storage: std::sync::Arc<StorageLayer>,
        state: super::super::ApiState,
        conv: Uuid,
        answer: Uuid,
    }

    fn store(storage: &StorageLayer, user: &str) -> SqliteConversationStore {
        SqliteConversationStore::for_agent(storage.pool.clone(), "default").with_user_id(user)
    }

    async fn fixture() -> Fixture {
        fixture_with_sink(RecordingSink::default()).await
    }

    async fn fixture_with_sink(sink: RecordingSink) -> Fixture {
        let sink = std::sync::Arc::new(sink);
        let (mut state, storage) = test_state("http://127.0.0.1:1").await;
        state.feedback_telemetry = sink.clone();
        let conv = seed_conversation(&storage).await;
        let mut reply = Message::assistant(conv, "Paris.");
        reply.traceparent = Some(TRACEPARENT.into());
        let answer = add(&storage, reply).await;
        Fixture {
            sink,
            storage,
            state,
            conv,
            answer,
        }
    }

    async fn add(storage: &StorageLayer, msg: Message) -> Uuid {
        let id = msg.id;
        store(storage, "system").save_message(&msg).await.unwrap();
        id
    }

    fn feedback_uri(conv: impl std::fmt::Display, msg: impl std::fmt::Display) -> String {
        format!("/conversations/{conv}/messages/{msg}/feedback")
    }

    async fn call(
        app: axum::Router,
        method: &str,
        uri: &str,
        content_type: Option<&str>,
        body: &str,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(ct) = content_type {
            req = req.header("content-type", ct);
        }
        let resp = app
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = body_bytes(resp.into_body()).await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn put(f: &Fixture, uri: &str, body: Value) -> (StatusCode, Value) {
        call(
            app(f.state.clone()),
            "PUT",
            uri,
            Some("application/json"),
            &body.to_string(),
        )
        .await
    }

    fn assert_error(status: StatusCode, body: &Value, want: StatusCode) {
        assert_eq!(status, want, "unexpected status, body: {body}");
        assert!(
            body.get("error").and_then(Value::as_str).is_some(),
            "expected the error envelope, got {body}"
        );
    }

    fn caller(role: Role, scopes: Vec<assistant_core::identity::Scope>) -> AuthContext {
        AuthContext {
            user_id: "system".into(),
            org_id: "default".into(),
            email: "system@example.test".into(),
            space_roles: HashMap::from([(SpaceId::from("default"), role)]),
            scopes,
            client_id: "test".into(),
        }
    }

    async fn stored(f: &Fixture) -> Option<assistant_storage::MessageFeedbackRow> {
        f.storage
            .message_feedback_store()
            .get(f.answer, "system")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn put_positive_returns_200_without_comment() {
        let f = fixture().await;
        let (status, body) = put(
            &f,
            &feedback_uri(f.conv, f.answer),
            json!({"rating": "positive"}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["rating"], "positive");
        assert_eq!(body["message_id"], f.answer.to_string());
        assert!(body.get("comment").is_none(), "comment must be omitted");
        assert!(body["created_at"].is_string() && body["updated_at"].is_string());
    }

    #[tokio::test]
    async fn put_replaces_vote_and_stores_comment() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);
        put(&f, &uri, json!({"rating": "positive"})).await;
        let (status, body) = put(
            &f,
            &uri,
            json!({"rating": "negative", "comment": "  wrong date "}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["rating"], "negative");
        assert_eq!(body["comment"], "wrong date");
        assert_eq!(
            stored(&f).await.unwrap().comment.as_deref(),
            Some("wrong date")
        );

        // Full replace: an absent comment clears the stored one.
        let (status, body) = put(&f, &uri, json!({"rating": "negative"})).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.get("comment").is_none());
        assert_eq!(stored(&f).await.unwrap().comment, None);
    }

    #[tokio::test]
    async fn whitespace_comment_is_stored_as_none() {
        let f = fixture().await;
        let (status, body) = put(
            &f,
            &feedback_uri(f.conv, f.answer),
            json!({"rating": "positive", "comment": " \n\t "}),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.get("comment").is_none());
        assert_eq!(stored(&f).await.unwrap().comment, None);
    }

    #[tokio::test]
    async fn comment_limit_counts_unicode_scalars() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);

        let too_long = "a".repeat(2001);
        let (status, body) =
            put(&f, &uri, json!({"rating": "negative", "comment": too_long})).await;
        assert_error(status, &body, StatusCode::BAD_REQUEST);
        assert!(stored(&f).await.is_none(), "nothing may be stored");

        // A family emoji is five scalars (three people joined by two ZWJs).
        let exact: String = "👨‍👩‍👧".repeat(400);
        assert_eq!(exact.chars().count(), 2000);
        let (status, body) = put(&f, &uri, json!({"rating": "negative", "comment": exact})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    #[tokio::test]
    async fn bad_bodies_return_400_with_envelope() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);

        let (s, b) = put(&f, &uri, json!({"rating": "meh"})).await;
        assert_error(s, &b, StatusCode::BAD_REQUEST);

        let (s, b) = call(
            app(f.state.clone()),
            "PUT",
            &uri,
            Some("application/json"),
            "{not json",
        )
        .await;
        assert_error(s, &b, StatusCode::BAD_REQUEST);

        let (s, b) = call(
            app(f.state.clone()),
            "PUT",
            &uri,
            None,
            r#"{"rating":"positive"}"#,
        )
        .await;
        assert_error(s, &b, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_ids_return_400() {
        let f = fixture().await;
        let (s, b) = put(
            &f,
            &feedback_uri("nope", f.answer),
            json!({"rating": "positive"}),
        )
        .await;
        assert_error(s, &b, StatusCode::BAD_REQUEST);
        let (s, b) = put(
            &f,
            &feedback_uri(f.conv, "nope"),
            json!({"rating": "positive"}),
        )
        .await;
        assert_error(s, &b, StatusCode::BAD_REQUEST);
        let (s, b) = call(
            app(f.state.clone()),
            "DELETE",
            &feedback_uri(f.conv, "nope"),
            None,
            "",
        )
        .await;
        assert_error(s, &b, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn ineligible_messages_return_400() {
        let f = fixture().await;
        let user = add(&f.storage, Message::user(f.conv, "hi")).await;
        let think = add(&f.storage, Message::assistant(f.conv, "<think>hmm</think>")).await;
        let mut calls = Message::assistant(f.conv, "");
        calls.tool_calls_json = Some("[]".into());
        let calls = add(&f.storage, calls).await;

        for id in [user, think, calls] {
            let (s, b) = put(&f, &feedback_uri(f.conv, id), json!({"rating": "positive"})).await;
            assert_error(s, &b, StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn foreign_targets_return_404() {
        let f = fixture().await;
        let body = json!({"rating": "positive"});

        // A conversation owned by somebody else.
        let theirs = store(&f.storage, "someone-else")
            .create_conversation(None)
            .await
            .unwrap()
            .id;
        let (s, b) = put(&f, &feedback_uri(theirs, f.answer), body.clone()).await;
        assert_error(s, &b, StatusCode::NOT_FOUND);

        // A message that lives in another conversation of the caller.
        let other = seed_conversation(&f.storage).await;
        let (s, b) = put(&f, &feedback_uri(other, f.answer), body.clone()).await;
        assert_error(s, &b, StatusCode::NOT_FOUND);

        // An unknown message id.
        let (s, b) = put(&f, &feedback_uri(f.conv, Uuid::new_v4()), body).await;
        assert_error(s, &b, StatusCode::NOT_FOUND);
        assert!(stored(&f).await.is_none());
    }

    #[tokio::test]
    async fn viewer_gets_403_on_put_and_delete() {
        let f = fixture().await;
        let viewer = || app_with_auth(f.state.clone(), caller(Role::Viewer, vec![]));
        let uri = feedback_uri(f.conv, f.answer);
        let (s, b) = call(
            viewer(),
            "PUT",
            &uri,
            Some("application/json"),
            r#"{"rating":"positive"}"#,
        )
        .await;
        assert_error(s, &b, StatusCode::FORBIDDEN);
        let (s, b) = call(viewer(), "DELETE", &uri, None, "").await;
        assert_error(s, &b, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn missing_scope_is_forbidden_but_org_admin_bypasses() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);
        let body = r#"{"rating":"positive"}"#;

        let member = app_with_auth(f.state.clone(), caller(Role::Member, vec![]));
        let (s, b) = call(member, "PUT", &uri, Some("application/json"), body).await;
        assert_error(s, &b, StatusCode::FORBIDDEN);

        let member = app_with_auth(f.state.clone(), caller(Role::Member, vec![]));
        let (s, b) = call(member, "DELETE", &uri, None, "").await;
        assert_error(s, &b, StatusCode::FORBIDDEN);

        let admin = app_with_auth(f.state.clone(), caller(Role::OrgAdmin, vec![]));
        let (s, b) = call(admin, "PUT", &uri, Some("application/json"), body).await;
        assert_eq!(s, StatusCode::OK, "{b}");
    }

    #[tokio::test]
    async fn delete_returns_204_twice_and_clears() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);
        put(&f, &uri, json!({"rating": "positive"})).await;
        assert!(stored(&f).await.is_some());

        for _ in 0..2 {
            let (s, _) = call(app(f.state.clone()), "DELETE", &uri, None, "").await;
            assert_eq!(s, StatusCode::NO_CONTENT);
        }
        assert!(stored(&f).await.is_none());
    }

    fn events(f: &Fixture) -> Vec<FeedbackTelemetry> {
        f.sink.events.lock().unwrap().clone()
    }

    #[tokio::test]
    async fn put_emits_the_rating_with_the_stored_traceparent() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);
        put(&f, &uri, json!({"rating": "positive"})).await;
        put(&f, &uri, json!({"rating": "negative"})).await;

        let got = events(&f);
        let labels: Vec<_> = got.iter().map(|e| e.label).collect();
        assert_eq!(labels, [FeedbackLabel::Positive, FeedbackLabel::Negative]);
        for e in &got {
            assert_eq!(e.traceparent.as_deref(), Some(TRACEPARENT));
            assert_eq!(e.conversation_id, f.conv);
            assert_eq!(e.message_id, f.answer);
            assert_eq!(e.agent_id, "default");
            assert!(!e.has_comment);
        }
    }

    #[tokio::test]
    async fn emitted_events_carry_has_comment_but_never_the_text() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);
        for rating in ["positive", "negative"] {
            put(
                &f,
                &uri,
                json!({"rating": rating, "comment": "secret-token-123"}),
            )
            .await;
        }
        put(&f, &uri, json!({"rating": "negative", "comment": "  "})).await;

        let got = events(&f);
        let has_comment: Vec<_> = got.iter().map(|e| e.has_comment).collect();
        assert_eq!(has_comment, [true, true, false]);
        assert!(
            !format!("{got:?}").contains("secret-token-123"),
            "comment text must not reach telemetry"
        );
    }

    #[tokio::test]
    async fn delete_of_a_vote_emits_cleared() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);
        put(&f, &uri, json!({"rating": "negative", "comment": "wrong"})).await;
        let (s, _) = call(app(f.state.clone()), "DELETE", &uri, None, "").await;
        assert_eq!(s, StatusCode::NO_CONTENT);

        let got = events(&f);
        assert_eq!(got.len(), 2);
        assert_eq!(got[1].label, FeedbackLabel::Cleared);
        assert_eq!(got[1].traceparent.as_deref(), Some(TRACEPARENT));
        assert!(got[1].has_comment, "the withdrawn vote had a comment");
    }

    #[tokio::test]
    async fn delete_with_nothing_stored_emits_nothing() {
        let f = fixture().await;
        let (s, _) = call(
            app(f.state.clone()),
            "DELETE",
            &feedback_uri(f.conv, f.answer),
            None,
            "",
        )
        .await;
        assert_eq!(s, StatusCode::NO_CONTENT);
        assert!(events(&f).is_empty());
    }

    #[tokio::test]
    async fn rejected_requests_emit_nothing() {
        let f = fixture().await;
        let uri = feedback_uri(f.conv, f.answer);
        put(&f, &uri, json!({"rating": "meh"})).await;
        put(
            &f,
            &uri,
            json!({"rating": "negative", "comment": "a".repeat(2001)}),
        )
        .await;
        assert!(events(&f).is_empty());
    }

    #[tokio::test]
    async fn sink_failure_does_not_fail_the_request() {
        let f = fixture_with_sink(RecordingSink {
            fail: true,
            ..Default::default()
        })
        .await;
        let uri = feedback_uri(f.conv, f.answer);
        let (s, b) = put(&f, &uri, json!({"rating": "positive"})).await;
        assert_eq!(s, StatusCode::OK, "{b}");
        let (s, _) = call(app(f.state.clone()), "DELETE", &uri, None, "").await;
        assert_eq!(s, StatusCode::NO_CONTENT);
        assert_eq!(events(&f).len(), 2, "both events were attempted");
    }
}
