//! Single source of truth for error responses on `/api/...` endpoints.
//!
//! Every 4xx/5xx response from a handler under `crates/web-ui/src/api/**`
//! MUST go through [`json_error`] so the body shape stays consistent with the
//! OpenAPI [`ErrorBody`](crate::openapi::ErrorBody) schema and the
//! AGENTS.md convention `{"error": "...message..."}`.
//!
//! OAuth2 endpoints (`/oauth/...`) follow RFC 6749 §5.2 and use
//! [`crate::oauth::OAuthErrorResponse`] verbatim — they are intentionally
//! exempt from this helper.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Build a JSON error response with the project-standard envelope.
///
/// ```text
/// HTTP/1.1 <status>
/// Content-Type: application/json
///
/// {"error":"<message>"}
/// ```
pub fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({"error": message.into()}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn json_error_returns_envelope_with_status_and_content_type() {
        let response = json_error(StatusCode::FORBIDDEN, "access denied");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let content_type = response
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.starts_with("application/json"),
            "unexpected content-type: {content_type}"
        );

        let body_bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body, serde_json::json!({"error": "access denied"}));
    }

    #[tokio::test]
    async fn json_error_accepts_string_and_str() {
        // Compiles with both &str and String — verifies the impl Into<String> bound.
        let _from_str = json_error(StatusCode::BAD_REQUEST, "literal");
        let _from_string = json_error(StatusCode::BAD_REQUEST, String::from("owned"));
    }
}
