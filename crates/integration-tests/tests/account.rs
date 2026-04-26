//! Cross-cutting integration test for the self-service account flow.
//!
//! Wires the account router (`/api/users/me*`) under the real
//! `assistant_web_ui::auth::require_auth` middleware so we exercise the full
//! bearer-token resolution path (JWT + API key), not just the handlers in
//! isolation.
//!
//! Walks through the spec's full happy path:
//!   1. log in (provision JWT + API key + refresh tokens)
//!   2. GET /api/users/me with bearer JWT
//!   3. PATCH /api/users/me — change email
//!   4. PATCH /api/users/me — change name
//!   5. POST /api/users/me/password — change password
//!   6. assert old refresh tokens have been revoked
//!   7. assert the calling JWT continues to work (still 200)
//!   8. assert the API key continues to work (still 200)

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::response::Response;
use chrono::Utc;
use serde_json::json;
use tower::ServiceExt;

use assistant_auth::api_keys::{ApiKeyRecord, ApiKeyStore, InMemoryApiKeyStore, generate_key};
use assistant_auth::jwt::{JwtKeyPair, JwtManager};
use assistant_auth::oauth2::server::{
    InMemoryRefreshTokenStore, RefreshTokenStore, StoredRefreshToken,
};
use assistant_auth::password::{hash_password, verify_password};
use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
use assistant_core::store::{Organization, User};
use assistant_core::{OrgStore, UserStore};
use assistant_storage::OrgStorageLayer;
use assistant_web_ui::api::account::{AccountApiState, UpdateCurrentUserResponse, account_router};
use assistant_web_ui::auth::{WebAuthConfig, require_auth};

const ORG_ID: &str = "org_test";
const ORG_SLUG: &str = "test";
const ALICE_ID: &str = "alice";
const BOB_ID: &str = "bob";
const ALICE_INITIAL_EMAIL: &str = "alice@test.local";
const ALICE_NEW_EMAIL: &str = "alice.updated@test.local";
const ALICE_INITIAL_PASSWORD: &str = "hunter2-strong!";
const ALICE_NEW_PASSWORD: &str = "freshpass-v2!";
const ALICE_REFRESH_TOKEN_A: &str = "alice-refresh-a";
const ALICE_REFRESH_TOKEN_B: &str = "alice-refresh-b";
const BOB_REFRESH_TOKEN: &str = "bob-refresh-x";
const JWT_SECRET: &[u8] = b"integration-account-test-secret-32!!";

struct TestEnv {
    app: Router,
    alice_jwt: String,
    alice_api_key_plaintext: String,
    layer: Arc<OrgStorageLayer>,
    refresh_store: Arc<InMemoryRefreshTokenStore>,
}

async fn setup() -> TestEnv {
    // -- Storage layer with seeded org + alice + bob -----------------------
    let layer = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());

    let now = Utc::now();
    let org = Organization {
        id: OrgId::from(ORG_ID),
        name: "Test Org".into(),
        slug: ORG_SLUG.into(),
        auth_mode: "password".into(),
        created_at: now,
        updated_at: now,
    };
    layer.org_store().create_org(&org).await.unwrap();

    let alice = User {
        id: UserId::from(ALICE_ID),
        org_id: OrgId::from(ORG_ID),
        email: ALICE_INITIAL_EMAIL.into(),
        name: "Alice".into(),
        password_hash: hash_password(ALICE_INITIAL_PASSWORD).unwrap(),
        idp_issuer: None,
        idp_subject: None,
        created_at: now,
        updated_at: now,
    };
    layer.user_store().create_user(&alice).await.unwrap();

    let bob = User {
        id: UserId::from(BOB_ID),
        org_id: OrgId::from(ORG_ID),
        email: "bob@test.local".into(),
        name: "Bob".into(),
        password_hash: hash_password("bob-password").unwrap(),
        idp_issuer: None,
        idp_subject: None,
        created_at: now,
        updated_at: now,
    };
    layer.user_store().create_user(&bob).await.unwrap();

    // -- Refresh tokens: 2 for alice + 1 for bob ---------------------------
    let refresh_store = Arc::new(InMemoryRefreshTokenStore::default());
    for tok in [ALICE_REFRESH_TOKEN_A, ALICE_REFRESH_TOKEN_B] {
        refresh_store
            .store_token(refresh_token(tok, ALICE_ID))
            .await
            .unwrap();
    }
    refresh_store
        .store_token(refresh_token(BOB_REFRESH_TOKEN, BOB_ID))
        .await
        .unwrap();

    // -- API key for alice -------------------------------------------------
    let api_key_store: Arc<dyn ApiKeyStore> = Arc::new(InMemoryApiKeyStore::new());
    let alice_api_key_plaintext = provision_api_key(&*api_key_store, ALICE_ID).await;

    // -- Auth + account routers --------------------------------------------
    // JwtKeyPair is not Clone, so we build two from the same shared secret:
    // one for the auth middleware to validate, one for us to sign with.
    let jwt_validate = Arc::new(JwtManager::new(
        JwtKeyPair::from_secret(JWT_SECRET),
        "https://test.local".into(),
        "https://test.local".into(),
    ));
    let jwt_sign = JwtManager::new(
        JwtKeyPair::from_secret(JWT_SECRET),
        "https://test.local".into(),
        "https://test.local".into(),
    );

    let auth_config = WebAuthConfig::new(jwt_validate, api_key_store, None, None, false);

    let account_state = AccountApiState {
        org_storage: layer.clone(),
        refresh_token_store: refresh_store.clone() as Arc<dyn RefreshTokenStore>,
    };

    let app = Router::new()
        .nest("/api", account_router().with_state(account_state))
        .route_layer(from_fn_with_state(auth_config, require_auth));

    // Sign alice's JWT — this is what the "logged-in client" carries.
    let alice_jwt = jwt_sign.sign(&alice_ctx(), ORG_SLUG, "Alice").unwrap();

    TestEnv {
        app,
        alice_jwt,
        alice_api_key_plaintext,
        layer,
        refresh_store,
    }
}

