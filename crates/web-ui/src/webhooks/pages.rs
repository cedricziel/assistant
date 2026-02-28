//! Server-side rendered HTML pages for webhook management.
//!
//! Follows the same patterns as the agent pages: dark theme, sidebar layout,
//! `format!()` string assembly, `default_css()`.

use std::net::IpAddr;

use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use sqlx::SqlitePool;
use tracing::{info, warn};

use assistant_storage::WebhookStore;

use crate::common::{html_escape, internal_error, render_sidebar};

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

// -- Page handlers --

/// `GET /webhooks` -- Lists all configured webhooks.
pub async fn list_webhooks(
    State(state): State<WebhookPagesState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let store = WebhookStore::new(state.pool);
    let webhooks = store.list().await.map_err(internal_error)?;
    let count = webhooks.len();

    let mut rows = String::new();
    for wh in &webhooks {
        let short_id = &wh.id[..8.min(wh.id.len())];
        let status_badge = if wh.active {
            "<span class=\"hdr-badge ok\">active</span>"
        } else {
            "<span class=\"hdr-badge error\">inactive</span>"
        };
        let verified_badge = if wh.verified_at.is_some() {
            "<span class=\"hdr-badge ok\">verified</span>"
        } else {
            "<span class=\"badge muted\">unverified</span>"
        };
        let event_count = wh.event_types.len();

        rows.push_str(&format!(
            "<tr onclick=\"window.location='/webhooks/{id}'\">\
             <td><span class=\"trace-id\">{short_id}&hellip;</span></td>\
             <td><span class=\"primary\">{name}</span></td>\
             <td class=\"url-cell\">{url}</td>\
             <td>{event_count} event{es}</td>\
             <td>{status_badge}</td>\
             <td>{verified_badge}</td>\
             </tr>",
            id = html_escape(&wh.id),
            short_id = html_escape(short_id),
            name = html_escape(&wh.name),
            url = html_escape(&wh.url),
            event_count = event_count,
            es = if event_count == 1 { "" } else { "s" },
            status_badge = status_badge,
            verified_badge = verified_badge,
        ));
    }

    let table = if webhooks.is_empty() {
        "<p class=\"empty\">No webhooks configured yet. Create one to get started.</p>".to_string()
    } else {
        format!(
            "<table class=\"trace-table\">\
             <thead><tr>\
             <th>ID</th><th>Name</th><th>URL</th><th>Events</th><th>Status</th><th>Verified</th>\
             </tr></thead>\
             <tbody>{rows}</tbody></table>",
            rows = rows,
        )
    };

    let content = format!(
        "<div class=\"panel\">\
         <div class=\"panel-head\">\
         <div><h2>Webhooks</h2>\
         <p>Manage outgoing webhook endpoints with HMAC-SHA256 verification.</p></div>\
         <span class=\"pill\">{count}</span>\
         </div>\
         {table}\
         <div style=\"margin-top:1.25rem\">\
         <a href=\"/webhooks/new\" class=\"action-btn\">+ New Webhook</a>\
         </div>\
         </div>",
        count = count,
        table = table,
    );

    let sidebar = render_sidebar("webhooks");
    let body = page_shell("Webhooks", &sidebar, &content);
    Ok(Html(body))
}

