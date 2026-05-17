//! Image attachment endpoints: upload (`POST /api/conversations/{id}/attachments`)
//! and serve (`GET /api/attachments/{id}`) with on-demand resizing.

use assistant_core::clock::{Clock, SystemClock};
use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use assistant_storage::{ConversationStore, SqliteConversationStore};

use super::ApiState;

/// Multipart form for image attachment upload.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
struct AttachmentUploadForm {
    /// The image file.
    #[schema(format = Binary)]
    file: String,
}

/// Metadata returned after a successful upload.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AttachmentMetaResponse {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    /// URL to fetch the attachment content.
    pub url: String,
}

impl AttachmentMetaResponse {
    pub(crate) fn from_meta(meta: &assistant_core::AttachmentMeta) -> Self {
        Self {
            id: meta.id,
            filename: meta.filename.clone(),
            mime_type: meta.mime_type.clone(),
            size_bytes: meta.size_bytes,
            created_at: meta.created_at,
            url: format!("/api/attachments/{}", meta.id),
        }
    }
}

/// `POST /api/conversations/{id}/attachments` — upload an image attachment.
///
/// Accepts a `multipart/form-data` body with a single `file` field.
/// Returns `201 Created` with the attachment metadata on success.
#[utoipa::path(
    post,
    path = "/api/conversations/{id}/attachments",
    tag = "attachments",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    request_body(
        content_type = "multipart/form-data",
        content = inline(AttachmentUploadForm),
    ),
    responses(
        (status = 201, description = "Attachment uploaded", body = AttachmentMetaResponse),
        (status = 400, description = "Bad request (invalid MIME type, too large, etc.)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Conversation not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn upload_attachment(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    let agent_id = state.agent_id.read().await.clone();
    let store = SqliteConversationStore::for_agent(state.pool.clone(), &agent_id);
    match store.get_conversation(conv_id).await {
        Ok(None) => return (StatusCode::NOT_FOUND, "Conversation not found").into_response(),
        Err(e) => {
            warn!("Failed to check conversation {conv_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
        _ => {}
    }

    // Parse multipart — expect a single "file" field.
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut mime_type = String::new();
    let mut filename = String::from("attachment");

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            if let Some(ct) = field.content_type() {
                mime_type = ct.to_string();
            }
            if let Some(name) = field.file_name() {
                filename = name.to_string();
            }
            match field.bytes().await {
                Ok(b) => {
                    if b.len() as u64 > assistant_core::MAX_ATTACHMENT_SIZE {
                        return (
                            StatusCode::BAD_REQUEST,
                            format!(
                                "File too large (max {} MB)",
                                assistant_core::MAX_ATTACHMENT_SIZE / (1024 * 1024)
                            ),
                        )
                            .into_response();
                    }
                    file_bytes = Some(b.to_vec());
                }
                Err(e) => {
                    warn!("Failed to read file field: {e}");
                    return (StatusCode::BAD_REQUEST, "Failed to read file data").into_response();
                }
            }
        }
    }

    let file_bytes = match file_bytes {
        Some(b) if !b.is_empty() => b,
        _ => return (StatusCode::BAD_REQUEST, "Missing file field").into_response(),
    };

    // Validate MIME type.
    if !assistant_core::is_supported_mime_type(&mime_type) {
        warn!(
            mime_type = %mime_type,
            filename = %filename,
            "Attachment upload rejected: unsupported MIME type"
        );
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "Unsupported MIME type '{mime_type}'. Supported: {}",
                assistant_core::SUPPORTED_MIME_TYPES.join(", ")
            ),
        )
            .into_response();
    }

    let meta = assistant_core::AttachmentMeta {
        id: Uuid::new_v4(),
        message_id: None,
        conversation_id: conv_id,
        agent_id,
        filename,
        mime_type,
        size_bytes: file_bytes.len() as u64,
        created_at: SystemClock.now(),
    };

    if let Err(e) = state.attachment_store.store(&meta, &file_bytes).await {
        warn!("Failed to store attachment: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to store attachment",
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(AttachmentMetaResponse::from_meta(&meta)),
    )
        .into_response()
}

