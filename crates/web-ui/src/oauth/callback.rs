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

use assistant_core::identity::OrgId;

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

    // Look up the pending OIDC session.
    let sessions = match &state.oidc_sessions {
        Some(s) => s,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("OIDC sessions not configured".to_string()),
            )
                .into_response();
        }
    };

    let pending = match sessions.take(&oidc_state).await {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Html("unknown or expired OIDC state parameter".to_string()),
            )
                .into_response();
        }
    };

    let oidc = match &state.oidc_provider {
        Some(p) => p,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("OIDC provider not configured".to_string()),
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

    // Provision or look up the user.
    // TODO: resolve org_id from OIDC config or pending session rather than
    // hardcoding "default".  For now single-org deployments use "default".
    let user_store = OrgOidcUserStore::new(state.org_storage.clone(), OrgId::from("default"));
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

    use assistant_auth::jwt::JwtKeyPair;
    use assistant_auth::jwt::JwtManager;
    use assistant_auth::oidc::{
        IdTokenAudience, IdTokenClaims, JwksResponse, OidcConfig, OidcDiscovery, OidcProvider,
    };
    use assistant_core::identity::OrgId;
    use assistant_core::store::{OrgStore, Organization};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::oidc_sessions::{PendingOidcSession, PendingOidcSessionStore};
    use super::*;

    // -- RSA test key (same as in oidc.rs tests) --

    const TEST_RSA_PRIVATE_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDfk5v7uyKn7Xee
tzR7ZRsra2drSEZHQYOHfIv8bm2zSrHrFdUqmTL0t6mj1jPYXWNBVZCyiAzL2qiN
dddT3kNBdpI80aMpIT3u/sOa5I0hq11nGAKeezGLqizOPcztgZQa3FYFve5HxnXT
K9UkboBqhvIXKUx8fZsO8sFHdscS4VGrNn9VLPz8UjEOmJsJEiaqSjw4qx+1WsBj
xTlQNqV/y/durbTCyNSUlRex1C4PEHGg+rr/yobUk9MqxGHNeKefKkSaDcij72lz
osvxelXHLoonV54X9VrpMnXdWAw7yJ6LbWsA9MVoEiqDfnDFWcaFsfQeCq+1/Hnd
b9jdEJPJAgMBAAECggEAYsYKSBzlUy44xkBnKcrBxZ12O7HbBpj9fGp8R+IbifXK
g7MKEX9MQUww4IZ+Mi0T8CXWvuEXUiqAg7qXjmBn8zBorADr5fxfKcqY7UHizgiw
w56abZy8h1j/4X/xHM69+V31jSTbdA9MN6aqTCWbizSiGLRwq6EsU17RH/rsOTzJ
yiap+GgvDBHRSMLPbH+A7A9hNZh6I6Dv9KReGUmOEKVQpl9aetJfxsCkXII7ztUL
HUQNZVluA2OqR1s7VpYO9yNmZoTtfqmUswIg2YaFXSjVgdDFNH6Rf3reufWehzKP
45ufflne4FT3f8VFXe8+FonCZE5RF2tIawC2gusWfwKBgQD1vX9Q2V5wKg/nQq/6
vVv370XmypVBajXvg0TP95E7p+EORGoDYj46LWNKZRifeGjavmM29jiqPnk8UZjC
3qEmZILZC2/IdsR7hVpEn7uyedc2wBFSD5pYR9333VsV+rbBMmexO8DdviR8L4Tr
8ESx+FsVnqkhBgGjQWjXOapf5wKBgQDo6Tlu1VZQ1ijLWYomeKDiKQloEHMSW3lP
vgd1XqTYLrR7kv6935cK6dY0ZvcfA4HE6+RU4pene/vU6bMxvSJjwNwhRhOS7CGl
k+5Mtdfq3+VhaXA5dmyYu2IrJ3WJg+9QjxAodek4kYx1W8MLJnKc2GzIuED+ewg0
PuMXTcm4zwKBgC+z820MZSq8341zAppX++xrRFSC6uph5cpy3v7H/idodWXBnhq+
DXpZqTad3WPHigM8hiH7NhDGQ96TsGXTtdCwHj5n2/E8LPQVdOpxX4xL3p1AN5yI
btvIR6yACdiAbM2gLUTYZp4k9QwuZU0vvQYXQgc2X3qLofHBFssA5LPtAoGAI3/w
zhDcQCP0QdJa+TQnqXEBywe+0kx4+AuJzXzoeT7dKXylMUGUHwi3KnOLNQHu1Jnz
ynBjFxcRskkQlAM066loo/WvZBRzqG4cwzpwN496wdc1ULzZHoppExTHmHcwkcHM
f65BJusgUn7zAo8QpxFhu1JCLceI35W6PUIQ/gcCgYEA34AmU6srEH6iJx7wFQez
vsldAzUeGESZCNai30KZPeUrFxGHNcWU72xkBJnHnZCcAejKtW/XqKd4AWY25buV
6vNdaNWeyJt50s+E8VTfGi2otEAiZh+dJxAmgLJ/PA0RDyBft2qGkKMSfS2PXdIr
q2HMPOhMSkWZ+rP+jJ76H6s=
-----END PRIVATE KEY-----"#;

    const TEST_RSA_N: &str = "35Ob-7sip-13nrc0e2UbK2tna0hGR0GDh3yL_G5ts0qx6xXVKpky9Lepo9Yz2F1jQVWQsogMy9qojXXXU95DQXaSPNGjKSE97v7DmuSNIatdZxgCnnsxi6oszj3M7YGUGtxWBb3uR8Z10yvVJG6AaobyFylMfH2bDvLBR3bHEuFRqzZ_VSz8_FIxDpibCRImqko8OKsftVrAY8U5UDalf8v3bq20wsjUlJUXsdQuDxBxoPq6_8qG1JPTKsRhzXinnypEmg3Io-9pc6LL8XpVxy6KJ1eeF_Va6TJ13VgMO8iei21rAPTFaBIqg35wxVnGhbH0Hgqvtfx53W_Y3RCTyQ";
    const TEST_RSA_E: &str = "AQAB";

    fn sign_test_id_token(claims: &IdTokenClaims) -> String {
        let encoding_key =
            jsonwebtoken::EncodingKey::from_rsa_pem(TEST_RSA_PRIVATE_PEM.as_bytes()).unwrap();
        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some("test-key-1".into());
        jsonwebtoken::encode(&header, claims, &encoding_key).unwrap()
    }

    fn test_jwk() -> assistant_auth::oidc::Jwk {
        assistant_auth::oidc::Jwk {
            kty: "RSA".into(),
            kid: Some("test-key-1".into()),
            alg: Some("RS256".into()),
            key_use: Some("sig".into()),
            n: Some(TEST_RSA_N.into()),
            e: Some(TEST_RSA_E.into()),
            crv: None,
            x: None,
            y: None,
        }
    }

    /// Set up a mock OIDC IdP (discovery, JWKS, token endpoint) and return
    /// `(OidcProvider, MockServer)`.
    async fn setup_idp() -> (OidcProvider, MockServer) {
        let server = MockServer::start().await;
        let issuer = server.uri();
        let jwks_uri = format!("{issuer}/jwks");

        let discovery = OidcDiscovery {
            issuer: issuer.clone(),
            authorization_endpoint: format!("{issuer}/authorize"),
            token_endpoint: format!("{issuer}/token"),
            userinfo_endpoint: None,
            jwks_uri,
            response_types_supported: vec!["code".into()],
            subject_types_supported: vec!["public".into()],
            id_token_signing_alg_values_supported: vec!["RS256".into()],
        };

        let jwks = JwksResponse {
            keys: vec![test_jwk()],
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

        let config = OidcConfig {
            issuer_url: issuer,
            client_id: "test-client".into(),
            client_secret: None,
            auto_provision: true,
            allowed_email_domains: None,
        };

        let provider = OidcProvider::discover(config).await.unwrap();
        (provider, server)
    }

    /// Build an `OAuthState` with OIDC support backed by the mock IdP.
    async fn test_oidc_state(provider: OidcProvider) -> (OAuthState, Arc<PendingOidcSessionStore>) {
        let org_storage = Arc::new(
            assistant_storage::OrgStorageLayer::new_in_memory()
                .await
                .unwrap(),
        );

        // Seed a test org.
        let now = chrono::Utc::now();
        org_storage
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("default"),
                name: "Default".into(),
                slug: "default".into(),
                auth_mode: "oidc".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();

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
            org_storage,
            issuer: "http://localhost".into(),
            secure_cookie: false,
            oidc_provider: Some(Arc::new(provider)),
            oidc_sessions: Some(sessions.clone()),
        };

        (state, sessions)
    }

    fn build_callback_app(state: OAuthState) -> Router {
        Router::new()
            .route("/oauth/callback", get(callback))
            .with_state(state)
    }

    #[tokio::test]
    async fn callback_exchanges_code_and_redirects() {
        let (provider, server) = setup_idp().await;
        let (state, sessions) = test_oidc_state(provider).await;

        // Register a client so the auth code can reference it.
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

        // Insert a pending OIDC session.
        let pending = PendingOidcSession::new(
            client_info.client_id.clone(),
            "http://localhost:9999/cb".into(),
            None,
            Some("client-state-xyz".into()),
            None,
        );
        let oidc_state_param = sessions.insert(pending).await;

        // Mock the IdP token endpoint to return a signed id_token.
        let claims = IdTokenClaims {
            iss: server.uri(),
            sub: "idp-user-42".into(),
            aud: IdTokenAudience::Single("test-client".into()),
            exp: chrono::Utc::now().timestamp() + 3600,
            iat: chrono::Utc::now().timestamp(),
            nonce: None,
            email: Some("alice@example.com".into()),
            email_verified: Some(true),
            name: Some("Alice".into()),
        };
        let id_token = sign_test_id_token(&claims);

        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "idp-access-token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "id_token": id_token,
            })))
            .mount(&server)
            .await;

        let app = build_callback_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri(&format!(
                        "/oauth/callback?code=idp-auth-code&state={oidc_state_param}"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

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
    async fn callback_rejects_unknown_state() {
        let (provider, _server) = setup_idp().await;
        let (state, _sessions) = test_oidc_state(provider).await;

        let app = build_callback_app(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/oauth/callback?code=some-code&state=bogus")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn callback_rejects_missing_code() {
        let (provider, _server) = setup_idp().await;
        let (state, _sessions) = test_oidc_state(provider).await;

        let app = build_callback_app(state);
        let resp = app
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
        let (provider, _server) = setup_idp().await;
        let (state, _sessions) = test_oidc_state(provider).await;

        let app = build_callback_app(state);
        let resp = app
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
