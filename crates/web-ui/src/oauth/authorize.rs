//! GET/POST /oauth/authorize — authorization endpoint.
//!
//! GET renders a login form (password mode) or redirects to an external IdP (OIDC).
//! POST validates credentials and redirects back with an authorization code.

use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use serde::Deserialize;

use super::oidc_sessions::PendingOidcSession;
use super::{OAuthState, escape_html_attr};

/// Query parameters for GET /oauth/authorize (RFC 6749 §4.1.1).
#[derive(Deserialize, utoipa::IntoParams)]
pub struct AuthorizeQuery {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub scope: Option<String>,
}

/// Form fields for POST /oauth/authorize (password credential submission).
#[derive(Deserialize, utoipa::ToSchema)]
pub struct AuthorizeForm {
    pub email: Option<String>,
    pub password: Option<String>,
    pub client_id: String,
    pub redirect_uri: String,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub scope: Option<String>,
}

/// GET /oauth/authorize — render login form.
#[utoipa::path(
    get,
    path = "/oauth/authorize",
    tag = "oauth",
    security(()),
    params(AuthorizeQuery),
    responses(
        (status = 200, description = "Login form HTML", content_type = "text/html"),
        (status = 303, description = "Redirect to external IdP (OIDC mode)"),
        (status = 400, description = "Invalid parameters", content_type = "text/html"),
    )
)]
pub async fn authorize_get(
    State(state): State<OAuthState>,
    Query(params): Query<AuthorizeQuery>,
) -> impl IntoResponse {
    let response_type = params.response_type.as_deref().unwrap_or("");
    if response_type != "code" {
        return (
            StatusCode::BAD_REQUEST,
            Html("error: response_type must be 'code'".to_string()),
        )
            .into_response();
    }

    let raw_client_id = params.client_id.as_deref().unwrap_or("");
    let raw_redirect_uri = params.redirect_uri.as_deref().unwrap_or("");
    let raw_code_challenge = params.code_challenge.as_deref().unwrap_or("");

    // Validate client_id and redirect_uri against registered clients.
    if let Err(resp) = validate_client(&state, raw_client_id, raw_redirect_uri).await {
        return resp;
    }

    // PKCE is mandatory (OAuth 2.1).
    if raw_code_challenge.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html("error: code_challenge is required (PKCE)".to_string()),
        )
            .into_response();
    }

    // -- OIDC mode: redirect to external IdP instead of rendering login form --
    if let (Some(oidc), Some(sessions)) = (&state.oidc_provider, &state.oidc_sessions) {
        let pending = PendingOidcSession::new(
            raw_client_id.to_owned(),
            raw_redirect_uri.to_owned(),
            params.code_challenge.clone(),
            params.state.clone(),
            params.scope.clone(),
        );
        let oidc_state = sessions.insert(pending).await;

        let our_callback = format!("{}/oauth/callback", state.issuer);
        let idp_auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope=openid%20email%20profile",
            oidc.discovery().authorization_endpoint,
            urlencoding::encode(oidc.client_id()),
            urlencoding::encode(&our_callback),
            urlencoding::encode(&oidc_state),
        );
        return Redirect::to(&idp_auth_url).into_response();
    }

    // -- Password mode: render login form --
    let client_id = escape_html_attr(raw_client_id);
    let redirect_uri = escape_html_attr(raw_redirect_uri);
    let state_param = escape_html_attr(params.state.as_deref().unwrap_or(""));
    let code_challenge = escape_html_attr(params.code_challenge.as_deref().unwrap_or(""));
    let scope = escape_html_attr(params.scope.as_deref().unwrap_or(""));

    // Render a simple login form.
    let html = format!(
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
<form method="POST" action="/oauth/authorize">
  <input type="email" name="email" placeholder="Email" required>
  <input type="password" name="password" placeholder="Password" required>
  <input type="hidden" name="client_id" value="{client_id}">
  <input type="hidden" name="redirect_uri" value="{redirect_uri}">
  <input type="hidden" name="state" value="{state_param}">
  <input type="hidden" name="code_challenge" value="{code_challenge}">
  <input type="hidden" name="scope" value="{scope}">
  <button type="submit">Sign In</button>
</form>
</body>
</html>"#
    );

    Html(html).into_response()
}

/// POST /oauth/authorize — validate credentials, generate auth code, redirect.
#[utoipa::path(
    post,
    path = "/oauth/authorize",
    tag = "oauth",
    security(()),
    request_body(content = AuthorizeForm, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 303, description = "Redirect with authorization code"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn authorize_post(
    State(state): State<OAuthState>,
    Form(form): Form<AuthorizeForm>,
) -> impl IntoResponse {
    // Validate client_id and redirect_uri against registered clients.
    if let Err(resp) = validate_client(&state, &form.client_id, &form.redirect_uri).await {
        return resp;
    }

    let email = form.email.unwrap_or_default();
    let password = form.password.unwrap_or_default();

    if email.is_empty() || password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html("error: email and password are required".to_string()),
        )
            .into_response();
    }

    // Look up user by email (cross-org) and verify password.
    let user = match state
        .org_storage
        .user_store()
        .get_user_by_email_any(&email)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Html("error: invalid email or password".to_string()),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("user lookup failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("error: internal server error".to_string()),
            )
                .into_response();
        }
    };

    // Verify password hash.
    if user.password_hash.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Html("error: invalid email or password".to_string()),
        )
            .into_response();
    }
    match assistant_auth::password::verify_password(&password, &user.password_hash) {
        Ok(true) => {}
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Html("error: invalid email or password".to_string()),
            )
                .into_response();
        }
    }

    // PKCE is mandatory per OAuth 2.1 §4.1.1 — reject requests without a
    // code_challenge to prevent authorization code interception attacks.
    if form.code_challenge.as_deref().unwrap_or("").is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html("error: code_challenge is required (PKCE)".to_string()),
        )
            .into_response();
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("error: failed to generate authorization code".to_string()),
            )
                .into_response();
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