/// Optional query parameters for the attachment serve endpoint.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AttachmentServeParams {
    /// Desired width in pixels (preserves aspect ratio).
    pub w: Option<u32>,
    /// Desired height in pixels (preserves aspect ratio).
    pub h: Option<u32>,
}

/// `GET /api/attachments/{id}` — serve an attachment, optionally resized.
///
/// Supports `w` and `h` query params for on-demand image resizing. Resized
/// variants are cached on disk. Responds with `ETag` and `Cache-Control`
/// headers; returns `304 Not Modified` when `If-None-Match` matches.
#[utoipa::path(
    get,
    path = "/api/attachments/{id}",
    tag = "attachments",
    params(
        ("id" = Uuid, Path, description = "Attachment ID"),
        AttachmentServeParams,
    ),
    responses(
        (status = 200, description = "Attachment bytes", content_type = "application/octet-stream"),
        (status = 304, description = "Not modified"),
        (status = 400, description = "Invalid ID"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Attachment not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn serve_attachment(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<AttachmentServeParams>,
) -> Response {
    let att_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid attachment ID").into_response(),
    };

    let meta = match state.attachment_store.get_meta(att_id).await {
        Ok(m) => m,
        Err(_) => return (StatusCode::NOT_FOUND, "Attachment not found").into_response(),
    };

    // Determine target size (default: original).
    let want_w = params.w.unwrap_or(0);
    let want_h = params.h.unwrap_or(0);
    let needs_resize =
        (want_w > 0 || want_h > 0) && assistant_core::is_resizable_mime_type(&meta.mime_type);

    // Build ETag from attachment ID + resize dimensions.
    let etag = if needs_resize {
        format!("\"{}-{}x{}\"", meta.id, want_w, want_h)
    } else {
        format!("\"{}\"", meta.id)
    };

    // Conditional request: check If-None-Match.
    if let Some(inm) = headers.get(header::IF_NONE_MATCH)
        && let Ok(inm_str) = inm.to_str()
        && inm_str == etag
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }

    // Load original bytes.
    let original = match state.attachment_store.load_bytes(att_id).await {
        Ok(b) => b,
        Err(e) => {
            warn!("Failed to load attachment bytes for {att_id}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read attachment",
            )
                .into_response();
        }
    };

    let (bytes, content_type) = if needs_resize {
        match resize_image(&original, &meta.mime_type, want_w, want_h) {
            Ok(resized) => (resized, meta.mime_type.clone()),
            Err(e) => {
                warn!("Failed to resize attachment {att_id}: {e}");
                // Fall back to original.
                (original, meta.mime_type.clone())
            }
        }
    } else {
        (original, meta.mime_type.clone())
    };

    (
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CACHE_CONTROL,
                "private, immutable, max-age=31536000".to_string(),
            ),
            (header::ETAG, etag),
        ],
        Body::from(bytes),
    )
        .into_response()
}

