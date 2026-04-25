//! REST JSON API for the caller's own account ("me" endpoints).
//!
//! ## Routes
//!
//! | Method | Path                          | Description              |
//! |--------|-------------------------------|--------------------------|
//! | GET    | `/api/users/me`               | Get own user record      |
//! | PATCH  | `/api/users/me`               | Update own name / email  |
//! | POST   | `/api/users/me/password`      | Change own password      |
//!
//! See `openspec/changes/self-service-account/specs/self-service-account/spec.md`
//! for the full requirement and scenario list.

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use assistant_auth::oauth2::server::RefreshTokenStore;
use assistant_auth::password;
use assistant_core::auth::AuthContext;
use assistant_core::store::Organization;
use assistant_core::{OrgStore, UserStore};
use assistant_storage::OrgStorageLayer;

use super::users::UserDetail;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct AccountApiState {
    pub org_storage: Arc<OrgStorageLayer>,
    pub refresh_token_store: Arc<dyn RefreshTokenStore>,
}

// -- Request / response types ------------------------------------------------

/// Body for `PATCH /api/users/me`. All fields optional; an empty body is a
/// no-op that returns the current `UserDetail`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateCurrentUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Response for `PATCH /api/users/me`. Contains the (possibly updated)
/// `UserDetail` plus a `previous_email` field present only when the email
/// actually changed.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateCurrentUserResponse {
    #[serde(flatten)]
    pub user: UserDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_email: Option<String>,
}

/// Body for `POST /api/users/me/password`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
struct ErrorBody {
    error: String,
}

// -- Router ------------------------------------------------------------------

pub fn account_router() -> Router<AccountApiState> {
    Router::new()
        .route(
            "/users/me",
            get(get_current_user).patch(update_current_user),
        )
        .route("/users/me/password", post(change_password))
}

// -- Helpers -----------------------------------------------------------------

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ErrorBody { error: msg.into() })).into_response()
}

fn user_detail(user: &assistant_core::store::User) -> UserDetail {
    UserDetail {
        id: user.id.0.clone(),
        org_id: user.org_id.0.clone(),
        email: user.email.clone(),
        name: user.name.clone(),
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    }
}

/// Returns `Some(org)` if the caller's org exists, otherwise `None`.
async fn load_org(state: &AccountApiState, ctx: &AuthContext) -> Result<Option<Organization>, ()> {
    state
        .org_storage
        .org_store()
        .get_org(&ctx.org_id)
        .await
        .map_err(|e| {
            tracing::error!("failed to load org for self-service: {e}");
        })
}

/// If the caller's org runs in OIDC mode, returns `Err(response)` with a 409
/// pointing the caller at their identity provider. Otherwise `Ok(())`.
async fn require_password_mode(state: &AccountApiState, ctx: &AuthContext) -> Result<(), Response> {
    let org = match load_org(state, ctx).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            return Err(err(StatusCode::INTERNAL_SERVER_ERROR, "org not found"));
        }
        Err(()) => {
            return Err(err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ));
        }
    };
    if org.auth_mode == "oidc" {
        // Use the IdP issuer if any of the org's users have one stamped on
        // them; otherwise fall back to a generic phrase. Looking up the
        // caller's own user is sufficient for the common case where every
        // user in the org was provisioned by the same IdP.
        let issuer = state
            .org_storage
            .user_store()
            .get_user(&ctx.user_id)
            .await
            .ok()
            .flatten()
            .and_then(|u| u.idp_issuer)
            .unwrap_or_else(|| "your identity provider".to_string());
        return Err(err(
            StatusCode::CONFLICT,
            format!("account managed by identity provider {issuer}"),
        ));
    }
    Ok(())
}

