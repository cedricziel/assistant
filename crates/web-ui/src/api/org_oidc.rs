//! REST JSON API for an organization's OpenID Connect settings.
//!
//! ## Routes
//!
//! | Method | Path                   | Description                   |
//! |--------|------------------------|-------------------------------|
//! | GET    | `/api/orgs/{id}/oidc`  | Get the org's OIDC settings   |
//! | PUT    | `/api/orgs/{id}/oidc`  | Create or replace them        |
//! | DELETE | `/api/orgs/{id}/oidc`  | Remove them                   |
//!
//! The client secret is write-only: responses report `has_client_secret`.

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Deserializer, Serialize};

use assistant_core::OrgStore;
use assistant_core::auth::AuthContext;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::identity::OrgId;
use assistant_core::store::{OrgOidcConfig, OrgOidcConfigStore};

use super::orgs::OrgsApiState;
use crate::errors::json_error;
use crate::openapi::ErrorBody;

// -- Request/response types --------------------------------------------------

/// An org's OIDC settings as returned by the API. The client secret is never
/// included.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct OrgOidcConfigResponse {
    pub issuer_url: String,
    pub client_id: String,
    /// Whether a client secret is stored.
    pub has_client_secret: bool,
    pub auto_provision: bool,
    /// Email domains allowed to sign in; empty allows any domain.
    pub allowed_email_domains: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Body for `PUT /api/orgs/{id}/oidc`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PutOrgOidcConfigRequest {
    /// `https` URL, or `http` for `localhost` / loopback addresses.
    pub issuer_url: String,
    pub client_id: String,
    /// Omit to keep the stored secret, `null` to clear it.
    #[serde(default, deserialize_with = "present")]
    #[schema(value_type = Option<String>, nullable)]
    pub client_secret: Option<Option<String>>,
    /// Defaults to `true`.
    #[serde(default = "default_true")]
    pub auto_provision: bool,
    #[serde(default)]
    pub allowed_email_domains: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Maps a present field (even `null`) to `Some`, so a missing field stays `None`.
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

impl From<OrgOidcConfig> for OrgOidcConfigResponse {
    fn from(c: OrgOidcConfig) -> Self {
        Self {
            issuer_url: c.issuer_url,
            client_id: c.client_id,
            has_client_secret: c.client_secret.is_some(),
            auto_provision: c.auto_provision,
            allowed_email_domains: c.allowed_email_domains,
            created_at: c.created_at.to_rfc3339(),
            updated_at: c.updated_at.to_rfc3339(),
        }
    }
}

// -- Validation --------------------------------------------------------------

fn validate_issuer_url(raw: &str) -> Result<String, &'static str> {
    let url = url::Url::parse(raw.trim()).map_err(|_| "issuer_url is not a valid URL")?;
    let loopback = matches!(
        url.host(),
        Some(url::Host::Domain("localhost"))
            | Some(url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST))
            | Some(url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST))
    );
    match url.scheme() {
        "https" => {}
        "http" if loopback => {}
        _ => return Err("issuer_url must use https (http is allowed only for localhost)"),
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn normalize_domains(raw: Vec<String>) -> Result<Vec<String>, &'static str> {
    raw.into_iter()
        .map(|d| {
            let d = d.trim().to_ascii_lowercase();
            if d.is_empty() || d.contains('@') || d.contains(char::is_whitespace) {
                Err("allowed_email_domains entries must be bare domains like example.com")
            } else {
                Ok(d)
            }
        })
        .collect()
}

/// Returns an error response unless the caller is an admin of `org_id`.
fn deny_unless_admin(ctx: &AuthContext, org_id: &OrgId) -> Option<Response> {
    if !ctx.is_org_admin() {
        return Some(json_error(StatusCode::FORBIDDEN, "org admin required"));
    }
    if ctx.org_id != *org_id {
        return Some(json_error(StatusCode::FORBIDDEN, "access denied"));
    }
    None
}

