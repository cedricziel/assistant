//! Audio handlers: synthesize-on-demand for assistant messages
//! (`GET /api/messages/{id}/audio`) and serve tool-synthesized blobs
//! (`GET /api/audio/{id}`).

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use tracing::warn;
use uuid::Uuid;

use assistant_core::types::conversation::MessageRole;
use assistant_storage::{ConversationStore, SqliteConversationStore};
use assistant_transcription::TtsRequest;

use super::ApiState;

/// `GET /api/messages/{id}/audio` — synthesize TTS audio for an assistant
/// message and return it as `audio/mpeg`.
#[utoipa::path(
    get,
    path = "/api/messages/{id}/audio",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Message ID")),
    responses(
        (status = 200, description = "MP3 audio bytes", content_type = "audio/mpeg"),
        (status = 400, description = "Invalid ID or non-assistant message"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Message not found"),
        (status = 503, description = "TTS not configured"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_message_audio(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    let tts_provider = match &state.tts_provider {
        Some(p) => p.clone(),
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, "TTS not configured").into_response();
        }
    };

    let msg_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid message ID").into_response(),
    };

    let agent_id = state.agent_id.read().await.clone();
    let store = SqliteConversationStore::for_agent(state.pool.clone(), &agent_id);

    let msg = match store.get_message(msg_id).await {
        Ok(Some(m)) => m,
        Ok(None) => return (StatusCode::NOT_FOUND, "Message not found").into_response(),
        Err(e) => {
            warn!("Failed to fetch message {msg_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    if !matches!(msg.role, MessageRole::Assistant) {
        return (
            StatusCode::BAD_REQUEST,
            "Only assistant messages can be synthesized",
        )
            .into_response();
    }

    match tts_provider
        .synthesize(TtsRequest {
            text: msg.content,
            voice: None,
            format: None,
            speed: None,
        })
        .await
    {
        Ok(result) => (
            [(header::CONTENT_TYPE, result.mime_type)],
            Body::from(result.audio_data),
        )
            .into_response(),
        Err(e) => {
            warn!("TTS synthesis failed for message {msg_id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "TTS synthesis failed").into_response()
        }
    }
}

/// `GET /api/audio/{id}` — serve a synthesized audio blob from the in-memory store.
#[utoipa::path(
    get,
    path = "/api/audio/{id}",
    tag = "conversations",
    params(("id" = Uuid, Path, description = "Audio blob ID")),
    responses(
        (status = 200, description = "MP3 audio bytes", content_type = "audio/mpeg"),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Audio not found or expired"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_audio(State(state): State<ApiState>, Path(id): Path<String>) -> Response {
    let audio_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid audio ID").into_response(),
    };

    match state.audio_store.get(audio_id).await {
        Some((bytes, mime)) => ([(header::CONTENT_TYPE, mime)], Body::from(bytes)).into_response(),
        None => (StatusCode::NOT_FOUND, "Audio not found or expired").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use uuid::Uuid;
    use wiremock::MockServer;

    use super::super::test_helpers::*;

    #[tokio::test]
    async fn get_audio_unknown_id_returns_404() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/audio/{}", Uuid::new_v4()))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_audio_invalid_id_returns_400() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/audio/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_audio_returns_stored_bytes() {
        let server = MockServer::start().await;
        let (state, _) = test_state(&server.uri()).await;

        let fake_audio = b"mp3-bytes".to_vec();
        let audio_id = state
            .audio_store
            .insert(fake_audio.clone(), "audio/mpeg".to_string())
            .await;

        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri(format!("/audio/{audio_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("audio/mpeg")
        );
        let bytes = body_bytes(resp.into_body()).await;
        assert_eq!(bytes, fake_audio);
    }
}
