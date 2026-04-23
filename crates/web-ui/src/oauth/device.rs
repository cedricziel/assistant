//! Device authorization grant endpoints (RFC 8628).
//!
//! - POST /oauth/device — initiate device flow
//! - GET  /oauth/device/verify — render user-code entry page
//! - POST /oauth/device/verify — submit user-code approval

use axum::Form;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json};
use serde::Deserialize;

use super::{DeviceCodeResponseSchema, OAuthError, OAuthErrorResponse, OAuthState};

/// POST /oauth/device request body.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct DeviceInitiateRequest {
    pub client_id: String,
    #[serde(default)]
    pub scope: Option<String>,
}

/// POST /oauth/device — initiate the device authorization flow.
#[utoipa::path(
    post,
    path = "/oauth/device",
    tag = "oauth",
    security(()),
    request_body(content = DeviceInitiateRequest, content_type = "application/x-www-form-urlencoded"),
    responses(
        (status = 200, description = "Device code issued", body = DeviceCodeResponseSchema),
        (status = 500, description = "Server error", body = OAuthErrorResponse),
    )
)]
pub async fn device_initiate(
    State(state): State<OAuthState>,
    Form(req): Form<DeviceInitiateRequest>,
) -> impl IntoResponse {
    let scopes: Vec<String> = req
        .scope
        .unwrap_or_default()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    match state.device_manager.initiate(&req.client_id, scopes).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OAuthError {
                error: "server_error",
                error_description: e.to_string(),
            }),
        )
            .into_response(),
    }
}

/// Query params for the verification page.
#[derive(Deserialize, Default, utoipa::IntoParams)]
pub struct VerifyQuery {
    #[serde(default)]
    pub user_code: Option<String>,
}

/// GET /oauth/device/verify — render page where user enters the code.
#[utoipa::path(
    get,
    path = "/oauth/device/verify",
    tag = "oauth",
    security(()),
    params(VerifyQuery),
    responses(
        (status = 200, description = "Device verification form HTML"),
    )
)]
pub async fn device_verify_page(Query(params): Query<VerifyQuery>) -> impl IntoResponse {
    let prefill = params.user_code.as_deref().unwrap_or("");

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head><title>Device Authorization</title>
<style>
  body {{ font-family: system-ui, sans-serif; max-width: 400px; margin: 80px auto; }}
  input {{ display: block; width: 100%; padding: 8px; margin: 8px 0; box-sizing: border-box;
           text-transform: uppercase; font-size: 1.5em; text-align: center; letter-spacing: 0.2em; }}
  button {{ padding: 10px 20px; cursor: pointer; }}
</style>
</head>
<body>
<h2>Authorize Device</h2>
<p>Enter the code displayed on your device:</p>
<form method="POST" action="/oauth/device/verify">
  <input type="text" name="user_code" placeholder="ABCD-EFGH" value="{prefill}" required
         pattern="[A-Za-z]{{4}}-[A-Za-z]{{4}}" maxlength="9" autocomplete="off">
  <input type="hidden" name="user_id" value="">
  <button type="submit">Authorize</button>
</form>
</body>
</html>"#
    );

    Html(html)
}

/// Form fields for POST /oauth/device/verify.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct DeviceVerifyForm {
    pub user_code: String,
    /// In a real deployment, user_id comes from the authenticated session.
    /// For now, accept it as a form field or default to "anonymous".
    #[serde(default)]
    pub user_id: Option<String>,
}

/// POST /oauth/device/verify — user approves the device code.
#[utoipa::path(
    post,
    path = "/oauth/device/verify",
    tag = "oauth",
    security(()),
    responses(
        (status = 200, description = "Device authorized"),
        (status = 400, description = "Invalid or expired code"),
    )
)]
pub async fn device_verify_submit(
    State(state): State<OAuthState>,
    Form(form): Form<DeviceVerifyForm>,
) -> impl IntoResponse {
    let user_code = form.user_code.trim().to_uppercase();
    let user_id = form.user_id.unwrap_or_else(|| "anonymous".to_string());

    match state.device_manager.complete(&user_code, &user_id).await {
        Ok(()) => Html(
            r#"<!DOCTYPE html>
<html>
<head><title>Device Authorized</title>
<style>body { font-family: system-ui, sans-serif; max-width: 400px; margin: 80px auto; }</style>
</head>
<body>
<h2>Device Authorized</h2>
<p>You can close this window and return to your device.</p>
</body>
</html>"#
                .to_string(),
        )
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Html(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>Error</title>
<style>body {{ font-family: system-ui, sans-serif; max-width: 400px; margin: 80px auto; }}</style>
</head>
<body>
<h2>Error</h2>
<p>{}</p>
<a href="/oauth/device/verify">Try again</a>
</body>
</html>"#,
                e
            )),
        )
            .into_response(),
    }
}
