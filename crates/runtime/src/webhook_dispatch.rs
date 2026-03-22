use anyhow::Result;
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use tracing::{info, warn};

use assistant_storage::StorageLayer;

type HmacSha256 = Hmac<Sha256>;

/// Deliver one event payload to all active webhooks subscribed to `event_type`
/// within the current assistant agent context.
pub async fn dispatch_event(
    storage: &StorageLayer,
    agent_id: &str,
    event_type: &str,
    payload: Value,
) -> Result<usize> {
    let store = storage.webhook_store_for_agent(agent_id);
    let webhooks = store.list_active_for_event(event_type).await?;
    if webhooks.is_empty() {
        return Ok(0);
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let body = serde_json::to_string(&payload)?;
    let mut delivered = 0usize;

    for webhook in webhooks {
        let signature = compute_signature(&webhook.secret, &body);
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
            }
            Ok(resp) => {
                warn!(
                    webhook_id = %webhook.id,
                    event_type,
                    status = %resp.status(),
                    "Webhook delivery failed with non-success status"
                );
            }
            Err(e) => {
                warn!(
                    webhook_id = %webhook.id,
                    event_type,
                    error = %e,
                    "Webhook delivery request failed"
                );
            }
        }
    }

    info!(
        event_type,
        attempted = payload_count_hint(&payload),
        delivered,
        "Dispatched webhook event"
    );
    Ok(delivered)
}

fn compute_signature(secret: &str, body: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn payload_count_hint(payload: &Value) -> usize {
    if payload.is_null() {
        0
    } else {
        1
    }
}
