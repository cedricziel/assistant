//! `/oauth/authorize` — authorization endpoint, and its email-first sign-in step.
//!
//! Without an `org`, GET asks for the user's email; `POST /oauth/authorize/identify`
//! finds the org that email belongs to and either redirects to that org's IdP
//! (OIDC) or shows the password form. With an `org`, GET goes straight there.
//! POST `/oauth/authorize` validates one org's password credentials and
//! redirects back with an authorization code.

use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

use assistant_core::identity::OrgId;
use assistant_core::store::{OrgStore, UserStore};

use super::oidc_sessions::PendingOidcSession;
use super::{OAuthState, escape_html_attr};
use crate::rate_limit::too_many_requests;

/// Query parameters for GET /oauth/authorize (RFC 6749 §4.1.1).
#[derive(Deserialize, utoipa::IntoParams)]
pub struct AuthorizeQuery {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    /// Must be `S256`; `plain` and an omitted value (which RFC 7636 §4.3
    /// defaults to `plain`) are rejected.
    pub code_challenge_method: Option<String>,
    pub scope: Option<String>,
    /// Slug of the org to sign in to. An org in OIDC mode redirects to its
    /// own identity provider; a password org shows the password form. Without
    /// this parameter the page asks for the user's email to find the org.
    pub org: Option<String>,
    /// Email the user already entered. Prefills the password form and is
    /// passed on to the org's identity provider as `login_hint`.
    pub login_hint: Option<String>,
}

/// Form fields for POST /oauth/authorize (password credential submission).
#[derive(Deserialize, utoipa::ToSchema)]
pub struct AuthorizeForm {
    pub email: Option<String>,
    pub password: Option<String>,
    /// Slug of the org the user signs in to. Emails are only unique per org.
    pub org: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    /// Must be `S256`.
    pub code_challenge_method: Option<String>,
    pub scope: Option<String>,
}

/// Form fields for POST /oauth/authorize/identify (the email-first step).
#[derive(Deserialize, utoipa::ToSchema)]
pub struct IdentifyForm {
    /// Email used to find the org the user signs in to.
    pub email: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    /// Must be `S256`.
    pub code_challenge_method: Option<String>,
    pub scope: Option<String>,
}

/// The authorization request, carried through every sign-in page.
struct OAuthParams {
    client_id: String,
    redirect_uri: String,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    scope: Option<String>,
}

impl OAuthParams {
    /// Hidden inputs that round-trip the request through a form.
    fn hidden_fields(&self) -> String {
        let field = |name: &str, value: &str| {
            format!(
                r#"<input type="hidden" name="{name}" value="{}">"#,
                escape_html_attr(value)
            )
        };
        [
            field("client_id", &self.client_id),
            field("redirect_uri", &self.redirect_uri),
            field("state", self.state.as_deref().unwrap_or("")),
            field(
                "code_challenge",
                self.code_challenge.as_deref().unwrap_or(""),
            ),
            // Requests that reach a form already passed the S256-only check.
            field("code_challenge_method", "S256"),
            field("scope", self.scope.as_deref().unwrap_or("")),
        ]
        .join("\n  ")
    }
}

impl From<&AuthorizeQuery> for OAuthParams {
    fn from(q: &AuthorizeQuery) -> Self {
        Self {
            client_id: q.client_id.clone().unwrap_or_default(),
            redirect_uri: q.redirect_uri.clone().unwrap_or_default(),
            state: q.state.clone(),
            code_challenge: q.code_challenge.clone(),
            code_challenge_method: q.code_challenge_method.clone(),
            scope: q.scope.clone(),
        }
    }
}

impl From<&AuthorizeForm> for OAuthParams {
    fn from(f: &AuthorizeForm) -> Self {
        Self {
            client_id: f.client_id.clone(),
            redirect_uri: f.redirect_uri.clone(),
            state: f.state.clone(),
            code_challenge: f.code_challenge.clone(),
            code_challenge_method: f.code_challenge_method.clone(),
            scope: f.scope.clone(),
        }
    }
}

impl From<&IdentifyForm> for OAuthParams {
    fn from(f: &IdentifyForm) -> Self {
        Self {
            client_id: f.client_id.clone(),
            redirect_uri: f.redirect_uri.clone(),
            state: f.state.clone(),
            code_challenge: f.code_challenge.clone(),
            code_challenge_method: f.code_challenge_method.clone(),
            scope: f.scope.clone(),
        }
    }
}

