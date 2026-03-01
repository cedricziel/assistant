//! Server-side rendered HTML pages for webhook management.
//!
//! All HTML is rendered via Askama templates under `templates/webhooks/`.

use std::net::IpAddr;

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::SqlitePool;
use tracing::{info, warn};

use assistant_storage::WebhookStore;

#[cfg(test)]
use crate::common::html_escape;
use crate::common::{internal_error, render_template, StaticUrls};

// -- Shared state --

/// State required by the webhook management pages.
#[derive(Clone)]
pub struct WebhookPagesState {
    pub pool: SqlitePool,
}

// -- Known event types --

/// Event types that can be subscribed to (matching MessageBus topics).
const EVENT_TYPES: &[(&str, &str)] = &[
    ("turn.request", "Turn requested"),
    ("turn.result", "Turn completed"),
    ("turn.status", "Turn status update"),
    ("tool.execute", "Tool execution requested"),
    ("tool.result", "Tool execution completed"),
    ("agent.spawn", "Sub-agent spawned"),
    ("agent.report", "Sub-agent report"),
    ("schedule.trigger", "Scheduled task triggered"),
];

// -- View models -------------------------------------------------------------

/// A row in the webhook list table.
struct WebhookRowView {
    id: String,
    short_id: String,
    name: String,
    url: String,
    event_count: usize,
    active: bool,
    verified: bool,
}

/// An available event type shown in the form reference list.
struct EventTypeView {
    value: &'static str,
    label: &'static str,
}

// -- Templates ---------------------------------------------------------------

/// Webhook list page (extends base.html).
#[derive(Template)]
#[template(path = "webhooks/list.html")]
struct WebhookListTemplate {
    active_page: &'static str,
    count: usize,
    rows: Vec<WebhookRowView>,
}

impl StaticUrls for WebhookListTemplate {}

/// Webhook detail page (extends base.html).
#[derive(Template)]
#[template(path = "webhooks/detail.html")]
struct WebhookDetailTemplate {
    active_page: &'static str,
    id: String,
    short_id: String,
    name: String,
    url: String,
    active: bool,
    toggle_label: String,
    verified_at: Option<String>,
    secret: String,
    event_types: Vec<String>,
    created_at: String,
    updated_at: String,
}

impl StaticUrls for WebhookDetailTemplate {}

/// Webhook create/edit form page (extends base.html).
#[derive(Template)]
#[template(path = "webhooks/form.html")]
struct WebhookFormTemplate {
    active_page: &'static str,
    heading: String,
    action: String,
    submit_label: String,
    name: String,
    url: String,
    active_checked: bool,
    event_types_csv: String,
    available_events: Vec<EventTypeView>,
}

impl StaticUrls for WebhookFormTemplate {}

/// Verification result page (extends base.html).
#[derive(Template)]
#[template(path = "webhooks/verify.html")]
struct WebhookVerifyTemplate {
    active_page: &'static str,
    id: String,
    url: String,
    success: bool,
    detail: String,
}

impl StaticUrls for WebhookVerifyTemplate {}

// -- Page handlers --

/// `GET /webhooks` -- Lists all configured webhooks.
pub async fn list_webhooks(
    State(state): State<WebhookPagesState>,
) -> Result<Response, (StatusCode, String)> {
    let store = WebhookStore::new(state.pool);
    let webhooks = store.list().await.map_err(internal_error)?;
    let count = webhooks.len();

    let rows: Vec<WebhookRowView> = webhooks
        .iter()
        .map(|wh| WebhookRowView {
            id: wh.id.clone(),
            short_id: wh.id[..8.min(wh.id.len())].to_string(),
            name: wh.name.clone(),
            url: wh.url.clone(),
            event_count: wh.event_types.len(),
            active: wh.active,
            verified: wh.verified_at.is_some(),
        })
        .collect();

    let tmpl = WebhookListTemplate {
        active_page: "webhooks",
        count,
        rows,
    };
    Ok(render_template(tmpl))
}

/// `GET /webhooks/new` -- Form to create a new webhook.
pub async fn new_webhook_form(State(_state): State<WebhookPagesState>) -> Response {
    let tmpl = build_form_template(None, "Create Webhook", "/webhooks", "Create");
    render_template(tmpl)
}

