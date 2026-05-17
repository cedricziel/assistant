//! REST JSON API for user management.
//!
//! ## Routes
//!
//! | Method | Path                             | Description     |
//! |--------|----------------------------------|-----------------|
//! | GET    | `/api/orgs/{org_id}/users`       | List users      |
//! | POST   | `/api/orgs/{org_id}/users`       | Create user     |
//! | GET    | `/api/orgs/{org_id}/users/{id}`  | Get user        |
//! | PATCH  | `/api/orgs/{org_id}/users/{id}`  | Update user     |
//! | DELETE | `/api/orgs/{org_id}/users/{id}`  | Delete user     |

use assistant_core::clock::{Clock, SystemClock};
use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use assistant_core::UserStore;
use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, UserId};
use assistant_core::store::User;
use assistant_storage::OrgStorageLayer;

use crate::errors::json_error;
use crate::openapi::ErrorBody;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct UsersApiState {
    pub org_storage: Arc<OrgStorageLayer>,
}

// -- Request/response types --------------------------------------------------

/// Summary of a user (no sensitive fields).
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSummary {
    pub id: String,
    pub email: String,
    pub name: String,
}

/// Full user detail (no password hash).
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserDetail {
    pub id: String,
    pub org_id: String,
    pub email: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Body for `POST /api/orgs/{org_id}/users`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    /// Initial password (required in password auth mode).
    pub password: Option<String>,
}

/// Body for `PATCH /api/orgs/{org_id}/users/{id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

// -- Router ------------------------------------------------------------------

