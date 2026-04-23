//! POST /oauth/register — dynamic client registration (RFC 7591).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};

use assistant_core::auth::ClientRegistration;

use super::{
    ClientInfoSchema, ClientRegistrationSchema, OAuthError, OAuthErrorResponse, OAuthState,
};

/// POST /oauth/register — register a new OAuth2 client.
#[utoipa::path(
    post,
    path = "/oauth/register",
    tag = "oauth",
    security(()),
    request_body(content = ClientRegistrationSchema, content_type = "application/json"),
    responses(
        (status = 201, description = "Client registered", body = ClientInfoSchema),
        (status = 400, description = "Invalid registration", body = OAuthErrorResponse),
    )
)]
pub async fn register(
    State(state): State<OAuthState>,
    Json(req): Json<ClientRegistration>,
) -> impl IntoResponse {
    match state.client_registrar.register(req).await {
        Ok(info) => (StatusCode::CREATED, Json(info)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(OAuthError {
                error: "invalid_client_metadata",
                error_description: e.to_string(),
            }),
        )
            .into_response(),
    }
}
