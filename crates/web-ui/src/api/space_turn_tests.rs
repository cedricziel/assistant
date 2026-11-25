//! A turn submitted through the API runs on the runtime of the caller's space.
//!
//! Two spaces of one org, each with its own database and its own orchestrator.
//! The caller holds a role in both; the `X-Space-Id` selector decides where
//! the turn lands.

#![cfg(test)]

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use wiremock::MockServer;

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Action, OrgId, ResourceKind, Role, Scope, SpaceId, UserId};
use assistant_storage::{ConversationStore as _, SqliteConversationStore, StorageLayer};

use super::test_helpers::{
    TEST_SUBMIT_TIMEOUT_SECS, TestSpace, app_with_auth, body_json, mount_llm_reply, state_over,
};

const ALPHA: &str = "spc_alpha";
const BETA: &str = "spc_beta";

fn caller() -> AuthContext {
    AuthContext {
        user_id: UserId::from("alice"),
        org_id: OrgId::from("org_acme"),
        email: "alice@example.test".into(),
        space_roles: HashMap::from([
            (SpaceId::from(ALPHA), Role::Member),
            (SpaceId::from(BETA), Role::Member),
        ]),
        scopes: vec![Scope::new(ResourceKind::Conversations, Action::Write)],
        client_id: "test".into(),
    }
}

async fn conversations_in(storage: &StorageLayer) -> usize {
    SqliteConversationStore::for_agent(storage.pool.clone(), "default")
        .list_conversations()
        .await
        .unwrap()
        .len()
}

#[tokio::test]
async fn a_quick_message_in_space_b_persists_in_b_and_never_touches_a() {
    assert!(TEST_SUBMIT_TIMEOUT_SECS > 0);
    let server = MockServer::start().await;
    mount_llm_reply(&server, "answer from beta's runtime").await;
    let alpha = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let beta = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let (state, _) = state_over(
        &server.uri(),
        &[
            TestSpace {
                id: ALPHA,
                org_id: "org_acme",
                org_slug: "acme",
                slug: "alpha",
                storage: alpha.clone(),
            },
            TestSpace {
                id: BETA,
                org_id: "org_acme",
                org_slug: "acme",
                slug: "beta",
                storage: beta.clone(),
            },
        ],
        true,
    )
    .await;

    let response = app_with_auth(state, caller())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quick-message")
                .header("content-type", "application/json")
                .header("X-Space-Id", BETA)
                .body(Body::from(r#"{"message":"hello"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = body_json(response.into_body()).await;
    assert_eq!(body["answer"], "answer from beta's runtime");
    let conversation = body["conversation_id"].as_str().unwrap().parse().unwrap();

    let in_beta = SqliteConversationStore::for_agent(beta.pool.clone(), "default");
    assert!(
        in_beta.load_history(conversation).await.unwrap().len() >= 2,
        "the turn is persisted in beta's database"
    );
    assert_eq!(conversations_in(&beta).await, 1);
    assert_eq!(
        conversations_in(&alpha).await,
        0,
        "alpha's database is untouched"
    );
}
