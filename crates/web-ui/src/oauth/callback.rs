//! GET /oauth/callback — OIDC IdP callback.
//!
//! After the user authenticates at the external IdP, the IdP redirects here
//! with an authorization code.  We exchange it for an id_token, validate,
//! provision/lookup the user, generate our own auth code, and redirect to the
//! original client's redirect_uri.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use serde::Deserialize;

use super::OAuthState;
use super::oidc_bridge::OrgOidcUserStore;

/// Query parameters the IdP sends to our callback endpoint.
#[derive(Deserialize, utoipa::IntoParams)]
pub struct CallbackQuery {
    /// Authorization code from the IdP.
    pub code: Option<String>,
    /// The `state` value we sent when redirecting to the IdP.
    pub state: Option<String>,
    /// Error code (if the IdP denied the request).
    pub error: Option<String>,
    /// Human-readable error description from the IdP.
    pub error_description: Option<String>,
}

/// GET /oauth/callback — handle the IdP redirect.
#[utoipa::path(
    get,
    path = "/oauth/callback",
    tag = "oauth",
    security(()),
    params(CallbackQuery),
    responses(
        (status = 303, description = "Redirect to client with authorization code"),
        (status = 400, description = "Invalid or missing parameters"),
        (status = 500, description = "Internal error during token exchange"),
    )
)]
pub async fn callback(
    State(state): State<OAuthState>,
    Query(params): Query<CallbackQuery>,
) -> impl IntoResponse {
    // If the IdP returned an error, surface it.
    if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or_default();
        return (
            StatusCode::BAD_REQUEST,
            Html(format!("OIDC error: {err} — {desc}")),
        )
            .into_response();
    }

    let code = match params.code {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Html("missing 'code' parameter from IdP".to_string()),
            )
                .into_response();
        }
    };

    let oidc_state = match params.state {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Html("missing 'state' parameter from IdP".to_string()),
            )
                .into_response();
        }
    };

    let pending = match state.oidc_sessions.take(&oidc_state).await {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Html("unknown or expired OIDC state parameter".to_string()),
            )
                .into_response();
        }
    };

    let oidc = match state.oidc.provider_for(&pending.org_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::error!(org_id = %pending.org_id, "OIDC callback for an org without OIDC settings");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("OIDC provider not configured".to_string()),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(org_id = %pending.org_id, "OIDC provider unavailable: {e:#}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("OIDC provider unavailable".to_string()),
            )
                .into_response();
        }
    };

    // Build our callback URL (the redirect_uri we told the IdP about).
    let our_callback = format!("{}/oauth/callback", state.issuer);

    // Exchange the IdP auth code for an id_token.
    let claims = match oidc.exchange_code(&code, &our_callback).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("OIDC code exchange failed: {e:#}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("failed to exchange authorization code with IdP".to_string()),
            )
                .into_response();
        }
    };

    // Provision or look up the user in the org that started the flow.
    let user_store = OrgOidcUserStore::new(state.org_storage.clone(), pending.org_id.clone());
    let user_id = match oidc.provision_or_lookup(&claims, &user_store).await {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("OIDC user provisioning failed: {e:#}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("failed to provision or look up user".to_string()),
            )
                .into_response();
        }
    };

    // Generate our own authorization code for the original client.
    let scopes: Vec<String> = pending
        .scope
        .unwrap_or_default()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let our_code = match state
        .oauth2_server
        .generate_auth_code(
            &user_id.0,
            &pending.client_id,
            &pending.redirect_uri,
            pending.code_challenge.as_deref(),
            scopes,
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("auth code generation failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("failed to generate authorization code".to_string()),
            )
                .into_response();
        }
    };

    // Redirect back to the original client's redirect_uri with our auth code.
    let mut redirect = pending.redirect_uri;
    redirect.push_str(if redirect.contains('?') { "&" } else { "?" });
    redirect.push_str(&format!("code={}", urlencoding::encode(&our_code)));
    if let Some(ref st) = pending.client_state {
        redirect.push_str(&format!("&state={}", urlencoding::encode(st)));
    }

    Redirect::to(&redirect).into_response()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use assistant_auth::jwt::{JwtKeyPair, JwtManager};
    use assistant_auth::oidc::{IdTokenAudience, IdTokenClaims};
    use assistant_core::identity::OrgId;
    use assistant_core::store::UserStore;

    use super::super::oidc_registry::OidcProviderRegistry;
    use super::super::oidc_sessions::{PendingOidcSession, PendingOidcSessionStore};
    use super::super::test_support::{
        mount_idp, seed_oidc_org, seed_org, sign_test_id_token, test_secrets,
    };
    use super::*;

    /// Build an `OAuthState` with no orgs; tests seed the orgs they need.
    async fn test_oidc_state() -> (OAuthState, Arc<PendingOidcSessionStore>) {
        let org_storage = Arc::new(
            assistant_storage::OrgStorageLayer::new_in_memory()
                .await
                .unwrap(),
        );

        let jwt_kp = JwtKeyPair::generate().unwrap();
        let jwt_manager = Arc::new(JwtManager::new(
            jwt_kp,
            "http://localhost".into(),
            "http://localhost".into(),
        ));

        let oauth2_server = Arc::new(assistant_auth::oauth2::server::OAuth2Server::new(
            Arc::new(org_storage.auth_code_store()),
            Arc::new(org_storage.refresh_token_store()),
        ));

        let client_registrar = Arc::new(assistant_auth::oauth2::clients::ClientRegistrar::new(
            Arc::new(org_storage.client_store()),
        ));

        let device_manager = Arc::new(assistant_auth::oauth2::device::DeviceCodeManager::new(
            Arc::new(org_storage.device_code_store()),
            "http://localhost/oauth/device/verify".into(),
        ));

        let sessions = Arc::new(PendingOidcSessionStore::new(Duration::from_secs(300)));

        let state = OAuthState {
            oauth2_server,
            jwt_manager,
            client_registrar,
            device_manager,
            org_storage: org_storage.clone(),
            issuer: "http://localhost".into(),
            secure_cookie: false,
            oidc: Arc::new(OidcProviderRegistry::new(org_storage, test_secrets())),
            oidc_sessions: sessions.clone(),
            rate_limits: Arc::new(crate::rate_limit::RateLimits::new(
                Arc::new(assistant_core::clock::SystemClock),
                &crate::rate_limit::RateLimitArgs::default(),
            )),
        };

        (state, sessions)
    }

    fn build_callback_app(state: OAuthState) -> Router {
        Router::new()
            .route("/oauth/callback", get(callback))
            .with_state(state)
    }

    /// Start a mock IdP with discovery, JWKS, and a token endpoint that
    /// returns an ID token for `email` addressed to `client_id`.
    async fn idp_issuing_token_for(client_id: &str, email: &str) -> MockServer {
        let server = MockServer::start().await;
        mount_idp(&server).await;

        let claims = IdTokenClaims {
            iss: server.uri(),
            sub: "idp-user-42".into(),
            aud: IdTokenAudience::Single(client_id.into()),
            exp: chrono::Utc::now().timestamp() + 3600,
            iat: chrono::Utc::now().timestamp(),
            nonce: None,
            email: Some(email.into()),
            email_verified: Some(true),
            name: Some("Alice".into()),
        };
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "idp-access-token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "id_token": sign_test_id_token(&claims),
            })))
            .mount(&server)
            .await;
        server
    }

    /// Register a client and park a pending session for `org_id`; returns the
    /// `state` value the IdP would echo back.
    async fn park_session(
        state: &OAuthState,
        sessions: &PendingOidcSessionStore,
        org_id: &str,
    ) -> String {
        let client_info = state
            .client_registrar
            .register(assistant_core::auth::ClientRegistration {
                client_name: "Test App".into(),
                redirect_uris: vec!["http://localhost:9999/cb".into()],
                grant_types: vec!["authorization_code".into()],
                response_types: Some(vec!["code".into()]),
                token_endpoint_auth_method: None,
            })
            .await
            .unwrap();

        sessions
            .insert(PendingOidcSession::new(
                OrgId::from(org_id),
                client_info.client_id,
                "http://localhost:9999/cb".into(),
                None,
                Some("client-state-xyz".into()),
                None,
            ))
            .await
    }

    async fn call_callback(state: OAuthState, oidc_state: &str) -> axum::response::Response {
        build_callback_app(state)
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/oauth/callback?code=idp-auth-code&state={oidc_state}"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn callback_exchanges_code_and_redirects() {
        let (state, sessions) = test_oidc_state().await;
        let idp = idp_issuing_token_for("test-client", "alice@example.com").await;
        seed_oidc_org(&state.org_storage, "org_a", "a", &idp.uri(), "test-client").await;
        let oidc_state = park_session(&state, &sessions, "org_a").await;

        let resp = call_callback(state, &oidc_state).await;

        assert_eq!(
            resp.status(),
            StatusCode::SEE_OTHER,
            "callback should redirect (303)"
        );
        let location = resp.headers().get("location").unwrap().to_str().unwrap();
        assert!(
            location.starts_with("http://localhost:9999/cb?code="),
            "redirect should point to client redirect_uri with code: {location}"
        );
        assert!(
            location.contains("state=client-state-xyz"),
            "redirect should forward the client state: {location}"
        );
    }

    #[tokio::test]
    async fn callback_signs_user_into_the_pending_sessions_org() {
        let (state, sessions) = test_oidc_state().await;
        let idp_a = idp_issuing_token_for("client-a", "alice@example.com").await;
        let idp_b = idp_issuing_token_for("client-b", "alice@example.com").await;
        seed_oidc_org(&state.org_storage, "org_a", "a", &idp_a.uri(), "client-a").await;
        seed_oidc_org(&state.org_storage, "org_b", "b", &idp_b.uri(), "client-b").await;
        let oidc_state = park_session(&state, &sessions, "org_b").await;

        let resp = call_callback(state.clone(), &oidc_state).await;

        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        let users = state.org_storage.user_store();
        assert!(
            users
                .get_user_by_email(&OrgId::from("org_b"), "alice@example.com")
                .await
                .unwrap()
                .is_some(),
            "user is provisioned in org B"
        );
        assert!(
            users
                .get_user_by_email(&OrgId::from("org_a"), "alice@example.com")
                .await
                .unwrap()
                .is_none(),
            "org A is untouched"
        );
        let token_calls_on_a = idp_a
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/token")
            .count();
        assert_eq!(token_calls_on_a, 0, "org A's IdP never saw the code");
    }

    #[tokio::test]
    async fn callback_fails_when_the_session_org_has_no_oidc_config() {
        let (state, sessions) = test_oidc_state().await;
        seed_org(&state.org_storage, "org_a", "a", "oidc").await;
        let oidc_state = park_session(&state, &sessions, "org_a").await;

        let resp = call_callback(state, &oidc_state).await;

        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn callback_rejects_unknown_state() {
        let (state, _sessions) = test_oidc_state().await;

        let resp = call_callback(state, "bogus").await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn callback_rejects_missing_code() {
        let (state, _sessions) = test_oidc_state().await;

        let resp = build_callback_app(state)
            .oneshot(
                Request::builder()
                    .uri("/oauth/callback?state=abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn callback_surfaces_idp_error() {
        let (state, _sessions) = test_oidc_state().await;

        let resp = build_callback_app(state)
            .oneshot(
                Request::builder()
                    .uri("/oauth/callback?error=access_denied&error_description=user+cancelled")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("access_denied"));
    }
}
