//! POST /oauth/token — exchange auth codes, refresh tokens, or device codes
//! for access/refresh token pairs.
//!
//! On successful auth-code and refresh-token exchanges the handler also sets
//! an `HttpOnly` session cookie (`assistant_session`) containing the JWT so
//! that browser-based clients get an automatic session.

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::{HeaderValue, SET_COOKIE};
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::warn;

use assistant_auth::oauth2::device::PollResult;
use assistant_auth::oauth2::server::{InvalidTarget, TokenPair};
use assistant_core::identity::{Action, ResourceKind, Scope};

use super::{OAuthError, OAuthErrorResponse, OAuthState, build_auth_context_for_user};

/// Standard OAuth2 token request (form-encoded per RFC 6749 §4.1.3).
#[derive(Deserialize, utoipa::ToSchema)]
pub struct TokenRequest {
    pub grant_type: String,
    /// Authorization code (for `authorization_code` grant).
    pub code: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    /// PKCE verifier (RFC 7636).
    pub code_verifier: Option<String>,
    /// Refresh token (for `refresh_token` grant).
    pub refresh_token: Option<String>,
    /// Device code (for device code grant).
    pub device_code: Option<String>,
    /// RFC 8707 resource the token is for. Only the MCP endpoint URL
    /// (`<public base URL>/mcp`) is accepted; any other value is rejected as
    /// `invalid_target`. On `authorization_code` and `refresh_token` it must
    /// equal the resource the grant was authorized for; omit it to inherit
    /// that one. The device code grant takes it here, as the device
    /// authorization request carries none.
    pub resource: Option<String>,
}

/// Standard OAuth2 token response.
#[derive(Serialize, utoipa::ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

/// POST /oauth/token
#[utoipa::path(
    post,
    path = "/oauth/token",
    tag = "oauth",
    security(()),
    request_body(content = TokenRequest, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Token issued", body = TokenResponse),
        (status = 400, description = "Invalid grant, or `invalid_target` for an unknown `resource`", body = OAuthErrorResponse),
        (status = 429, description = "Rate limit exceeded; see the Retry-After header"),
        (status = 500, description = "Server error", body = OAuthErrorResponse),
    )
)]
pub async fn token(
    State(state): State<OAuthState>,
    Form(mut req): Form<TokenRequest>,
) -> impl IntoResponse {
    req.resource = req.resource.filter(|r| !r.is_empty());
    if let Err(e) = state.validate_resource(req.resource.as_deref()) {
        return grant_error(e.into());
    }
    match req.grant_type.as_str() {
        "authorization_code" => handle_auth_code(state, req).await,
        "refresh_token" => handle_refresh(state, req).await,
        "urn:ietf:params:oauth:grant-type:device_code" => handle_device_code(state, req).await,
        _ => (
            StatusCode::BAD_REQUEST,
            Json(OAuthError {
                error: "unsupported_grant_type",
                error_description: format!("unsupported grant_type: {}", req.grant_type),
            }),
        )
            .into_response(),
    }
}

async fn handle_auth_code(state: OAuthState, req: TokenRequest) -> axum::response::Response {
    let code = match req.code {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(OAuthError {
                    error: "invalid_request",
                    error_description: "missing code parameter".into(),
                }),
            )
                .into_response();
        }
    };
    let client_id = match req.client_id {
        Some(c) if !c.is_empty() => c,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(OAuthError {
                    error: "invalid_request",
                    error_description: "missing client_id parameter".into(),
                }),
            )
                .into_response();
        }
    };
    let redirect_uri = match req.redirect_uri {
        Some(r) if !r.is_empty() => r,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(OAuthError {
                    error: "invalid_request",
                    error_description: "missing redirect_uri parameter".into(),
                }),
            )
                .into_response();
        }
    };

    match state
        .oauth2_server
        .exchange_auth_code(
            &code,
            &client_id,
            &redirect_uri,
            req.code_verifier.as_deref(),
            req.resource.as_deref(),
        )
        .await
    {
        Ok(pair) => token_response(&state, &client_id, pair).await,
        Err(e) => grant_error(e),
    }
}