/// `GET /webhooks/new` -- Form to create a new webhook.
pub async fn new_webhook_form(State(_state): State<WebhookPagesState>) -> Html<String> {
    let form = render_webhook_form(None, "Create Webhook", "/webhooks", "Create");
    let sidebar = render_sidebar("webhooks");
    let body = page_shell("New Webhook", &sidebar, &form);
    Html(body)
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
) -> Result<Html<String>, (StatusCode, String)> {
    let store = WebhookStore::new(state.pool);
    let wh = store
        .get(&id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")))?;

    // -- Header --
    let status_badge = if wh.active {
        " <span class=\"hdr-badge ok\">active</span>"
    } else {
        " <span class=\"hdr-badge error\">inactive</span>"
    };

    let header = format!(
        "<div class=\"trace-header-bar\">\
         <a class=\"hdr-back\" href=\"/webhooks\">&larr; Webhooks</a>\
         <span class=\"hdr-sep\">|</span>\
         <span class=\"hdr-trace-id\">{short_id}&hellip;</span>\
         <span class=\"hdr-svc\">{name}</span>{status_badge}\
         </div>",
        short_id = html_escape(&id[..8.min(id.len())]),
        name = html_escape(&wh.name),
        status_badge = status_badge,
    );

    // -- URL --
    let url_section = format!(
        "<div class=\"agent-section\">\
         <h3>Endpoint URL</h3>\
         <p><code>{url}</code></p>\
         </div>",
        url = html_escape(&wh.url),
    );

    // -- Verification status --
    let verified_section = match wh.verified_at {
        Some(ts) => format!(
            "<div class=\"agent-section\">\
             <h3>Verification</h3>\
             <p><span class=\"hdr-badge ok\">verified</span> Last verified: {ts}</p>\
             </div>",
            ts = format_ts(ts),
        ),
        None => "<div class=\"agent-section\">\
                 <h3>Verification</h3>\
                 <p><span class=\"badge muted\">unverified</span> \
                 This webhook has not been verified yet. Use the Verify button to send a test payload.</p>\
                 </div>"
            .to_string(),
    };

    // -- Secret (masked with reveal) --
    let secret_section = format!(
        "<div class=\"agent-section\">\
         <h3>Signing Secret</h3>\
         <p>Payloads are signed with <code>HMAC-SHA256</code> using this secret. \
         The signature is sent in the <code>X-Webhook-Signature</code> header as <code>sha256=&lt;hex&gt;</code>.</p>\
         <div class=\"secret-box\">\
         <code id=\"secret-value\" class=\"secret-masked\" onclick=\"this.classList.toggle('secret-masked')\">\
         {secret}</code>\
         <span class=\"secret-hint\">(click to reveal/hide)</span>\
         </div>\
         </div>",
        secret = html_escape(&wh.secret),
    );

    // -- Event types --
    let events_html = if wh.event_types.is_empty() {
        "<p class=\"empty\">No event types selected — this webhook will not fire.</p>".to_string()
    } else {
        let badges: String = wh
            .event_types
            .iter()
            .map(|e| format!("<span class=\"badge\">{}</span> ", html_escape(e)))
            .collect();
        format!("<p>{badges}</p>")
    };
    let events_section = format!(
        "<div class=\"agent-section\">\
         <h3>Subscribed Events</h3>\
         {events}\
         </div>",
        events = events_html,
    );

    // -- Timestamps --
    let ts_section = format!(
        "<div class=\"agent-section\">\
         <h3>Timestamps</h3>\
         <p>Created: {created} &mdash; Updated: {updated}</p>\
         </div>",
        created = format_ts(wh.created_at),
        updated = format_ts(wh.updated_at),
    );

    // -- Actions --
    let toggle_label = if wh.active { "Disable" } else { "Enable" };
    let actions = format!(
        "<div class=\"agent-actions\">\
         <a href=\"/webhooks/{id}/edit\" class=\"action-btn\">Edit</a>\
         <form method=\"POST\" action=\"/webhooks/{id}/verify\" style=\"display:inline\">\
         <button type=\"submit\" class=\"action-btn secondary\">Verify</button>\
         </form>\
         <form method=\"POST\" action=\"/webhooks/{id}/toggle\" style=\"display:inline\">\
         <button type=\"submit\" class=\"action-btn secondary\">{toggle_label}</button>\
         </form>\
         <form method=\"POST\" action=\"/webhooks/{id}/rotate-secret\" style=\"display:inline\" \
               onsubmit=\"return confirm('Rotate the signing secret? The old secret will be invalidated and verification will be cleared.')\">\
         <button type=\"submit\" class=\"action-btn secondary\">Rotate Secret</button>\
         </form>\
         <form method=\"POST\" action=\"/webhooks/{id}/delete\" style=\"display:inline\" \
               onsubmit=\"return confirm('Delete this webhook?')\">\
         <button type=\"submit\" class=\"action-btn danger\">Delete</button>\
         </form>\
         </div>",
        id = html_escape(&wh.id),
        toggle_label = toggle_label,
    );

    let detail = format!(
        "<div class=\"trace-detail\">\
         {header}\
         {url}\
         {actions}\
         {verified}\
         {secret}\
         {events}\
         {ts}\
         </div>",
        header = header,
        url = url_section,
        actions = actions,
        verified = verified_section,
        secret = secret_section,
        events = events_section,
        ts = ts_section,
    );

    let sidebar = render_sidebar("webhooks");
    let body = page_shell(&format!("Webhook: {}", wh.name), &sidebar, &detail);
    Ok(Html(body))
}

/// `GET /webhooks/:id/edit` -- Edit form for an existing webhook.
pub async fn edit_webhook_form(
    State(state): State<WebhookPagesState>,
    Path(id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let store = WebhookStore::new(state.pool);
    let wh = store
        .get(&id)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, format!("Webhook '{id}' not found")))?;

    let form = render_webhook_form(
        Some(&wh),
        "Edit Webhook",
        &format!("/webhooks/{id}/edit"),
        "Save Changes",
    );
    let sidebar = render_sidebar("webhooks");
    let body = page_shell(&format!("Edit: {}", wh.name), &sidebar, &form);
    Ok(Html(body))
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
) -> Result<Html<String>, (StatusCode, String)> {
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

    let badge = if success {
        "<span class=\"hdr-badge ok\">success</span>"
    } else {
        "<span class=\"hdr-badge error\">failed</span>"
    };

    let content = format!(
        "<div class=\"panel\">\
         <div class=\"panel-head\"><h2>Verification Result</h2></div>\
         <div class=\"verify-result\">\
         <p>{badge} {detail}</p>\
         <p class=\"verify-info\">A <code>POST</code> was sent to <code>{url}</code> with:</p>\
         <ul>\
         <li>Header: <code>X-Webhook-Signature: sha256=&lt;hex&gt;</code></li>\
         <li>Header: <code>X-Webhook-Event: webhook.verify</code></li>\
         <li>Body: <code>{{\"type\":\"webhook.verify\", ...}}</code></li>\
         </ul>\
         <p>The receiving endpoint should validate the HMAC-SHA256 signature using the \
         shared secret to confirm authenticity.</p>\
         </div>\
         <div style=\"margin-top:1.25rem\">\
         <a href=\"/webhooks/{id}\" class=\"action-btn secondary\">&larr; Back to Webhook</a>\
         </div>\
         </div>",
        badge = badge,
        detail = html_escape(&detail),
        url = html_escape(&wh.url),
        id = html_escape(&wh.id),
    );

    let sidebar = render_sidebar("webhooks");
    let body = page_shell("Verify Webhook", &sidebar, &content);
    Ok(Html(body))
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

fn page_shell(title: &str, sidebar: &str, content: &str) -> String {
    let content_html = format!(
        "<div class=\"layout\">\
         <aside class=\"sidebar\">{sidebar}</aside>\
         <main class=\"main\">{content}</main>\
         </div>",
        sidebar = sidebar,
        content = content,
    );
    let page_css = format!("{}\n{}", crate::default_css(), webhooks_css());
    crate::legacy::render_page(
        "webhooks",
        title,
        "Management",
        title,
        &page_css,
        &content_html,
        "",
    )
}

fn render_webhook_form(
    wh: Option<&assistant_storage::WebhookRecord>,
    heading: &str,
    action: &str,
    submit_label: &str,
) -> String {
    let name = wh.map(|w| w.name.as_str()).unwrap_or("");
    let url = wh.map(|w| w.url.as_str()).unwrap_or("");
    let active = wh.map(|w| w.active).unwrap_or(true);
    let event_types_csv = wh.map(|w| w.event_types.join(", ")).unwrap_or_default();

    let active_checked = if active { "checked" } else { "" };

    // Build the reference list of available event types.
    let mut event_ref = String::new();
    for (value, label) in EVENT_TYPES {
        event_ref.push_str(&format!(
            "<li><code>{value}</code> &mdash; {label}</li>",
            value = html_escape(value),
            label = html_escape(label),
        ));
    }

    format!(
        "<div class=\"panel\">\
         <div class=\"panel-head\"><h2>{heading}</h2></div>\
         <form method=\"POST\" action=\"{action}\" class=\"agent-form\">\
         <div class=\"form-group\">\
           <label>Name *</label>\
           <input type=\"text\" name=\"name\" value=\"{name}\" required \
                  placeholder=\"My Webhook\">\
         </div>\
         <div class=\"form-group\">\
           <label>URL *</label>\
           <input type=\"text\" name=\"url\" value=\"{url}\" required \
                  placeholder=\"https://example.com/webhook\">\
         </div>\
         <div class=\"form-group\">\
           <label class=\"checkbox-label\">\
             <input type=\"checkbox\" name=\"active\" value=\"on\" {active_checked}>\
             Active\
           </label>\
         </div>\
         <div class=\"form-group\">\
           <label>Event Types (comma-separated)</label>\
           <input type=\"text\" name=\"event_types\" value=\"{event_types}\" \
                  placeholder=\"turn.result, tool.result\">\
           <details class=\"event-ref\">\
           <summary>Available event types</summary>\
           <ul>{event_ref}</ul>\
           </details>\
         </div>\
         <div class=\"form-actions\">\
           <button type=\"submit\" class=\"action-btn\">{submit_label}</button>\
           <a href=\"/webhooks\" class=\"action-btn secondary\">Cancel</a>\
         </div>\
         </form>\
         </div>",
        heading = html_escape(heading),
        action = html_escape(action),
        name = html_escape(name),
        url = html_escape(url),
        active_checked = active_checked,
        event_types = html_escape(&event_types_csv),
        event_ref = event_ref,
        submit_label = html_escape(submit_label),
    )
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

    // -- render_sidebar --

    #[test]
    fn render_sidebar_marks_active_item() {
        let html = render_sidebar("webhooks");
        assert!(
            html.contains("facet-link active"),
            "should have an active class",
        );
        // The Webhooks link should be the active one.
        assert!(html.contains("href=\"/webhooks\""));
        // Other links should be present but not active.
        assert!(html.contains("href=\"/traces\""));
        assert!(html.contains("href=\"/agents\""));
    }

    #[test]
    fn render_sidebar_includes_all_nav_items() {
        let html = render_sidebar("traces");
        assert!(html.contains("Traces"));
        assert!(html.contains("Logs"));
        assert!(html.contains("Agents"));
        assert!(html.contains("Webhooks"));
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

/// Additional CSS for webhook management pages.
fn webhooks_css() -> &'static str {
    r#"
    .url-cell {
        font-family: ui-monospace, monospace;
        font-size: 0.85rem;
        max-width: 300px;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }
    .event-ref {
        margin-top: 0.4rem;
        font-size: 0.85rem;
        color: #8ba2c6;
    }
    .event-ref summary {
        cursor: pointer;
        color: #6ec6ff;
    }
    .event-ref ul {
        margin: 0.3rem 0 0 1.2rem;
        padding: 0;
    }
    .event-ref li {
        margin-bottom: 0.2rem;
    }
    .event-ref code {
        background: rgba(94, 195, 255, 0.1);
        padding: 0.1rem 0.4rem;
        border-radius: 4px;
        font-size: 0.85rem;
    }
    .secret-box {
        background: #020511;
        border: 1px solid #15243b;
        border-radius: 8px;
        padding: 0.75rem 1rem;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        margin-top: 0.5rem;
    }
    .secret-box code {
        font-family: ui-monospace, monospace;
        font-size: 0.9rem;
        word-break: break-all;
        cursor: pointer;
        user-select: all;
    }
    .secret-masked {
        color: transparent !important;
        text-shadow: 0 0 8px rgba(110, 198, 255, 0.5);
    }
    .secret-hint {
        font-size: 0.8rem;
        color: #5a7396;
        white-space: nowrap;
    }
    .verify-result {
        padding: 1rem 0;
    }
    .verify-result ul {
        margin: 0.5rem 0 0.5rem 1.2rem;
        padding: 0;
    }
    .verify-result li {
        margin-bottom: 0.3rem;
        font-size: 0.9rem;
    }
    .verify-result code {
        background: rgba(94, 195, 255, 0.1);
        padding: 0.1rem 0.4rem;
        border-radius: 4px;
        font-size: 0.85rem;
    }
    .verify-info {
        margin-top: 1rem;
        color: #8ba2c6;
        font-size: 0.9rem;
    }
    .agent-form {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    .form-group {
        display: flex;
        flex-direction: column;
        gap: 0.35rem;
    }
    .form-group label {
        font-size: 0.85rem;
        color: #8aa5d8;
        text-transform: uppercase;
        letter-spacing: 0.06em;
    }
    .form-group input[type=text] {
        background: #020511;
        border: 1px solid #15243b;
        border-radius: 8px;
        color: #e5e9f0;
        padding: 0.5rem 0.75rem;
        font-size: 0.9rem;
        font-family: inherit;
        width: 100%;
    }
    .form-actions {
        display: flex;
        gap: 0.75rem;
        margin-top: 0.5rem;
    }
    .checkbox-label {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        cursor: pointer;
    }
    .checkbox-label input[type=checkbox] {
        accent-color: #6ec6ff;
        width: 16px;
        height: 16px;
    }
    .action-btn {
        display: inline-block;
        background: linear-gradient(135deg, #64cafe, #8b5dff);
        border: none;
        border-radius: 8px;
        color: #050b16;
        padding: 0.5rem 1.2rem;
        font-weight: 600;
        font-size: 0.9rem;
        cursor: pointer;
        text-decoration: none;
        text-align: center;
    }
    .action-btn.secondary {
        background: rgba(255,255,255,0.08);
        color: #c2d6f0;
    }
    .action-btn.danger {
        background: rgba(248, 113, 113, 0.25);
        color: #ffb4b4;
    }
    .agent-actions {
        display: flex;
        gap: 0.75rem;
        flex-wrap: wrap;
        margin: 1.25rem 0;
        padding-bottom: 1.25rem;
        border-bottom: 1px solid #0f1f36;
    }
    .agent-section {
        margin: 1.25rem 0;
    }
    .agent-section h3 {
        margin: 0 0 0.6rem;
        color: #8aa5d8;
        font-size: 0.9rem;
        text-transform: uppercase;
        letter-spacing: 0.08em;
    }
    .agent-section p {
        margin: 0.3rem 0;
    }
    .agent-section code {
        background: rgba(94, 195, 255, 0.1);
        padding: 0.15rem 0.5rem;
        border-radius: 4px;
        font-family: ui-monospace, monospace;
        font-size: 0.88rem;
    }
    "#
}
