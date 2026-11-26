//! Branch metadata and `POST /api/conversations/{id}/switch-branch`
//! (ADR-0018, D9).
//!
//! `GET /api/conversations/{id}` lists only the active path. Each row of it
//! that has siblings says so, so the client can offer a picker: edited user
//! messages carry their versions as `branch`, and the versions of a reply are
//! projected onto the reply's last visible assistant row (or, when the reply
//! has none, onto the user row as `reply_branch`).

use std::collections::HashMap;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::types::conversation::{Message, MessageRole};
use assistant_storage::ConversationStore;

use crate::errors::json_error;
use crate::openapi::ErrorBody;
use crate::space::{Access, SpaceCtx};

use super::ApiState;
use super::conversations::{
    ConversationDetail, caller_store, conversation_detail, owned_conversation,
};
use super::messages::caller_can_post;

/// Where a message sits among its siblings.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct BranchInfo {
    /// Zero-based position of the active version, in creation order.
    pub index: usize,
    /// How many versions there are (always at least 2).
    pub count: usize,
    /// The first message of every version, in creation order. Switching to
    /// any of them selects that version.
    pub sibling_ids: Vec<Uuid>,
}

/// Branch metadata for the rows of an active path, keyed by message id.
#[derive(Debug, Default)]
pub(crate) struct BranchProjection {
    /// Versions of a user message, or of the reply ending at this row.
    pub branch: HashMap<Uuid, BranchInfo>,
    /// Versions of the reply to this user row, when the reply has no visible
    /// assistant row to carry them.
    pub reply_branch: HashMap<Uuid, BranchInfo>,
}

fn info_for(siblings: &[Message], id: Uuid) -> Option<BranchInfo> {
    if siblings.len() < 2 {
        return None;
    }
    Some(BranchInfo {
        index: siblings.iter().position(|m| m.id == id)?,
        count: siblings.len(),
        sibling_ids: siblings.iter().map(|m| m.id).collect(),
    })
}

/// Work out the branch metadata of `path`, the active path of `conversation_id`
/// from the root down. `visible` says whether an assistant row shows to the
/// user (it has text or attachments).
///
/// Failing to read siblings only drops the pickers: the conversation still
/// loads.
pub(crate) async fn project_branches(
    store: &impl ConversationStore,
    conversation_id: Uuid,
    path: &[Message],
    visible: impl Fn(&Message) -> bool,
) -> BranchProjection {
    let mut out = BranchProjection::default();
    for (i, user) in path.iter().enumerate() {
        if user.role != MessageRole::User {
            continue;
        }
        let siblings = |id: Uuid| async move {
            store
                .list_siblings(conversation_id, id)
                .await
                .unwrap_or_else(|e| {
                    warn!(%conversation_id, "listing siblings of {id} failed: {e:#}");
                    None
                })
                .and_then(|siblings| info_for(&siblings, id))
        };
        if let Some(info) = siblings(user.id).await {
            out.branch.insert(user.id, info);
        }

        let Some(first) = path.get(i + 1).filter(|m| m.parent_id == Some(user.id)) else {
            continue;
        };
        let Some(info) = siblings(first.id).await else {
            continue;
        };
        let reply_end = path[i + 1..]
            .iter()
            .position(|m| m.role == MessageRole::User)
            .map_or(path.len(), |p| i + 1 + p);
        match path[i + 1..reply_end]
            .iter()
            .rev()
            .find(|m| m.role == MessageRole::Assistant && visible(m))
        {
            Some(last) => out.branch.insert(last.id, info),
            None => out.reply_branch.insert(user.id, info),
        };
    }
    out
}

/// Body for `POST /api/conversations/{id}/switch-branch`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SwitchBranchRequest {
    /// Any message of the version to switch to. The active path continues
    /// from it through the most recently created child at each fork.
    pub message_id: String,
}