async fn handle_refresh(state: OAuthState, req: TokenRequest) -> axum::response::Response {
    let refresh_token = match req.refresh_token {
        Some(t) => t,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(OAuthError {
                    error: "invalid_request",
                    error_description: "missing refresh_token parameter".into(),
                }),
            )
                .into_response();
        }
    };
    let client_id = req.client_id.unwrap_or_default();

    match state
        .oauth2_server
        .refresh(&refresh_token, &client_id, req.resource.as_deref())
        .await
    {
        Ok(pair) => token_response(&state, &client_id, pair).await,
        Err(e) => grant_error(e),
    }
}

/// Sign the access token for a successful auth-code or refresh exchange and
/// build the response (with the session cookie when the token is for the API).
async fn token_response(
    state: &OAuthState,
    client_id: &str,
    pair: TokenPair,
) -> axum::response::Response {
    let ttl = state.jwt_manager.access_ttl_secs();
    let jwt =
        match issue_access_jwt(state, &pair.user_id, client_id, pair.resource.as_deref()).await {
            Ok(t) => t,
            Err(resp) => return resp,
        };

    let cookie = session_cookie_for(state, &pair, &jwt, ttl);
    let mut resp = Json(TokenResponse {
        access_token: jwt,
        token_type: "Bearer",
        expires_in: ttl,
        refresh_token: Some(pair.refresh_token),
    })
    .into_response();
    if let Some(cookie) = cookie {
        resp.headers_mut().insert(SET_COOKIE, cookie);
    }
    resp
}

/// Map a failed grant to its OAuth error: `invalid_target` for a conflicting
/// `resource`, `invalid_grant` for everything else.
fn grant_error(e: anyhow::Error) -> axum::response::Response {
    let error = if e.downcast_ref::<InvalidTarget>().is_some() {
        "invalid_target"
    } else {
        "invalid_grant"
    };
    (
        StatusCode::BAD_REQUEST,
        Json(OAuthError {
            error,
            error_description: e.to_string(),
        }),
    )
        .into_response()
}

/// The session cookie for a freshly issued token, if it gets one.
///
/// Only a token for the web API may become the browser session: an MCP token
/// is rejected there and would log the browser out.
fn session_cookie_for(
    state: &OAuthState,
    pair: &TokenPair,
    jwt: &str,
    ttl: u64,
) -> Option<HeaderValue> {
    if pair.resource.is_some() {
        return None;
    }
    let cookie = session_cookie(jwt, state.secure_cookie, ttl);
    if cookie.is_none() {
        warn!("skipping Set-Cookie header: session_cookie returned None (invalid header value)");
    }
    cookie
}

/// Sign the access JWT for `user_id`, bound to the user's current session
/// generation.
///
/// Answers `invalid_grant` when the user no longer exists, so a refresh token
/// outliving its user cannot mint new access tokens.
async fn issue_access_jwt(
    state: &OAuthState,
    user_id: &str,
    client_id: &str,
    resource: Option<&str>,
) -> Result<String, axum::response::Response> {
    let server_error = |description: String| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OAuthError {
                error: "server_error",
                error_description: description,
            }),
        )
            .into_response()
    };

    let mut identity = match build_auth_context_for_user(&state.org_storage, user_id, client_id)
        .await
    {
        Ok(Some(identity)) => identity,
        Ok(None) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(OAuthError {
                    error: "invalid_grant",
                    error_description: "user no longer exists".into(),
                }),
            )
                .into_response());
        }
        Err(e) => {
            tracing::error!(user_id = %user_id, error = %e, "loading user for token issuance failed");
            return Err(server_error("failed to load user".into()));
        }
    };

    let org = identity.ctx.org_id.to_string();
    let signed = match resource {
        None => state
            .jwt_manager
            .sign(&identity.ctx, &org, user_id, identity.token_version),
        Some(resource) => {
            // A token for the MCP endpoint carries exactly the MCP scope,
            // whatever else the client asked for.
            identity.ctx.scopes = vec![Scope::new(ResourceKind::Mcp, Action::Execute)];
            state.jwt_manager.sign_for_audience(
                &identity.ctx,
                &org,
                user_id,
                identity.token_version,
                resource,
            )
        }
    };
    signed.map_err(|e| server_error(e.to_string()))
}