/// GET /oauth/authorize — render the email step, the password form, or
/// redirect to the org's IdP.
#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tag = "oauth",
    security(()),
    params(AuthorizeQuery),
    responses(
        (status = 200, description = "Email form (no `org`) or password form (password org), as HTML", content_type = "text/html"),
        (status = 303, description = "Redirect to the org's external IdP (OIDC mode)"),
        (status = 400, description = "Invalid parameters or unknown org", content_type = "text/html"),
        (status = 500, description = "Org is in OIDC mode but has no OIDC settings", content_type = "text/html"),
        (status = 502, description = "The org's IdP could not be reached", content_type = "text/html"),
    )
)]
pub async fn authorize_get(
    State(state): State<OAuthState>,
    Query(query): Query<AuthorizeQuery>,
) -> impl IntoResponse {
    if query.response_type.as_deref() != Some("code") {
        return html_error(StatusCode::BAD_REQUEST, "response_type must be 'code'");
    }

    let params = OAuthParams::from(&query);
    if let Some(resp) = reject_invalid_request(&state, &params).await {
        return resp;
    }

    let Some(slug) = query.org.as_deref() else {
        return email_form(&params);
    };
    let org = match state.org_storage.org_store().get_org_by_slug(slug).await {
        Ok(Some(org)) => org,
        Ok(None) => return html_error(StatusCode::BAD_REQUEST, "unknown organization"),
        Err(e) => {
            tracing::error!("org lookup failed: {e:#}");
            return internal_error();
        }
    };
    let login_hint = query.login_hint.as_deref().filter(|h| !h.is_empty());
    if org.auth_mode == "oidc" {
        return redirect_to_org_idp(&state, org.id, &params, login_hint).await;
    }
    password_form(&params, &org.slug, login_hint.unwrap_or(""))
}

/// POST /oauth/authorize/identify — find the org an email signs in to.
///
/// A sole match continues in that org (IdP redirect or password form); no
/// match or several get the same org-slug form so the response does not say
/// which case applied.
#[utoipa::path(
    post,
    path = "/oauth/authorize/identify",
    tag = "oauth",
    security(()),
    request_body(content = IdentifyForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Password form (sole password-org match) or org-slug form, as HTML", content_type = "text/html"),
        (status = 303, description = "Redirect to the matching org's external IdP (OIDC mode)"),
        (status = 400, description = "Invalid parameters", content_type = "text/html"),
        (status = 429, description = "Rate limit exceeded; see the Retry-After header"),
        (status = 500, description = "Internal server error, or the org is in OIDC mode but has no OIDC settings", content_type = "text/html"),
        (status = 502, description = "The org's IdP could not be reached", content_type = "text/html"),
    )
)]
pub async fn identify_post(
    State(state): State<OAuthState>,
    Form(form): Form<IdentifyForm>,
) -> impl IntoResponse {
    let params = OAuthParams::from(&form);
    if let Some(resp) = reject_invalid_request(&state, &params).await {
        return resp;
    }

    let email = form.email.trim();
    if email.is_empty() {
        return html_error(StatusCode::BAD_REQUEST, "email is required");
    }

    let users = match state
        .org_storage
        .user_store()
        .list_users_by_email_any(email)
        .await
    {
        Ok(users) => users,
        Err(e) => {
            tracing::error!("user lookup failed: {e:#}");
            return internal_error();
        }
    };
    let [user] = users.as_slice() else {
        return org_form(&params, email);
    };

    let org = match state.org_storage.org_store().get_org(&user.org_id).await {
        Ok(Some(org)) => org,
        Ok(None) => {
            tracing::error!(org_id = %user.org_id, "user belongs to a missing org");
            return internal_error();
        }
        Err(e) => {
            tracing::error!("org lookup failed: {e:#}");
            return internal_error();
        }
    };
    if org.auth_mode == "oidc" {
        return redirect_to_org_idp(&state, org.id, &params, Some(email)).await;
    }
    password_form(&params, &org.slug, email)
}

// -- Pages --

/// Wrap `body` in the minimal sign-in page.
fn sign_in_page(body: &str) -> Response {
    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head><title>Sign In</title>
<style>
  body {{ font-family: system-ui, sans-serif; max-width: 400px; margin: 80px auto; }}
  input {{ display: block; width: 100%; padding: 8px; margin: 8px 0; box-sizing: border-box; }}
  button {{ padding: 10px 20px; cursor: pointer; }}
</style>
</head>
<body>
<h2>Sign In</h2>
{body}
</body>
</html>"#
    ))
    .into_response()
}

/// First step: ask for the email so the org can be found.
fn email_form(params: &OAuthParams) -> Response {
    let hidden = params.hidden_fields();
    sign_in_page(&format!(
        r#"<form method="POST" action="/oauth/authorize/identify">
  <input type="email" name="email" placeholder="Email" required autofocus>
  {hidden}
  <button type="submit">Continue</button>
</form>"#
    ))
}

