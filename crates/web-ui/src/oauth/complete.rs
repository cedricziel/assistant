//! GET /oauth/complete — authorization completion endpoint.
//!
//! This endpoint serves as the redirect target for password-mode OAuth2
//! authorization flows.  After the user authenticates via POST /oauth/authorize,
//! the server redirects here with `?code=...`.
//!
//! **Why not `/oauth/callback`?**  That route is reserved for OIDC IdP callbacks
//! (which carry a `state` parameter referencing a pending OIDC session).  Using
//! the same path for password-mode redirects causes the OIDC handler to reject
//! the request because no `state` is present.
//!
//! This handler returns the authorization code as JSON so that programmatic
//! clients (e.g. Dio on Flutter web, where `followRedirects: false` may not be
//! honoured) can extract it from the response body.

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};

/// Query parameters for GET /oauth/complete.
#[derive(Deserialize, utoipa::IntoParams)]
pub struct CompleteQuery {
    /// Authorization code from the authorize endpoint.
    pub code: Option<String>,
    /// Client-supplied state (forwarded from the authorize request).
    pub state: Option<String>,
    /// Error code (if authorization failed).
    pub error: Option<String>,
    /// Human-readable error description.
    pub error_description: Option<String>,
}

/// JSON response from the completion endpoint.
#[derive(Serialize, utoipa::ToSchema)]
pub struct CompleteResponse {
    /// The authorization code.
    pub code: String,
    /// The client state, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

/// GET /oauth/complete — return the authorization code as JSON.
#[utoipa::path(
    get,
    path = "/oauth/complete",
    tag = "oauth",
    security(()),
    params(CompleteQuery),
    responses(
        (status = 200, description = "Authorization code", body = CompleteResponse),
        (status = 400, description = "Missing or error parameters"),
    )
)]
pub async fn complete(Query(params): Query<CompleteQuery>) -> impl IntoResponse {
    // Surface authorization errors.
    if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or_default();
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": err,
                "error_description": desc,
            })),
        )
            .into_response();
    }

    let code = match params.code {
        Some(c) if !c.is_empty() => c,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "missing 'code' parameter",
                })),
            )
                .into_response();
        }
    };

    Json(CompleteResponse {
        code,
        state: params.state,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    use super::*;

    fn build_app() -> Router {
        Router::new().route("/oauth/complete", get(complete))
    }

    #[tokio::test]
    async fn complete_returns_code_as_json() {
        let app = build_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/oauth/complete?code=auth_code_123&state=my_state")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "auth_code_123");
        assert_eq!(json["state"], "my_state");
    }

    #[tokio::test]
    async fn complete_omits_state_when_absent() {
        let app = build_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/oauth/complete?code=auth_code_456")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "auth_code_456");
        assert!(json.get("state").is_none());
    }

    #[tokio::test]
    async fn complete_rejects_missing_code() {
        let app = build_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/oauth/complete?state=abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn complete_surfaces_error() {
        let app = build_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/oauth/complete?error=access_denied&error_description=user+cancelled")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "access_denied");
    }
}
