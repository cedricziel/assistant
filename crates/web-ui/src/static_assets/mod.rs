//! Fingerprinted static asset serving.
//!
//! All CSS and JS is embedded into the binary at compile time, SHA-256
//! fingerprinted, and served at `/static/<name>.<hash>.<ext>` with immutable
//! cache headers.  The content hash changes when any source file is modified,
//! automatically busting browser and service-worker caches.
//!
//! Vendored third-party scripts (htmx, htmx-ext-sse) are also embedded here
//! so the app has **zero CDN dependencies** at runtime.

use std::sync::LazyLock;

use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use sha2::{Digest, Sha256};

// -- Embedded sources --------------------------------------------------------

const BASE_CSS: &str = include_str!("base.css");
const DEFAULT_CSS: &str = include_str!("../../templates/partials/default_css.html");
const ENTITY_CSS: &str = include_str!("../../templates/partials/entity_css.html");

// First-party JS modules.
const APP_JS: &str = include_str!("app.js");
const CHAT_JS: &str = include_str!("chat.js");
const TRACE_DETAIL_JS: &str = include_str!("trace-detail.js");
const AGENT_FORM_JS: &str = include_str!("agent-form.js");

// Vendored third-party JS (committed to repo, no CDN fetch at runtime).
const HTMX_JS: &str = include_str!("vendor/htmx.min.js");
const HTMX_SSE_JS: &str = include_str!("vendor/sse.js");

// -- Fingerprinted asset -----------------------------------------------------

/// A single static asset with its content, fingerprinted URL, and hash.
struct Asset {
    content: &'static str,
    url: String,
    hash: String,
}

/// Compute the fingerprinted URL for an embedded asset.
fn fingerprint(name: &str, ext: &str, content: &str) -> Asset {
    let digest = Sha256::digest(content.as_bytes());
    let hash = hex::encode(&digest[..6]); // 12 hex chars
    let url = format!("/static/{name}.{hash}.{ext}");
    Asset {
        content: "", // filled by caller for owned content
        url,
        hash,
    }
}

/// Same as [`fingerprint`] but for `&'static str` content (no allocation).
fn fingerprint_static(name: &str, ext: &str, content: &'static str) -> Asset {
    let digest = Sha256::digest(content.as_bytes());
    let hash = hex::encode(&digest[..6]);
    let url = format!("/static/{name}.{hash}.{ext}");
    Asset { content, url, hash }
}

// -- Bundles computed once at startup ----------------------------------------

/// Concatenated app CSS (base + default + entity).
struct CssBundle {
    content: String,
    url: String,
    hash: String,
}

static CSS_BUNDLE: LazyLock<CssBundle> = LazyLock::new(|| {
    let content = format!(
        "{}\n\n/* -- default -- */\n{}\n\n/* -- entity -- */\n{}",
        BASE_CSS, DEFAULT_CSS, ENTITY_CSS,
    );
    let fp = fingerprint("app", "css", &content);
    CssBundle {
        content,
        url: fp.url,
        hash: fp.hash,
    }
});

static HTMX_ASSET: LazyLock<Asset> = LazyLock::new(|| fingerprint_static("htmx", "js", HTMX_JS));

static SSE_ASSET: LazyLock<Asset> =
    LazyLock::new(|| fingerprint_static("htmx-sse", "js", HTMX_SSE_JS));

static APP_JS_ASSET: LazyLock<Asset> = LazyLock::new(|| fingerprint_static("app", "js", APP_JS));

static CHAT_JS_ASSET: LazyLock<Asset> = LazyLock::new(|| fingerprint_static("chat", "js", CHAT_JS));

static TRACE_DETAIL_JS_ASSET: LazyLock<Asset> =
    LazyLock::new(|| fingerprint_static("trace-detail", "js", TRACE_DETAIL_JS));

static AGENT_FORM_JS_ASSET: LazyLock<Asset> =
    LazyLock::new(|| fingerprint_static("agent-form", "js", AGENT_FORM_JS));

// -- Public API --------------------------------------------------------------

/// Fingerprinted URL for the app stylesheet (e.g. `/static/app.a1b2c3.css`).
pub fn app_css_url() -> &'static str {
    &CSS_BUNDLE.url
}

/// Fingerprinted URL for the vendored htmx script.
pub fn htmx_url() -> &'static str {
    &HTMX_ASSET.url
}

/// Fingerprinted URL for the vendored htmx-ext-sse script.
pub fn htmx_sse_url() -> &'static str {
    &SSE_ASSET.url
}

/// Fingerprinted URL for the app shell JS.
pub fn app_js_url() -> &'static str {
    &APP_JS_ASSET.url
}

/// Fingerprinted URL for chat-specific JS.
pub fn chat_js_url() -> &'static str {
    &CHAT_JS_ASSET.url
}

/// Fingerprinted URL for the trace detail viewer JS.
pub fn trace_detail_js_url() -> &'static str {
    &TRACE_DETAIL_JS_ASSET.url
}

/// Fingerprinted URL for the agent form validator JS.
pub fn agent_form_js_url() -> &'static str {
    &AGENT_FORM_JS_ASSET.url
}

// -- Route handlers ----------------------------------------------------------

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

async fn serve_htmx() -> Response {
    serve_js_immutable(HTMX_ASSET.content)
}

async fn serve_sse() -> Response {
    serve_js_immutable(SSE_ASSET.content)
}

async fn serve_app_js() -> Response {
    serve_js_immutable(APP_JS_ASSET.content)
}

async fn serve_chat_js() -> Response {
    serve_js_immutable(CHAT_JS_ASSET.content)
}

async fn serve_trace_detail_js() -> Response {
    serve_js_immutable(TRACE_DETAIL_JS_ASSET.content)
}

async fn serve_agent_form_js() -> Response {
    serve_js_immutable(AGENT_FORM_JS_ASSET.content)
}

/// Serve a JS asset with aggressive immutable cache headers.
fn serve_js_immutable(content: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        content,
    )
        .into_response()
}

// -- Router ------------------------------------------------------------------

/// Static asset routes.  Mount in the public (no-auth) scope so the
/// browser and service worker can fetch them before authentication.
pub fn static_router() -> Router {
    Router::new()
        // CSS
        .route(&CSS_BUNDLE.url, get(serve_css))
        .route("/static/app.css", get(serve_css_stable))
        // Vendored JS
        .route(&HTMX_ASSET.url, get(serve_htmx))
        .route(&SSE_ASSET.url, get(serve_sse))
        // First-party JS
        .route(&APP_JS_ASSET.url, get(serve_app_js))
        .route(&CHAT_JS_ASSET.url, get(serve_chat_js))
        .route(&TRACE_DETAIL_JS_ASSET.url, get(serve_trace_detail_js))
        .route(&AGENT_FORM_JS_ASSET.url, get(serve_agent_form_js))
}