/// `POST /api/conversations/{id}/switch-branch` — make another version of a
/// message the active one.
///
/// Lands on the most recently created leaf below `message_id`. Does not
/// change `updated_at`. While a turn is working it returns `409`; while a turn
/// only waits for an approval or an answer, that wait is cancelled first.
#[utoipa::path(
    post,
    path = "/api/conversations/{id}/switch-branch",
    operation_id = "switch_conversation_branch",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    request_body = SwitchBranchRequest,
    responses(
        (status = 200, description = "The conversation on its new active path", body = ConversationDetail),
        (status = 400, description = "Invalid conversation or message ID", body = ErrorBody),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Read-only access, or missing conversations:write scope", body = ErrorBody),
        (status = 404, description = "Conversation or message not found", body = ErrorBody),
        (status = 409, description = "A turn is still working in this conversation", body = ErrorBody),
        (status = 500, description = "Internal server error", body = ErrorBody),
    ),
    security(("bearer_token" = []))
)]
pub async fn switch_conversation_branch(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<String>,
    body: Result<Json<SwitchBranchRequest>, JsonRejection>,
) -> Response {
    if let Err(denied) = ctx.require(Access::Write) {
        return denied.into_response();
    }
    if !caller_can_post(&auth) {
        return json_error(StatusCode::FORBIDDEN, "Missing conversations:write scope");
    }
    let Ok(conv_id) = Uuid::parse_str(&id) else {
        return json_error(StatusCode::BAD_REQUEST, "Invalid conversation ID");
    };
    let Json(body) = match body {
        Ok(body) => body,
        Err(rejection) => return json_error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    let Ok(message_id) = Uuid::parse_str(&body.message_id) else {
        return json_error(StatusCode::BAD_REQUEST, "Invalid message ID");
    };

    let store = caller_store(&ctx, &state).await.with_broadcaster(
        state
            .conversation_broadcasters
            .for_space(&ctx.org_slug, &ctx.space_slug),
    );
    let conv = match owned_conversation(&store, conv_id).await {
        Ok(conv) => conv,
        Err(response) => return response,
    };
    // Look before taking the lease, so a bad id never cancels a pending
    // approval.
    match store.get_message(message_id).await {
        Ok(Some(m)) if m.conversation_id == conv_id => {}
        Ok(_) => return json_error(StatusCode::NOT_FOUND, "Message not found"),
        Err(e) => {
            warn!("Failed to load message {message_id}: {e:#}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error");
        }
    }

    let orchestrator = match state.runtime(&ctx).await {
        Ok(runtime) => runtime.orchestrator.clone(),
        Err(response) => return response,
    };
    let _lease = match orchestrator
        .acquire_exclusive_for_branch(conv_id, Uuid::new_v4())
        .await
    {
        Ok(lease) => lease,
        Err(conflict) => return json_error(StatusCode::CONFLICT, conflict.to_string()),
    };

    match store.switch_to(conv_id, message_id).await {
        Ok(Some(_)) => Json(conversation_detail(&ctx, &state, &store, &conv).await).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Message not found"),
        Err(e) => {
            warn!("Failed to switch conversation {conv_id} to {message_id}: {e:#}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to switch branch")
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::{DateTime, Duration, Utc};
    use tower::ServiceExt;
    use uuid::Uuid;

    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{Action, ResourceKind, Role, Scope, SpaceId};
    use assistant_core::types::conversation::{Message, MessageRole};
    use assistant_storage::{
        ConversationStore, InteractionStatus, InteractionStore, NewInteraction,
        SqliteConversationStore, StorageLayer,
    };

    use super::super::ApiState;
    use super::super::test_helpers::*;

    /// Test fixture: a conversation whose rows get strictly increasing
    /// `created_at` values, so "most recently created" is unambiguous.
    struct Tree {
        store: SqliteConversationStore,
        conv: Uuid,
        clock: DateTime<Utc>,
    }

    impl Tree {
        async fn new(storage: &StorageLayer) -> Self {
            let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
                .with_user_id("system");
            let conv = store.create_conversation(None).await.unwrap().id;
            Self {
                store,
                conv,
                clock: Utc::now() - Duration::hours(1),
            }
        }

        fn stamp(&mut self, mut msg: Message) -> Message {
            self.clock += Duration::seconds(1);
            msg.created_at = self.clock;
            msg
        }

        /// Append under the active leaf.
        async fn push(&mut self, role: MessageRole, text: &str) -> Uuid {
            let msg = self.stamp(Message::new(self.conv, role, text));
            self.store.save_message(&msg).await.unwrap();
            msg.id
        }

        /// Insert under `parent` and make the row the active leaf.
        async fn fork(&mut self, parent: Option<Uuid>, role: MessageRole, text: &str) -> Uuid {
            let msg = self.stamp(Message::new(self.conv, role, text));
            self.store.branch_message(&msg, parent).await.unwrap();
            msg.id
        }

        async fn leaf_path(&self) -> Vec<Uuid> {
            self.store
                .load_history(self.conv)
                .await
                .unwrap()
                .iter()
                .map(|m| m.id)
                .collect()
        }
    }

    async fn get_detail(state: ApiState, conv: Uuid) -> serde_json::Value {
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/conversations/{conv}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        body_json(resp.into_body()).await
    }

    fn row<'a>(detail: &'a serde_json::Value, id: Uuid) -> &'a serde_json::Value {
        detail["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == id.to_string())
            .unwrap_or_else(|| panic!("message {id} not in {detail}"))
    }

    fn ids(detail: &serde_json::Value) -> Vec<String> {
        detail["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_str().unwrap().to_string())
            .collect()
    }

    // -- GET /conversations/{id}: branch metadata ------------------------------

    #[tokio::test]
    async fn root_has_no_parent_id_and_children_name_theirs() {
        let (state, storage) = event_log_state().await;
        let mut t = Tree::new(&storage).await;
        let u1 = t.push(MessageRole::User, "hi").await;
        let a1 = t.push(MessageRole::Assistant, "hello").await;

        let detail = get_detail(state, t.conv).await;

        assert!(
            row(&detail, u1).get("parent_id").is_none(),
            "a root omits parent_id"
        );
        assert_eq!(row(&detail, a1)["parent_id"], u1.to_string());
    }

    #[tokio::test]
    async fn edited_user_message_carries_branch() {
        let (state, storage) = event_log_state().await;
        let mut t = Tree::new(&storage).await;
        t.push(MessageRole::User, "one").await;
        let a1 = t.push(MessageRole::Assistant, "reply").await;
        let u2 = t.push(MessageRole::User, "two").await;
        let u2_edit = t.fork(Some(a1), MessageRole::User, "two, edited").await;

        let detail = get_detail(state, t.conv).await;

        let branch = &row(&detail, u2_edit)["branch"];
        assert_eq!(branch["index"], 1);
        assert_eq!(branch["count"], 2);
        assert_eq!(
            branch["sibling_ids"],
            serde_json::json!([u2.to_string(), u2_edit.to_string()])
        );
    }

    #[tokio::test]
    async fn edited_first_message_branches_across_roots() {
        let (state, storage) = event_log_state().await;
        let mut t = Tree::new(&storage).await;
        let u1 = t.push(MessageRole::User, "one").await;
        let u1_edit = t.fork(None, MessageRole::User, "one, edited").await;

        let detail = get_detail(state, t.conv).await;

        let branch = &row(&detail, u1_edit)["branch"];
        assert_eq!(
            branch["sibling_ids"],
            serde_json::json!([u1.to_string(), u1_edit.to_string()]),
            "roots are siblings of each other"
        );
    }

    #[tokio::test]
    async fn reply_branch_lands_on_the_last_visible_assistant_row() {
        let (state, storage) = event_log_state().await;
        let mut t = Tree::new(&storage).await;
        let u1 = t.push(MessageRole::User, "question").await;
        // Reply one: a hidden tool-call row, then the visible text.
        let first_a = t.push(MessageRole::Assistant, "").await;
        t.push(MessageRole::Assistant, "answer one").await;
        // Reply two (active): a hidden thinking row, then the visible text.
        let first_b = t.fork(Some(u1), MessageRole::Assistant, "").await;
        let visible_b = t.push(MessageRole::Assistant, "answer two").await;

        let detail = get_detail(state, t.conv).await;

        let branch = &row(&detail, visible_b)["branch"];
        assert_eq!(branch["index"], 1);
        assert_eq!(branch["count"], 2);
        assert_eq!(
            branch["sibling_ids"],
            serde_json::json!([first_a.to_string(), first_b.to_string()]),
            "sibling_ids are first-row ids"
        );
        assert!(
            row(&detail, first_b).get("branch").is_none(),
            "the hidden first row carries nothing"
        );
        assert!(row(&detail, u1).get("reply_branch").is_none());
    }

    #[tokio::test]
    async fn reply_branch_sits_on_the_user_row_without_a_visible_reply() {
        let (state, storage) = event_log_state().await;
        let mut t = Tree::new(&storage).await;
        let u1 = t.push(MessageRole::User, "question").await;
        let first_a = t.push(MessageRole::Assistant, "answer one").await;
        let hidden = t.fork(Some(u1), MessageRole::Assistant, "").await;

        let detail = get_detail(state, t.conv).await;

        let reply_branch = &row(&detail, u1)["reply_branch"];
        assert_eq!(reply_branch["index"], 1);
        assert_eq!(reply_branch["count"], 2);
        assert_eq!(
            reply_branch["sibling_ids"],
            serde_json::json!([first_a.to_string(), hidden.to_string()])
        );
        assert!(row(&detail, hidden).get("branch").is_none());
    }

    #[tokio::test]
    async fn branch_is_omitted_without_siblings() {
        let (state, storage) = event_log_state().await;
        let mut t = Tree::new(&storage).await;
        let u1 = t.push(MessageRole::User, "hi").await;
        let a1 = t.push(MessageRole::Assistant, "hello").await;

        let detail = get_detail(state, t.conv).await;

        for id in [u1, a1] {
            let m = row(&detail, id);
            assert!(m.get("branch").is_none(), "no siblings: {m}");
            assert!(m.get("reply_branch").is_none(), "no siblings: {m}");
        }
    }

    // -- POST /conversations/{id}/switch-branch --------------------------------

    async fn switch_as(
        state: ApiState,
        auth: AuthContext,
        conv: &str,
        body: String,
    ) -> (StatusCode, serde_json::Value) {
        let resp = app_with_auth(state, auth)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/conversations/{conv}/switch-branch"))
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
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

    async fn switch(state: ApiState, conv: Uuid, message: Uuid) -> (StatusCode, serde_json::Value) {
        switch_as(
            state,
            AuthContext::system(),
            &conv.to_string(),
            format!(r#"{{"message_id":"{message}"}}"#),
        )
        .await
    }

    fn caller(role: Role, scopes: Option<Vec<Scope>>) -> AuthContext {
        let mut ctx = AuthContext::system();
        ctx.space_roles = std::collections::HashMap::from([(SpaceId::from("default"), role)]);
        if let Some(scopes) = scopes {
            ctx.scopes = scopes;
        }
        ctx
    }

    /// U1 with an older reply A1 and the active, newer reply A1b.
    async fn two_replies(storage: &StorageLayer) -> (Tree, Uuid, Uuid, Uuid) {
        let mut t = Tree::new(storage).await;
        let u1 = t.push(MessageRole::User, "question").await;
        let a1 = t.push(MessageRole::Assistant, "old").await;
        let a1b = t.fork(Some(u1), MessageRole::Assistant, "new").await;
        (t, u1, a1, a1b)
    }

    #[tokio::test]
    async fn switch_returns_the_new_path() {
        let (state, storage) = event_log_state().await;
        let (t, u1, a1, _) = two_replies(&storage).await;

        let (status, json) = switch(state, t.conv, a1).await;

        assert_eq!(status, StatusCode::OK, "{json}");
        assert_eq!(ids(&json), [u1.to_string(), a1.to_string()]);
        assert_eq!(t.leaf_path().await, [u1, a1], "the leaf was persisted");
    }

    #[tokio::test]
    async fn switch_descends_to_the_most_recent_child_at_each_fork() {
        let (state, storage) = event_log_state().await;
        let mut t = Tree::new(&storage).await;
        let u1 = t.push(MessageRole::User, "q").await;
        let a1 = t.push(MessageRole::Assistant, "old reply").await;
        let u2_old = t.push(MessageRole::User, "follow-up one").await;
        let u2_new = t.fork(Some(a1), MessageRole::User, "follow-up two").await;
        let a2 = t.push(MessageRole::Assistant, "tail").await;
        let a1b = t.fork(Some(u1), MessageRole::Assistant, "new reply").await;
        assert_eq!(t.leaf_path().await, [u1, a1b], "precondition");

        let (status, json) = switch(state.clone(), t.conv, a1).await;
        assert_eq!(status, StatusCode::OK, "{json}");
        assert_eq!(
            ids(&json),
            [u1, a1, u2_new, a2].map(|i| i.to_string()),
            "the newer follow-up wins at the fork under a1"
        );

        let (_, json) = switch(state, t.conv, u2_old).await;
        assert_eq!(
            ids(&json),
            [u1, a1, u2_old].map(|i| i.to_string()),
            "switching to the older sibling lands on it"
        );
    }

    #[tokio::test]
    async fn switch_does_not_change_updated_at() {
        let (state, storage) = event_log_state().await;
        let (t, _, a1, _) = two_replies(&storage).await;
        let before = t.store.get_conversation(t.conv).await.unwrap().unwrap();

        let (status, json) = switch(state, t.conv, a1).await;

        assert_eq!(status, StatusCode::OK);
        let after = t.store.get_conversation(t.conv).await.unwrap().unwrap();
        assert_eq!(after.updated_at, before.updated_at);
        assert_eq!(
            json["updated_at"],
            serde_json::to_value(before.updated_at).unwrap()
        );
    }

    #[tokio::test]
    async fn switch_broadcasts_an_upsert() {
        let (state, storage) = event_log_state().await;
        let (t, _, a1, _) = two_replies(&storage).await;
        let mut rx = state
            .conversation_broadcasters
            .for_space("default", "default")
            .subscribe();

        let (status, _) = switch(state, t.conv, a1).await;

        assert_eq!(status, StatusCode::OK);
        assert!(
            matches!(
                rx.try_recv(),
                Ok(assistant_storage::ConversationEvent::Upserted(_))
            ),
            "other tabs must reload"
        );
    }

    #[tokio::test]
    async fn switch_to_a_foreign_or_unknown_message_is_404() {
        let (state, storage) = event_log_state().await;
        let (t, ..) = two_replies(&storage).await;
        let mut other = Tree::new(&storage).await;
        let foreign = other.push(MessageRole::User, "elsewhere").await;
        let leaf_before = t.leaf_path().await;

        for message in [foreign, Uuid::new_v4()] {
            let (status, json) = switch(state.clone(), t.conv, message).await;
            assert_eq!(status, StatusCode::NOT_FOUND);
            assert!(json["error"].is_string(), "missing error key: {json}");
        }
        assert_eq!(t.leaf_path().await, leaf_before, "nothing moved");
    }

    #[tokio::test]
    async fn switch_in_a_foreign_conversation_is_404() {
        let (state, storage) = event_log_state().await;
        let (t, _, a1, _) = two_replies(&storage).await;
        let mallory = caller(Role::Member, None);
        let mut mallory = mallory;
        mallory.user_id = assistant_core::identity::UserId::from("mallory");

        let (status, json) = switch_as(
            state,
            mallory,
            &t.conv.to_string(),
            format!(r#"{{"message_id":"{a1}"}}"#),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(json["error"].is_string(), "missing error key: {json}");
    }

    #[tokio::test]
    async fn switch_with_an_invalid_uuid_is_400() {
        let (state, storage) = event_log_state().await;
        let (t, ..) = two_replies(&storage).await;

        let cases = [
            (t.conv.to_string(), r#"{"message_id":"nope"}"#.to_string()),
            (t.conv.to_string(), r#"{"message_id":""}"#.to_string()),
            (
                "nope".to_string(),
                format!(r#"{{"message_id":"{}"}}"#, Uuid::new_v4()),
            ),
        ];
        for (conv, body) in cases {
            let (status, json) = switch_as(state.clone(), AuthContext::system(), &conv, body).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{conv}");
            assert!(json["error"].is_string(), "missing error key: {json}");
        }
    }

    #[tokio::test]
    async fn switch_without_conversations_write_scope_is_403() {
        let (state, storage) = event_log_state().await;
        let (t, _, a1, _) = two_replies(&storage).await;
        let restricted = caller(
            Role::Member,
            Some(vec![Scope::new(ResourceKind::Personas, Action::Read)]),
        );

        let (status, json) = switch_as(
            state,
            restricted,
            &t.conv.to_string(),
            format!(r#"{{"message_id":"{a1}"}}"#),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(json["error"].is_string(), "missing error key: {json}");
    }

    #[tokio::test]
    async fn switch_by_a_read_only_member_is_403() {
        let (state, storage) = event_log_state().await;
        let (t, _, a1, _) = two_replies(&storage).await;

        let (status, json) = switch_as(
            state,
            caller(Role::Viewer, None),
            &t.conv.to_string(),
            format!(r#"{{"message_id":"{a1}"}}"#),
        )
        .await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(json["error"].is_string(), "missing error key: {json}");
        assert_eq!(t.leaf_path().await.len(), 2);
    }

    async fn orchestrator(state: &ApiState) -> std::sync::Arc<assistant_runtime::Orchestrator> {
        state
            .runtimes
            .get("default", "default")
            .await
            .unwrap()
            .orchestrator
            .clone()
    }

    #[tokio::test]
    async fn switch_while_a_turn_is_working_is_409() {
        let (state, storage) = event_log_state().await;
        let (t, _, a1, a1b) = two_replies(&storage).await;
        let orch = orchestrator(&state).await;
        let _working = orch
            .turn_registry()
            .try_begin(
                t.conv,
                Uuid::new_v4(),
                assistant_runtime::orchestrator::TurnMode::Shared,
            )
            .expect("lease");

        let (status, json) = switch(state, t.conv, a1).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert!(json["error"].is_string(), "missing error key: {json}");
        assert_eq!(t.leaf_path().await.last(), Some(&a1b), "leaf unchanged");
    }

    #[tokio::test]
    async fn switch_while_a_turn_waits_for_an_approval_cancels_it() {
        let (state, storage) = event_log_state().await;
        let (t, _, a1, _) = two_replies(&storage).await;
        let orch = orchestrator(&state).await;
        let run = Uuid::new_v4();
        let lease = orch
            .turn_registry()
            .try_begin(
                t.conv,
                run,
                assistant_runtime::orchestrator::TurnMode::Shared,
            )
            .expect("lease");
        let interaction = Uuid::new_v4().to_string();
        storage
            .interaction_store()
            .create(NewInteraction {
                id: interaction.clone(),
                conversation_id: t.conv.to_string(),
                run_id: run.to_string(),
                user_id: "system".into(),
                kind: "tool_approval".into(),
                tool_call_id: Some("call-1".into()),
                request: serde_json::json!({}),
                expires_at: Utc::now() + Duration::minutes(10),
            })
            .await
            .unwrap();
        // The cancelled run lets go once its interaction resolves, as a real
        // run does on its way out.
        let watcher = storage.clone();
        let release = tokio::spawn(async move {
            loop {
                let row = watcher.interaction_store().get(&interaction).await.unwrap();
                if row.is_some_and(|r| r.status == InteractionStatus::Cancelled) {
                    drop(lease);
                    return;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        });

        let (status, json) = switch(state, t.conv, a1).await;

        assert_eq!(status, StatusCode::OK, "{json}");
        assert_eq!(ids(&json).last().unwrap(), &a1.to_string());
        release.await.unwrap();
    }
}