/// Build the `Set-Cookie` header value for the `assistant_session` JWT cookie.
///
/// Returns `None` only if the constructed cookie string contains bytes that
/// `HeaderValue` rejects — JWTs are base64url + dots so this cannot happen for
/// a well-formed JWT, but we propagate the failure rather than panicking.
fn session_cookie(jwt: &str, secure: bool, max_age_secs: u64) -> Option<HeaderValue> {
    let secure_attr = if secure { "; Secure" } else { "" };
    let value = format!(
        "assistant_session={jwt}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_secs}{secure_attr}"
    );
    HeaderValue::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::session_cookie;

    #[test]
    fn session_cookie_emits_httponly_samesite_lax_cookie() {
        let cookie = session_cookie("eyJhbGciOiJIUzI1NiJ9.payload.sig", false, 3600).unwrap();
        let s = cookie.to_str().unwrap();
        assert!(s.starts_with("assistant_session=eyJ"));
        assert!(s.contains("HttpOnly"));
        assert!(s.contains("SameSite=Lax"));
        assert!(s.contains("Path=/"));
        assert!(s.contains("Max-Age=3600"));
    }

    #[test]
    fn session_cookie_adds_secure_attr_when_secure_true() {
        let cookie = session_cookie("jwt.value.sig", true, 7200).unwrap();
        let s = cookie.to_str().unwrap();
        assert!(s.contains("; Secure"));
        assert!(s.contains("Max-Age=7200"));
    }

    #[test]
    fn session_cookie_omits_secure_attr_when_secure_false() {
        let cookie = session_cookie("jwt.value.sig", false, 60).unwrap();
        let s = cookie.to_str().unwrap();
        assert!(!s.contains("Secure"));
    }

    #[test]
    fn session_cookie_returns_none_for_rejected_bytes() {
        // Newline characters are forbidden in cookie values.
        let result = session_cookie("bad\nvalue", false, 60);
        assert!(result.is_none());
    }
}

async fn handle_device_code(state: OAuthState, req: TokenRequest) -> axum::response::Response {
    let device_code = match req.device_code {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(OAuthError {
                    error: "invalid_request",
                    error_description: "missing device_code parameter".into(),
                }),
            )
                .into_response();
        }
    };

    match state.device_manager.poll(&device_code).await {
        Ok(PollResult::Authorized { user_id, scopes }) => {
            let client_id = req.client_id.clone().unwrap_or_else(|| "cli".to_string());

            // Issue a refresh token so the CLI can maintain its session.
            let pair = match state
                .oauth2_server
                .issue_token_pair(&user_id, &client_id, scopes, req.resource.as_deref())
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(OAuthError {
                            error: "server_error",
                            error_description: e.to_string(),
                        }),
                    )
                        .into_response();
                }
            };

            let ttl = state.jwt_manager.access_ttl_secs();
            let jwt = match issue_access_jwt(&state, &user_id, &client_id, req.resource.as_deref())
                .await
            {
                Ok(t) => t,
                Err(resp) => return resp,
            };

            Json(TokenResponse {
                access_token: jwt,
                token_type: "Bearer",
                expires_in: ttl,
                refresh_token: Some(pair.refresh_token),
            })
            .into_response()
        }
        Ok(PollResult::Pending) => (
            StatusCode::BAD_REQUEST,
            Json(OAuthError {
                error: "authorization_pending",
                error_description: "the user has not yet authorized this device".into(),
            }),
        )
            .into_response(),
        Ok(PollResult::Expired) => (
            StatusCode::BAD_REQUEST,
            Json(OAuthError {
                error: "expired_token",
                error_description: "the device code has expired".into(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(OAuthError {
                error: "invalid_grant",
                error_description: e.to_string(),
            }),
        )
            .into_response(),
    }
}
