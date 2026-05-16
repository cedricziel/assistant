//! `GET /api/capabilities` — report server capability flags.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use super::ApiState;

/// Server capability flags.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ServerCapabilities {
    /// Whether the server can accept voice messages (STT configured).
    pub voice_send: bool,
    /// Whether the server can serve TTS audio for messages (TTS configured).
    pub voice_receive: bool,
}

/// `GET /api/capabilities` — return server capability flags.
#[utoipa::path(
    get,
    path = "/api/capabilities",
    tag = "capabilities",
    responses(
        (status = 200, description = "Server capabilities", body = ServerCapabilities),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_capabilities(State(state): State<ApiState>) -> Response {
    Json(ServerCapabilities {
        voice_send: state.transcription_provider.is_some(),
        voice_receive: state.tts_provider.is_some(),
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use wiremock::MockServer;

    use super::super::test_helpers::*;

    #[tokio::test]
    async fn capabilities_no_providers_returns_false() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        // Both providers absent — both flags must be false.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/capabilities")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["voice_send"], false);
        assert_eq!(json["voice_receive"], false);
    }
}
