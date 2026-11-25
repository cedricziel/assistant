//! OAuth2 endpoints for the assistant web UI.
//!
//! Wires the `assistant-auth` crate's OAuth2 server, device code flow,
//! and client registration into Axum HTTP handlers.

pub mod authorize;
pub mod callback;
pub mod complete;
pub mod device;
pub mod oidc_bridge;
pub mod oidc_sessions;
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
use assistant_auth::oidc::OidcProvider;
use assistant_core::auth::AuthContext;
use assistant_core::identity::UserId;
use assistant_core::store::{MembershipStore, UserStore};
use assistant_storage::OrgStorageLayer;

use self::oidc_sessions::PendingOidcSessionStore;

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
    /// When `true`, session cookies get the `Secure` attribute.
    pub secure_cookie: bool,
    /// OIDC provider (present when `auth_mode = "oidc"`).
    pub oidc_provider: Option<Arc<OidcProvider>>,
    /// Pending OIDC authorization sessions (state → original params).
    pub oidc_sessions: Option<Arc<PendingOidcSessionStore>>,
    /// Rate limiters; the password endpoint uses the per-account lockout.
    pub rate_limits: Arc<crate::rate_limit::RateLimits>,
}

/// Build the public OAuth2 router (no auth required on these endpoints).
///
/// Mounted at `/oauth/*` in the main router.
pub fn oauth_router() -> Router<OAuthState> {
    Router::new()
        // Authorization endpoint.
        .route("/oauth/authorize", get(authorize::authorize_get))
        .route("/oauth/authorize", post(authorize::authorize_post))
        // OIDC IdP callback (receives code from external IdP).
        .route("/oauth/callback", get(callback::callback))
        // Password-mode redirect completion (returns code as JSON).
        .route("/oauth/complete", get(complete::complete))
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

/// A user's identity as read from `org.db` at token issuance.
pub(crate) struct IssuedIdentity {
    pub ctx: AuthContext,
    /// Session generation the access token must be bound to.
    pub token_version: u64,
}

/// Build the [`AuthContext`] for a user who has just authenticated, with the
/// real `org_id`, `email` and `space_roles` from the org-level store.
///
/// `Ok(None)` means the user no longer exists, so no token may be issued.
pub(crate) async fn build_auth_context_for_user(
    org_storage: &OrgStorageLayer,
    user_id: &str,
    client_id: &str,
) -> anyhow::Result<Option<IssuedIdentity>> {
    let user_id_typed = UserId::from(user_id);

    let Some(user) = org_storage.user_store().get_user(&user_id_typed).await? else {
        return Ok(None);
    };
    let Some(session) = org_storage
        .user_store()
        .session_state(&user_id_typed)
        .await?
    else {
        return Ok(None);
    };
    let space_roles = org_storage
        .membership_store()
        .get_space_roles(&user_id_typed)
        .await?;

    Ok(Some(IssuedIdentity {
        ctx: AuthContext {
            user_id: user.id,
            org_id: user.org_id,
            email: user.email,
            space_roles,
            scopes: vec![],
            client_id: client_id.to_string(),
        },
        token_version: session.token_version,
    }))
}

#[cfg(test)]
mod escape_tests {
    use super::escape_html_attr;

    #[test]
    fn escape_html_attr_replaces_all_dangerous_chars() {
        assert_eq!(escape_html_attr("&"), "&amp;");
        assert_eq!(escape_html_attr("\""), "&quot;");
        assert_eq!(escape_html_attr("'"), "&#x27;");
        assert_eq!(escape_html_attr("<"), "&lt;");
        assert_eq!(escape_html_attr(">"), "&gt;");
    }

    #[test]
    fn escape_html_attr_handles_compound_input() {
        let input = r#"<script>alert("xss & 'pwn'")</script>"#;
        let escaped = escape_html_attr(input);
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(!escaped.contains('"'));
        assert!(!escaped.contains('\''));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
        assert!(escaped.contains("&amp;"));
        assert!(escaped.contains("&quot;"));
        assert!(escaped.contains("&#x27;"));
    }

    #[test]
    fn escape_html_attr_preserves_safe_text() {
        assert_eq!(escape_html_attr(""), "");
        assert_eq!(escape_html_attr("hello world"), "hello world");
        assert_eq!(escape_html_attr("0123abc-._~"), "0123abc-._~");
        // UTF-8 / emoji passes through unchanged.
        assert_eq!(escape_html_attr("héllo 🦀"), "héllo 🦀");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use assistant_auth::jwt::JwtKeyPair;
    use assistant_core::auth::RefreshTokenStore;
    use assistant_core::identity::OrgId;
    use assistant_core::store::{OrgStore, Organization, User, UserStore};

    /// Generate a PKCE pair for tests that don't specifically test PKCE behaviour.
    fn test_pkce() -> (String, String) {
        let verifier = assistant_auth::oauth2::pkce::generate_verifier();
        let challenge = assistant_auth::oauth2::pkce::challenge_from_verifier(&verifier);
        (verifier, challenge)
    }

    /// Build a test `OAuthState` backed by in-memory stores.
    async fn test_state() -> OAuthState {
        test_state_with_clock().await.0
    }

    /// Like [`test_state`], also returning the clock driving the rate limiters.
    async fn test_state_with_clock() -> (OAuthState, assistant_core::clock::FakeClock) {
        let clock = assistant_core::clock::FakeClock::new_at_seed();
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

        let rate_limits = Arc::new(crate::rate_limit::RateLimits::new(
            Arc::new(clock.clone()),
            &crate::rate_limit::RateLimitArgs {
                auth_rate_limit_per_minute: 1000,
                register_rate_limit_per_hour: 1000,
                login_max_failures: 3,
                trust_forwarded_for: false,
            },
        ));

        let state = OAuthState {
            oauth2_server,
            jwt_manager,
            client_registrar,
            device_manager,
            org_storage,
            issuer: "http://localhost".into(),
            secure_cookie: false,
            oidc_provider: None,
            oidc_sessions: None,
            rate_limits,
        };
        (state, clock)
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
        let (_verifier, challenge) = test_pkce();

        // Register a client first so authorize_get can validate it.
        let app = build_app(state.clone());
        let reg_body = serde_json::json!({
            "client_name": "Form Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
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

        let app = build_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(format!("/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri=http://localhost/cb&code_challenge={challenge}&code_challenge_method=S256"))
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
    async fn authorize_get_redirects_to_idp_in_oidc_mode() {
        use assistant_auth::oidc::{Jwk, JwksResponse, OidcConfig, OidcDiscovery, OidcProvider};
        use oidc_sessions::PendingOidcSessionStore;
        use std::time::Duration;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // Stand up a mock IdP for OIDC discovery.
        let server = MockServer::start().await;
        let issuer = server.uri();
        let discovery = OidcDiscovery {
            issuer: issuer.clone(),
            authorization_endpoint: format!("{issuer}/authorize"),
            token_endpoint: format!("{issuer}/token"),
            userinfo_endpoint: None,
            jwks_uri: format!("{issuer}/jwks"),
            response_types_supported: vec!["code".into()],
            subject_types_supported: vec!["public".into()],
            id_token_signing_alg_values_supported: vec!["RS256".into()],
        };
        let jwks = JwksResponse {
            keys: vec![Jwk {
                kty: "RSA".into(),
                kid: Some("k1".into()),
                alg: Some("RS256".into()),
                key_use: Some("sig".into()),
                n: Some("test".into()),
                e: Some("AQAB".into()),
                crv: None,
                x: None,
                y: None,
            }],
        };
        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&discovery))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/jwks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
            .mount(&server)
            .await;

        let oidc = OidcProvider::discover(OidcConfig {
            issuer_url: issuer.clone(),
            client_id: "oidc-client".into(),
            client_secret: None,
            auto_provision: true,
            allowed_email_domains: None,
        })
        .await
        .unwrap();

        // Build state with OIDC provider.
        let mut state = test_state().await;
        let sessions = Arc::new(PendingOidcSessionStore::new(Duration::from_secs(300)));
        state.oidc_provider = Some(Arc::new(oidc));
        state.oidc_sessions = Some(sessions);

        // Register a client.
        let app = build_app(state.clone());
        let reg_body = serde_json::json!({
            "client_name": "OIDC Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
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

        // GET /oauth/authorize should redirect to the IdP, not render a form.
        let (_verifier, challenge) = test_pkce();
        let app = build_app(state);
        let resp = app.oneshot(
            Request::builder()
                .uri(format!("/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri=http://localhost/cb&state=mystate&code_challenge={challenge}&code_challenge_method=S256"))
                .body(Body::empty())
                .unwrap(),
        ).await.unwrap();

        // Should be a redirect (303 from Redirect::to).
        assert!(
            resp.status().is_redirection(),
            "OIDC authorize_get should redirect, got {}",
            resp.status()
        );
        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        assert!(
            location.starts_with(&format!("{issuer}/authorize?")),
            "should redirect to IdP authorize endpoint: {location}"
        );
        assert!(
            location.contains("client_id=oidc-client"),
            "should include IdP client_id: {location}"
        );
        assert!(
            location.contains("scope=openid"),
            "should request openid scope: {location}"
        );
    }

    #[tokio::test]
    async fn client_registration_round_trip() {
        let state = test_state().await;
        let app = build_app(state);

        let body = serde_json::json!({
            "client_name": "Test App",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
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

    async fn revoke_token(state: &OAuthState, token: &str) -> StatusCode {
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/revoke")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!("token={token}")))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }

    async fn alice_token_version(state: &OAuthState) -> u64 {
        state
            .org_storage
            .user_store()
            .session_state(&assistant_core::identity::UserId::from("usr_alice"))
            .await
            .unwrap()
            .unwrap()
            .token_version
    }

    #[tokio::test]
    async fn revoking_a_refresh_token_ends_all_of_the_users_sessions() {
        let state = test_state().await;
        let web = state
            .oauth2_server
            .issue_token_pair("usr_alice", "webui", vec![])
            .await
            .unwrap();
        let cli = state
            .oauth2_server
            .issue_token_pair("usr_alice", "cli", vec![])
            .await
            .unwrap();

        assert_eq!(
            revoke_token(&state, &web.refresh_token).await,
            StatusCode::OK
        );

        assert_eq!(alice_token_version(&state).await, 1, "access tokens voided");
        let refresh_tokens = state.org_storage.refresh_token_store();
        for token in [&web.refresh_token, &cli.refresh_token] {
            assert!(
                refresh_tokens.get_token(token).await.unwrap().is_none(),
                "every refresh token of the user is revoked"
            );
        }
    }

    #[tokio::test]
    async fn revoking_an_access_token_ends_all_of_the_users_sessions() {
        let state = test_state().await;
        let identity = build_auth_context_for_user(&state.org_storage, "usr_alice", "webui")
            .await
            .unwrap()
            .unwrap();
        let jwt = state
            .jwt_manager
            .sign(&identity.ctx, "test", "Alice", identity.token_version)
            .unwrap();

        assert_eq!(revoke_token(&state, &jwt).await, StatusCode::OK);

        assert_eq!(alice_token_version(&state).await, 1);
    }

    #[tokio::test]
    async fn revoking_an_unknown_token_changes_nothing() {
        let state = test_state().await;

        assert_eq!(revoke_token(&state, "not-a-token").await, StatusCode::OK);

        assert_eq!(alice_token_version(&state).await, 0);
    }

    #[tokio::test]
    async fn full_auth_code_flow() {
        let state = test_state().await;
        let (verifier, challenge) = test_pkce();

        // Step 1: Register a client.
        let app = build_app(state.clone());
        let reg_body = serde_json::json!({
            "client_name": "Flow Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
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

        // Step 2: POST /oauth/authorize with credentials + PKCE challenge.
        let app = build_app(state.clone());
        let form = format!(
            "email=alice%40example.com&password=secret123&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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

        // Step 3: Exchange code for tokens (with PKCE verifier).
        let app = build_app(state.clone());
        let token_form = format!(
            "grant_type=authorization_code&code={code}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_verifier={verifier}"
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

    #[tokio::test]
    async fn auth_code_exchange_sets_session_cookie() {
        let state = test_state().await;
        let (verifier, challenge) = test_pkce();

        // Register a client.
        let app = build_app(state.clone());
        let reg_body = serde_json::json!({
            "client_name": "Cookie Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
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

        // Authorize (get auth code with PKCE challenge).
        let app = build_app(state.clone());
        let form = format!(
            "email=alice%40example.com&password=secret123&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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
        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        let code = location
            .split("code=")
            .nth(1)
            .unwrap()
            .split('&')
            .next()
            .unwrap();

        // Exchange code for tokens (with PKCE verifier).
        let app = build_app(state.clone());
        let token_form = format!(
            "grant_type=authorization_code&code={code}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_verifier={verifier}"
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

        assert_eq!(resp.status(), StatusCode::OK);

        // Verify that a session cookie is set.
        let set_cookie = resp
            .headers()
            .get("set-cookie")
            .expect("token exchange should set a session cookie")
            .to_str()
            .unwrap();
        assert!(
            set_cookie.contains("assistant_session="),
            "cookie should be named assistant_session: {set_cookie}"
        );
        assert!(
            set_cookie.contains("HttpOnly"),
            "cookie should be HttpOnly: {set_cookie}"
        );
        assert!(
            set_cookie.contains("SameSite=Lax"),
            "cookie should be SameSite=Lax: {set_cookie}"
        );

        // Verify the JSON body still contains tokens.
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tokens: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(tokens["access_token"].is_string());
        assert!(tokens["refresh_token"].is_string());
    }

    #[tokio::test]
    async fn refresh_token_exchange_sets_session_cookie() {
        let state = test_state().await;
        let (verifier, challenge) = test_pkce();

        // Register client + get auth code + exchange for tokens (setup).
        let app = build_app(state.clone());
        let reg_body = serde_json::json!({
            "client_name": "Refresh Cookie Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
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

        let app = build_app(state.clone());
        let form = format!(
            "email=alice%40example.com&password=secret123&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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
        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        let code = location
            .split("code=")
            .nth(1)
            .unwrap()
            .split('&')
            .next()
            .unwrap();

        let app = build_app(state.clone());
        let token_form = format!(
            "grant_type=authorization_code&code={code}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_verifier={verifier}"
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
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tokens: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let refresh_token = tokens["refresh_token"].as_str().unwrap();

        // Now refresh and check for cookie.
        let app = build_app(state.clone());
        let refresh_form =
            format!("grant_type=refresh_token&refresh_token={refresh_token}&client_id={client_id}");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/token")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(refresh_form))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        // Verify session cookie is also set on refresh.
        let set_cookie = resp
            .headers()
            .get("set-cookie")
            .expect("refresh should also set a session cookie")
            .to_str()
            .unwrap();
        assert!(
            set_cookie.contains("assistant_session="),
            "cookie should be named assistant_session: {set_cookie}"
        );
    }

    /// Regression: an auth-code exchange must produce a JWT whose `org_id`
    /// claim matches the user's actual org, not a hardcoded "default".
    /// Otherwise `/api/orgs` returns an empty list and the user is stuck on
    /// the org/space selector.
    #[tokio::test]
    async fn auth_code_jwt_carries_real_org_id() {
        use assistant_auth::jwt::JwtKeyPair;

        let state = test_state().await;
        let (verifier, challenge) = test_pkce();

        // Register a client.
        let app = build_app(state.clone());
        let reg_body = serde_json::json!({
            "client_name": "Org ID Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"],
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

        // Authorize.
        let app = build_app(state.clone());
        let form = format!(
            "email=alice%40example.com&password=secret123&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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
        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        let code = location
            .split("code=")
            .nth(1)
            .unwrap()
            .split('&')
            .next()
            .unwrap();

        // Exchange.
        let app = build_app(state.clone());
        let token_form = format!(
            "grant_type=authorization_code&code={code}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_verifier={verifier}"
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
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tokens: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let access_token = tokens["access_token"].as_str().unwrap();

        // Decode the JWT and check the org_id claim is the user's real org.
        let claims = state.jwt_manager.validate(access_token).unwrap();
        assert_eq!(
            claims.org_id, "org_test",
            "JWT should carry the user's real org_id, not a hardcoded value"
        );
        assert_eq!(claims.email, "alice@example.com");
        // sanity-check: signature checks pass through the existing key pair.
        let _ = JwtKeyPair::generate();
    }

    // -- token issuance is bound to the user's session generation ------------

    async fn refresh_for(state: &OAuthState, refresh_token: &str) -> axum::response::Response {
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/token")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "grant_type=refresh_token&refresh_token={refresh_token}&client_id=webui"
                    )))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn issued_identity_carries_the_current_token_version() {
        let state = test_state().await;
        let alice = assistant_core::identity::UserId::from("usr_alice");
        state
            .org_storage
            .user_store()
            .bump_token_version(&alice)
            .await
            .unwrap();

        let identity = build_auth_context_for_user(&state.org_storage, "usr_alice", "webui")
            .await
            .unwrap()
            .expect("user exists");

        assert_eq!(identity.token_version, 1);
        assert_eq!(identity.ctx.org_id, OrgId::from("org_test"));
    }

    #[tokio::test]
    async fn no_identity_is_issued_for_a_missing_user() {
        let state = test_state().await;

        let identity = build_auth_context_for_user(&state.org_storage, "usr_ghost", "webui")
            .await
            .unwrap();

        assert!(identity.is_none());
    }

    #[tokio::test]
    async fn refreshed_jwt_is_bound_to_the_current_token_version() {
        let state = test_state().await;
        let pair = state
            .oauth2_server
            .issue_token_pair("usr_alice", "webui", vec![])
            .await
            .unwrap();
        state
            .org_storage
            .user_store()
            .bump_token_version(&assistant_core::identity::UserId::from("usr_alice"))
            .await
            .unwrap();

        let resp = refresh_for(&state, &pair.refresh_token).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tokens: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let claims = state
            .jwt_manager
            .validate(tokens["access_token"].as_str().unwrap())
            .unwrap();
        assert_eq!(claims.tv, 1);
    }

    #[tokio::test]
    async fn refresh_for_a_deleted_user_is_refused() {
        let state = test_state().await;
        let pair = state
            .oauth2_server
            .issue_token_pair("usr_alice", "webui", vec![])
            .await
            .unwrap();
        state
            .org_storage
            .user_store()
            .delete_user(&assistant_core::identity::UserId::from("usr_alice"))
            .await
            .unwrap();

        let resp = refresh_for(&state, &pair.refresh_token).await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(resp.headers().get("set-cookie").is_none());
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(err["error"], "invalid_grant");
    }

    // -- per-account lockout and timing -------------------------------------

    async fn register_lockout_client(state: &OAuthState) -> String {
        let reg_body = serde_json::json!({
            "client_name": "Lockout Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
        });
        let resp = build_app(state.clone())
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
        client["client_id"].as_str().unwrap().to_string()
    }

    async fn attempt_login(
        state: &OAuthState,
        client_id: &str,
        email: &str,
        password: &str,
    ) -> axum::response::Response {
        let (_verifier, challenge) = test_pkce();
        let form = format!(
            "email={}&password={password}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256",
            urlencoding::encode(email)
        );
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/authorize")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn repeated_failures_lock_the_account_with_retry_after() {
        let (state, clock) = test_state_with_clock().await;
        let client_id = register_lockout_client(&state).await;

        for _ in 0..3 {
            let resp = attempt_login(&state, &client_id, "alice@example.com", "wrong").await;
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        let locked = attempt_login(&state, &client_id, "alice@example.com", "secret123").await;
        assert_eq!(
            locked.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "even the right password is refused while locked"
        );
        let retry: u64 = locked
            .headers()
            .get("retry-after")
            .expect("Retry-After header")
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!((1..=60).contains(&retry));

        clock.advance(std::time::Duration::from_secs(60));
        let ok = attempt_login(&state, &client_id, "alice@example.com", "secret123").await;
        assert_eq!(ok.status(), StatusCode::SEE_OTHER, "lockout expires");
    }

    #[tokio::test]
    async fn lockout_is_case_insensitive_on_the_email() {
        let (state, _clock) = test_state_with_clock().await;
        let client_id = register_lockout_client(&state).await;
        for email in [
            "alice@example.com",
            "ALICE@example.com",
            " Alice@Example.com",
        ] {
            attempt_login(&state, &client_id, email, "wrong").await;
        }
        let resp = attempt_login(&state, &client_id, "alice@EXAMPLE.com", "secret123").await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn unknown_emails_are_throttled_like_known_ones() {
        let (state, _clock) = test_state_with_clock().await;
        let client_id = register_lockout_client(&state).await;
        for _ in 0..3 {
            let resp = attempt_login(&state, &client_id, "ghost@example.com", "wrong").await;
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }
        let resp = attempt_login(&state, &client_id, "ghost@example.com", "wrong").await;
        assert_eq!(
            resp.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "lockout must not reveal whether the account exists"
        );
    }

    #[tokio::test]
    async fn successful_login_clears_the_failure_count() {
        let (state, _clock) = test_state_with_clock().await;
        let client_id = register_lockout_client(&state).await;
        for _ in 0..2 {
            attempt_login(&state, &client_id, "alice@example.com", "wrong").await;
        }
        let ok = attempt_login(&state, &client_id, "alice@example.com", "secret123").await;
        assert_eq!(ok.status(), StatusCode::SEE_OTHER);
        for _ in 0..2 {
            let resp = attempt_login(&state, &client_id, "alice@example.com", "wrong").await;
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "counter was reset");
        }
    }

    #[tokio::test]
    async fn unknown_email_takes_as_long_as_a_wrong_password() {
        let (state, _clock) = test_state_with_clock().await;
        let client_id = register_lockout_client(&state).await;

        let started = std::time::Instant::now();
        attempt_login(&state, &client_id, "alice@example.com", "wrong").await;
        let known = started.elapsed();

        let started = std::time::Instant::now();
        attempt_login(&state, &client_id, "ghost@example.com", "wrong").await;
        let unknown = started.elapsed();

        assert!(
            unknown * 2 > known,
            "unknown-user path ({unknown:?}) must run a password hash like the known-user path ({known:?})"
        );
    }

    async fn register_test_client(state: &OAuthState) -> String {
        let reg_body = serde_json::json!({
            "client_name": "S256 Test",
            "redirect_uris": ["http://localhost/cb"],
            "grant_types": ["authorization_code"],
            "response_types": ["code"]
        });
        let resp = build_app(state.clone())
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
        client["client_id"].as_str().unwrap().to_string()
    }

    async fn authorize_get_status(state: &OAuthState, method_param: &str) -> StatusCode {
        let client_id = register_test_client(state).await;
        let (_verifier, challenge) = test_pkce();
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri=http://localhost/cb&code_challenge={challenge}{method_param}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }

    async fn authorize_post_status(state: &OAuthState, method_param: &str) -> StatusCode {
        let client_id = register_test_client(state).await;
        let (_verifier, challenge) = test_pkce();
        let form = format!(
            "email=alice%40example.com&password=secret123&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}{method_param}"
        );
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/authorize")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn authorize_get_accepts_s256() {
        let state = test_state().await;
        assert_eq!(
            authorize_get_status(&state, "&code_challenge_method=S256").await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn authorize_get_rejects_plain_and_missing_method() {
        let state = test_state().await;
        for param in [
            "&code_challenge_method=plain",
            "&code_challenge_method=s256",
            "",
        ] {
            assert_eq!(
                authorize_get_status(&state, param).await,
                StatusCode::BAD_REQUEST,
                "GET with {param:?} must be rejected"
            );
        }
    }

    #[tokio::test]
    async fn authorize_post_accepts_s256() {
        let state = test_state().await;
        assert_eq!(
            authorize_post_status(&state, "&code_challenge_method=S256").await,
            StatusCode::SEE_OTHER
        );
    }

    #[tokio::test]
    async fn authorize_post_rejects_plain_and_missing_method() {
        let state = test_state().await;
        for param in ["&code_challenge_method=plain", ""] {
            assert_eq!(
                authorize_post_status(&state, param).await,
                StatusCode::BAD_REQUEST,
                "POST with {param:?} must be rejected"
            );
        }
    }

    #[tokio::test]
    async fn authorize_form_carries_the_challenge_method() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;
        let (_verifier, challenge) = test_pkce();
        let resp = build_app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri=http://localhost/cb&code_challenge={challenge}&code_challenge_method=S256"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(
            String::from_utf8_lossy(&body).contains(r#"name="code_challenge_method" value="S256""#),
            "form must round-trip the challenge method"
        );
    }
}
