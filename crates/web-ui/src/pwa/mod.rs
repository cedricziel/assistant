//! PWA (Progressive Web App) asset routes.
//!
//! Serves the web app manifest, service worker, icons, and offline page.
//! All assets are embedded into the binary via `include_str!()` /
//! `include_bytes!()` — no external static-file directory required.
//!
//! # Cache versioning
//!
//! The service worker template contains an `__APP_VERSION__` placeholder that
//! is replaced at startup:
//!
//! - **Release builds**: `CARGO_PKG_VERSION` (e.g. `0.1.21`) — caches
//!   invalidate on every version bump.
//! - **Debug builds**: `CARGO_PKG_VERSION-dev.<startup_ts>` — caches
//!   invalidate on every server restart so developers never see stale pages.

use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;

// -- Embedded assets ---------------------------------------------------------

const MANIFEST: &str = include_str!("manifest.webmanifest");
const SW_TEMPLATE: &str = include_str!("sw.js");
const OFFLINE_PAGE: &str = include_str!("offline.html");
const ICON_SVG: &str = include_str!("icon.svg");
const ICON_MASKABLE_SVG: &str = include_str!("icon-maskable.svg");

/// Service worker with `__APP_VERSION__` replaced once at process start.
static SERVICE_WORKER: LazyLock<String> = LazyLock::new(|| {
    let version = if cfg!(debug_assertions) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("{}-dev.{}", env!("CARGO_PKG_VERSION"), ts)
    } else {
        env!("CARGO_PKG_VERSION").to_string()
    };
    SW_TEMPLATE
        .replace("__APP_VERSION__", &version)
        .replace("__APP_CSS_URL__", crate::static_assets::app_css_url())
        .replace("__HTMX_URL__", crate::static_assets::htmx_url())
        .replace("__HTMX_SSE_URL__", crate::static_assets::htmx_sse_url())
        .replace("__APP_JS_URL__", crate::static_assets::app_js_url())
        .replace("__CHAT_JS_URL__", crate::static_assets::chat_js_url())
        .replace(
            "__TRACE_DETAIL_JS_URL__",
            crate::static_assets::trace_detail_js_url(),
        )
        .replace(
            "__AGENT_FORM_JS_URL__",
            crate::static_assets::agent_form_js_url(),
        )
});

// -- Route handlers ----------------------------------------------------------

async fn manifest() -> Response {
    (
        [(header::CONTENT_TYPE, "application/manifest+json")],
        MANIFEST,
    )
        .into_response()
}

/// The service worker **must** be served from the root scope (`/sw.js`) so it
/// can intercept fetch events for all routes.  We still keep the source under
/// `src/pwa/` for organisational clarity.
async fn service_worker() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript"),
            // Allow the SW to control the entire origin.
            (
                header::HeaderName::from_static("service-worker-allowed"),
                "/",
            ),
            // Always revalidate — the browser compares byte-for-byte and
            // only reinstalls when the content has actually changed.
            (header::CACHE_CONTROL, "no-cache"),
        ],
        SERVICE_WORKER.as_str(),
    )
        .into_response()
}

async fn offline() -> Html<&'static str> {
    Html(OFFLINE_PAGE)
}

async fn icon() -> Response {
    ([(header::CONTENT_TYPE, "image/svg+xml")], ICON_SVG).into_response()
}

async fn icon_maskable() -> Response {
    ([(header::CONTENT_TYPE, "image/svg+xml")], ICON_MASKABLE_SVG).into_response()
}

// -- Router ------------------------------------------------------------------

/// Public PWA routes (no auth required).
///
/// Mount this **outside** the auth middleware so the browser can always fetch
/// the manifest, service worker, icons, and offline fallback.
pub fn pwa_router() -> Router {
    Router::new()
        .route("/sw.js", get(service_worker))
        .route("/pwa/manifest.webmanifest", get(manifest))
        .route("/pwa/icon.svg", get(icon))
        .route("/pwa/icon-maskable.svg", get(icon_maskable))
        .route("/pwa/offline", get(offline))
}
