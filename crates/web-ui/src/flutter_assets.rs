//! Serves the embedded Flutter web build.
//!
//! The `app/build/web/` directory is embedded at compile time by `rust-embed`
//! (the build script in `build.rs` runs `flutter build web` beforehand).
//!
//! All unmatched routes fall through to [`flutter_handler`], which implements
//! SPA routing: known assets are served with their correct MIME type; unknown
//! paths are answered with `index.html` so Flutter's `go_router` can take over.

use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

/// Embeds the contents of `app/build/web/` at compile time.
///
/// The path is relative to this crate's `CARGO_MANIFEST_DIR` (`crates/web-ui`).
#[derive(RustEmbed)]
#[folder = "../../app/build/web/"]
struct FlutterAssets;

/// Axum handler that serves embedded Flutter web assets.
///
/// - Known paths: served with the detected MIME type.
/// - Unknown paths: `index.html` is returned so client-side routing works.
pub(crate) async fn flutter_handler(uri: Uri) -> impl IntoResponse {
    let raw = uri.path().trim_start_matches('/');
    let path = if raw.is_empty() { "index.html" } else { raw };

    serve_asset(path)
}

fn serve_asset(path: &str) -> Response {
    match FlutterAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();
            Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(content.data))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        None => {
            // SPA fallback: Flutter's go_router handles routing client-side.
            match FlutterAssets::get("index.html") {
                Some(content) => Response::builder()
                    .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                    .body(Body::from(content.data))
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}
