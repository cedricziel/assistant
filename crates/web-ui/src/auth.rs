//! Authentication middleware for the web UI.
//!
//! Resolves an [`AuthContext`] from incoming requests using (in order):
//!
//! 1. `Authorization: Bearer <JWT>` — validated by [`JwtManager`]
//! 2. Session cookie containing a JWT
//! 3. `Authorization: Bearer <api_key>` — resolved via [`ApiKeyStore`]
//! 4. `Authorization: Bearer <legacy_token>` — backward-compatible static token
//!
//! The resolved [`AuthContext`] is injected as an [`Extension`] so all
//! downstream handlers can extract it.
//!
//! The legacy token (`ASSISTANT_WEB_TOKEN`) is supported as a migration bridge:
//! it maps to a default org-admin context so existing single-user deployments
//! continue to work without reconfiguration.

use std::sync::Arc;

use axum::Form;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{COOKIE, HOST, LOCATION, ORIGIN, REFERER, SET_COOKIE};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use assistant_auth::api_keys::{self, ApiKeyStore};
use assistant_auth::jwt::JwtManager;
use assistant_auth::middleware::{AuthState, RequireScope};
use assistant_core::auth::AuthContext;
use assistant_core::identity::{Action, ResourceKind, SpaceId};

// -- Configuration ----------------------------------------------------------

/// Session cookie name.
const SESSION_COOKIE: &str = "assistant_session";

/// Shared authentication configuration for the web UI.
///
/// Wraps [`AuthState`] (JWT + API key + legacy token resolution) and adds
/// web-specific settings like cookie attributes.
#[derive(Clone)]
pub struct WebAuthConfig {
    /// Auth state for token resolution (JWT, API key, legacy).
    pub auth_state: AuthState,
    /// When `true`, the `Secure` attribute is added to session cookies.
    pub secure_cookie: bool,
}

impl WebAuthConfig {
    /// Create a new [`WebAuthConfig`].
    ///
    /// - `jwt_manager`: signs and validates JWTs.
    /// - `api_key_store`: resolves `ask_live_` API keys.
    /// - `legacy_token`: optional backward-compatible static token.
    /// - `legacy_context`: [`AuthContext`] returned when the legacy token matches.
    /// - `secure_cookie`: set `true` for non-loopback addresses (HTTPS required).
    pub fn new(
        jwt_manager: Arc<JwtManager>,
        api_key_store: Arc<dyn ApiKeyStore>,
        legacy_token: Option<String>,
        legacy_context: Option<AuthContext>,
        secure_cookie: bool,
    ) -> Self {
        Self {
            auth_state: AuthState {
                jwt_manager,
                api_key_store,
                legacy_token,
                legacy_context,
            },
            secure_cookie,
        }
    }
}

// -- Token resolution -------------------------------------------------------

/// Try to resolve an [`AuthContext`] from a bearer token string.
///
/// Resolution order: JWT → API key → legacy token.
async fn resolve_bearer(auth: &AuthState, token: &str) -> Option<AuthContext> {
    // 1. Try JWT.
    if let Ok(ctx) = auth.jwt_manager.validate_to_context(token) {
        return Some(ctx);
    }

    // 2. Try API key.
    if token.starts_with("ask_live_")
        && let Ok(ctx) = api_keys::resolve_key(token, auth.api_key_store.as_ref()).await
    {
        return Some(ctx);
    }

    // 3. Try legacy token.
    if let Some(ref legacy) = auth.legacy_token
        && token == legacy
        && let Some(ref ctx) = auth.legacy_context
    {
        return Some(ctx.clone());
    }

    None
}

// -- Middleware --------------------------------------------------------------

