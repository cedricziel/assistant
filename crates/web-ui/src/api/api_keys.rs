//! REST JSON API for API key management.
//!
//! ## Routes
//!
//! | Method | Path                          | Description            |
//! |--------|-------------------------------|------------------------|
//! | GET    | `/api/users/me/api-keys`      | List own API keys      |
//! | POST   | `/api/users/me/api-keys`      | Create a new API key   |
//! | DELETE | `/api/users/me/api-keys/{id}` | Revoke an API key      |

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

use assistant_auth::api_keys;
use assistant_core::auth::{ApiKeyRecord, ApiKeyStore, AuthContext};
use assistant_core::identity::Scope;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct ApiKeysApiState {
    pub api_key_store: Arc<dyn ApiKeyStore>,
}

// -- Request/response types --------------------------------------------------

/// Summary of an API key (no secret).
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiKeySummary {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

/// Body for `POST /api/users/me/api-keys`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    /// Scopes in `"resource:action"` format.
    pub scopes: Option<Vec<String>>,
}

/// Response for `POST /api/users/me/api-keys` — includes plaintext key (shown once).
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    /// The full API key — shown only on creation.
    pub plaintext: String,
    pub scopes: Vec<String>,
}

// -- Router ------------------------------------------------------------------

pub fn api_keys_router() -> Router<ApiKeysApiState> {
    Router::new()
        .route(
            "/users/me/api-keys",
            get(list_api_keys).post(create_api_key),
        )
        .route(
            "/users/me/api-keys/{id}",
            axum::routing::delete(delete_api_key),
        )
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/users/me/api-keys` — list the caller's API keys.
#[utoipa::path(
    get,
    path = "/api/users/me/api-keys",
    tag = "api-keys",
    security(("bearer_token" = []), ("oauth2" = ["api_keys:read"])),
    responses(
        (status = 200, description = "List of API keys", body = Vec<ApiKeySummary>),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn list_api_keys(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<ApiKeysApiState>,
) -> Response {
    match state.api_key_store.list_for_user(&ctx.user_id).await {
        Ok(keys) => {
            let summaries: Vec<ApiKeySummary> = keys
                .into_iter()
                .map(|k| ApiKeySummary {
                    id: k.id,
                    name: k.name,
                    key_prefix: k.key_prefix,
                    scopes: k
                        .scopes
                        .iter()
                        .map(|s| format!("{}:{}", s.resource, s.action))
                        .collect(),
                    created_at: k.created_at.to_rfc3339(),
                    expires_at: k.expires_at.map(|dt| dt.to_rfc3339()),
                })
                .collect();
            Json(summaries).into_response()
        }
        Err(e) => {
            tracing::error!("failed to list API keys: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
        }
    }
}

/// `POST /api/users/me/api-keys` — create a new scoped API key.
#[utoipa::path(
    post,
    path = "/api/users/me/api-keys",
    tag = "api-keys",
    security(("bearer_token" = []), ("oauth2" = ["api_keys:write"])),
    request_body = CreateApiKeyRequest,
    responses(
        (status = 201, description = "API key created (plaintext shown once)", body = CreateApiKeyResponse),
        (status = 400, description = "Invalid request"),
    )
)]
pub async fn create_api_key(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<ApiKeysApiState>,
    Json(body): Json<CreateApiKeyRequest>,
) -> Response {
    if body.name.is_empty() {
        return (StatusCode::BAD_REQUEST, "name is required").into_response();
    }

    let generated = api_keys::generate_key();

    let scopes: Vec<Scope> = match body
        .scopes
        .unwrap_or_default()
        .iter()
        .map(|s| {
            let (r, a) = s
                .split_once(':')
                .ok_or_else(|| format!("invalid scope format: {s}"))?;
            let resource = r
                .parse()
                .map_err(|_| format!("invalid resource in scope: {s}"))?;
            let action = a
                .parse()
                .map_err(|_| format!("invalid action in scope: {s}"))?;
            Ok::<_, String>(Scope::new(resource, action))
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(v) => v,
        Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
    };

    let key_id = format!("key_{}", uuid::Uuid::new_v4());
    let record = ApiKeyRecord {
        id: key_id.clone(),
        user_id: ctx.user_id.clone(),
        name: body.name.clone(),
        key_hash: generated.key_hash.clone(),
        key_prefix: generated.key_prefix.clone(),
        org_id: ctx.org_id.clone(),
        space_roles: ctx.space_roles.clone(),
        scopes: scopes.clone(),
        expires_at: None,
        created_at: SystemClock.now(),
    };

    if let Err(e) = state.api_key_store.store_key(record).await {
        tracing::error!("failed to store API key: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to create API key",
        )
            .into_response();
    }

    let resp = CreateApiKeyResponse {
        id: key_id,
        name: body.name,
        key_prefix: generated.key_prefix,
        plaintext: generated.plaintext,
        scopes: scopes
            .iter()
            .map(|s| format!("{}:{}", s.resource, s.action))
            .collect(),
    };
    (StatusCode::CREATED, Json(resp)).into_response()
}

/// `DELETE /api/users/me/api-keys/{id}` — revoke an API key.
#[utoipa::path(
    delete,
    path = "/api/users/me/api-keys/{id}",
    tag = "api-keys",
    security(("bearer_token" = []), ("oauth2" = ["api_keys:write"])),
    params(("id" = String, Path, description = "API key ID")),
    responses(
        (status = 204, description = "API key revoked"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn delete_api_key(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<ApiKeysApiState>,
    Path(id): Path<String>,
) -> Response {
    match state.api_key_store.revoke(&id, &ctx.user_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "API key not found").into_response(),
        Err(e) => {
            tracing::error!("failed to revoke API key: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "failed to revoke key").into_response()
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

    use assistant_auth::api_keys::InMemoryApiKeyStore;
    use assistant_core::identity::{OrgId, Role, SpaceId, UserId};

    fn make_user_ctx() -> AuthContext {
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

    fn test_app(state: ApiKeysApiState, ctx: AuthContext) -> Router {
        Router::new()
            .merge(api_keys_router().with_state(state))
            .layer(Extension(ctx))
    }

    fn setup_state() -> ApiKeysApiState {
        ApiKeysApiState {
            api_key_store: Arc::new(InMemoryApiKeyStore::new()),
        }
    }

    #[tokio::test]
    async fn create_and_list_api_keys() {
        let state = setup_state();
        let app = test_app(state.clone(), make_user_ctx());

        // Create
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/users/me/api-keys")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_string(&serde_json::json!({
                            "name": "CI Key"
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
        let created: CreateApiKeyResponse = serde_json::from_slice(&body).unwrap();
        assert!(created.plaintext.starts_with("ask_live_"));

        // List
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/users/me/api-keys")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let keys: Vec<ApiKeySummary> = serde_json::from_slice(&body).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "CI Key");
    }

    #[tokio::test]
    async fn revoke_api_key() {
        let state = setup_state();
        let ctx = make_user_ctx();

        // Seed a key.
        let generated = api_keys::generate_key();
        let record = ApiKeyRecord {
            id: "key_123".into(),
            user_id: ctx.user_id.clone(),
            name: "Test".into(),
            key_hash: generated.key_hash,
            key_prefix: generated.key_prefix,
            org_id: ctx.org_id.clone(),
            space_roles: ctx.space_roles.clone(),
            scopes: vec![],
            expires_at: None,
            created_at: chrono::Utc::now(),
        };
        state.api_key_store.store_key(record).await.unwrap();

        let app = test_app(state.clone(), ctx.clone());

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/users/me/api-keys/key_123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // Verify it's gone.
        let keys = state
            .api_key_store
            .list_for_user(&ctx.user_id)
            .await
            .unwrap();
        assert!(keys.is_empty());
    }
}
