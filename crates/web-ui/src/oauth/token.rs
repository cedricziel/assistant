//! POST /oauth/token — exchange auth codes, refresh tokens, or device codes
//! for access/refresh token pairs.
//!
//! On successful auth-code and refresh-token exchanges the handler also sets
//! an `HttpOnly` session cookie (`assistant_session`) containing the JWT so
//! that browser-based clients get an automatic session.

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};

use assistant_auth::oauth2::device::PollResult;

use super::{OAuthError, OAuthErrorResponse, OAuthState, stub_auth_context};

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
        (status = 400, description = "Invalid grant", body = OAuthErrorResponse),
        (status = 500, description = "Server error", body = OAuthErrorResponse),
    )
)]
pub async fn token(
    State(state): State<OAuthState>,
    Form(req): Form<TokenRequest>,
) -> impl IntoResponse {
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
        )
        .await
    {
        Ok(pair) => {
            // Sign a JWT access token for the user.
            let ctx = stub_auth_context(&pair.user_id, "default", "");
            let ttl = state.jwt_manager.access_ttl_secs();
            let jwt = match state
                .jwt_manager
                .sign(&ctx, ctx.org_id.as_ref(), &pair.user_id)
            {
                Ok(t) => t,
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

            let cookie = session_cookie(&jwt, state.secure_cookie, ttl);
            let mut resp = Json(TokenResponse {
                access_token: jwt,
                token_type: "Bearer",
                expires_in: ttl,
                refresh_token: Some(pair.refresh_token),
            })
            .into_response();
            resp.headers_mut()
                .insert(SET_COOKIE, cookie.parse().unwrap());
            resp
        }
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
        .refresh(&refresh_token, &client_id)
        .await
    {
        Ok(pair) => {
            let ctx = stub_auth_context(&pair.user_id, "default", "");
            let ttl = state.jwt_manager.access_ttl_secs();
            let jwt = match state
                .jwt_manager
                .sign(&ctx, ctx.org_id.as_ref(), &pair.user_id)
            {
                Ok(t) => t,
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

            let cookie = session_cookie(&jwt, state.secure_cookie, ttl);
            let mut resp = Json(TokenResponse {
                access_token: jwt,
                token_type: "Bearer",
                expires_in: ttl,
                refresh_token: Some(pair.refresh_token),
            })
            .into_response();
            resp.headers_mut()
                .insert(SET_COOKIE, cookie.parse().unwrap());
            resp
        }
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

/// Build the `Set-Cookie` value for the `assistant_session` JWT cookie.
fn session_cookie(jwt: &str, secure: bool, max_age_secs: u64) -> String {
    let secure_attr = if secure { "; Secure" } else { "" };
    format!(
        "assistant_session={jwt}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age_secs}{secure_attr}"
    )
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
                .issue_token_pair(&user_id, &client_id, scopes)
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

            let ctx = stub_auth_context(&user_id, "default", "");
            let ttl = state.jwt_manager.access_ttl_secs();
            let jwt = match state.jwt_manager.sign(&ctx, ctx.org_id.as_ref(), &user_id) {
                Ok(t) => t,
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