pub fn users_api_router() -> Router<UsersApiState> {
    Router::new()
        .route("/orgs/{org_id}/users", get(list_users).post(create_user))
        .route(
            "/orgs/{org_id}/users/{id}",
            get(get_user).patch(update_user).delete(delete_user),
        )
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/orgs/{org_id}/users` — list users in an org.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/users",
    tag = "users",
    security(("bearer_token" = []), ("oauth2" = ["users:read"])),
    params(("org_id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "List of users", body = Vec<UserSummary>),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
    )
)]
pub async fn list_users(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<UsersApiState>,
    Path(org_id): Path<String>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "org admin required");
    }
    let store = state.org_storage.user_store();
    match store.list_users(&org_id).await {
        Ok(users) => {
            let summaries: Vec<UserSummary> = users
                .into_iter()
                .map(|u| UserSummary {
                    id: u.id.0,
                    email: u.email,
                    name: u.name,
                })
                .collect();
            Json(summaries).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list users: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `POST /api/orgs/{org_id}/users` — create/invite a user.
#[utoipa::path(
    post,
    path = "/api/orgs/{org_id}/users",
    tag = "users",
    security(("bearer_token" = []), ("oauth2" = ["users:write"])),
    params(("org_id" = String, Path, description = "Organization ID")),
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = UserDetail),
        (status = 400, description = "Invalid request", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 409, description = "Duplicate email", body = ErrorBody),
    )
)]
pub async fn create_user(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<UsersApiState>,
    Path(org_id): Path<String>,
    Json(body): Json<CreateUserRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "org admin required");
    }

    if body.email.is_empty() || body.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "email and name are required");
    }

    // Check for duplicate email in this org.
    let store = state.org_storage.user_store();
    match store.get_user_by_email(&org_id, &body.email).await {
        Ok(Some(_)) => {
            return json_error(StatusCode::CONFLICT, "email already exists in this org");
        }
        Ok(None) => {}
        Err(e) => {
            tracing::error!("duplicate check failed: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    }

    // Hash password if provided.
    let password_hash = match body.password {
        Some(ref pw) if !pw.is_empty() => match assistant_auth::password::hash_password(pw) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("password hashing failed: {e}");
                return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
            }
        },
        _ => String::new(),
    };

    let now = SystemClock.now();
    let user = User {
        id: UserId::from(format!("usr_{}", uuid::Uuid::new_v4())),
        org_id,
        email: body.email,
        name: body.name,
        password_hash,
        idp_issuer: None,
        idp_subject: None,
        created_at: now,
        updated_at: now,
    };

    if let Err(e) = store.create_user(&user).await {
        tracing::error!("failed to create user: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to create user");
    }

    let detail = UserDetail {
        id: user.id.0,
        org_id: user.org_id.0,
        email: user.email,
        name: user.name,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };
    (StatusCode::CREATED, Json(detail)).into_response()
}

/// `GET /api/orgs/{org_id}/users/{id}` — get user detail.
#[utoipa::path(
    get,
    path = "/api/orgs/{org_id}/users/{id}",
    tag = "users",
    security(("bearer_token" = []), ("oauth2" = ["users:read"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "User ID"),
    ),
    responses(
        (status = 200, description = "User detail", body = UserDetail),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn get_user(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<UsersApiState>,
    Path((org_id, id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let user_id = UserId::from(id);

    // Users can see their own profile; org admins can see anyone in the same org.
    if ctx.user_id != user_id && !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let store = state.org_storage.user_store();
    match store.get_user(&user_id).await {
        Ok(Some(user)) => {
            let detail = UserDetail {
                id: user.id.0,
                org_id: user.org_id.0,
                email: user.email,
                name: user.name,
                created_at: user.created_at.to_rfc3339(),
                updated_at: user.updated_at.to_rfc3339(),
            };
            Json(detail).into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "user not found"),
        Err(e) => {
            tracing::error!("failed to get user: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

/// `PATCH /api/orgs/{org_id}/users/{id}` — update user name or email.
#[utoipa::path(
    patch,
    path = "/api/orgs/{org_id}/users/{id}",
    tag = "users",
    security(("bearer_token" = []), ("oauth2" = ["users:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "User ID"),
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = UserDetail),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn update_user(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<UsersApiState>,
    Path((org_id, id)): Path<(String, String)>,
    Json(body): Json<UpdateUserRequest>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let user_id = UserId::from(id);

    // Users can update their own profile; org admins can update anyone in the same org.
    if ctx.user_id != user_id && !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }

    let store = state.org_storage.user_store();
    let mut user = match store.get_user(&user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "user not found"),
        Err(e) => {
            tracing::error!("failed to get user: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error");
        }
    };

    if let Some(name) = body.name {
        user.name = name;
    }
    if let Some(email) = body.email {
        user.email = email;
    }
    user.updated_at = SystemClock.now();

    if let Err(e) = store.update_user(&user).await {
        tracing::error!("failed to update user: {e}");
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to update user");
    }

    let detail = UserDetail {
        id: user.id.0,
        org_id: user.org_id.0,
        email: user.email,
        name: user.name,
        created_at: user.created_at.to_rfc3339(),
        updated_at: user.updated_at.to_rfc3339(),
    };
    Json(detail).into_response()
}

/// `DELETE /api/orgs/{org_id}/users/{id}` — remove a user (org-admin only).
#[utoipa::path(
    delete,
    path = "/api/orgs/{org_id}/users/{id}",
    tag = "users",
    security(("bearer_token" = []), ("oauth2" = ["users:write"])),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("id" = String, Path, description = "User ID"),
    ),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody),
    )
)]
pub async fn delete_user(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<UsersApiState>,
    Path((org_id, id)): Path<(String, String)>,
) -> Response {
    let org_id = OrgId::from(org_id);
    if ctx.org_id != org_id {
        return json_error(StatusCode::FORBIDDEN, "access denied");
    }
    if !ctx.is_org_admin() {
        return json_error(StatusCode::FORBIDDEN, "org admin required");
    }

    let user_id = UserId::from(id);
    let store = state.org_storage.user_store();

    match store.delete_user(&user_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "user not found"),
        Err(e) => {
            tracing::error!("failed to delete user: {e}");
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "failed to delete user")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::collections::HashMap;
    use tower::ServiceExt;

    use assistant_core::OrgStore;
    use assistant_core::identity::{Role, SpaceId};
    use assistant_core::store::Organization;

    fn make_admin_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("default"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("org_1"),
            email: "admin@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn make_member_ctx() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        AuthContext {
            user_id: UserId::from("alice"),
            org_id: OrgId::from("org_1"),
            email: "alice@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    fn test_app(state: UsersApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(users_api_router().with_state(state))
            .layer(Extension(ctx))
    }

    async fn setup_state() -> UsersApiState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        // Seed the org so FK constraints on users.org_id are satisfied.
        let org = Organization {
            id: OrgId::from("org_1"),
            name: "Test Org".into(),
            slug: "test".into(),
            auth_mode: "password".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        org_storage.org_store().create_org(&org).await.unwrap();
        UsersApiState { org_storage }
    }

    #[tokio::test]
    async fn create_and_list_users() {
        let state = setup_state().await;
        let app = test_app(state.clone(), make_admin_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "email": "bob@acme.com",
                            "name": "Bob",
                            "password": "secret123"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let detail: UserDetail = serde_json::from_slice(&body).unwrap();
        assert_eq!(detail.email, "bob@acme.com");

        // List
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let users: Vec<UserSummary> = serde_json::from_slice(&body).unwrap();
        assert_eq!(users.len(), 1);
    }

    #[tokio::test]
    async fn member_cannot_list_users() {
        let state = setup_state().await;
        let app = test_app(state, make_member_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/orgs/org_1/users")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn duplicate_email_returns_409() {
        let state = setup_state().await;

        // Seed a user.
        let now = chrono::Utc::now();
        let user = User {
            id: UserId::from("usr_existing"),
            org_id: OrgId::from("org_1"),
            email: "dupe@acme.com".into(),
            name: "Existing".into(),
            password_hash: String::new(),
            idp_issuer: None,
            idp_subject: None,
            created_at: now,
            updated_at: now,
        };
        state
            .org_storage
            .user_store()
            .create_user(&user)
            .await
            .unwrap();

        let app = test_app(state, make_admin_ctx());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/orgs/org_1/users")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "email": "dupe@acme.com",
                            "name": "Duplicate"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn delete_user_requires_admin() {
        let state = setup_state().await;

        // Seed a user.
        let now = chrono::Utc::now();
        let user = User {
            id: UserId::from("usr_del"),
            org_id: OrgId::from("org_1"),
            email: "del@acme.com".into(),
            name: "Del".into(),
            password_hash: String::new(),
            idp_issuer: None,
            idp_subject: None,
            created_at: now,
            updated_at: now,
        };
        state
            .org_storage
            .user_store()
            .create_user(&user)
            .await
            .unwrap();

        // Member cannot delete.
        let app = test_app(state.clone(), make_member_ctx());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/users/usr_del")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // Admin can delete.
        let app = test_app(state, make_admin_ctx());
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/orgs/org_1/users/usr_del")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
}