fn alice_ctx() -> AuthContext {
    let mut space_roles = HashMap::new();
    space_roles.insert(SpaceId::from("default"), Role::Member);
    AuthContext {
        user_id: UserId::from(ALICE_ID),
        org_id: OrgId::from(ORG_ID),
        email: ALICE_INITIAL_EMAIL.into(),
        space_roles,
        scopes: vec![],
        client_id: "integration-test".into(),
    }
}

fn refresh_token(token: &str, user_id: &str) -> StoredRefreshToken {
    StoredRefreshToken {
        token: token.into(),
        user_id: user_id.into(),
        client_id: "integration-test".into(),
        scopes: vec![],
        expires_at: Utc::now() + chrono::Duration::days(30),
        consumed: false,
    }
}

async fn provision_api_key(store: &dyn ApiKeyStore, user_id: &str) -> String {
    let key = generate_key();
    let mut space_roles = HashMap::new();
    space_roles.insert(SpaceId::from("default"), Role::Member);
    let record = ApiKeyRecord {
        id: format!("key_{user_id}"),
        user_id: UserId::from(user_id),
        name: "test-key".into(),
        key_hash: key.key_hash.clone(),
        key_prefix: key.key_prefix.clone(),
        org_id: OrgId::from(ORG_ID),
        space_roles,
        scopes: vec![],
        expires_at: None,
        created_at: Utc::now(),
    };
    store.store_key(record).await.unwrap();
    key.plaintext
}

async fn body_json<T: serde::de::DeserializeOwned>(resp: Response) -> T {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn get_me(bearer: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/api/users/me")
        .header("authorization", format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn full_self_service_account_flow() {
    let env = setup().await;

    // -- 1. GET /api/users/me with bearer JWT --------------------------------
    let resp = env
        .app
        .clone()
        .oneshot(get_me(&env.alice_jwt))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "logged-in /me must succeed");
    let detail: serde_json::Value = body_json(resp).await;
    assert_eq!(detail["email"], ALICE_INITIAL_EMAIL);
    assert_eq!(detail["id"], ALICE_ID);
    assert_eq!(detail["org_id"], ORG_ID);

    // -- 2. PATCH email ------------------------------------------------------
    let resp = env
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/users/me")
                .header("authorization", format!("Bearer {}", env.alice_jwt))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"email": ALICE_NEW_EMAIL})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: UpdateCurrentUserResponse = body_json(resp).await;
    assert_eq!(body.user.email, ALICE_NEW_EMAIL);
    assert_eq!(body.previous_email.as_deref(), Some(ALICE_INITIAL_EMAIL));

    // -- 3. PATCH name -------------------------------------------------------
    let resp = env
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/users/me")
                .header("authorization", format!("Bearer {}", env.alice_jwt))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"name": "Alice Renamed"})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: UpdateCurrentUserResponse = body_json(resp).await;
    assert_eq!(body.user.name, "Alice Renamed");
    assert!(body.previous_email.is_none());

    // -- 4. POST password change --------------------------------------------
    let resp = env
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/users/me/password")
                .header("authorization", format!("Bearer {}", env.alice_jwt))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "current_password": ALICE_INITIAL_PASSWORD,
                        "new_password": ALICE_NEW_PASSWORD,
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "password change should return 204"
    );

    // Persisted hash now verifies the new password.
    let stored = env
        .layer
        .user_store()
        .get_user(&UserId::from(ALICE_ID))
        .await
        .unwrap()
        .unwrap();
    assert!(
        verify_password(ALICE_NEW_PASSWORD, &stored.password_hash).unwrap(),
        "new password must verify against the stored hash"
    );

    // -- 5. Old refresh tokens are revoked, bob's survives ------------------
    assert!(
        env.refresh_store
            .get_token(ALICE_REFRESH_TOKEN_A)
            .await
            .unwrap()
            .is_none(),
        "alice's first refresh token should be revoked"
    );
    assert!(
        env.refresh_store
            .get_token(ALICE_REFRESH_TOKEN_B)
            .await
            .unwrap()
            .is_none(),
        "alice's second refresh token should be revoked"
    );
    assert!(
        env.refresh_store
            .get_token(BOB_REFRESH_TOKEN)
            .await
            .unwrap()
            .is_some(),
        "bob's refresh token must NOT be touched by alice's password change"
    );

    // -- 6. Calling JWT continues to work (until natural expiry) ------------
    let resp = env
        .app
        .clone()
        .oneshot(get_me(&env.alice_jwt))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "the JWT used to call change-password must keep working"
    );
    let detail: serde_json::Value = body_json(resp).await;
    assert_eq!(
        detail["email"], ALICE_NEW_EMAIL,
        "JWT continues to map to the same user — now showing the updated email"
    );

    // -- 7. API keys still work after password change -----------------------
    let resp = env
        .app
        .clone()
        .oneshot(get_me(&env.alice_api_key_plaintext))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "API key auth must survive a password change"
    );
    let detail: serde_json::Value = body_json(resp).await;
    assert_eq!(detail["id"], ALICE_ID);
    assert_eq!(detail["email"], ALICE_NEW_EMAIL);
}

#[tokio::test]
async fn unauthenticated_request_is_rejected() {
    let env = setup().await;

    // No Authorization header at all.
    let resp = env
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Garbage bearer.
    let resp = env
        .app
        .clone()
        .oneshot(get_me("not-a-real-token"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