fn internal(context: &str, e: anyhow::Error) -> Response {
    tracing::error!("{context}: {e:#}");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

// -- Router ------------------------------------------------------------------

pub fn org_oidc_api_router() -> Router<OrgsApiState> {
    Router::new().route(
        "/orgs/{id}/oidc",
        get(get_org_oidc).put(put_org_oidc).delete(delete_org_oidc),
    )
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/orgs/{id}/oidc` — get the org's OIDC settings.
#[utoipa::path(
    get,
    path = "/api/orgs/{id}/oidc",
    tag = "orgs",
    security(("bearer_token" = []), ("oauth2" = ["org:manage"])),
    params(("id" = String, Path, description = "Organization ID")),
    responses(
        (status = 200, description = "OIDC settings", body = OrgOidcConfigResponse),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "No OIDC settings for this org", body = ErrorBody),
    )
)]
pub async fn get_org_oidc(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<OrgsApiState>,
    Path(id): Path<String>,
) -> Response {
    let org_id = OrgId::from(id);
    if let Some(resp) = deny_unless_admin(&ctx, &org_id) {
        return resp;
    }
    match state
        .org_storage
        .oidc_config_store()
        .get_oidc_config(&org_id)
        .await
    {
        Ok(Some(c)) => Json(OrgOidcConfigResponse::from(c)).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "no OIDC settings for this org"),
        Err(e) => internal("failed to get OIDC config", e),
    }
}

/// `PUT /api/orgs/{id}/oidc` — create or replace the org's OIDC settings.
#[utoipa::path(
    put,
    path = "/api/orgs/{id}/oidc",
    tag = "orgs",
    security(("bearer_token" = []), ("oauth2" = ["org:manage"])),
    params(("id" = String, Path, description = "Organization ID")),
    request_body = PutOrgOidcConfigRequest,
    responses(
        (status = 200, description = "OIDC settings saved", body = OrgOidcConfigResponse),
        (status = 400, description = "Invalid settings", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "Org not found", body = ErrorBody),
    )
)]
pub async fn put_org_oidc(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<OrgsApiState>,
    Path(id): Path<String>,
    Json(body): Json<PutOrgOidcConfigRequest>,
) -> Response {
    let org_id = OrgId::from(id);
    if let Some(resp) = deny_unless_admin(&ctx, &org_id) {
        return resp;
    }
    let issuer_url = match validate_issuer_url(&body.issuer_url) {
        Ok(u) => u,
        Err(msg) => return json_error(StatusCode::BAD_REQUEST, msg),
    };
    let client_id = body.client_id.trim().to_string();
    if client_id.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "client_id is required");
    }
    let allowed_email_domains = match normalize_domains(body.allowed_email_domains) {
        Ok(d) => d,
        Err(msg) => return json_error(StatusCode::BAD_REQUEST, msg),
    };

    match state.org_storage.org_store().get_org(&org_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "org not found"),
        Err(e) => return internal("failed to get org", e),
    }

    let store = state.org_storage.oidc_config_store();
    let existing = match store.get_oidc_config(&org_id).await {
        Ok(c) => c,
        Err(e) => return internal("failed to get OIDC config", e),
    };
    let now = SystemClock.now();
    let client_secret = match body.client_secret {
        Some(secret) => secret.filter(|s| !s.is_empty()),
        None => existing.as_ref().and_then(|c| c.client_secret.clone()),
    };
    let config = OrgOidcConfig {
        org_id,
        issuer_url,
        client_id,
        client_secret,
        auto_provision: body.auto_provision,
        allowed_email_domains,
        created_at: existing.map_or(now, |c| c.created_at),
        updated_at: now,
    };
    if let Err(e) = store.upsert_oidc_config(&config).await {
        return internal("failed to save OIDC config", e);
    }
    Json(OrgOidcConfigResponse::from(config)).into_response()
}

/// `DELETE /api/orgs/{id}/oidc` — remove the org's OIDC settings.
///
/// Refused while the org's `auth_mode` is `"oidc"`.
#[utoipa::path(
    delete,
    path = "/api/orgs/{id}/oidc",
    tag = "orgs",
    security(("bearer_token" = []), ("oauth2" = ["org:manage"])),
    params(("id" = String, Path, description = "Organization ID")),
    responses(
        (status = 204, description = "OIDC settings removed"),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Forbidden", body = ErrorBody),
        (status = 404, description = "No OIDC settings for this org", body = ErrorBody),
        (status = 409, description = "Org still uses OIDC sign-in", body = ErrorBody),
    )
)]
pub async fn delete_org_oidc(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<OrgsApiState>,
    Path(id): Path<String>,
) -> Response {
    let org_id = OrgId::from(id);
    if let Some(resp) = deny_unless_admin(&ctx, &org_id) {
        return resp;
    }
    match state.org_storage.org_store().get_org(&org_id).await {
        Ok(Some(org)) if org.auth_mode == "oidc" => {
            return json_error(
                StatusCode::CONFLICT,
                "org signs in with OIDC; set auth_mode to password first",
            );
        }
        Ok(_) => {}
        Err(e) => return internal("failed to get org", e),
    }
    match state
        .org_storage
        .oidc_config_store()
        .delete_oidc_config(&org_id)
        .await
    {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "no OIDC settings for this org"),
        Err(e) => internal("failed to delete OIDC config", e),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::{Extension, Router};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use assistant_core::OrgStore;
    use assistant_core::auth::AuthContext;
    use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
    use assistant_core::store::{OrgOidcConfigStore, Organization};
    use assistant_storage::OrgStorageLayer;

    use super::*;
    use crate::api::orgs::orgs_api_router;

    fn ctx(role: Role) -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("default"), role);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("org_1"),
            email: "admin@test.local".into(),
            space_roles,
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    async fn setup() -> OrgsApiState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        let now = chrono::Utc::now();
        org_storage
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("org_1"),
                name: "Acme".into(),
                slug: "acme".into(),
                auth_mode: "password".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        OrgsApiState { org_storage }
    }

    fn app(state: OrgsApiState, role: Role) -> Router {
        Router::new()
            .merge(orgs_api_router().with_state(state.clone()))
            .merge(org_oidc_api_router().with_state(state))
            .layer(Extension(ctx(role)))
    }

    async fn send(
        app: &Router,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(uri);
        let body = match body {
            Some(b) => {
                req = req.header("content-type", "application/json");
                Body::from(b.to_string())
            }
            None => Body::empty(),
        };
        let resp = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, json)
    }

    fn valid_body() -> Value {
        json!({
            "issuer_url": "https://idp.acme.com/realms/main",
            "client_id": "assistant",
            "client_secret": "s3cret",
            "auto_provision": false,
            "allowed_email_domains": ["acme.com"]
        })
    }

    #[tokio::test]
    async fn get_without_config_is_404() {
        let app = app(setup().await, Role::OrgAdmin);
        let (status, _) = send(&app, "GET", "/orgs/org_1/oidc", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn put_then_get_hides_secret() {
        let app = app(setup().await, Role::OrgAdmin);
        let (status, put) = send(&app, "PUT", "/orgs/org_1/oidc", Some(valid_body())).await;
        assert_eq!(status, StatusCode::OK, "PUT failed: {put}");

        let (status, got) = send(&app, "GET", "/orgs/org_1/oidc", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(got["issuer_url"], "https://idp.acme.com/realms/main");
        assert_eq!(got["client_id"], "assistant");
        assert_eq!(got["auto_provision"], false);
        assert_eq!(got["allowed_email_domains"], json!(["acme.com"]));
        assert_eq!(got["has_client_secret"], true);
        assert!(
            got.get("client_secret").is_none(),
            "secret must never be returned: {got}"
        );
    }

    #[tokio::test]
    async fn put_without_secret_keeps_existing_and_null_clears_it() {
        let state = setup().await;
        let app = app(state.clone(), Role::OrgAdmin);
        send(&app, "PUT", "/orgs/org_1/oidc", Some(valid_body())).await;

        let mut body = valid_body();
        body.as_object_mut().unwrap().remove("client_secret");
        let (status, got) = send(&app, "PUT", "/orgs/org_1/oidc", Some(body)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(got["has_client_secret"], true, "omitted secret is kept");

        let mut body = valid_body();
        body["client_secret"] = Value::Null;
        let (_, got) = send(&app, "PUT", "/orgs/org_1/oidc", Some(body)).await;
        assert_eq!(got["has_client_secret"], false, "null secret is cleared");

        let stored = state
            .org_storage
            .oidc_config_store()
            .get_oidc_config(&OrgId::from("org_1"))
            .await
            .unwrap()
            .unwrap();
        assert!(stored.client_secret.is_none());
    }

    #[tokio::test]
    async fn put_normalizes_email_domains() {
        let app = app(setup().await, Role::OrgAdmin);
        let mut body = valid_body();
        body["allowed_email_domains"] = json!([" Acme.COM ", "acme.org"]);
        let (status, got) = send(&app, "PUT", "/orgs/org_1/oidc", Some(body)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            got["allowed_email_domains"],
            json!(["acme.com", "acme.org"])
        );
    }

    #[tokio::test]
    async fn put_rejects_invalid_input() {
        let app = app(setup().await, Role::OrgAdmin);
        let cases = [
            ("issuer_url", json!("http://idp.acme.com")),
            ("issuer_url", json!("not a url")),
            ("client_id", json!("  ")),
            ("allowed_email_domains", json!(["user@acme.com"])),
            ("allowed_email_domains", json!([""])),
        ];
        for (field, value) in cases {
            let mut body = valid_body();
            body[field] = value.clone();
            let (status, _) = send(&app, "PUT", "/orgs/org_1/oidc", Some(body)).await;
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "{field} = {value} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn put_allows_plain_http_for_localhost() {
        let app = app(setup().await, Role::OrgAdmin);
        let mut body = valid_body();
        body["issuer_url"] = json!("http://localhost:8080/realms/dev");
        let (status, _) = send(&app, "PUT", "/orgs/org_1/oidc", Some(body)).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn member_is_forbidden() {
        let app = app(setup().await, Role::Member);
        let (status, _) = send(&app, "GET", "/orgs/org_1/oidc", None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = send(&app, "PUT", "/orgs/org_1/oidc", Some(valid_body())).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        let (status, _) = send(&app, "DELETE", "/orgs/org_1/oidc", None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn other_org_is_forbidden() {
        let app = app(setup().await, Role::OrgAdmin);
        let (status, _) = send(&app, "PUT", "/orgs/org_2/oidc", Some(valid_body())).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn delete_removes_config() {
        let app = app(setup().await, Role::OrgAdmin);
        send(&app, "PUT", "/orgs/org_1/oidc", Some(valid_body())).await;

        let (status, _) = send(&app, "DELETE", "/orgs/org_1/oidc", None).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (status, _) = send(&app, "DELETE", "/orgs/org_1/oidc", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_refused_while_org_uses_oidc() {
        let app = app(setup().await, Role::OrgAdmin);
        send(&app, "PUT", "/orgs/org_1/oidc", Some(valid_body())).await;
        let (status, _) = send(
            &app,
            "PATCH",
            "/orgs/org_1",
            Some(json!({"auth_mode": "oidc"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = send(&app, "DELETE", "/orgs/org_1/oidc", None).await;
        assert_eq!(
            status,
            StatusCode::CONFLICT,
            "switch back to password before removing the IdP"
        );
    }

    #[tokio::test]
    async fn patch_auth_mode_oidc_requires_config() {
        let app = app(setup().await, Role::OrgAdmin);
        let (status, _) = send(
            &app,
            "PATCH",
            "/orgs/org_1",
            Some(json!({"auth_mode": "oidc"})),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn patch_rejects_unknown_auth_mode() {
        let app = app(setup().await, Role::OrgAdmin);
        let (status, _) = send(
            &app,
            "PATCH",
            "/orgs/org_1",
            Some(json!({"auth_mode": "saml"})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_org_cannot_start_in_oidc_mode() {
        let app = app(setup().await, Role::OrgAdmin);
        let (status, _) = send(
            &app,
            "POST",
            "/orgs",
            Some(json!({"name": "New", "slug": "new", "auth_mode": "oidc"})),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "a new org has no OIDC settings yet"
        );
    }
}