/// Ask for the org slug; submits as a GET so it lands in the `?org=` path.
fn org_form(params: &OAuthParams, email: &str) -> Response {
    let hidden = params.hidden_fields();
    let login_hint = escape_html_attr(email);
    sign_in_page(&format!(
        r#"<p>Enter the name of your organization to continue.</p>
<form method="GET" action="/oauth/authorize">
  <input type="text" name="org" placeholder="Organization" required autofocus>
  <input type="hidden" name="response_type" value="code">
  <input type="hidden" name="login_hint" value="{login_hint}">
  {hidden}
  <button type="submit">Continue</button>
</form>"#
    ))
}

/// Password form for one org.
fn password_form(params: &OAuthParams, org_slug: &str, email: &str) -> Response {
    let hidden = params.hidden_fields();
    let org = escape_html_attr(org_slug);
    let email = escape_html_attr(email);
    sign_in_page(&format!(
        r#"<form method="POST" action="/oauth/authorize">
  <input type="email" name="email" placeholder="Email" value="{email}" required>
  <input type="password" name="password" placeholder="Password" required>
  <input type="hidden" name="org" value="{org}">
  {hidden}
  <button type="submit">Sign In</button>
</form>"#
    ))
}

fn html_error(status: StatusCode, message: &str) -> Response {
    (status, Html(format!("error: {message}"))).into_response()
}

fn internal_error() -> Response {
    html_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
}

/// Send the user to `org_id`'s IdP, remembering the original request so the
/// callback can finish it.
async fn redirect_to_org_idp(
    state: &OAuthState,
    org_id: OrgId,
    params: &OAuthParams,
    login_hint: Option<&str>,
) -> Response {
    let oidc = match state.oidc.provider_for(&org_id).await {
        Ok(Some(provider)) => provider,
        Ok(None) => {
            tracing::error!(%org_id, "org is in OIDC mode but has no OIDC settings");
            return html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "single sign-on is not configured for this organization",
            );
        }
        Err(e) => {
            tracing::error!(%org_id, "OIDC provider unavailable: {e:#}");
            return html_error(
                StatusCode::BAD_GATEWAY,
                "the identity provider is unavailable",
            );
        }
    };

    let pending = PendingOidcSession::new(
        org_id,
        params.client_id.clone(),
        params.redirect_uri.clone(),
        params.code_challenge.clone(),
        params.state.clone(),
        params.scope.clone(),
    );
    let oidc_state = state.oidc_sessions.insert(pending).await;

    let our_callback = format!("{}/oauth/callback", state.issuer);
    let mut idp_auth_url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope=openid%20email%20profile",
        oidc.discovery().authorization_endpoint,
        urlencoding::encode(oidc.client_id()),
        urlencoding::encode(&our_callback),
        urlencoding::encode(&oidc_state),
    );
    if let Some(hint) = login_hint {
        idp_auth_url.push_str(&format!("&login_hint={}", urlencoding::encode(hint)));
    }
    Redirect::to(&idp_auth_url).into_response()
}

