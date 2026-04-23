//! OAuth2 endpoints for the assistant web UI.
//!
//! Wires the `assistant-auth` crate's OAuth2 server, device code flow,
//! and client registration into Axum HTTP handlers.

pub mod authorize;
pub mod device;
pub mod register;
pub mod revoke;
pub mod token;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use assistant_auth::jwt::JwtManager;
use assistant_auth::oauth2::clients::ClientRegistrar;
use assistant_auth::oauth2::device::DeviceCodeManager;
use assistant_auth::oauth2::server::OAuth2Server;
use assistant_core::auth::AuthContext;
use assistant_storage::OrgStorageLayer;

/// Shared state for all OAuth2 endpoints.
#[derive(Clone)]
pub struct OAuthState {
    /// OAuth2 authorization code + refresh token server.
    pub oauth2_server: Arc<OAuth2Server>,
    /// JWT manager for signing access tokens.
    pub jwt_manager: Arc<JwtManager>,
    /// Client registrar for dynamic client registration.
    pub client_registrar: Arc<ClientRegistrar>,
    /// Device code flow manager.
    pub device_manager: Arc<DeviceCodeManager>,
    /// Org-level storage (user lookups, password verification, etc.).
    pub org_storage: Arc<OrgStorageLayer>,
    /// The server's issuer URL (used in metadata and JWTs).
    pub issuer: String,
}

/// Build the public OAuth2 router (no auth required on these endpoints).
///
/// Mounted at `/oauth/*` in the main router.
pub fn oauth_router() -> Router<OAuthState> {
    Router::new()
        // Authorization endpoint.
        .route("/oauth/authorize", get(authorize::authorize_get))
        .route("/oauth/authorize", post(authorize::authorize_post))
        // Token endpoint.
        .route("/oauth/token", post(token::token))
        // Dynamic client registration.
        .route("/oauth/register", post(register::register))
        // Device code flow.
        .route("/oauth/device", post(device::device_initiate))
        .route("/oauth/device/verify", get(device::device_verify_page))
        .route("/oauth/device/verify", post(device::device_verify_submit))
        // Token revocation.
        .route("/oauth/revoke", post(revoke::revoke))
        // Server metadata (RFC 8414).
        .route(
            "/.well-known/oauth-authorization-server",
            get(revoke::metadata),
        )
}

/// Escape a string for safe inclusion in an HTML attribute value.
pub(crate) fn escape_html_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Standard OAuth2 error response body.
#[derive(serde::Serialize)]
pub(crate) struct OAuthError {
    pub error: &'static str,
    pub error_description: String,
}

/// OAuth2 error response — OpenAPI schema type.
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct OAuthErrorResponse {
    /// Error code (e.g. `invalid_grant`, `invalid_request`).
    pub error: String,
    /// Human-readable description of the error.
    pub error_description: String,
}

/// OpenAPI schema mirror for [`assistant_core::auth::ClientRegistration`].
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ClientRegistrationSchema {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Option<Vec<String>>,
    pub token_endpoint_auth_method: Option<String>,
}

/// OpenAPI schema mirror for [`assistant_core::auth::ClientInfo`].
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ClientInfoSchema {
    pub client_id: String,
    pub client_name: String,
    pub client_secret: Option<String>,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub token_endpoint_auth_method: String,
}

