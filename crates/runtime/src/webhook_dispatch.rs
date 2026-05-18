use anyhow::Result;
use hmac::{Hmac, KeyInit, Mac};
use opentelemetry::{
    Context as OtelContext, KeyValue, global,
    trace::{Span as _, TraceContextExt, Tracer as _},
};
use serde_json::Value;
use sha2::Sha256;
use tracing::{info, warn};

use assistant_storage::{StorageLayer, WebhookStore};

type HmacSha256 = Hmac<Sha256>;

/// Deliver one event payload to all active webhooks subscribed to `event_type`
/// within the current assistant agent context.
pub async fn dispatch_event(
    storage: &StorageLayer,
    agent_id: &str,
    event_type: &str,
    payload: Value,
) -> Result<usize> {
    let tracer = global::tracer("assistant.webhooks");
    let parent_cx = OtelContext::current();
    let mut dispatch_span = tracer.start_with_context("webhook.dispatch", &parent_cx);
    dispatch_span.set_attribute(KeyValue::new("agent_id", agent_id.to_string()));
    dispatch_span.set_attribute(KeyValue::new("event_type", event_type.to_string()));
    let dispatch_cx = parent_cx.with_span(dispatch_span);

    let store = storage.webhook_store_for_agent(agent_id);
    let webhooks = store.list_active_for_event(event_type).await?;
    if webhooks.is_empty() {
        dispatch_cx
            .span()
            .set_attribute(KeyValue::new("webhook.subscriber_count", 0_i64));
        dispatch_cx.span().end();
        return Ok(0);
    }

    dispatch_cx.span().set_attribute(KeyValue::new(
        "webhook.subscriber_count",
        webhooks.len() as i64,
    ));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let body = serde_json::to_string(&payload)?;
    let mut delivered = 0usize;

    for webhook in webhooks {
        let mut delivery_span = tracer.start_with_context("webhook.delivery", &dispatch_cx);
        delivery_span.set_attribute(KeyValue::new("webhook.id", webhook.id.clone()));
        delivery_span.set_attribute(KeyValue::new("webhook.url", webhook.url.clone()));

        let signature = compute_signature(&webhook.secret, &body)?;
        let result = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", format!("sha256={signature}"))
            .header("X-Webhook-Event", event_type)
            .header("X-Webhook-Id", &webhook.id)
            .body(body.clone())
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                delivered += 1;
                delivery_span.set_attribute(KeyValue::new(
                    "http.status_code",
                    resp.status().as_u16() as i64,
                ));
                delivery_span.set_attribute(KeyValue::new("webhook.status", "ok"));
            }
            Ok(resp) => {
                delivery_span.set_attribute(KeyValue::new(
                    "http.status_code",
                    resp.status().as_u16() as i64,
                ));
                delivery_span.set_attribute(KeyValue::new("webhook.status", "error"));
                warn!(
                    webhook_id = %webhook.id,
                    event_type,
                    status = %resp.status(),
                    "Webhook delivery failed with non-success status"
                );
            }
            Err(e) => {
                delivery_span.set_attribute(KeyValue::new("webhook.status", "error"));
                delivery_span.set_attribute(KeyValue::new("error", true));
                delivery_span.set_attribute(KeyValue::new("error.message", e.to_string()));
                warn!(
                    webhook_id = %webhook.id,
                    event_type,
                    error = %e,
                    "Webhook delivery request failed"
                );
            }
        }

        delivery_span.end();
    }

    dispatch_cx
        .span()
        .set_attribute(KeyValue::new("webhook.delivered", delivered as i64));
    dispatch_cx.span().end();

    info!(
        event_type,
        attempted = payload_count_hint(&payload),
        delivered,
        "Dispatched webhook event"
    );
    Ok(delivered)
}

fn compute_signature(secret: &str, body: &str) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow::anyhow!("invalid HMAC key length: {e}"))?;
    mac.update(body.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn payload_count_hint(payload: &Value) -> usize {
    if payload.is_null() { 0 } else { 1 }
}