/// POST /oauth/authorize — validate one org's password credentials, generate
/// an auth code, redirect.
#[utoipa::path(
    post,
    path = "/oauth/authorize",
    tag = "oauth",
    security(()),
    request_body(content = AuthorizeForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 303, description = "Redirect with authorization code"),
        (status = 400, description = "Invalid request, unknown org, or the org uses single sign-on"),
        (status = 401, description = "Invalid credentials"),
        (status = 429, description = "Too many failed attempts; see Retry-After"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn authorize_post(
    State(state): State<OAuthState>,
    Form(form): Form<AuthorizeForm>,
) -> impl IntoResponse {
    let params = OAuthParams::from(&form);
    if let Some(resp) = reject_invalid_request(&state, &params).await {
        return resp;
    }

    let org = match state
        .org_storage
        .org_store()
        .get_org_by_slug(&form.org)
        .await
    {
        Ok(Some(org)) => org,
        Ok(None) => return html_error(StatusCode::BAD_REQUEST, "unknown organization"),
        Err(e) => {
            tracing::error!("org lookup failed: {e:#}");
            return internal_error();
        }
    };
    if org.auth_mode == "oidc" {
        return html_error(
            StatusCode::BAD_REQUEST,
            "password sign-in is not available for this organization",
        );
    }

    let email = form.email.unwrap_or_default();
    let password = form.password.unwrap_or_default();

    if email.is_empty() || password.is_empty() {
        return html_error(StatusCode::BAD_REQUEST, "email and password are required");
    }

    // Lock out an account after repeated failures. Unknown emails are counted
    // too, so the lockout does not reveal which accounts exist. The key is
    // per org because the same email can belong to users in several orgs.
    let account_key = format!("{}:{}", org.id, email.trim().to_lowercase());
    let failures = &state.rate_limits.login_failures;
    if let Err(retry_after) = failures.check(&account_key) {
        return too_many_requests(retry_after);
    }
    let invalid_credentials = || {
        let _ = failures.try_acquire(&account_key);
        html_error(StatusCode::UNAUTHORIZED, "invalid email or password")
    };

    // Look up the user in this org and verify the password.
    let user = match state
        .org_storage
        .user_store()
        .get_user_by_email(&org.id, &email)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            // Burn the same argon2 time as a real check so response timing
            // does not reveal whether the email is registered.
            assistant_auth::password::verify_dummy(&password);
            return invalid_credentials();
        }
        Err(e) => {
            tracing::error!("user lookup failed: {e}");
            return internal_error();
        }
    };

    // Verify password hash.
    if user.password_hash.is_empty() {
        assistant_auth::password::verify_dummy(&password);
        return invalid_credentials();
    }
    match assistant_auth::password::verify_password(&password, &user.password_hash) {
        Ok(true) => failures.reset(&account_key),
        _ => return invalid_credentials(),
    }

    // Generate authorization code.
    let scopes: Vec<String> = form
        .scope
        .unwrap_or_default()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let code = match state
        .oauth2_server
        .generate_auth_code(
            &user.id.0,
            &form.client_id,
            &form.redirect_uri,
            form.code_challenge.as_deref(),
            scopes,
        )
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("auth code generation failed: {e}");
            return html_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to generate authorization code",
            );
        }
    };

    // Build redirect URI with code and state (URL-encoded).
    let mut redirect = form.redirect_uri.clone();
    redirect.push_str(if redirect.contains('?') { "&" } else { "?" });
    redirect.push_str(&format!("code={}", urlencoding::encode(&code)));
    if let Some(ref st) = form.state {
        redirect.push_str(&format!("&state={}", urlencoding::encode(st)));
    }

    Redirect::to(&redirect).into_response()
}

/// The checks every sign-in step applies to the authorization request: a
/// registered client and redirect URI, and S256 PKCE. `Some` is the rejection.
///
/// PKCE is mandatory per OAuth 2.1 §4.1.1: a request without a code_challenge
/// is rejected to prevent authorization code interception.
async fn reject_invalid_request(state: &OAuthState, params: &OAuthParams) -> Option<Response> {
    if let Err(resp) = validate_client(state, &params.client_id, &params.redirect_uri).await {
        return Some(resp);
    }
    if params.code_challenge.as_deref().unwrap_or("").is_empty() {
        return Some(html_error(
            StatusCode::BAD_REQUEST,
            "code_challenge is required (PKCE)",
        ));
    }
    reject_non_s256(params.code_challenge_method.as_deref())
}

/// PKCE is S256-only (OAuth 2.1 §4.1.1): reject `plain` and an omitted method.
fn reject_non_s256(method: Option<&str>) -> Option<axum::response::Response> {
    (method != Some("S256")).then(|| {
        (
            StatusCode::BAD_REQUEST,
            Html("error: code_challenge_method must be S256".to_string()),
        )
            .into_response()
    })
}

/// Validate that `client_id` is a registered client and `redirect_uri` is
/// in its allowed redirect URIs. Returns `Err(response)` on failure.
async fn validate_client(
    state: &OAuthState,
    client_id: &str,
    redirect_uri: &str,
) -> Result<(), axum::response::Response> {
    if client_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Html("error: missing client_id".to_string()),
        )
            .into_response());
    }

    let client = match state.client_registrar.get(client_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Html("error: unknown client_id".to_string()),
            )
                .into_response());
        }
        Err(e) => {
            tracing::error!("client lookup failed: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("error: internal server error".to_string()),
            )
                .into_response());
        }
    };

    // redirect_uri MUST be present and MUST exactly match one of the
    // registered URIs (RFC 6749 §3.1.2.3).  Allowing an empty redirect_uri
    // would let an attacker skip validation entirely.
    if redirect_uri.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Html("error: redirect_uri is required".to_string()),
        )
            .into_response());
    }
    if !client.redirect_uris.contains(&redirect_uri.to_string()) {
        return Err((
            StatusCode::BAD_REQUEST,
            Html("error: redirect_uri not registered for this client".to_string()),
        )
            .into_response());
    }

    Ok(())
}
