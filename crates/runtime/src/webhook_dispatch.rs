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

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_storage::{StorageLayer, WebhookStore};
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn compute_signature_is_deterministic() {
        let s1 = compute_signature("secret", r#"{"event":"x"}"#).unwrap();
        let s2 = compute_signature("secret", r#"{"event":"x"}"#).unwrap();
        assert_eq!(s1, s2);
        // 32 bytes hex-encoded → 64 chars.
        assert_eq!(s1.len(), 64);
    }

    #[test]
    fn compute_signature_changes_with_secret_or_body() {
        let s1 = compute_signature("s1", "body").unwrap();
        let s2 = compute_signature("s2", "body").unwrap();
        let s3 = compute_signature("s1", "body2").unwrap();
        assert_ne!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn payload_count_hint_returns_zero_for_null_else_one() {
        assert_eq!(payload_count_hint(&Value::Null), 0);
        assert_eq!(payload_count_hint(&serde_json::json!({"a": 1})), 1);
        assert_eq!(payload_count_hint(&serde_json::json!([1, 2])), 1);
        assert_eq!(payload_count_hint(&serde_json::json!("text")), 1);
    }

    #[tokio::test]
    async fn dispatch_event_returns_zero_when_no_active_subscribers() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let delivered = dispatch_event(
            &storage,
            "default",
            "test.event",
            serde_json::json!({"x": 1}),
        )
        .await
        .unwrap();
        assert_eq!(delivered, 0);
    }

    #[tokio::test]
    async fn dispatch_event_delivers_to_active_subscribers_with_signature() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .and(header("X-Webhook-Event", "test.event"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.webhook_store_for_agent("default");
        store
            .create(
                "wh-1",
                "test",
                &format!("{}/hook", server.uri()),
                "secret",
                &["test.event".to_string()],
            )
            .await
            .unwrap();

        let delivered = dispatch_event(
            &storage,
            "default",
            "test.event",
            serde_json::json!({"hello": "world"}),
        )
        .await
        .unwrap();
        assert_eq!(delivered, 1);
    }

    #[tokio::test]
    async fn dispatch_event_records_non_success_as_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/hook"))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&server)
            .await;

        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.webhook_store_for_agent("default");
        store
            .create(
                "wh-fail",
                "test",
                &format!("{}/hook", server.uri()),
                "secret",
                &["evt".to_string()],
            )
            .await
            .unwrap();

        let delivered = dispatch_event(&storage, "default", "evt", serde_json::json!({}))
            .await
            .unwrap();
        // 500 is non-success → not counted as delivered.
        assert_eq!(delivered, 0);
    }

    #[tokio::test]
    async fn dispatch_event_records_transport_error_as_failure() {
        // No mock mounted → connection fails → request errors out.
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.webhook_store_for_agent("default");
        store
            .create(
                "wh-bad",
                "test",
                "http://127.0.0.1:1/never-binds",
                "secret",
                &["evt".to_string()],
            )
            .await
            .unwrap();

        let delivered = dispatch_event(&storage, "default", "evt", serde_json::json!({}))
            .await
            .unwrap();
        assert_eq!(delivered, 0);
    }
}