/// Axum middleware that enforces authentication on every matched route.
///
/// Resolves an [`AuthContext`] from Bearer tokens or session cookies and
/// inserts it as an [`Extension<AuthContext>`] for downstream handlers.
///
/// Unauthenticated browser requests are redirected to `/login`.
/// Unauthenticated API requests receive `401 Unauthorized`.
pub async fn require_auth(
    State(config): State<WebAuthConfig>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth = config.auth_state.clone();

    // Extract bearer token and session cookie JWT before any async work,
    // so we hold only owned values across .await points.
    let bearer_token: Option<String> = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_string());

    let cookie_jwt: Option<String> = request
        .headers()
        .get(COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| extract_cookie(cookies, SESSION_COOKIE))
        .map(|s| s.to_string());

    // 1. Check Authorization: Bearer <token>.
    if let Some(ref token) = bearer_token
        && let Some(ctx) = resolve_bearer(&auth, token).await
    {
        request.extensions_mut().insert(ctx);
        return next.run(request).await;
    }

    // 2. Check session cookie for a JWT.
    if let Some(ref jwt) = cookie_jwt
        && let Ok(ctx) = auth.jwt_manager.validate_to_context(jwt)
    {
        request.extensions_mut().insert(ctx);
        return next.run(request).await;
    }

    // 3. Not authenticated — decide response type.
    let accepts_html = request
        .headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);

    if accepts_html {
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, "/login")
            .body(Body::empty())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header("WWW-Authenticate", "Bearer")
            .body(Body::from("Unauthorized"))
            .unwrap()
    }
}

/// Middleware that enforces same-origin checks for cookie-authenticated mutations.
///
/// Browser CSRF relies on ambient cookies on cross-site requests. For state-changing
/// methods we reject requests whose `Origin`/`Referer` do not match `Host`.
/// Bearer-authenticated API requests are exempt from this check.
pub async fn require_same_origin_mutation(request: Request, next: Next) -> Response {
    match *request.method() {
        Method::GET | Method::HEAD | Method::OPTIONS => return next.run(request).await,
        _ => {}
    }

    if request
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("Bearer "))
        .unwrap_or(false)
    {
        return next.run(request).await;
    }

    let Some(host) = request.headers().get(HOST).and_then(|h| h.to_str().ok()) else {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Forbidden"))
            .unwrap();
    };

    let origin_host = request
        .headers()
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_host_from_url)
        .or_else(|| {
            request
                .headers()
                .get(REFERER)
                .and_then(|value| value.to_str().ok())
                .and_then(parse_host_from_url)
        });

    let Some(origin_host) = origin_host else {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Forbidden"))
            .unwrap();
    };

    if !origin_host.eq_ignore_ascii_case(host) {
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::from("Forbidden"))
            .unwrap();
    }

    next.run(request).await
}

// -- Permission guard -------------------------------------------------------

/// Tower [`Layer`] that checks whether the authenticated user has the required
/// scope (space + resource + action) before passing the request to the inner
/// service.
///
/// Must be applied **after** [`require_auth`] in the middleware stack so that
/// `Extension<AuthContext>` is present in the request extensions.
///
/// Returns 401 if no [`AuthContext`] is found, 403 if the scope check fails.
///
/// # Usage
///
/// ```ignore
/// Router::new()
///     .route("/personas", get(list))
///     .route_layer(require_scope_layer(
///         SpaceId::from("eng"),
///         ResourceKind::Personas,
///         Action::Read,
///     ))
/// ```
pub fn require_scope_layer(
    space_id: SpaceId,
    resource: ResourceKind,
    action: Action,
) -> RequireScopeLayer {
    RequireScopeLayer {
        scope: RequireScope {
            space_id,
            resource,
            action,
            target_id: None,
        },
    }
}

/// A [`tower::Layer`] produced by [`require_scope_layer`].
#[derive(Clone)]
pub struct RequireScopeLayer {
    scope: RequireScope,
}

impl<S> tower::Layer<S> for RequireScopeLayer {
    type Service = RequireScopeService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireScopeService {
            inner,
            scope: self.scope.clone(),
        }
    }
}

/// Tower [`Service`] that enforces a scope check before forwarding to the
/// inner service.
#[derive(Clone)]
pub struct RequireScopeService<S> {
    inner: S,
    scope: RequireScope,
}

impl<S> tower::Service<Request> for RequireScopeService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = S::Error;
    type Future =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, S::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let scope = self.scope.clone();
        let mut inner = self.inner.clone();
        // Swap so the ready inner is used (standard tower pattern).
        std::mem::swap(&mut self.inner, &mut inner);

        Box::pin(async move {
            let Some(ctx) = request.extensions().get::<AuthContext>() else {
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header("WWW-Authenticate", "Bearer")
                    .body(Body::from("Unauthorized"))
                    .unwrap());
            };

            if let Err(err) = scope.check(ctx) {
                return Ok((
                    StatusCode::FORBIDDEN,
                    format!(
                        "insufficient permissions: requires {}:{} in space {}",
                        err.resource, err.action, err.space_id,
                    ),
                )
                    .into_response());
            }

            inner.call(request).await
        })
    }
}

