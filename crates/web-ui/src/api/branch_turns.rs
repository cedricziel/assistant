//! Edit and regenerate: `POST /api/conversations/{id}/messages/{message_id}/edit`
//! and `.../regenerate` (ADR-0018, D3, D6, D8).
//!
//! Both start an anchored turn that grows a new branch of the message tree and
//! stream it with the same frames as `send_message`. They take the
//! conversation's exclusive lease for the whole turn, so a concurrent edit,
//! regenerate or send gets `409`. A turn that only waits for an approval or an
//! answer is cancelled first (D7a).

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::Deserialize;
use tracing::warn;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::bus_messages::TurnAnchor;
use assistant_core::types::conversation::{Message, MessageRole};
use assistant_storage::ConversationStore;

use crate::errors::json_error;
use crate::openapi::ErrorBody;
use crate::space::{Access, SpaceCtx};

use super::ApiState;
use super::conversations::{caller_store, owned_conversation};
use super::messages::{RunOptions, caller_can_post, start_turn, stream_turn_to_sse};

/// Body for `POST /api/conversations/{id}/messages/{message_id}/edit`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EditMessageRequest {
    /// The new text. The edited message's attachments carry forward.
    pub message: String,
}

/// A validated branch request: the conversation and the message it addresses.
struct Target {
    conv_id: Uuid,
    message: Message,
    store: assistant_storage::SqliteConversationStore,
}

/// The checks edit and regenerate share, in the order that keeps a bad
/// request from touching anything: access, scope, ids, ownership, message.
async fn resolve_target(
    ctx: &SpaceCtx,
    state: &ApiState,
    auth: &AuthContext,
    conversation: &str,
    message: &str,
) -> Result<Target, Response> {
    if let Err(denied) = ctx.require(Access::Write) {
        return Err(denied.into_response());
    }
    if !caller_can_post(auth) {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "Missing conversations:write scope",
        ));
    }
    let Ok(conv_id) = Uuid::parse_str(conversation) else {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Invalid conversation ID",
        ));
    };
    let Ok(message_id) = Uuid::parse_str(message) else {
        return Err(json_error(StatusCode::BAD_REQUEST, "Invalid message ID"));
    };
    let store = caller_store(ctx, state).await;
    owned_conversation(&store, conv_id).await?;
    match store.get_message(message_id).await {
        Ok(Some(message)) if message.conversation_id == conv_id => Ok(Target {
            conv_id,
            message,
            store,
        }),
        Ok(_) => Err(json_error(StatusCode::NOT_FOUND, "Message not found")),
        Err(e) => {
            warn!("Failed to load message {message_id}: {e:#}");
            Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            ))
        }
    }
}

/// Acquire the conversation for a branch turn and stream it.
///
/// `prompt` is the text the turn answers; `user_message_id` is announced in
/// `run_started` (the id the run saves its user row under, or the answered
/// message for a regenerate).
async fn run_branch_turn(
    ctx: &SpaceCtx,
    state: &ApiState,
    auth: &AuthContext,
    conv_id: Uuid,
    anchor: TurnAnchor,
    user_message_id: Uuid,
    prompt: String,
) -> Response {
    let orchestrator = match state.runtime(ctx).await {
        Ok(runtime) => runtime.orchestrator.clone(),
        Err(response) => return response,
    };
    // The run id doubles as the request id, so `/stop` finds the turn.
    let run_id = Uuid::new_v4();
    let lease = match orchestrator
        .acquire_exclusive_for_branch(conv_id, run_id)
        .await
    {
        Ok(lease) => lease,
        Err(conflict) => return json_error(StatusCode::CONFLICT, conflict.to_string()),
    };
    let opts = RunOptions::web_turn(anchor).with_user_message_id(user_message_id);
    let turn = opts.submit_turn(prompt.clone(), conv_id, auth, run_id, lease);
    let (event_rx, turn_result_rx) = start_turn(orchestrator, turn);
    stream_turn_to_sse(
        ctx,
        state,
        run_id,
        conv_id,
        opts,
        prompt,
        event_rx,
        turn_result_rx,
    )
    .await
}