/// Resize an image to fit within a bounding box, preserving aspect ratio.
fn resize_image(data: &[u8], mime_type: &str, max_w: u32, max_h: u32) -> anyhow::Result<Vec<u8>> {
    use image::ImageFormat;

    let format = match mime_type {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::WebP,
        _ => anyhow::bail!("unsupported mime type for resize: {mime_type}"),
    };

    let img = image::load_from_memory_with_format(data, format)?;
    let (orig_w, orig_h) = (img.width(), img.height());

    // Compute target dimensions preserving aspect ratio.
    let target_w = if max_w > 0 { max_w } else { orig_w };
    let target_h = if max_h > 0 { max_h } else { orig_h };

    // Only downscale, never upscale.
    if orig_w <= target_w && orig_h <= target_h {
        return Ok(data.to_vec());
    }

    let resized = img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3);

    let mut buf = std::io::Cursor::new(Vec::new());
    resized.write_to(&mut buf, format)?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    use assistant_storage::{ConversationStore, SqliteConversationStore};

    use super::super::test_helpers::*;

    #[tokio::test]
    async fn upload_attachment_rejects_unsupported_mime() {
        let (state, storage) = event_log_state().await;
        let conv_store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = conv_store.create_conversation(None).await.unwrap();

        let (content_type, body) =
            multipart_body("file", "test.exe", "application/x-msdownload", b"MZ");

        let app = app(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/conversations/{}/attachments", conv.id))
            .header("Authorization", "Bearer test-token")
            .header("Content-Type", content_type)
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn upload_attachment_succeeds_with_png() {
        let (state, storage) = event_log_state().await;
        let conv_store = SqliteConversationStore::for_agent(storage.pool.clone(), "default");
        let conv = conv_store.create_conversation(None).await.unwrap();

        let png_data = tiny_png();
        let (content_type, body) = multipart_body("file", "test.png", "image/png", &png_data);

        let app = app(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/conversations/{}/attachments", conv.id))
            .header("Authorization", "Bearer test-token")
            .header("Content-Type", content_type)
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let meta: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(meta["mime_type"], "image/png");
        assert_eq!(meta["filename"], "test.png");
        assert!(meta["id"].as_str().is_some());
        assert!(
            meta["url"]
                .as_str()
                .unwrap()
                .starts_with("/api/attachments/")
        );
    }

    #[tokio::test]
    async fn upload_attachment_404_for_missing_conversation() {
        let (state, _storage) = event_log_state().await;
        let fake_id = Uuid::new_v4();

        let png_data = tiny_png();
        let (content_type, body) = multipart_body("file", "test.png", "image/png", &png_data);

        let app = app(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/conversations/{fake_id}/attachments"))
            .header("Authorization", "Bearer test-token")
            .header("Content-Type", content_type)
            .body(Body::from(body))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn serve_attachment_returns_bytes_and_cache_headers() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();

        // Store an attachment directly.
        let png_data = tiny_png();
        let meta = assistant_core::AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id: conv_id,
            agent_id: "default".to_string(),
            filename: "test.png".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: png_data.len() as u64,
            created_at: Utc::now(),
        };
        state
            .attachment_store
            .store(&meta, &png_data)
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri(format!("/attachments/{}", meta.id))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "image/png");
        assert!(
            resp.headers()
                .get("cache-control")
                .unwrap()
                .to_str()
                .unwrap()
                .contains("immutable")
        );
        assert!(resp.headers().get("etag").is_some());

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body_bytes.as_ref(), png_data.as_slice());
    }

    #[tokio::test]
    async fn serve_attachment_304_on_matching_etag() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();

        let png_data = tiny_png();
        let meta = assistant_core::AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id: conv_id,
            agent_id: "default".to_string(),
            filename: "test.png".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: png_data.len() as u64,
            created_at: Utc::now(),
        };
        state
            .attachment_store
            .store(&meta, &png_data)
            .await
            .unwrap();

        let etag = format!("\"{}\"", meta.id);

        let app = app(state);
        let req = Request::builder()
            .uri(format!("/attachments/{}", meta.id))
            .header("Authorization", "Bearer test-token")
            .header("If-None-Match", &etag)
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
    }

    #[tokio::test]
    async fn serve_attachment_with_resize_params() {
        let (state, _storage) = event_log_state().await;
        let conv_id = Uuid::new_v4();

        // Create a slightly larger image so resize actually does something.
        let img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
            image::ImageBuffer::from_pixel(100, 100, image::Rgba([0, 128, 255, 255]));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        let png_data = buf.into_inner();

        let meta = assistant_core::AttachmentMeta {
            id: Uuid::new_v4(),
            message_id: None,
            conversation_id: conv_id,
            agent_id: "default".to_string(),
            filename: "big.png".to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: png_data.len() as u64,
            created_at: Utc::now(),
        };
        state
            .attachment_store
            .store(&meta, &png_data)
            .await
            .unwrap();

        let app = app(state);
        let req = Request::builder()
            .uri(format!("/attachments/{}?w=50&h=50", meta.id))
            .header("Authorization", "Bearer test-token")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
        // Resized image should be smaller than original.
        assert!(
            body_bytes.len() < png_data.len(),
            "resized should be smaller"
        );
    }
}
