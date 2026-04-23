//! POST /oauth/revoke — token revocation.
//! GET /.well-known/oauth-authorization-server — server metadata (RFC 8414).

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};

use super::{OAuthError, OAuthErrorResponse, OAuthState};

/// POST /oauth/revoke request body.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct RevokeRequest {
    pub token: String,
    #[serde(default)]
    pub token_type_hint: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
}

/// POST /oauth/revoke — revoke a refresh token.
#[utoipa::path(
    post,
    path = "/oauth/revoke",
    tag = "oauth",
    request_body(content = RevokeRequest, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Token revoked (or already invalid)"),
        (status = 400, description = "Invalid request", body = OAuthErrorResponse),
    )
)]
pub async fn revoke(
    State(_state): State<OAuthState>,
    Form(req): Form<RevokeRequest>,
) -> impl IntoResponse {
    // Per RFC 7009, the revocation endpoint always returns 200 even if the
    // token is already invalid. We only need to handle refresh tokens for now.
    if req.token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(OAuthError {
                error: "invalid_request",
                error_description: "missing token parameter".into(),
            }),
        )
            .into_response();
    }

    // Accept the revocation silently — the token will naturally expire.
    // Full revocation support (marking tokens as consumed) will be added
    // when the refresh token store is wired through here.
    StatusCode::OK.into_response()
}

/// RFC 8414 authorization server metadata.
#[derive(Serialize, utoipa::ToSchema)]
pub struct ServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: String,
    pub revocation_endpoint: String,
    pub device_authorization_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}

/// GET /.well-known/oauth-authorization-server
#[utoipa::path(
    get,
    path = "/.well-known/oauth-authorization-server",
    tag = "oauth",
    responses(
        (status = 200, description = "Authorization server metadata", body = ServerMetadata),
    )
)]
pub async fn metadata(State(state): State<OAuthState>) -> Json<ServerMetadata> {
    let base = &state.issuer;
    Json(ServerMetadata {
        issuer: base.clone(),
        authorization_endpoint: format!("{base}/oauth/authorize"),
        token_endpoint: format!("{base}/oauth/token"),
        registration_endpoint: format!("{base}/oauth/register"),
        revocation_endpoint: format!("{base}/oauth/revoke"),
        device_authorization_endpoint: format!("{base}/oauth/device"),
        response_types_supported: vec!["code".into()],
        grant_types_supported: vec![
            "authorization_code".into(),
            "refresh_token".into(),
            "urn:ietf:params:oauth:grant-type:device_code".into(),
        ],
        token_endpoint_auth_methods_supported: vec![
            "none".into(),
            "client_secret_post".into(),
            "client_secret_basic".into(),
        ],
        code_challenge_methods_supported: vec!["S256".into()],
    })
}