/// OpenAPI schema mirror for [`assistant_auth::oauth2::device::DeviceCodeResponse`].
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct DeviceCodeResponseSchema {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

/// Helper to build an [`AuthContext`] stub for a user who just authenticated
/// via password or OIDC (before full space roles are loaded).
pub(crate) fn stub_auth_context(user_id: &str, org_id: &str, email: &str) -> AuthContext {
    AuthContext {
        user_id: assistant_core::identity::UserId::from(user_id),
        org_id: assistant_core::identity::OrgId::from(org_id),
        email: email.to_string(),
        space_roles: std::collections::HashMap::new(),
        scopes: vec![],
        client_id: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use assistant_auth::jwt::JwtKeyPair;
    use assistant_core::identity::OrgId;
    use assistant_core::store::{OrgStore, Organization, User, UserStore};

    /// Build a test `OAuthState` backed by in-memory stores.
    async fn test_state() -> OAuthState {
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());

        // Seed a test org and user.
        let now = chrono::Utc::now();
        org_storage
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("org_test"),
                name: "Test Org".into(),
                slug: "test".into(),
                auth_mode: "password".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();

        let password_hash = assistant_auth::password::hash_password("secret123").unwrap();
        org_storage
            .user_store()
            .create_user(&User {
                id: assistant_core::identity::UserId::from("usr_alice"),
                org_id: OrgId::from("org_test"),
                email: "alice@example.com".into(),
                name: "Alice".into(),
                password_hash,
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();

        let jwt_key_pair = JwtKeyPair::generate().unwrap();
        let jwt_manager = Arc::new(JwtManager::new(
            jwt_key_pair,
            "http://localhost".into(),
            "http://localhost".into(),
        ));

        let oauth2_server = Arc::new(OAuth2Server::new(
            Arc::new(org_storage.auth_code_store()),
            Arc::new(org_storage.refresh_token_store()),
        ));

        let client_registrar = Arc::new(ClientRegistrar::new(Arc::new(org_storage.client_store())));

        let device_manager = Arc::new(DeviceCodeManager::new(
            Arc::new(org_storage.device_code_store()),
            "http://localhost/oauth/device/verify".into(),
        ));

        OAuthState {
            oauth2_server,
            jwt_manager,
            client_registrar,
            device_manager,
            org_storage,
            issuer: "http://localhost".into(),
        }
    }

    fn build_app(state: OAuthState) -> Router {
        oauth_router().with_state(state)
    }

    #[tokio::test]
    async fn metadata_returns_endpoints() {
        let state = test_state().await;
        let app = build_app(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/oauth-authorization-server")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK, "metadata should return 200");

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let meta: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(meta["issuer"], "http://localhost");
        assert_eq!(meta["token_endpoint"], "http://localhost/oauth/token");
        assert_eq!(
            meta["authorization_endpoint"],
            "http://localhost/oauth/authorize"
        );
    }

    #[tokio::test]
    async fn authorize_get_rejects_missing_response_type() {
        let state = test_state().await;
        let app = build_app(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/oauth/authorize?client_id=test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "missing response_type should be rejected"
        );
    }

    #[tokio::test]
    async fn authorize_get_renders_form() {
        let state = test_state().await;
        let app = build_app(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/oauth/authorize?response_type=code&client_id=test&redirect_uri=http://localhost/cb")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "valid authorize GET should return 200"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8_lossy(&body);
        assert!(
            html.contains("Sign In"),
            "form should contain Sign In heading"
        );
        assert!(
            html.contains("client_id"),
            "form should contain client_id hidden field"
        );
    }

    #[tokio::test]
    async fn client_registration_round_trip() {
        let state = test_state().await;
        let app = build_app(state);

        let body = serde_json::json!({
            "client_name": "Test App",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"]
        });

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::CREATED,
            "registration should return 201"
        );

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let info: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            info["client_id"].is_string(),
            "response should contain client_id"
        );
        assert_eq!(info["client_name"], "Test App");
    }

    #[tokio::test]
    async fn token_rejects_unsupported_grant_type() {
        let state = test_state().await;
        let app = build_app(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/token")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("grant_type=password&username=a&password=b"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "unsupported grant_type should return 400"
        );

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(err["error"], "unsupported_grant_type");
    }

    #[tokio::test]
    async fn revoke_accepts_any_token() {
        let state = test_state().await;
        let app = build_app(state);

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/revoke")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("token=some_token"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "revoke should return 200 per RFC 7009"
        );
    }

    #[tokio::test]
    async fn full_auth_code_flow() {
        let state = test_state().await;

        // Step 1: Register a client.
        let app = build_app(state.clone());
        let reg_body = serde_json::json!({
            "client_name": "Flow Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"]
        });
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/register")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&reg_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let client: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let client_id = client["client_id"].as_str().unwrap();

        // Step 2: POST /oauth/authorize with credentials.
        let app = build_app(state.clone());
        let form = format!(
            "email=alice%40example.com&password=secret123&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb"
        );
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/authorize")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::SEE_OTHER,
            "authorize POST should redirect (303 or 302)"
        );

        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        assert!(
            location.starts_with("http://localhost/cb?code="),
            "redirect should contain authorization code: {location}"
        );

        // Extract the code from the redirect URI.
        let code = location
            .split("code=")
            .nth(1)
            .unwrap()
            .split('&')
            .next()
            .unwrap();

        // Step 3: Exchange code for tokens.
        let app = build_app(state.clone());
        let token_form = format!(
            "grant_type=authorization_code&code={code}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb"
        );
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/token")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(token_form))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "token exchange should return 200"
        );

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tokens: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            tokens["access_token"].is_string(),
            "response should contain access_token"
        );
        assert_eq!(tokens["token_type"], "Bearer");
        assert!(
            tokens["refresh_token"].is_string(),
            "response should contain refresh_token"
        );
    }
}
