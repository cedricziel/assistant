//! OAuth2 endpoints for the assistant web UI.
//!
//! Wires the `assistant-auth` crate's OAuth2 server, device code flow,
//! and client registration into Axum HTTP handlers.

pub mod authorize;
pub mod callback;
pub mod complete;
pub mod device;
pub mod oidc_bridge;
pub mod oidc_registry;
pub mod oidc_sessions;
pub mod register;
pub mod revoke;
#[cfg(test)]
mod test_support;
pub mod token;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};

use assistant_auth::jwt::JwtManager;
use assistant_auth::oauth2::clients::ClientRegistrar;
use assistant_auth::oauth2::device::DeviceCodeManager;
use assistant_auth::oauth2::server::OAuth2Server;
use assistant_core::auth::AuthContext;
use assistant_core::identity::UserId;
use assistant_core::store::{MembershipStore, UserStore};
use assistant_storage::OrgStorageLayer;

use self::oidc_registry::OidcProviderRegistry;
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
    /// Per-org OIDC providers, discovered from each org's stored config.
    pub oidc: Arc<OidcProviderRegistry>,
    /// Pending OIDC authorization sessions (state → original params).
    pub oidc_sessions: Arc<PendingOidcSessionStore>,
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
        .route("/oauth/authorize/identify", post(authorize::identify_post))
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
            org_storage: org_storage.clone(),
            issuer: "http://localhost".into(),
            secure_cookie: false,
            oidc: Arc::new(oidc_registry::OidcProviderRegistry::new(
                org_storage.clone(),
                test_support::test_secrets(),
            )),
            oidc_sessions: Arc::new(PendingOidcSessionStore::new(
                std::time::Duration::from_secs(300),
            )),
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

    /// GET `/oauth/authorize`, optionally selecting an org by slug.
    async fn authorize(
        state: &OAuthState,
        client_id: &str,
        org: Option<&str>,
    ) -> axum::response::Response {
        let (_verifier, challenge) = test_pkce();
        let org_param = org.map(|o| format!("&org={o}")).unwrap_or_default();
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri=http://localhost/cb&state=mystate&code_challenge={challenge}&code_challenge_method=S256{org_param}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    fn location(resp: &axum::response::Response) -> String {
        resp.headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    async fn body_text(resp: axum::response::Response) -> String {
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&body).into_owned()
    }

    #[tokio::test]
    async fn authorize_get_redirects_to_the_selected_orgs_idp() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, Some("acme")).await;

        assert!(
            resp.status().is_redirection(),
            "OIDC authorize_get should redirect, got {}",
            resp.status()
        );
        let location = location(&resp);
        assert!(
            location.starts_with(&format!("{}/authorize?", idp.uri())),
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
    async fn authorize_get_remembers_the_org_for_the_callback() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, Some("acme")).await;

        let location = location(&resp);
        let idp_state = url::Url::parse(&location)
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.into_owned())
            .unwrap();
        let pending = state.oidc_sessions.take(&idp_state).await.unwrap();
        assert_eq!(pending.org_id, OrgId::from("org_acme"));
        assert_eq!(pending.client_state.as_deref(), Some("mystate"));
    }

    #[tokio::test]
    async fn authorize_get_routes_each_org_to_its_own_idp() {
        let state = test_state().await;
        let idp_a = wiremock::MockServer::start().await;
        let idp_b = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp_a).await;
        test_support::mount_idp(&idp_b).await;
        test_support::seed_oidc_org(&state.org_storage, "org_a", "a", &idp_a.uri(), "client-a")
            .await;
        test_support::seed_oidc_org(&state.org_storage, "org_b", "b", &idp_b.uri(), "client-b")
            .await;
        let client_id = register_test_client(&state).await;

        let to_a = location(&authorize(&state, &client_id, Some("a")).await);
        let to_b = location(&authorize(&state, &client_id, Some("b")).await);

        assert!(
            to_a.starts_with(&format!("{}/authorize?", idp_a.uri())),
            "{to_a}"
        );
        assert!(to_a.contains("client_id=client-a"), "{to_a}");
        assert!(
            to_b.starts_with(&format!("{}/authorize?", idp_b.uri())),
            "{to_b}"
        );
        assert!(to_b.contains("client_id=client-b"), "{to_b}");
    }

    #[tokio::test]
    async fn authorize_get_renders_form_for_password_org() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, Some("test")).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_text(resp).await.contains("Sign In"));
    }

    #[tokio::test]
    async fn authorize_get_without_org_does_not_redirect_even_when_an_oidc_org_exists() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, None).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(body_text(resp).await.contains("Sign In"));
    }

    #[tokio::test]
    async fn authorize_get_rejects_unknown_org_slug() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, Some("no-such-org")).await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn authorize_get_reports_bad_gateway_when_idp_discovery_fails() {
        let state = test_state().await;
        // Nothing is mounted, so discovery returns 404.
        let idp = wiremock::MockServer::start().await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, Some("acme")).await;

        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let body = body_text(resp).await;
        assert!(
            !body.contains(&idp.uri()),
            "must not leak IdP details: {body}"
        );
    }

    #[tokio::test]
    async fn authorize_get_reports_server_error_when_oidc_org_has_no_config() {
        let state = test_state().await;
        test_support::seed_org(&state.org_storage, "org_acme", "acme", "oidc").await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, Some("acme")).await;

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
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
            "email=alice%40example.com&password=secret123&org=test&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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
            "email=alice%40example.com&password=secret123&org=test&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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
            "email=alice%40example.com&password=secret123&org=test&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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
            "email=alice%40example.com&password=secret123&org=test&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256"
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
        attempt_login_in(state, client_id, "test", email, password).await
    }

    async fn attempt_login_in(
        state: &OAuthState,
        client_id: &str,
        org: &str,
        email: &str,
        password: &str,
    ) -> axum::response::Response {
        let (_verifier, challenge) = test_pkce();
        let form = format!(
            "email={}&password={password}&org={org}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}&code_challenge_method=S256",
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
            "email=alice%40example.com&password=secret123&org=test&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge={challenge}{method_param}"
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

    // -- Email-first sign-in --

    /// Create a user in `org_id`; an empty `password` makes an IdP-only user.
    async fn seed_user(state: &OAuthState, id: &str, org_id: &str, email: &str, password: &str) {
        let now = chrono::Utc::now();
        state
            .org_storage
            .user_store()
            .create_user(&User {
                id: assistant_core::identity::UserId::from(id),
                org_id: OrgId::from(org_id),
                email: email.into(),
                name: id.into(),
                password_hash: if password.is_empty() {
                    String::new()
                } else {
                    assistant_auth::password::hash_password(password).unwrap()
                },
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
    }

    /// POST `/oauth/authorize/identify` with the given extra form fields.
    async fn identify_with(
        state: &OAuthState,
        client_id: &str,
        email: &str,
        extra: &str,
    ) -> axum::response::Response {
        // A fixed challenge keeps two identify responses comparable.
        let challenge = "fixed-challenge";
        let form = format!(
            "email={}&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&state=mystate&code_challenge={challenge}{extra}",
            urlencoding::encode(email)
        );
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/authorize/identify")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    async fn identify(
        state: &OAuthState,
        client_id: &str,
        email: &str,
    ) -> axum::response::Response {
        identify_with(state, client_id, email, "&code_challenge_method=S256").await
    }

    #[tokio::test]
    async fn authorize_get_without_org_asks_for_the_email() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, None).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let html = body_text(resp).await;
        assert!(
            html.contains(r#"action="/oauth/authorize/identify""#),
            "{html}"
        );
        assert!(!html.contains(r#"type="password""#), "{html}");
        for field in [
            format!(r#"name="client_id" value="{client_id}""#),
            r#"name="redirect_uri" value="http://localhost/cb""#.to_string(),
            r#"name="state" value="mystate""#.to_string(),
            r#"name="code_challenge_method" value="S256""#.to_string(),
            r#"name="code_challenge" value=""#.to_string(),
            r#"name="scope" value="""#.to_string(),
        ] {
            assert!(html.contains(&field), "missing {field} in {html}");
        }
    }

    #[tokio::test]
    async fn identify_sends_a_sole_oidc_match_to_its_idp_with_a_login_hint() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        seed_user(&state, "usr_carol", "org_acme", "carol@example.com", "").await;
        let client_id = register_test_client(&state).await;

        let resp = identify(&state, &client_id, "carol@example.com").await;

        assert!(resp.status().is_redirection(), "got {}", resp.status());
        let location = location(&resp);
        assert!(
            location.starts_with(&format!("{}/authorize?", idp.uri())),
            "{location}"
        );
        assert!(location.contains("client_id=oidc-client"), "{location}");
        assert!(
            location.contains("login_hint=carol%40example.com"),
            "{location}"
        );
        let idp_state = url::Url::parse(&location)
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.into_owned())
            .unwrap();
        let pending = state.oidc_sessions.take(&idp_state).await.unwrap();
        assert_eq!(pending.org_id, OrgId::from("org_acme"));
        assert_eq!(pending.client_state.as_deref(), Some("mystate"));
    }

    #[tokio::test]
    async fn identify_shows_the_password_form_for_a_sole_password_match() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;

        let resp = identify(&state, &client_id, "alice@example.com").await;

        assert_eq!(resp.status(), StatusCode::OK);
        let html = body_text(resp).await;
        assert!(html.contains(r#"action="/oauth/authorize""#), "{html}");
        assert!(html.contains(r#"name="org" value="test""#), "{html}");
        assert!(html.contains(r#"value="alice@example.com""#), "{html}");
        assert!(html.contains(r#"type="password""#), "{html}");
    }

    #[tokio::test]
    async fn identify_escapes_the_email_it_echoes() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;
        let email = r#""><b>x@example.com"#;
        seed_user(&state, "usr_x", "org_test", email, "pw").await;

        let html = body_text(identify(&state, &client_id, email).await).await;

        assert!(!html.contains("<b>x@"), "{html}");
        assert!(html.contains("&quot;&gt;&lt;b&gt;x@example.com"), "{html}");
    }

    #[tokio::test]
    async fn identify_asks_for_the_org_when_the_email_is_unknown_or_ambiguous() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_globex",
            "globex",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        test_support::seed_org(&state.org_storage, "org_acme", "acme", "password").await;
        seed_user(&state, "usr_d1", "org_acme", "dup@example.com", "pw").await;
        seed_user(&state, "usr_d2", "org_globex", "dup@example.com", "pw").await;
        let client_id = register_test_client(&state).await;

        let ambiguous = identify(&state, &client_id, "dup@example.com").await;
        let unknown = identify(&state, &client_id, "ghost@example.com").await;

        assert_eq!(ambiguous.status(), StatusCode::OK);
        assert_eq!(unknown.status(), StatusCode::OK);
        let ambiguous = body_text(ambiguous).await;
        let unknown = body_text(unknown).await;
        assert_eq!(
            ambiguous.replace("dup@example.com", "ghost@example.com"),
            unknown,
            "both cases must render the same page"
        );
        for leaked in ["acme", "globex", "Test Org"] {
            assert!(!ambiguous.contains(leaked), "{leaked} leaked: {ambiguous}");
        }
        assert!(ambiguous.contains(r#"method="GET""#), "{ambiguous}");
        assert!(
            ambiguous.contains(r#"action="/oauth/authorize""#),
            "{ambiguous}"
        );
        assert!(ambiguous.contains(r#"name="org""#), "{ambiguous}");
        for field in [
            r#"name="response_type" value="code""#,
            r#"name="login_hint" value="dup@example.com""#,
            r#"name="state" value="mystate""#,
            r#"name="code_challenge_method" value="S256""#,
        ] {
            assert!(ambiguous.contains(field), "missing {field}: {ambiguous}");
        }
    }

    #[tokio::test]
    async fn identify_applies_the_same_request_checks_as_the_authorize_page() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;

        let plain = identify_with(
            &state,
            &client_id,
            "alice@example.com",
            "&code_challenge_method=plain",
        )
        .await;
        let no_method = identify_with(&state, &client_id, "alice@example.com", "").await;
        let unknown_client = identify(&state, "no-such-client", "alice@example.com").await;
        let no_email = identify(&state, &client_id, "  ").await;
        let no_challenge = build_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/authorize/identify")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(format!(
                        "email=a%40b.c&client_id={client_id}&redirect_uri=http%3A%2F%2Flocalhost%2Fcb&code_challenge_method=S256"
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        for (what, resp) in [
            ("plain", plain),
            ("missing method", no_method),
            ("unknown client", unknown_client),
            ("blank email", no_email),
            ("missing challenge", no_challenge),
        ] {
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{what}");
        }
    }

    #[tokio::test]
    async fn authorize_get_with_a_login_hint_prefills_the_password_form() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;
        let (_verifier, challenge) = test_pkce();

        let resp = build_app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri=http://localhost/cb&code_challenge={challenge}&code_challenge_method=S256&org=test&login_hint=alice%40example.com"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let html = body_text(resp).await;
        assert!(html.contains(r#"name="org" value="test""#), "{html}");
        assert!(html.contains(r#"value="alice@example.com""#), "{html}");
    }

    #[tokio::test]
    async fn authorize_get_forwards_the_login_hint_to_the_idp() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        let client_id = register_test_client(&state).await;
        let (_verifier, challenge) = test_pkce();

        let resp = build_app(state.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/oauth/authorize?response_type=code&client_id={client_id}&redirect_uri=http://localhost/cb&code_challenge={challenge}&code_challenge_method=S256&org=acme&login_hint=dana%40example.com"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let location = location(&resp);
        assert!(
            location.contains("login_hint=dana%40example.com"),
            "{location}"
        );
    }

    #[tokio::test]
    async fn authorize_get_without_a_login_hint_sends_none_to_the_idp() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        let client_id = register_test_client(&state).await;

        let resp = authorize(&state, &client_id, Some("acme")).await;

        assert!(!location(&resp).contains("login_hint"));
    }

    /// A second password org whose user shares alice's email but not her password.
    async fn seed_namesake_org(state: &OAuthState) {
        test_support::seed_org(&state.org_storage, "org_other", "other", "password").await;
        seed_user(
            state,
            "usr_alice2",
            "org_other",
            "alice@example.com",
            "other-pass",
        )
        .await;
    }

    #[tokio::test]
    async fn password_login_authenticates_against_the_selected_org_only() {
        let state = test_state().await;
        seed_namesake_org(&state).await;
        let client_id = register_test_client(&state).await;

        for (org, password, expected) in [
            ("test", "secret123", StatusCode::SEE_OTHER),
            ("other", "other-pass", StatusCode::SEE_OTHER),
            ("test", "other-pass", StatusCode::UNAUTHORIZED),
            ("other", "secret123", StatusCode::UNAUTHORIZED),
        ] {
            let resp =
                attempt_login_in(&state, &client_id, org, "alice@example.com", password).await;
            assert_eq!(resp.status(), expected, "org={org} password={password}");
        }
    }

    #[tokio::test]
    async fn password_login_issues_the_code_for_the_selected_orgs_user() {
        let state = test_state().await;
        seed_namesake_org(&state).await;
        let client_id = register_test_client(&state).await;

        let resp = attempt_login_in(
            &state,
            &client_id,
            "other",
            "alice@example.com",
            "other-pass",
        )
        .await;

        let code = url::Url::parse(&location(&resp))
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.into_owned())
            .unwrap();
        let codes = state.org_storage.auth_code_store();
        let stored = assistant_core::auth::AuthCodeStore::take_code(&codes, &code)
            .await
            .unwrap()
            .expect("code was stored");
        assert_eq!(stored.user_id, "usr_alice2");
    }

    #[tokio::test]
    async fn password_login_is_refused_for_an_oidc_org() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        seed_user(&state, "usr_carol", "org_acme", "carol@example.com", "pw").await;
        let client_id = register_test_client(&state).await;

        let resp = attempt_login_in(&state, &client_id, "acme", "carol@example.com", "pw").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn password_login_is_refused_for_an_unknown_org() {
        let state = test_state().await;
        let client_id = register_test_client(&state).await;

        let resp = attempt_login_in(
            &state,
            &client_id,
            "no-such-org",
            "alice@example.com",
            "secret123",
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn lockout_in_one_org_does_not_lock_the_namesake_in_another() {
        let (state, _clock) = test_state_with_clock().await;
        seed_namesake_org(&state).await;
        let client_id = register_lockout_client(&state).await;
        for _ in 0..3 {
            attempt_login_in(&state, &client_id, "test", "alice@example.com", "wrong").await;
        }

        let resp = attempt_login_in(
            &state,
            &client_id,
            "other",
            "alice@example.com",
            "other-pass",
        )
        .await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
    }

    // -- Device flow: the password check is scoped to one org --

    /// Start a device flow and return its `(device_code, user_code)`.
    async fn start_device_flow(state: &OAuthState) -> (String, String) {
        let resp = state
            .device_manager
            .initiate("device-client", vec![])
            .await
            .unwrap();
        (resp.device_code, resp.user_code)
    }

    /// POST `/oauth/device/verify`; `org: None` leaves the field out.
    async fn verify_device(
        state: &OAuthState,
        user_code: &str,
        org: Option<&str>,
        email: &str,
        password: &str,
    ) -> axum::response::Response {
        let org_field = org.map(|o| format!("&org={o}")).unwrap_or_default();
        let form = format!(
            "user_code={user_code}&email={}&password={password}{org_field}",
            urlencoding::encode(email)
        );
        build_app(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/oauth/device/verify")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    /// Who approved the device flow, if anyone.
    async fn device_approver(state: &OAuthState, device_code: &str) -> Option<String> {
        match state.device_manager.poll(device_code).await.unwrap() {
            assistant_auth::oauth2::device::PollResult::Authorized { user_id, .. } => Some(user_id),
            _ => None,
        }
    }

    #[tokio::test]
    async fn device_verify_page_asks_for_the_org() {
        let state = test_state().await;

        let resp = build_app(state)
            .oneshot(
                Request::builder()
                    .uri("/oauth/device/verify")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let html = body_text(resp).await;
        assert!(
            html.contains(r#"name="org""#),
            "the form needs an organization field: {html}"
        );
    }

    #[tokio::test]
    async fn device_verify_authenticates_against_the_selected_org_only() {
        let state = test_state().await;
        seed_namesake_org(&state).await;

        for (org, password, expected, approver) in [
            ("test", "secret123", StatusCode::OK, Some("usr_alice")),
            ("other", "other-pass", StatusCode::OK, Some("usr_alice2")),
            ("test", "other-pass", StatusCode::UNAUTHORIZED, None),
            ("other", "secret123", StatusCode::UNAUTHORIZED, None),
        ] {
            let (device_code, user_code) = start_device_flow(&state).await;
            let resp =
                verify_device(&state, &user_code, Some(org), "alice@example.com", password).await;
            assert_eq!(resp.status(), expected, "org={org} password={password}");
            assert_eq!(
                device_approver(&state, &device_code).await.as_deref(),
                approver,
                "org={org} password={password}"
            );
        }
    }

    #[tokio::test]
    async fn device_verify_is_refused_for_an_oidc_org() {
        let state = test_state().await;
        let idp = wiremock::MockServer::start().await;
        test_support::mount_idp(&idp).await;
        test_support::seed_oidc_org(
            &state.org_storage,
            "org_acme",
            "acme",
            &idp.uri(),
            "oidc-client",
        )
        .await;
        seed_user(&state, "usr_carol", "org_acme", "carol@example.com", "pw").await;
        let (device_code, user_code) = start_device_flow(&state).await;

        let resp = verify_device(&state, &user_code, Some("acme"), "carol@example.com", "pw").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(device_approver(&state, &device_code).await, None);
    }

    #[tokio::test]
    async fn device_verify_is_refused_for_an_unknown_org() {
        let state = test_state().await;
        let (device_code, user_code) = start_device_flow(&state).await;

        let resp = verify_device(
            &state,
            &user_code,
            Some("no-such-org"),
            "alice@example.com",
            "secret123",
        )
        .await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(device_approver(&state, &device_code).await, None);
    }

    #[tokio::test]
    async fn device_verify_requires_an_org() {
        let state = test_state().await;
        let (device_code, user_code) = start_device_flow(&state).await;

        let resp = verify_device(&state, &user_code, None, "alice@example.com", "secret123").await;

        assert!(
            resp.status().is_client_error(),
            "a submission without an org must be rejected, got {}",
            resp.status()
        );
        assert_eq!(device_approver(&state, &device_code).await, None);
    }

    #[tokio::test]
    async fn device_verify_locks_out_one_orgs_account_only() {
        let (state, _clock) = test_state_with_clock().await;
        seed_namesake_org(&state).await;
        let (_, user_code) = start_device_flow(&state).await;
        for _ in 0..3 {
            let resp = verify_device(
                &state,
                &user_code,
                Some("test"),
                "alice@example.com",
                "wrong",
            )
            .await;
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        let locked = verify_device(
            &state,
            &user_code,
            Some("test"),
            "alice@example.com",
            "secret123",
        )
        .await;
        assert_eq!(locked.status(), StatusCode::TOO_MANY_REQUESTS);

        let (device_code, user_code) = start_device_flow(&state).await;
        let other = verify_device(
            &state,
            &user_code,
            Some("other"),
            "alice@example.com",
            "other-pass",
        )
        .await;
        assert_eq!(other.status(), StatusCode::OK);
        assert_eq!(
            device_approver(&state, &device_code).await.as_deref(),
            Some("usr_alice2")
        );
    }
}