fn validate_email(email: &str) -> Result<(), String> {
    if email.is_empty() {
        return Err("email must not be empty".into());
    }
    if !email.contains('@') {
        return Err("email must contain '@'".into());
    }
    Ok(())
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/users/me` — return the caller's `UserDetail`. Works in any
/// `auth_mode`.
#[utoipa::path(
    get,
    path = "/api/users/me",
    tag = "account",
    operation_id = "get_current_user",
    security(("bearer_token" = []), ("oauth2" = ["users:read"])),
    responses(
        (status = 200, description = "Current user detail", body = UserDetail),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn get_current_user(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<AccountApiState>,
) -> Response {
    match state.org_storage.user_store().get_user(&ctx.user_id).await {
        Ok(Some(user)) => Json(user_detail(&user)).into_response(),
        Ok(None) => err(StatusCode::NOT_FOUND, "user not found"),
        Err(e) => {
            tracing::error!("failed to get current user: {e}");
            err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `PATCH /api/users/me` — update the caller's name and/or email.
#[utoipa::path(
    patch,
    path = "/api/users/me",
    tag = "account",
    operation_id = "update_current_user",
    security(("bearer_token" = []), ("oauth2" = ["users:write"])),
    request_body = UpdateCurrentUserRequest,
    responses(
        (status = 200, description = "Updated user", body = UpdateCurrentUserResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Email collision or OIDC-managed account"),
    )
)]
pub async fn update_current_user(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<AccountApiState>,
    Json(body): Json<UpdateCurrentUserRequest>,
) -> Response {
    if let Err(resp) = require_password_mode(&state, &ctx).await {
        return resp;
    }

    let user_store = state.org_storage.user_store();
    let mut user = match user_store.get_user(&ctx.user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return err(StatusCode::NOT_FOUND, "user not found"),
        Err(e) => {
            tracing::error!("failed to load user for update: {e}");
            return err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    };

    let previous_email = user.email.clone();
    let mut email_changed = false;

    if let Some(name) = body.name {
        user.name = name;
    }
    if let Some(new_email) = body.email {
        if let Err(reason) = validate_email(&new_email) {
            return err(StatusCode::BAD_REQUEST, reason);
        }
        if new_email != previous_email {
            // Collision check: another user in the same org has this email?
            match user_store.get_user_by_email(&ctx.org_id, &new_email).await {
                Ok(Some(existing)) if existing.id != user.id => {
                    return err(StatusCode::CONFLICT, "email already exists in this org");
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("email collision lookup failed: {e}");
                    return err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
                }
            }
            user.email = new_email;
            email_changed = true;
        }
    }

    user.updated_at = Utc::now();
    if let Err(e) = user_store.update_user(&user).await {
        tracing::error!("failed to update user: {e}");
        return err(StatusCode::INTERNAL_SERVER_ERROR, "failed to update user");
    }

    let resp = UpdateCurrentUserResponse {
        user: user_detail(&user),
        previous_email: email_changed.then_some(previous_email),
    };
    Json(resp).into_response()
}

/// `POST /api/users/me/password` — change the caller's password and revoke
/// all their refresh tokens. The current access JWT keeps working until it
/// naturally expires; API keys are untouched.
#[utoipa::path(
    post,
    path = "/api/users/me/password",
    tag = "account",
    operation_id = "change_password",
    security(("bearer_token" = []), ("oauth2" = ["users:write"])),
    request_body = ChangePasswordRequest,
    responses(
        (status = 204, description = "Password changed"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Current password incorrect or unauthorized"),
        (status = 409, description = "OIDC-managed account"),
    )
)]
pub async fn change_password(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<AccountApiState>,
    Json(body): Json<ChangePasswordRequest>,
) -> Response {
    if let Err(resp) = require_password_mode(&state, &ctx).await {
        return resp;
    }

    if body.new_password.is_empty() {
        return err(StatusCode::BAD_REQUEST, "new password must not be empty");
    }

    let user_store = state.org_storage.user_store();
    let mut user = match user_store.get_user(&ctx.user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return err(StatusCode::NOT_FOUND, "user not found"),
        Err(e) => {
            tracing::error!("failed to load user for password change: {e}");
            return err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    };

    match password::verify_password(&body.current_password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => return err(StatusCode::UNAUTHORIZED, "current password is incorrect"),
        Err(e) => {
            tracing::error!("password verification failed: {e}");
            return err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    }

    let new_hash = match password::hash_password(&body.new_password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("password hashing failed: {e}");
            return err(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    };

    user.password_hash = new_hash;
    user.updated_at = Utc::now();
    if let Err(e) = user_store.update_user(&user).await {
        tracing::error!("failed to persist new password: {e}");
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to update password",
        );
    }

    // Revoke every refresh token for this user. The empty-string sentinel
    // ensures no real token is excepted (no token's value is the empty
    // string), so all sessions get logged out on next refresh.
    if let Err(e) = state
        .refresh_token_store
        .revoke_for_user_except(&ctx.user_id.0, "")
        .await
    {
        tracing::error!("failed to revoke refresh tokens: {e}");
        // The password is already changed; do not roll back. Surface a 500
        // so the client knows something went sideways with sessions, but the
        // password is committed.
        return err(
            StatusCode::INTERNAL_SERVER_ERROR,
            "password changed but session revocation failed",
        );
    }

    StatusCode::NO_CONTENT.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::collections::HashMap;
    use tower::ServiceExt;

    use assistant_auth::oauth2::server::{InMemoryRefreshTokenStore, StoredRefreshToken};
    use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
    use assistant_core::store::User;

    const ORG_ID: &str = "org_1";
    const USER_ID: &str = "alice";
    const PASSWORD: &str = "hunter2";
    const ALT_USER_ID: &str = "bob";

    fn ctx_for(user_id: &str, email: &str) -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("default"), Role::Member);
        AuthContext {
            user_id: UserId::from(user_id),
            org_id: OrgId::from(ORG_ID),
            email: email.into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    async fn seed_org(layer: &OrgStorageLayer, auth_mode: &str) {
        let org = Organization {
            id: OrgId::from(ORG_ID),
            name: "Test Org".into(),
            slug: "test".into(),
            auth_mode: auth_mode.into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        layer.org_store().create_org(&org).await.unwrap();
    }

    async fn seed_user(layer: &OrgStorageLayer, id: &str, email: &str, password: &str) {
        let now = Utc::now();
        let hash = if password.is_empty() {
            String::new()
        } else {
            password::hash_password(password).unwrap()
        };
        let user = User {
            id: UserId::from(id),
            org_id: OrgId::from(ORG_ID),
            email: email.into(),
            name: id.into(),
            password_hash: hash,
            idp_issuer: None,
            idp_subject: None,
            created_at: now,
            updated_at: now,
        };
        layer.user_store().create_user(&user).await.unwrap();
    }

    async fn seed_oidc_user(layer: &OrgStorageLayer, id: &str, email: &str, issuer: &str) {
        let now = Utc::now();
        let user = User {
            id: UserId::from(id),
            org_id: OrgId::from(ORG_ID),
            email: email.into(),
            name: id.into(),
            password_hash: String::new(),
            idp_issuer: Some(issuer.into()),
            idp_subject: Some("subject".into()),
            created_at: now,
            updated_at: now,
        };
        layer.user_store().create_user(&user).await.unwrap();
    }

    async fn setup_password_state() -> AccountApiState {
        let layer = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        seed_org(&layer, "password").await;
        seed_user(&layer, USER_ID, "alice@test.local", PASSWORD).await;
        AccountApiState {
            org_storage: layer,
            refresh_token_store: Arc::new(InMemoryRefreshTokenStore::default()),
        }
    }

    async fn setup_oidc_state(issuer: &str) -> AccountApiState {
        let layer = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        seed_org(&layer, "oidc").await;
        seed_oidc_user(&layer, USER_ID, "alice@idp.local", issuer).await;
        AccountApiState {
            org_storage: layer,
            refresh_token_store: Arc::new(InMemoryRefreshTokenStore::default()),
        }
    }

    fn test_app(state: AccountApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(account_router().with_state(state))
            .layer(Extension(ctx))
    }

    fn json_body<T: serde::Serialize>(value: &T) -> Body {
        Body::from(serde_json::to_vec(value).unwrap())
    }

    async fn body_json<T: serde::de::DeserializeOwned>(resp: Response) -> T {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // -- GET /api/users/me ---------------------------------------------------

    #[tokio::test]
    async fn get_current_user_returns_profile() {
        let state = setup_password_state().await;
        let app = test_app(state, ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/users/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let detail: UserDetail = body_json(resp).await;
        assert_eq!(detail.id, USER_ID);
        assert_eq!(detail.org_id, ORG_ID);
        assert_eq!(detail.email, "alice@test.local");
    }

    #[tokio::test]
    async fn get_current_user_works_for_oidc_org() {
        let state = setup_oidc_state("https://idp.example/").await;
        let app = test_app(state, ctx_for(USER_ID, "alice@idp.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/users/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let detail: UserDetail = body_json(resp).await;
        assert_eq!(detail.email, "alice@idp.local");
    }

    // -- PATCH /api/users/me -------------------------------------------------

    #[tokio::test]
    async fn patch_updates_name() {
        let state = setup_password_state().await;
        let app = test_app(state.clone(), ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/users/me")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({"name": "Alice Updated"})))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: UpdateCurrentUserResponse = body_json(resp).await;
        assert_eq!(body.user.name, "Alice Updated");
        assert!(body.previous_email.is_none());

        let stored = state
            .org_storage
            .user_store()
            .get_user(&UserId::from(USER_ID))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.name, "Alice Updated");
    }

    #[tokio::test]
    async fn patch_updates_email_and_returns_previous() {
        let state = setup_password_state().await;
        let app = test_app(state.clone(), ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/users/me")
                    .header("content-type", "application/json")
                    .body(json_body(
                        &serde_json::json!({"email": "alice2@test.local"}),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: UpdateCurrentUserResponse = body_json(resp).await;
        assert_eq!(body.user.email, "alice2@test.local");
        assert_eq!(body.previous_email.as_deref(), Some("alice@test.local"));
    }

    #[tokio::test]
    async fn patch_email_collision_returns_409() {
        let state = setup_password_state().await;
        seed_user(&state.org_storage, ALT_USER_ID, "bob@test.local", "pw").await;
        let app = test_app(state, ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/users/me")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({"email": "bob@test.local"})))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body: ErrorBody = body_json(resp).await;
        assert_eq!(body.error, "email already exists in this org");
    }

    #[tokio::test]
    async fn patch_empty_body_is_noop() {
        let state = setup_password_state().await;
        let app = test_app(state, ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/users/me")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({})))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: UpdateCurrentUserResponse = body_json(resp).await;
        assert_eq!(body.user.email, "alice@test.local");
        assert!(body.previous_email.is_none());
    }

    #[tokio::test]
    async fn patch_invalid_email_rejected() {
        let state = setup_password_state().await;
        let app = test_app(state, ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/users/me")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({"email": "not-an-email"})))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let app = test_app(setup_password_state().await, ctx_for(USER_ID, "x"));
        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/users/me")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({"email": ""})))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn patch_oidc_org_returns_409() {
        let state = setup_oidc_state("https://idp.example/").await;
        let app = test_app(state, ctx_for(USER_ID, "alice@idp.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/users/me")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({"name": "blocked"})))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body: ErrorBody = body_json(resp).await;
        assert!(
            body.error.contains("https://idp.example/"),
            "error should name the issuer: {}",
            body.error
        );
        assert!(
            body.error
                .starts_with("account managed by identity provider")
        );
    }

    // -- POST /api/users/me/password ----------------------------------------

    fn refresh_token(token: &str, user_id: &str) -> StoredRefreshToken {
        StoredRefreshToken {
            token: token.into(),
            user_id: user_id.into(),
            client_id: "test".into(),
            scopes: vec![],
            expires_at: Utc::now() + chrono::Duration::days(30),
            consumed: false,
        }
    }

    #[tokio::test]
    async fn change_password_success_revokes_all_refresh_tokens() {
        let state = setup_password_state().await;
        // Seed two refresh tokens for alice and one for bob.
        seed_user(&state.org_storage, ALT_USER_ID, "bob@test.local", "x").await;
        for tok in ["alice-tok-1", "alice-tok-2"] {
            state
                .refresh_token_store
                .store_token(refresh_token(tok, USER_ID))
                .await
                .unwrap();
        }
        state
            .refresh_token_store
            .store_token(refresh_token("bob-tok", ALT_USER_ID))
            .await
            .unwrap();

        let app = test_app(state.clone(), ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users/me/password")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({
                        "current_password": PASSWORD,
                        "new_password": "freshpass"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Hash updated (and we can verify the new password).
        let user = state
            .org_storage
            .user_store()
            .get_user(&UserId::from(USER_ID))
            .await
            .unwrap()
            .unwrap();
        assert!(password::verify_password("freshpass", &user.password_hash).unwrap());

        // Alice's refresh tokens are gone, bob's survives.
        assert!(
            state
                .refresh_token_store
                .get_token("alice-tok-1")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            state
                .refresh_token_store
                .get_token("alice-tok-2")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            state
                .refresh_token_store
                .get_token("bob-tok")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn change_password_wrong_current_returns_401() {
        let state = setup_password_state().await;
        let original_hash = state
            .org_storage
            .user_store()
            .get_user(&UserId::from(USER_ID))
            .await
            .unwrap()
            .unwrap()
            .password_hash;
        let app = test_app(state.clone(), ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users/me/password")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({
                        "current_password": "wrong",
                        "new_password": "freshpass"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body: ErrorBody = body_json(resp).await;
        assert_eq!(body.error, "current password is incorrect");

        // Stored hash is unchanged.
        let after = state
            .org_storage
            .user_store()
            .get_user(&UserId::from(USER_ID))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(after.password_hash, original_hash);
    }

    #[tokio::test]
    async fn change_password_empty_new_returns_400() {
        let state = setup_password_state().await;
        let app = test_app(state, ctx_for(USER_ID, "alice@test.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users/me/password")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({
                        "current_password": PASSWORD,
                        "new_password": ""
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body: ErrorBody = body_json(resp).await;
        assert_eq!(body.error, "new password must not be empty");
    }

    #[tokio::test]
    async fn change_password_oidc_org_returns_409() {
        let state = setup_oidc_state("https://idp.example/").await;
        let app = test_app(state, ctx_for(USER_ID, "alice@idp.local"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users/me/password")
                    .header("content-type", "application/json")
                    .body(json_body(&serde_json::json!({
                        "current_password": "anything",
                        "new_password": "newpass"
                    })))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body: ErrorBody = body_json(resp).await;
        assert!(body.error.contains("https://idp.example/"));
    }
}