/// `POST /webhooks` -- Creates a new webhook from form data.
pub async fn create_webhook(
    State(state): State<WebhookPagesState>,
    Form(form): Form<WebhookFormData>,
) -> Response {
    if let Err(e) = validate_webhook_url(&form.url) {
        return (StatusCode::BAD_REQUEST, e).into_response();
    }

    let store = WebhookStore::new(state.pool);
    let id = uuid::Uuid::new_v4().to_string();
    let secret = generate_secret();
    let event_types = form.selected_event_types();

    match store
        .create(&id, &form.name, &form.url, &secret, &event_types)
        .await
    {
        Ok(()) => {
            info!(webhook_id = %id, name = %form.name, "Webhook created");
            Redirect::to(&format!("/webhooks/{id}")).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /webhooks/:id` -- Webhook detail page.
pub async fn show_webhook(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let store = WebhookStore::new(state.pool);
    let wh = store
        .get(&id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")))?;

    let tmpl = WebhookDetailTemplate {
        active_page: "webhooks",
        id: wh.id.clone(),
        short_id: wh.id[..8.min(wh.id.len())].to_string(),
        name: wh.name.clone(),
        url: wh.url.clone(),
        active: wh.active,
        toggle_label: if wh.active {
            "Disable".to_string()
        } else {
            "Enable".to_string()
        },
        verified_at: wh.verified_at.map(format_ts),
        secret: wh.secret.clone(),
        event_types: wh.event_types.clone(),
        created_at: format_ts(wh.created_at),
        updated_at: format_ts(wh.updated_at),
    };
    Ok(render_template(tmpl))
}

/// `GET /webhooks/:id/edit` -- Edit form for an existing webhook.
pub async fn edit_webhook_form(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let store = WebhookStore::new(state.pool);
    let wh = store
        .get(&id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")))?;

    let tmpl = build_form_template(
        Some(&wh),
        "Edit Webhook",
        &format!("/webhooks/{id}/edit"),
        "Save Changes",
    );
    Ok(render_template(tmpl))
}

/// `POST /webhooks/:id/edit` -- Updates a webhook from form data.
pub async fn update_webhook(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
    Form(form): Form<WebhookFormData>,
) -> Response {
    if let Err(e) = validate_webhook_url(&form.url) {
        return (StatusCode::BAD_REQUEST, e).into_response();
    }

    let store = WebhookStore::new(state.pool);
    let event_types = form.selected_event_types();
    let active = form.active.is_some();

    match store
        .update(&id, &form.name, &form.url, &event_types, active)
        .await
    {
        Ok(true) => Redirect::to(&format!("/webhooks/{id}")).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Webhook not found".to_string()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /webhooks/:id/delete` -- Deletes a webhook.
pub async fn delete_webhook(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
) -> Response {
    let store = WebhookStore::new(state.pool);
    match store.delete(&id).await {
        Ok(true) => Redirect::to("/webhooks").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /webhooks/:id/toggle` -- Toggles the active state.
pub async fn toggle_webhook(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
) -> Response {
    let store = WebhookStore::new(state.pool);
    match store.toggle_active(&id).await {
        Ok(true) => Redirect::to(&format!("/webhooks/{id}")).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /webhooks/:id/rotate-secret` -- Regenerates the signing secret.
pub async fn rotate_secret(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
) -> Response {
    let store = WebhookStore::new(state.pool);
    let new_secret = generate_secret();
    match store.rotate_secret(&id, &new_secret).await {
        Ok(true) => Redirect::to(&format!("/webhooks/{id}")).into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /webhooks/:id/verify` -- Sends a signed test payload to the webhook URL.
///
/// Sends an `application/json` POST with a `webhook.verify` event, signed with
/// HMAC-SHA256. The webhook is considered verified if the remote returns a 2xx
/// status code.
pub async fn verify_webhook(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let store = WebhookStore::new(state.pool);
    let wh = store
        .get(&id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")))?;

    // SSRF protection: validate the destination URL before making the request.
    validate_webhook_url(&wh.url).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    let payload = serde_json::json!({
        "type": "webhook.verify",
        "webhook_id": wh.id,
        "timestamp": Utc::now().to_rfc3339(),
    });
    let body = serde_json::to_string(&payload).unwrap_or_default();
    let signature = compute_signature(&wh.secret, &body);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(internal_error)?;

    let result = client
        .post(&wh.url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", format!("sha256={signature}"))
        .header("X-Webhook-Event", "webhook.verify")
        .header("X-Webhook-Id", &wh.id)
        .body(body)
        .send()
        .await;

    let (success, detail) = match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                info!(webhook_id = %wh.id, "Webhook verified successfully");
                store.mark_verified(&wh.id).await.map_err(internal_error)?;
                (true, format!("Remote responded with {status}"))
            } else {
                warn!(webhook_id = %wh.id, %status, "Webhook verification failed: non-2xx");
                (false, format!("Remote responded with {status}"))
            }
        }
        Err(e) => {
            warn!(webhook_id = %wh.id, error = %e, "Webhook verification failed: connection error");
            (false, format!("Connection error: {e}"))
        }
    };

    let tmpl = WebhookVerifyTemplate {
        active_page: "webhooks",
        id: wh.id.clone(),
        url: wh.url.clone(),
        success,
        detail,
    };
    Ok(render_template(tmpl))
}

// -- Form data --

#[derive(Debug, Deserialize)]
pub struct WebhookFormData {
    pub name: String,
    pub url: String,
    /// Comma-separated list of event types (matches the agent-form pattern for
    /// `input_modes` / `output_modes`). `serde_urlencoded` doesn't support
    /// repeated-key-to-Vec, so a single string is the safe approach.
    #[serde(default)]
    pub event_types: String,
    #[serde(default)]
    pub active: Option<String>,
}

impl WebhookFormData {
    fn selected_event_types(&self) -> Vec<String> {
        let known: std::collections::HashSet<&str> = EVENT_TYPES.iter().map(|(k, _)| *k).collect();
        let candidates: Vec<String> = self
            .event_types
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        for c in &candidates {
            if !known.contains(c.as_str()) {
                warn!(event_type = %c, "Ignoring unrecognized event type");
            }
        }
        candidates
            .into_iter()
            .filter(|s| known.contains(s.as_str()))
            .collect()
    }
}

// -- URL validation (SSRF protection) --

/// Validate that a webhook URL is safe for server-side requests.
///
/// Rejects non-HTTP(S) schemes, loopback addresses, and private/link-local
/// CIDRs to prevent SSRF attacks against internal services.
///
/// **Limitation**: This validates the URL at parse time, not after DNS resolution.
/// DNS rebinding attacks (where a hostname initially resolves to a public IP but
/// later resolves to a private IP) are not mitigated. For high-security
/// deployments, consider using a DNS-resolving validation library or
/// network-level controls.
fn validate_webhook_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;

    // Enforce HTTP(S) scheme only.
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "Unsupported scheme '{other}': only http and https are allowed"
            ))
        }
    }

    let host = parsed.host_str().ok_or("URL has no host")?;

    // Block well-known loopback hostnames.
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" || lower == "127.0.0.1" || lower == "::1" || lower == "[::1]" {
        return Err("Loopback addresses are not allowed".to_string());
    }

    // If the host parses as an IP address, check for private/link-local ranges.
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip(ip) {
            return Err(format!("Private/reserved IP address {ip} is not allowed"));
        }
    }
    // Also handle bracket-wrapped IPv6 (e.g. "[::1]").
    let trimmed = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = trimmed.parse::<IpAddr>() {
        if is_private_ip(ip) {
            return Err(format!("Private/reserved IP address {ip} is not allowed"));
        }
    }

    Ok(())
}

/// Returns `true` if the IP is loopback, private (RFC 1918 / RFC 4193), or
/// link-local.
fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()            // 127.0.0.0/8
                || v4.is_private()      // 10/8, 172.16/12, 192.168/16
                || v4.is_link_local()   // 169.254/16
                || v4.is_unspecified()  // 0.0.0.0
                || v4.is_broadcast() // 255.255.255.255
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()            // ::1
                || v6.is_unspecified()  // ::
                // ULA (fc00::/7)
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // Link-local (fe80::/10)
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

// -- Crypto helpers --

/// Generate a 32-byte random hex secret for HMAC signing.
fn generate_secret() -> String {
    use std::fmt::Write;
    let bytes: [u8; 32] = rand_bytes();
    let mut s = String::with_capacity(64);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Collect 32 random bytes from the OS CSPRNG.
fn rand_bytes() -> [u8; 32] {
    let mut buf = [0u8; 32];
    getrandom::getrandom(&mut buf).expect("OS RNG should be available");
    buf
}

/// Compute HMAC-SHA256 of `body` using `secret`, returning hex.
fn compute_signature(secret: &str, body: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

// -- Rendering helpers --

fn format_ts(ts: DateTime<Utc>) -> String {
    ts.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Build a [`WebhookFormTemplate`] for either create or edit.
fn build_form_template(
    wh: Option<&assistant_storage::WebhookRecord>,
    heading: &str,
    action: &str,
    submit_label: &str,
) -> WebhookFormTemplate {
    let name = wh.map(|w| w.name.clone()).unwrap_or_default();
    let url = wh.map(|w| w.url.clone()).unwrap_or_default();
    let active_checked = wh.map(|w| w.active).unwrap_or(true);
    let event_types_csv = wh.map(|w| w.event_types.join(", ")).unwrap_or_default();

    let available_events: Vec<EventTypeView> = EVENT_TYPES
        .iter()
        .map(|(v, l)| EventTypeView { value: v, label: l })
        .collect();

    WebhookFormTemplate {
        active_page: "webhooks",
        heading: heading.to_string(),
        action: action.to_string(),
        submit_label: submit_label.to_string(),
        name,
        url,
        active_checked,
        event_types_csv,
        available_events,
    }
}

/// Render a webhook form as an HTML string (used by tests).
#[cfg(test)]
fn render_webhook_form(
    wh: Option<&assistant_storage::WebhookRecord>,
    heading: &str,
    action: &str,
    submit_label: &str,
) -> String {
    let tmpl = build_form_template(wh, heading, action, submit_label);
    tmpl.render()
        .expect("webhook form template should render in tests")
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;

    // -- compute_signature --

    #[test]
    fn compute_signature_matches_known_vector() {
        // RFC 4231 Test Case 2: HMAC-SHA256 with "Jefe" / "what do ya want for nothing?"
        let sig = compute_signature("Jefe", "what do ya want for nothing?");
        assert_eq!(
            sig, "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            "should match RFC 4231 test vector",
        );
    }

    #[test]
    fn compute_signature_empty_body() {
        let sig = compute_signature("secret", "");
        assert!(
            !sig.is_empty(),
            "signature of empty body should still produce output"
        );
        assert_eq!(sig.len(), 64, "SHA-256 hex is always 64 chars");
    }

    #[test]
    fn compute_signature_different_secrets_differ() {
        let s1 = compute_signature("secret-a", "payload");
        let s2 = compute_signature("secret-b", "payload");
        assert_ne!(
            s1, s2,
            "different secrets must produce different signatures"
        );
    }

    #[test]
    fn compute_signature_different_bodies_differ() {
        let s1 = compute_signature("same-secret", "body-a");
        let s2 = compute_signature("same-secret", "body-b");
        assert_ne!(s1, s2, "different bodies must produce different signatures");
    }

    // -- generate_secret --

    #[test]
    fn generate_secret_length_and_hex() {
        let s = generate_secret();
        assert_eq!(s.len(), 64, "32 bytes => 64 hex chars");
        assert!(
            s.chars().all(|c| c.is_ascii_hexdigit()),
            "secret must be lowercase hex, got: {s}",
        );
    }

    #[test]
    fn generate_secret_unique_on_repeated_calls() {
        let a = generate_secret();
        let b = generate_secret();
        assert_ne!(a, b, "two generated secrets should differ");
    }

    // -- html_escape --

    #[test]
    fn html_escape_replaces_special_chars() {
        assert_eq!(html_escape("&"), "&amp;");
        assert_eq!(html_escape("<"), "&lt;");
        assert_eq!(html_escape(">"), "&gt;");
        assert_eq!(html_escape("\""), "&quot;");
        assert_eq!(html_escape("'"), "&#39;");
    }

    #[test]
    fn html_escape_combined_xss_payload() {
        let input = "<script>alert(\"xss\")</script>";
        let escaped = html_escape(input);
        assert!(!escaped.contains('<'), "should not contain raw <");
        assert!(!escaped.contains('>'), "should not contain raw >");
        assert!(escaped.contains("&lt;script&gt;"));
    }

    #[test]
    fn html_escape_passthrough_for_safe_text() {
        assert_eq!(html_escape("hello world"), "hello world");
        assert_eq!(html_escape(""), "");
    }

    // -- format_ts --

    #[test]
    fn format_ts_produces_expected_format() {
        use chrono::TimeZone;
        let ts = Utc.with_ymd_and_hms(2025, 6, 15, 14, 30, 0).unwrap();
        assert_eq!(format_ts(ts), "2025-06-15 14:30:00 UTC");
    }

    // -- WebhookFormData::selected_event_types --

    #[test]
    fn selected_event_types_splits_csv() {
        let form = WebhookFormData {
            name: "x".into(),
            url: "https://x.test".into(),
            event_types: "turn.result, tool.result".into(),
            active: None,
        };
        assert_eq!(
            form.selected_event_types(),
            vec!["turn.result".to_string(), "tool.result".to_string()],
        );
    }

    #[test]
    fn selected_event_types_trims_whitespace() {
        let form = WebhookFormData {
            name: "x".into(),
            url: "https://x.test".into(),
            event_types: " turn.result ,  tool.result , ".into(),
            active: None,
        };
        assert_eq!(
            form.selected_event_types(),
            vec!["turn.result".to_string(), "tool.result".to_string()],
        );
    }

    #[test]
    fn selected_event_types_empty_string_returns_empty() {
        let form = WebhookFormData {
            name: "x".into(),
            url: "https://x.test".into(),
            event_types: "".into(),
            active: None,
        };
        assert!(form.selected_event_types().is_empty());
    }

    #[test]
    fn selected_event_types_single_value() {
        let form = WebhookFormData {
            name: "x".into(),
            url: "https://x.test".into(),
            event_types: "turn.result".into(),
            active: None,
        };
        assert_eq!(form.selected_event_types(), vec!["turn.result"]);
    }

    #[test]
    fn selected_event_types_filters_unknown() {
        let form = WebhookFormData {
            name: "x".into(),
            url: "https://x.test".into(),
            event_types: "turn.result, bogus.event, tool.result".into(),
            active: None,
        };
        assert_eq!(
            form.selected_event_types(),
            vec!["turn.result".to_string(), "tool.result".to_string()],
            "unknown event types should be filtered out",
        );
    }

    #[test]
    fn selected_event_types_all_unknown_returns_empty() {
        let form = WebhookFormData {
            name: "x".into(),
            url: "https://x.test".into(),
            event_types: "not.real, also.fake".into(),
            active: None,
        };
        assert!(
            form.selected_event_types().is_empty(),
            "all-unknown input should produce empty vec",
        );
    }

    // -- render_webhook_form --

    #[test]
    fn render_webhook_form_new_has_empty_fields() {
        let html = render_webhook_form(None, "Create Webhook", "/webhooks", "Create");
        assert!(html.contains("Create Webhook"));
        assert!(html.contains("action=\"/webhooks\""));
        assert!(
            html.contains("value=\"\""),
            "name should be empty for new form"
        );
        // Active should be checked by default for new webhooks.
        assert!(html.contains("checked"));
    }

    #[test]
    fn render_webhook_form_edit_populates_fields() {
        let wh = assistant_storage::WebhookRecord {
            id: "wh-test".into(),
            name: "My Hook".into(),
            url: "https://example.com/hook".into(),
            secret: "s3cret".into(),
            event_types: vec!["turn.result".to_string()],
            active: false,
            verified_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let html = render_webhook_form(
            Some(&wh),
            "Edit Webhook",
            "/webhooks/wh-test/edit",
            "Save Changes",
        );
        assert!(html.contains("Edit Webhook"));
        assert!(html.contains("My Hook"));
        assert!(html.contains("https://example.com/hook"));
        // Active is false, so the checkbox should NOT be checked.
        assert!(!html.contains("name=\"active\" value=\"on\" checked"));
        // The event types input should contain the comma-separated value.
        assert!(html.contains("turn.result"));
    }

    // -- validate_webhook_url (SSRF protection) --

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_webhook_url("https://example.com/hook").is_ok());
    }

    #[test]
    fn validate_url_accepts_http() {
        assert!(validate_webhook_url("http://example.com/hook").is_ok());
    }

    #[test]
    fn validate_url_rejects_ftp() {
        let err = validate_webhook_url("ftp://example.com/file").unwrap_err();
        assert!(err.contains("Unsupported scheme"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_file_scheme() {
        let err = validate_webhook_url("file:///etc/passwd").unwrap_err();
        assert!(err.contains("Unsupported scheme"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_localhost() {
        let err = validate_webhook_url("http://localhost:8080/hook").unwrap_err();
        assert!(err.contains("Loopback"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_127_0_0_1() {
        let err = validate_webhook_url("http://127.0.0.1/hook").unwrap_err();
        assert!(err.contains("not allowed"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_ipv6_loopback() {
        let err = validate_webhook_url("http://[::1]:8080/hook").unwrap_err();
        assert!(err.contains("not allowed"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_private_10_network() {
        let err = validate_webhook_url("http://10.0.0.1/hook").unwrap_err();
        assert!(err.contains("Private"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_private_172_16() {
        let err = validate_webhook_url("http://172.16.0.1/hook").unwrap_err();
        assert!(err.contains("Private"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_private_192_168() {
        let err = validate_webhook_url("http://192.168.1.1/hook").unwrap_err();
        assert!(err.contains("Private"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_link_local() {
        let err = validate_webhook_url("http://169.254.1.1/hook").unwrap_err();
        assert!(err.contains("Private"), "got: {err}");
    }

    #[test]
    fn validate_url_rejects_invalid_url() {
        let err = validate_webhook_url("not a url").unwrap_err();
        assert!(err.contains("Invalid URL"), "got: {err}");
    }

    #[test]
    fn validate_url_accepts_public_ip() {
        assert!(validate_webhook_url("https://203.0.113.1/hook").is_ok());
    }
}