/// `POST /api/conversations/{id}/messages/{message_id}/edit` — replace a user
/// message and stream the new reply.
///
/// Saves the new text as a sibling of `message_id`, carries the original's
/// attachments forward, makes the new branch active and answers it. The
/// original message and everything below it stay stored. The first frame is
/// `run_started{run_id, user_message_id}`, the id the new message is saved
/// under. Returns `409` while a turn is working or another edit or regenerate
/// runs.
#[utoipa::path(
    post,
    path = "/api/conversations/{id}/messages/{message_id}/edit",
    operation_id = "edit_message",
    tag = "conversations",
    params(
        ("id" = Uuid, Path, description = "Conversation ID"),
        ("message_id" = Uuid, Path, description = "The user message to edit"),
    ),
    request_body = EditMessageRequest,
    responses(
        (status = 200, description = "SSE stream of assistant tokens and final message",
         content_type = "text/event-stream"),
        (status = 400, description = "Invalid ID, blank message, or not a user message", body = ErrorBody),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Read-only access, or missing conversations:write scope", body = ErrorBody),
        (status = 404, description = "Conversation or message not found", body = ErrorBody),
        (status = 409, description = "A turn is still working in this conversation", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn edit_message(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path((id, message_id)): Path<(String, String)>,
    body: Result<Json<EditMessageRequest>, JsonRejection>,
) -> Response {
    let target = match resolve_target(&ctx, &state, &auth, &id, &message_id).await {
        Ok(target) => target,
        Err(response) => return response,
    };
    let Json(body) = match body {
        Ok(body) => body,
        Err(rejection) => return json_error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    let text = body.message.trim().to_string();
    if text.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Message cannot be empty");
    }
    if target.message.role != MessageRole::User {
        return json_error(StatusCode::BAD_REQUEST, "Only user messages can be edited");
    }
    let anchor = TurnAnchor::Edit {
        parent_message_id: target.message.parent_id,
        edited_message_id: target.message.id,
    };
    run_branch_turn(
        &ctx,
        &state,
        &auth,
        target.conv_id,
        anchor,
        Uuid::new_v4(),
        text,
    )
    .await
}

/// `POST /api/conversations/{id}/messages/{message_id}/regenerate` — answer a
/// user message again and stream the new reply.
///
/// `message_id` is a user message or any row of a reply; a reply row resolves
/// to the user message above it. No user message is saved: the new reply
/// becomes another version of the old one. The first frame is
/// `run_started{run_id, user_message_id}` with the answered message's id.
/// Returns `409` while a turn is working or another edit or regenerate runs.
#[utoipa::path(
    post,
    path = "/api/conversations/{id}/messages/{message_id}/regenerate",
    operation_id = "regenerate_message",
    tag = "conversations",
    params(
        ("id" = Uuid, Path, description = "Conversation ID"),
        ("message_id" = Uuid, Path, description = "A user message, or any row of the reply to regenerate"),
    ),
    responses(
        (status = 200, description = "SSE stream of assistant tokens and final message",
         content_type = "text/event-stream"),
        (status = 400, description = "Invalid ID, or no user message to answer", body = ErrorBody),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Read-only access, or missing conversations:write scope", body = ErrorBody),
        (status = 404, description = "Conversation or message not found", body = ErrorBody),
        (status = 409, description = "A turn is still working in this conversation", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn regenerate_message(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path((id, message_id)): Path<(String, String)>,
) -> Response {
    let target = match resolve_target(&ctx, &state, &auth, &id, &message_id).await {
        Ok(target) => target,
        Err(response) => return response,
    };
    let user = if target.message.role == MessageRole::User {
        target.message
    } else {
        match nearest_user(&target.store, target.conv_id, target.message.id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                return json_error(StatusCode::BAD_REQUEST, "No user message to regenerate");
            }
            Err(response) => return response,
        }
    };
    let anchor = TurnAnchor::Regenerate {
        user_message_id: user.id,
    };
    run_branch_turn(
        &ctx,
        &state,
        &auth,
        target.conv_id,
        anchor,
        user.id,
        user.content,
    )
    .await
}

/// The closest user message at or above `message_id`.
async fn nearest_user(
    store: &impl ConversationStore,
    conv_id: Uuid,
    message_id: Uuid,
) -> Result<Option<Message>, Response> {
    match store.ancestors(conv_id, message_id).await {
        Ok(chain) => Ok(chain
            .unwrap_or_default()
            .into_iter()
            .rev()
            .find(|m| m.role == MessageRole::User)),
        Err(e) => {
            warn!("Failed to walk up from {message_id}: {e:#}");
            Err(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use tower::ServiceExt;
    use uuid::Uuid;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_core::AttachmentMeta;
    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{Action, ResourceKind, Role, Scope, SpaceId};
    use assistant_core::types::conversation::{Message, MessageRole};
    use assistant_runtime::orchestrator::TurnMode;
    use assistant_storage::{
        AttachmentStore, ConversationStore, InteractionStatus, InteractionStore, NewInteraction,
        SqliteConversationStore, StorageLayer,
    };

    use super::super::ApiState;
    use super::super::test_helpers::*;

    /// U1 → A1 → U2 → A2, owned by the caller `app` injects.
    struct Tree {
        store: SqliteConversationStore,
        conv: Uuid,
        u1: Uuid,
        a1: Uuid,
        u2: Uuid,
        a2: Uuid,
    }

    async fn seed(storage: &StorageLayer) -> Tree {
        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(None).await.unwrap().id;
        let mut ids = Vec::new();
        for (role, text) in [
            (MessageRole::User, "U1"),
            (MessageRole::Assistant, "A1"),
            (MessageRole::User, "U2"),
            (MessageRole::Assistant, "A2"),
        ] {
            let msg = Message::new(conv, role, text);
            store.save_message(&msg).await.unwrap();
            ids.push(msg.id);
        }
        Tree {
            store,
            conv,
            u1: ids[0],
            a1: ids[1],
            u2: ids[2],
            a2: ids[3],
        }
    }

    impl Tree {
        async fn active(&self) -> Vec<Message> {
            self.store.load_history(self.conv).await.unwrap()
        }

        async fn active_texts(&self) -> Vec<String> {
            self.active().await.into_iter().map(|m| m.content).collect()
        }
    }

    fn caller(role: Role, scopes: Option<Vec<Scope>>) -> AuthContext {
        let mut ctx = AuthContext::system();
        ctx.space_roles = std::collections::HashMap::from([(SpaceId::from("default"), role)]);
        if let Some(scopes) = scopes {
            ctx.scopes = scopes;
        }
        ctx
    }

    fn restricted() -> AuthContext {
        caller(
            Role::Member,
            Some(vec![Scope::new(ResourceKind::Personas, Action::Read)]),
        )
    }

    async fn orchestrator(state: &ApiState) -> Arc<assistant_runtime::Orchestrator> {
        state
            .runtimes
            .get("default", "default")
            .await
            .unwrap()
            .orchestrator
            .clone()
    }

    async fn post_as(
        state: ApiState,
        auth: AuthContext,
        uri: String,
        body: &str,
    ) -> axum::response::Response {
        app_with_auth(state, auth)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn edit_as(
        state: ApiState,
        auth: AuthContext,
        conv: &str,
        message: &str,
        text: &str,
    ) -> axum::response::Response {
        let body = serde_json::json!({ "message": text }).to_string();
        post_as(
            state,
            auth,
            format!("/conversations/{conv}/messages/{message}/edit"),
            &body,
        )
        .await
    }

    async fn edit(
        state: ApiState,
        conv: Uuid,
        message: Uuid,
        text: &str,
    ) -> axum::response::Response {
        edit_as(
            state,
            AuthContext::system(),
            &conv.to_string(),
            &message.to_string(),
            text,
        )
        .await
    }

    async fn regenerate_as(
        state: ApiState,
        auth: AuthContext,
        conv: &str,
        message: &str,
    ) -> axum::response::Response {
        post_as(
            state,
            auth,
            format!("/conversations/{conv}/messages/{message}/regenerate"),
            "",
        )
        .await
    }

    async fn regenerate(state: ApiState, conv: Uuid, message: Uuid) -> axum::response::Response {
        regenerate_as(
            state,
            AuthContext::system(),
            &conv.to_string(),
            &message.to_string(),
        )
        .await
    }

    async fn send(state: ApiState, conv: Uuid, text: &str) -> axum::response::Response {
        let body = serde_json::json!({ "message": text }).to_string();
        post_as(
            state,
            AuthContext::system(),
            format!("/conversations/{conv}/messages"),
            &body,
        )
        .await
    }

    /// One parsed SSE frame: `(event, data)`.
    type Frame = (String, serde_json::Value);

    async fn frames(resp: axum::response::Response) -> Vec<Frame> {
        let bytes = body_bytes(resp.into_body()).await;
        String::from_utf8_lossy(&bytes)
            .split("\n\n")
            .filter(|block| !block.trim().is_empty())
            .map(|block| {
                let (mut event, mut data) = (String::new(), String::new());
                for line in block.lines() {
                    if let Some(v) = line.strip_prefix("event:") {
                        event = v.trim().to_string();
                    } else if let Some(v) = line.strip_prefix("data:") {
                        data = v.trim().to_string();
                    }
                }
                (event, serde_json::from_str(&data).unwrap_or_default())
            })
            .collect()
    }

    async fn error_body(resp: axum::response::Response) -> serde_json::Value {
        let json = body_json(resp.into_body()).await;
        assert!(json["error"].is_string(), "missing error key: {json}");
        json
    }

    /// An LLM that answers after `delay`, so a turn stays in flight.
    async fn mount_slow_llm(server: &MockServer, reply: &str, delay: Duration) {
        let body = serde_json::json!({
            "model": "test",
            "message": { "role": "assistant", "content": reply },
            "done": true
        });
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(body)
                    .set_delay(delay),
            )
            .mount(server)
            .await;
    }

    async fn attach(storage: &StorageLayer, conv: Uuid, message: Uuid) -> AttachmentMeta {
        let meta = AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id: conv,
            agent_id: "default".into(),
            filename: "notes.txt".into(),
            mime_type: "text/plain".into(),
            size_bytes: 5,
            created_at: Utc::now(),
        };
        let store = storage.attachment_store();
        store.store(&meta, b"notes").await.unwrap();
        store.link_to_message(meta.id, message).await.unwrap();
        meta
    }

    // -- edit ------------------------------------------------------------------

    #[tokio::test]
    async fn edit_streams_run_started_with_the_user_message_id_and_ends_with_done() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "edited reply").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;

        let resp = edit(state, t.conv, t.u2, "U2 corrected").await;

        assert_eq!(resp.status(), StatusCode::OK);
        let frames = frames(resp).await;
        let (first, started) = frames.first().unwrap();
        assert_eq!(first, "run_started");
        assert!(Uuid::parse_str(started["run_id"].as_str().unwrap()).is_ok());
        let new_user = Uuid::parse_str(started["user_message_id"].as_str().unwrap()).unwrap();
        assert_ne!(new_user, t.u2, "the edit is a new row");
        assert_eq!(frames.last().unwrap().0, "done");

        let active = t.active().await;
        assert_eq!(
            t.active_texts().await,
            ["U1", "A1", "U2 corrected", "edited reply"]
        );
        assert_eq!(active[2].id, new_user, "the pre-assigned id was persisted");
        assert_eq!(active[2].parent_id, Some(t.a1), "a sibling of U2");
        assert!(
            t.store.get_message(t.u2).await.unwrap().is_some()
                && t.store.get_message(t.a2).await.unwrap().is_some(),
            "the original branch stays stored"
        );
    }

    #[tokio::test]
    async fn edit_of_an_assistant_message_is_400() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;

        let resp = edit(state, t.conv, t.a2, "nope").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = error_body(resp).await;
        assert_eq!(json["error"], "Only user messages can be edited");
        assert_eq!(t.active().await.len(), 4, "nothing changed");
    }

    #[tokio::test]
    async fn edit_with_a_blank_message_is_400() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;

        let resp = edit(state, t.conv, t.u2, "   ").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        error_body(resp).await;
    }

    #[tokio::test]
    async fn edit_with_invalid_ids_is_400() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;

        for (conv, message) in [
            ("nope".to_string(), t.u2.to_string()),
            (t.conv.to_string(), "nope".to_string()),
        ] {
            let resp = edit_as(state.clone(), AuthContext::system(), &conv, &message, "x").await;
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{conv} {message}");
            error_body(resp).await;
        }
    }

    #[tokio::test]
    async fn edit_of_an_unknown_or_foreign_message_is_404() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;
        let other = seed(&storage).await;

        for message in [Uuid::new_v4(), other.u1] {
            let resp = edit(state.clone(), t.conv, message, "x").await;
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            error_body(resp).await;
        }
        let resp = edit(state, Uuid::new_v4(), t.u2, "x").await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        error_body(resp).await;
    }

    #[tokio::test]
    async fn edit_without_conversations_write_scope_is_403() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;

        let resp = edit_as(
            state,
            restricted(),
            &t.conv.to_string(),
            &t.u2.to_string(),
            "x",
        )
        .await;

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        error_body(resp).await;
    }

    #[tokio::test]
    async fn edit_by_a_read_only_member_is_403() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;

        let resp = edit_as(
            state,
            caller(Role::Viewer, None),
            &t.conv.to_string(),
            &t.u2.to_string(),
            "x",
        )
        .await;

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        error_body(resp).await;
        assert_eq!(t.active().await.len(), 4);
    }

    #[tokio::test]
    async fn edit_of_an_attachment_bearing_message_keeps_its_attachments() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "reply").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;
        let meta = attach(&storage, t.conv, t.u2).await;

        let resp = edit(state, t.conv, t.u2, "U2 corrected").await;
        assert_eq!(resp.status(), StatusCode::OK);
        frames(resp).await;

        let new_user = t.active().await[2].id;
        let carried = storage
            .attachment_store()
            .list_for_message(new_user)
            .await
            .unwrap();
        assert_eq!(
            carried.iter().map(|m| m.id).collect::<Vec<_>>(),
            [meta.id],
            "the edited message carries its attachment forward"
        );
    }

    #[tokio::test]
    async fn edit_of_a_message_off_the_active_path_makes_its_branch_active() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "reply").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;
        // Move the active path off U2 onto a newer reply to U1.
        let other = Message::new(t.conv, MessageRole::Assistant, "A1b");
        t.store.branch_message(&other, Some(t.u1)).await.unwrap();

        let resp = edit(state, t.conv, t.u2, "U2 corrected").await;

        assert_eq!(resp.status(), StatusCode::OK);
        frames(resp).await;
        assert_eq!(
            t.active_texts().await,
            ["U1", "A1", "U2 corrected", "reply"],
            "the new branch grows from where U2 lives"
        );
    }

    // -- regenerate ------------------------------------------------------------

    #[tokio::test]
    async fn regenerate_adds_one_reply_and_no_user_row() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "A2 again").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;

        let resp = regenerate(state, t.conv, t.u2).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let frames = frames(resp).await;
        let (first, started) = frames.first().unwrap();
        assert_eq!(first, "run_started");
        assert_eq!(
            started["user_message_id"],
            t.u2.to_string(),
            "a regenerate answers the existing user message"
        );
        assert_eq!(frames.last().unwrap().0, "done");

        assert_eq!(t.active_texts().await, ["U1", "A1", "U2", "A2 again"]);
        let replies = t.store.children_of(t.conv, Some(t.u2)).await.unwrap();
        assert_eq!(replies.len(), 2, "old and new reply");
        let users = t.store.children_of(t.conv, Some(t.a1)).await.unwrap();
        assert_eq!(users.len(), 1, "no new user row");
    }

    #[tokio::test]
    async fn regenerate_by_a_reply_row_resolves_to_the_user_ancestor() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "A2 again").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;

        let resp = regenerate(state, t.conv, t.a2).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let frames = frames(resp).await;
        assert_eq!(frames[0].1["user_message_id"], t.u2.to_string());
        assert_eq!(t.active_texts().await, ["U1", "A1", "U2", "A2 again"]);
        assert_eq!(
            t.store.children_of(t.conv, Some(t.a1)).await.unwrap().len(),
            1,
            "no new user row"
        );
    }

    #[tokio::test]
    async fn regenerate_without_conversations_write_scope_is_403() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;

        let resp = regenerate_as(state, restricted(), &t.conv.to_string(), &t.u2.to_string()).await;

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        error_body(resp).await;
    }

    #[tokio::test]
    async fn regenerate_of_an_unknown_message_is_404() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;

        let resp = regenerate(state, t.conv, Uuid::new_v4()).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        error_body(resp).await;
    }

    // -- concurrency -----------------------------------------------------------

    #[tokio::test]
    async fn concurrent_edits_admit_exactly_one() {
        let server = MockServer::start().await;
        mount_slow_llm(&server, "reply", Duration::from_millis(400)).await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;

        let first = edit(state.clone(), t.conv, t.u2, "first").await;
        let second = edit(state, t.conv, t.u2, "second").await;

        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(second.status(), StatusCode::CONFLICT);
        error_body(second).await;
        frames(first).await;
        assert_eq!(t.active_texts().await, ["U1", "A1", "first", "reply"]);
    }

    #[tokio::test]
    async fn the_lease_is_released_when_the_edit_ends() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "reply").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;

        frames(edit(state.clone(), t.conv, t.u2, "first").await).await;
        let orch = orchestrator(&state).await;
        for _ in 0..100 {
            if !orch.conversation_turn_in_flight(t.conv) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let resp = regenerate(state, t.conv, t.u2).await;
        assert_eq!(resp.status(), StatusCode::OK, "a later branch is accepted");
        frames(resp).await;
    }

    #[tokio::test]
    async fn edit_and_regenerate_are_409_while_a_send_is_working() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;
        let _working = orchestrator(&state)
            .await
            .turn_registry()
            .try_begin(t.conv, Uuid::new_v4(), TurnMode::Shared)
            .expect("lease");

        let edited = edit(state.clone(), t.conv, t.u2, "x").await;
        let regenerated = regenerate(state, t.conv, t.u2).await;

        for resp in [edited, regenerated] {
            assert_eq!(resp.status(), StatusCode::CONFLICT);
            error_body(resp).await;
        }
        assert_eq!(t.active().await.len(), 4, "nothing was saved");
    }

    #[tokio::test]
    async fn a_plain_send_is_409_while_an_edit_runs_and_stores_no_user_message() {
        let server = MockServer::start().await;
        mount_slow_llm(&server, "reply", Duration::from_millis(400)).await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;

        let edited = edit(state.clone(), t.conv, t.u2, "edited").await;
        assert_eq!(edited.status(), StatusCode::OK);
        let sent = send(state, t.conv, "interloper").await;

        assert_eq!(sent.status(), StatusCode::CONFLICT);
        error_body(sent).await;
        frames(edited).await;
        let texts = t.active_texts().await;
        assert!(
            !texts.iter().any(|c| c == "interloper"),
            "the refused send stored nothing: {texts:?}"
        );
    }

    #[tokio::test]
    async fn a_send_is_accepted_while_another_send_runs() {
        let (state, storage) = event_log_state().await;
        let t = seed(&storage).await;
        let _other = orchestrator(&state)
            .await
            .turn_registry()
            .try_begin(t.conv, Uuid::new_v4(), TurnMode::Shared)
            .expect("lease");
        // Sends share the conversation: the shared lease must not refuse one.
        // (No worker runs here, so the turn itself is not awaited.)
        let resp = send(state, t.conv, "second").await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    // -- pending approvals -----------------------------------------------------

    async fn hold_a_turn_waiting_for_approval(
        state: &ApiState,
        storage: &Arc<StorageLayer>,
        conv: Uuid,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let run = Uuid::new_v4();
        let lease = orchestrator(state)
            .await
            .turn_registry()
            .try_begin(conv, run, TurnMode::Shared)
            .expect("lease");
        let interaction = Uuid::new_v4().to_string();
        storage
            .interaction_store()
            .create(NewInteraction {
                id: interaction.clone(),
                conversation_id: conv.to_string(),
                run_id: run.to_string(),
                user_id: "system".into(),
                kind: "tool_approval".into(),
                tool_call_id: Some("call-1".into()),
                request: serde_json::json!({"tool_name": "bash"}),
                expires_at: Utc::now() + chrono::Duration::minutes(10),
            })
            .await
            .unwrap();
        // The cancelled run lets go once its interaction resolves, as a real
        // run does on its way out.
        let watcher = storage.clone();
        let id = interaction.clone();
        let release = tokio::spawn(async move {
            loop {
                let row = watcher.interaction_store().get(&id).await.unwrap();
                if row.is_some_and(|r| r.status == InteractionStatus::Cancelled) {
                    drop(lease);
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        (interaction, release)
    }

    #[tokio::test]
    async fn edit_cancels_a_pending_approval_and_is_accepted() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "reply").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;
        let (interaction, release) =
            hold_a_turn_waiting_for_approval(&state, &storage, t.conv).await;

        let resp = edit(state, t.conv, t.u2, "U2 corrected").await;

        assert_eq!(resp.status(), StatusCode::OK);
        frames(resp).await;
        release.await.unwrap();
        let row = storage
            .interaction_store()
            .get(&interaction)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, InteractionStatus::Cancelled);
        assert_eq!(
            t.active_texts().await,
            ["U1", "A1", "U2 corrected", "reply"]
        );
    }

    #[tokio::test]
    async fn regenerate_cancels_a_pending_approval_and_is_accepted() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "A2 again").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;
        let (interaction, release) =
            hold_a_turn_waiting_for_approval(&state, &storage, t.conv).await;

        let resp = regenerate(state, t.conv, t.u2).await;

        assert_eq!(resp.status(), StatusCode::OK);
        frames(resp).await;
        release.await.unwrap();
        let row = storage
            .interaction_store()
            .get(&interaction)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.status, InteractionStatus::Cancelled);
        assert_eq!(t.active_texts().await, ["U1", "A1", "U2", "A2 again"]);
    }

    // -- cancelled regenerate --------------------------------------------------

    #[tokio::test]
    async fn a_cancelled_regenerate_keeps_the_old_reply_as_the_other_version() {
        let server = MockServer::start().await;
        mount_slow_llm(&server, "never arrives", Duration::from_secs(30)).await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;
        let orchestrator = orchestrator(&state).await;

        let resp = regenerate(state, t.conv, t.u2).await;
        let run_id = Uuid::parse_str(
            resp.headers()
                .get("x-run-id")
                .and_then(|v| v.to_str().ok())
                .expect("X-Run-Id header"),
        )
        .unwrap();
        // Cancel once a worker claimed the turn and called the LLM.
        for _ in 0..200 {
            if !server
                .received_requests()
                .await
                .unwrap_or_default()
                .is_empty()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(orchestrator.is_turn_in_flight(run_id).await, "claimed");
        orchestrator.cancel_turn(run_id).await;
        let frames = frames(resp).await;

        let (event, payload) = frames.last().unwrap();
        assert_eq!(event, "agent_error");
        assert_eq!(payload["reason"], "cancelled");
        let stopped = Uuid::parse_str(payload["message_id"].as_str().unwrap()).unwrap();
        assert_eq!(
            t.active_texts().await,
            ["U1", "A1", "U2", "_Stopped._"],
            "the stop is a reply to U2 on the active path"
        );
        let replies = t.store.children_of(t.conv, Some(t.u2)).await.unwrap();
        assert_eq!(
            replies.iter().map(|m| m.id).collect::<Vec<_>>(),
            [t.a2, stopped],
            "the old reply stays reachable as a sibling"
        );
    }

    // -- send_message: user_message_id -----------------------------------------

    #[tokio::test]
    async fn send_message_run_started_carries_the_id_its_user_row_gets() {
        let server = MockServer::start().await;
        mount_llm_reply(&server, "hello").await;
        let (state, storage) = test_state(&server.uri()).await;
        let t = seed(&storage).await;

        let resp = send(state.clone(), t.conv, "U3").await;
        assert_eq!(resp.status(), StatusCode::OK);
        let frames = frames(resp).await;
        let (_, started) = &frames[0];
        let run_id = started["run_id"].as_str().unwrap().to_string();
        let user_message_id =
            Uuid::parse_str(started["user_message_id"].as_str().unwrap()).unwrap();

        let user = t.store.get_message(user_message_id).await.unwrap().unwrap();
        assert_eq!(user.content, "U3", "the user row has the announced id");

        // Replay from the start carries it too.
        let replay = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/conversations/{}/runs/{run_id}/events/stream?since=0",
                        t.conv
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let replayed = self::frames(replay).await;
        assert_eq!(replayed[0].0, "run_started");
        assert_eq!(
            replayed[0].1["user_message_id"],
            user_message_id.to_string()
        );
    }
}