// -- Login page -------------------------------------------------------------

fn login_html(error: Option<&str>) -> Html<String> {
    let error_block = match error {
        Some(msg) => {
            format!(r#"<div class="login-error" id="login-error" role="alert">{msg}</div>"#)
        }
        None => String::new(),
    };
    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>Login - Assistant</title>
  <style>
    body{{margin:0;background:#030712;color:#e5e9f0;font-family:system-ui,sans-serif}}
    .login-container{{display:flex;align-items:center;justify-content:center;min-height:100vh;padding:2rem}}
    .login-card{{background:#0a1628;border:1px solid #0b1b32;border-radius:12px;padding:2.5rem;width:100%;max-width:400px}}
    .login-brand p{{text-transform:uppercase;letter-spacing:.2em;color:#7aa2ff;margin:0 0 .2rem;font-size:.75rem}}
    .login-brand h2{{margin:0;color:#e5e9f0}}
    .login-error{{background:rgba(239,68,68,.15);border:1px solid rgba(239,68,68,.3);color:#fca5a5;padding:.75rem 1rem;border-radius:8px;margin-bottom:1.5rem;font-size:.9rem}}
    .login-form{{display:flex;flex-direction:column;gap:.75rem;margin-top:1.5rem}}
    .login-form label{{font-size:.85rem;color:#8aa5d8;text-transform:uppercase;letter-spacing:.08em}}
    .login-form input{{background:#030712;border:1px solid #1a2744;border-radius:8px;padding:.7rem 1rem;color:#e5e9f0;font-size:1rem;outline:none}}
    .login-form input:focus{{border-color:#6ec6ff}}
    .login-form button{{background:#2563eb;color:#fff;border:none;border-radius:8px;padding:.7rem;font-size:1rem;cursor:pointer;margin-top:.5rem}}
    .login-form button:hover{{background:#1d4ed8}}
  </style>
</head>
<body>
  <div class="login-container">
    <div class="login-card">
      <div class="login-brand"><p>assistant</p><h2>Agent Manager</h2></div>
      {error_block}
      <form method="POST" action="/login" class="login-form">
        <label for="token">Auth Token</label>
        <input type="password" id="token" name="token" placeholder="Enter your token" required autofocus/>
        <button type="submit">Sign in</button>
      </form>
    </div>
  </div>
</body>
</html>"#
    ))
}

/// `GET /login` — render the login form.
pub async fn login_page() -> Html<String> {
    login_html(None)
}

/// `POST /login` — validate the submitted token and set a session cookie (JWT).
#[derive(Deserialize)]
pub(crate) struct LoginForm {
    token: String,
}

pub(crate) async fn login_submit(
    State(config): State<WebAuthConfig>,
    Form(form): Form<LoginForm>,
) -> Response {
    let auth = &config.auth_state;

    // Resolve the token (could be a legacy token, JWT, or API key).
    let ctx = resolve_bearer(auth, &form.token).await;

    match ctx {
        Some(ctx) => {
            // Sign a session JWT for the cookie using the resolved context's identity.
            let display = if ctx.email.is_empty() {
                ctx.user_id.to_string()
            } else {
                ctx.email.clone()
            };
            let jwt = match auth.jwt_manager.sign(&ctx, ctx.org_id.as_ref(), &display) {
                Ok(t) => t,
                Err(_) => return login_html(Some("Internal error.")).into_response(),
            };

            let ttl = auth.jwt_manager.access_ttl_secs();
            let secure = if config.secure_cookie { "; Secure" } else { "" };
            let cookie = format!(
                "{}={}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{}",
                SESSION_COOKIE, jwt, ttl, secure,
            );
            Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(LOCATION, "/")
                .header(SET_COOKIE, cookie)
                .body(Body::empty())
                .unwrap()
        }
        None => login_html(Some("Invalid token.")).into_response(),
    }
}

/// `POST /logout` — clear the session cookie and redirect to login.
pub async fn logout(State(config): State<WebAuthConfig>) -> Response {
    let secure = if config.secure_cookie { "; Secure" } else { "" };
    let cookie = format!(
        "{}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0{}",
        SESSION_COOKIE, secure,
    );
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(LOCATION, "/login")
        .header(SET_COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

// -- Helpers ----------------------------------------------------------------

/// Extract a named cookie value from a `Cookie` header string.
fn extract_cookie<'a>(cookies: &'a str, name: &str) -> Option<&'a str> {
    for pair in cookies.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(name) {
            let value = value.trim_start();
            if let Some(value) = value.strip_prefix('=') {
                return Some(value.trim());
            }
        }
    }
    None
}

fn parse_host_from_url(value: &str) -> Option<String> {
    let value = value.trim();
    let value = value
        .strip_prefix("http://")
        .or_else(|| value.strip_prefix("https://"))?;
    let host = value.split('/').next()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::Extension;
    use axum::Router;
    use axum::routing::get;
    use tower::ServiceExt;

    use assistant_auth::api_keys::InMemoryApiKeyStore;
    use assistant_auth::jwt::JwtKeyPair;
    use assistant_core::identity::{OrgId, Role, Scope, SpaceId, UserId};

    use super::*;

    /// Build a [`WebAuthConfig`] backed by in-memory stores with a shared
    /// JWT secret so we can sign tokens in tests.
    fn test_config() -> (WebAuthConfig, Arc<JwtManager>) {
        let secret = b"test-secret-that-is-long-enough!!";
        let kp_state = JwtKeyPair::from_secret(secret);
        let kp_sign = JwtKeyPair::from_secret(secret);

        let jwt_state = Arc::new(JwtManager::new(
            kp_state,
            "https://test.local".into(),
            "https://test.local".into(),
        ));
        let jwt_sign = Arc::new(JwtManager::new(
            kp_sign,
            "https://test.local".into(),
            "https://test.local".into(),
        ));

        let config = WebAuthConfig::new(
            jwt_state,
            Arc::new(InMemoryApiKeyStore::new()),
            Some("legacy-token-123".into()),
            Some(make_admin_context()),
            false,
        );

        (config, jwt_sign)
    }

    fn make_admin_context() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("default"), Role::OrgAdmin);
        AuthContext {
            user_id: UserId::from("admin"),
            org_id: OrgId::from("default"),
            email: "admin@local".into(),
            space_roles,
            scopes: vec![Scope::new(
                assistant_core::identity::ResourceKind::Org,
                assistant_core::identity::Action::Manage,
            )],
            client_id: "legacy".into(),
        }
    }

    fn make_user_context() -> AuthContext {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        AuthContext {
            user_id: UserId::from("alice"),
            org_id: OrgId::from("acme"),
            email: "alice@acme.com".into(),
            space_roles,
            scopes: vec![
                Scope::new(ResourceKind::Conversations, Action::Read),
                Scope::new(ResourceKind::Conversations, Action::Write),
                Scope::new(ResourceKind::Personas, Action::Read),
            ],
            client_id: "webui".into(),
        }
    }

    /// Build a test app that returns the extracted AuthContext user_id.
    fn test_app(config: WebAuthConfig) -> Router {
        async fn whoami(Extension(ctx): Extension<AuthContext>) -> String {
            ctx.user_id.to_string()
        }

        Router::new()
            .route("/whoami", get(whoami))
            .route_layer(axum::middleware::from_fn_with_state(config, require_auth))
    }

    // -- Tests: Bearer JWT ---------------------------------------------------

    #[tokio::test]
    async fn bearer_jwt_produces_auth_context() {
        let (config, jwt_sign) = test_config();
        let app = test_app(config);

        let ctx = make_user_context();
        let token = jwt_sign.sign(&ctx, "acme", "Alice").unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK, "valid JWT should pass auth");
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "alice",
            "AuthContext should contain the JWT user_id"
        );
    }

    // -- Tests: Legacy token -------------------------------------------------

    #[tokio::test]
    async fn bearer_legacy_token_produces_admin_context() {
        let (config, _jwt_sign) = test_config();
        let app = test_app(config);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("authorization", "Bearer legacy-token-123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "legacy token should pass auth"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "admin",
            "legacy token should map to admin context"
        );
    }

    // -- Tests: API key ------------------------------------------------------

    #[tokio::test]
    async fn bearer_api_key_produces_auth_context() {
        let (config, _jwt_sign) = test_config();

        // Store an API key in the in-memory store.
        let key = assistant_auth::api_keys::generate_key();
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);
        let record = assistant_auth::api_keys::ApiKeyRecord {
            id: "key_test".into(),
            user_id: UserId::from("bob"),
            name: "Test Key".into(),
            key_hash: key.key_hash.clone(),
            key_prefix: key.key_prefix.clone(),
            org_id: OrgId::from("acme"),
            space_roles,
            scopes: vec![],
            expires_at: None,
            created_at: chrono::Utc::now(),
        };
        config
            .auth_state
            .api_key_store
            .store_key(record)
            .await
            .unwrap();

        let app = test_app(config);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("authorization", format!("Bearer {}", key.plaintext))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "valid API key should pass auth"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "bob",
            "API key should resolve to the key owner"
        );
    }

    // -- Tests: Session cookie -----------------------------------------------

    #[tokio::test]
    async fn session_cookie_jwt_produces_auth_context() {
        let (config, jwt_sign) = test_config();
        let app = test_app(config);

        let ctx = make_user_context();
        let jwt = jwt_sign.sign(&ctx, "acme", "Alice").unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("cookie", format!("{}={}", SESSION_COOKIE, jwt))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "valid session cookie JWT should pass auth"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "alice",
            "session cookie should resolve to JWT user"
        );
    }

    // -- Tests: Unauthenticated requests ------------------------------------

    #[tokio::test]
    async fn missing_auth_returns_401_for_api() {
        let (config, _jwt_sign) = test_config();
        let app = test_app(config);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "missing auth should return 401 for API requests"
        );
    }

    #[tokio::test]
    async fn missing_auth_redirects_browsers() {
        let (config, _jwt_sign) = test_config();
        let app = test_app(config);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("accept", "text/html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::SEE_OTHER,
            "missing auth should redirect browsers"
        );
        assert_eq!(
            resp.headers().get("location").unwrap().to_str().unwrap(),
            "/login"
        );
    }

    #[tokio::test]
    async fn invalid_bearer_returns_401() {
        let (config, _jwt_sign) = test_config();
        let app = test_app(config);

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("authorization", "Bearer invalid-garbage")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "invalid bearer should return 401"
        );
    }

    #[tokio::test]
    async fn expired_session_cookie_returns_401() {
        let (config, _jwt_sign) = test_config();
        let app = test_app(config);

        // Sign a JWT with a different secret so it fails validation.
        let other_kp = JwtKeyPair::from_secret(b"other-secret-that-is-long-enough");
        let other_mgr = JwtManager::new(
            other_kp,
            "https://test.local".into(),
            "https://test.local".into(),
        );
        let ctx = make_user_context();
        let bad_jwt = other_mgr.sign(&ctx, "acme", "Alice").unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/whoami")
                    .header("cookie", format!("{}={}", SESSION_COOKIE, bad_jwt))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "invalid session cookie should return 401"
        );
    }

    // -- Tests: RequireScope middleware ----------------------------------------

    use assistant_core::identity::{Action, ResourceKind};

    /// Build a test app with auth + scope check on a protected route.
    fn scoped_app(
        config: WebAuthConfig,
        space: &str,
        resource: ResourceKind,
        action: Action,
    ) -> Router {
        async fn whoami(Extension(ctx): Extension<AuthContext>) -> String {
            ctx.user_id.to_string()
        }

        Router::new()
            .route("/protected", get(whoami))
            .route_layer(require_scope_layer(SpaceId::from(space), resource, action))
            .route_layer(axum::middleware::from_fn_with_state(config, require_auth))
    }

    #[tokio::test]
    async fn scope_allows_admin() {
        let (config, jwt_sign) = test_config();
        let app = scoped_app(config, "any-space", ResourceKind::Users, Action::Manage);

        let ctx = make_admin_context();
        let token = jwt_sign.sign(&ctx, "default", "Admin").unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/protected")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "org admin should bypass scope checks"
        );
    }

    #[tokio::test]
    async fn scope_allows_matching_permission() {
        let (config, jwt_sign) = test_config();
        let app = scoped_app(config, "eng", ResourceKind::Conversations, Action::Read);

        // make_user_context has no scopes but has Member role in "eng" space.
        // Member can read conversations.
        let ctx = make_user_context();
        let token = jwt_sign.sign(&ctx, "acme", "Alice").unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/protected")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "member with matching space/resource/action should pass"
        );
    }

    #[tokio::test]
    async fn scope_denies_wrong_space() {
        let (config, jwt_sign) = test_config();
        let app = scoped_app(
            config,
            "marketing",
            ResourceKind::Conversations,
            Action::Read,
        );

        let ctx = make_user_context(); // only has "eng" space
        let token = jwt_sign.sign(&ctx, "acme", "Alice").unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/protected")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "wrong space should return 403"
        );
    }

    #[tokio::test]
    async fn scope_denies_insufficient_action() {
        let (config, jwt_sign) = test_config();
        let app = scoped_app(config, "eng", ResourceKind::Users, Action::Manage);

        let ctx = make_user_context(); // Member in "eng", no manage scope
        let token = jwt_sign.sign(&ctx, "acme", "Alice").unwrap();

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/protected")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "insufficient action should return 403"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        assert!(
            body_str.contains("insufficient permissions"),
            "403 body should explain the denial"
        );
    }

    #[tokio::test]
    async fn scope_returns_401_without_auth_context() {
        // No auth middleware → no AuthContext extension → should 401
        async fn whoami(Extension(ctx): Extension<AuthContext>) -> String {
            ctx.user_id.to_string()
        }

        let app = Router::new()
            .route("/protected", get(whoami))
            .route_layer(require_scope_layer(
                SpaceId::from("eng"),
                ResourceKind::Conversations,
                Action::Read,
            ));

        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "missing AuthContext should return 401"
        );
    }

    // -- Tests: CSRF (require_same_origin_mutation) ---------------------------

    use axum::routing::post;

    /// Build a test app with CSRF middleware on a POST endpoint.
    fn csrf_app() -> Router {
        async fn ok() -> &'static str {
            "ok"
        }

        Router::new()
            .route("/action", post(ok))
            .route("/action", get(ok))
            .route_layer(axum::middleware::from_fn(require_same_origin_mutation))
    }

    #[tokio::test]
    async fn csrf_allows_safe_methods() {
        let app = csrf_app();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/action")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "GET should bypass CSRF check"
        );
    }

    #[tokio::test]
    async fn csrf_allows_bearer_auth() {
        let app = csrf_app();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/action")
                    .header("authorization", "Bearer some-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Bearer-authenticated POST should bypass CSRF check"
        );
    }

    #[tokio::test]
    async fn csrf_allows_matching_origin() {
        let app = csrf_app();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/action")
                    .header("host", "example.com")
                    .header("origin", "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "POST with matching Origin should pass"
        );
    }

    #[tokio::test]
    async fn csrf_blocks_mismatched_origin() {
        let app = csrf_app();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/action")
                    .header("host", "example.com")
                    .header("origin", "https://evil.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "POST with mismatched Origin should be blocked"
        );
    }

    #[tokio::test]
    async fn csrf_allows_mixed_case_origin() {
        let app = csrf_app();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/action")
                    .header("host", "Example.COM")
                    .header("origin", "https://example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "POST with case-different but matching Origin should pass"
        );
    }

    #[tokio::test]
    async fn csrf_blocks_missing_origin_on_mutation() {
        let app = csrf_app();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/action")
                    .header("host", "example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "POST without Origin/Referer should be blocked"
        );
    }

    #[tokio::test]
    async fn csrf_falls_back_to_referer() {
        let app = csrf_app();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/action")
                    .header("host", "example.com")
                    .header("referer", "https://example.com/page")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "POST with matching Referer (no Origin) should pass"
        );
    }

    // -- Tests: Helpers (kept from original) ----------------------------------

    #[test]
    fn test_extract_cookie() {
        let cookies = "foo=bar; assistant_session=abc123; other=val";
        assert_eq!(extract_cookie(cookies, "assistant_session"), Some("abc123"));
        assert_eq!(extract_cookie(cookies, "foo"), Some("bar"));
        assert_eq!(extract_cookie(cookies, "missing"), None);
    }

    #[test]
    fn test_extract_cookie_single() {
        assert_eq!(
            extract_cookie("assistant_session=xyz", "assistant_session"),
            Some("xyz")
        );
    }
}
