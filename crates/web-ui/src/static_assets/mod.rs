//! Fingerprinted static asset serving.
//!
//! All CSS is embedded into the binary at compile time, concatenated into a
//! single stylesheet, and served at `/static/app.<hash>.css` with immutable
//! cache headers. The content hash changes when any source CSS file is
//! modified, automatically busting browser and service-worker caches.

use std::sync::LazyLock;

use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use sha2::{Digest, Sha256};

// -- Embedded CSS sources ----------------------------------------------------

const BASE_CSS: &str = include_str!("base.css");
const DEFAULT_CSS: &str = include_str!("../../templates/partials/default_css.html");
const ENTITY_CSS: &str = include_str!("../../templates/partials/entity_css.html");

// -- Computed at startup -----------------------------------------------------

/// Concatenated CSS content and its fingerprinted URL, computed once.
struct StaticCssBundle {
    content: String,
    url: String,
    hash: String,
}

static CSS_BUNDLE: LazyLock<StaticCssBundle> = LazyLock::new(|| {
    let content = format!(
        "{}\n\n/* -- default -- */\n{}\n\n/* -- entity -- */\n{}",
        BASE_CSS, DEFAULT_CSS, ENTITY_CSS,
    );

    let digest = Sha256::digest(content.as_bytes());
    let hash = hex::encode(&digest[..6]); // 12 hex chars
    let url = format!("/static/app.{hash}.css");

    StaticCssBundle { content, url, hash }
});

// -- Public API --------------------------------------------------------------

/// The fingerprinted URL for the app stylesheet (e.g. `/static/app.a1b2c3d4e5f6.css`).
///
/// Pass this to every template struct that extends `base.html`.
pub fn app_css_url() -> &'static str {
    &CSS_BUNDLE.url
}

// -- Route handler -----------------------------------------------------------

/// Serve the concatenated, fingerprinted CSS.
///
/// The hash in the URL changes whenever the CSS content changes, so we can
/// set an aggressive immutable cache policy.
async fn serve_css() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        CSS_BUNDLE.content.as_str(),
    )
        .into_response()
}

// -- Router ------------------------------------------------------------------

/// Static asset routes.  Mount in the public (no-auth) scope so the
/// browser and service worker can fetch them before authentication.
pub fn static_router() -> Router {
    // Serve at the fingerprinted path. Axum path params can't include dots
    // in the middle of a segment, so we match the full literal path.
    //
    // We also serve at `/static/app.css` as a stable alias (with a shorter
    // cache lifetime) so that the first page-load before the SW installs
    // still works.
    Router::new()
        .route(&CSS_BUNDLE.url, get(serve_css))
        .route("/static/app.css", get(serve_css_stable))
}

/// Stable-URL fallback with short cache + ETag for non-SW requests.
async fn serve_css_stable() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (
                header::CACHE_CONTROL,
                "public, max-age=300, must-revalidate",
            ),
            (header::ETAG, CSS_BUNDLE.hash.as_str()),
        ],
        CSS_BUNDLE.content.as_str(),
    )
        .into_response()
}
