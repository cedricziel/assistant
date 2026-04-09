//! VAPID key provisioning and Web Push dispatch.
//!
//! ## Key generation
//!
//! On first `assistant webui serve` startup, if `[notifications]` is absent or
//! lacks VAPID keys, a new P-256 key pair is generated and written back to
//! `~/.assistant/config.toml`.  The public key is served at
//! `GET /api/push/vapid-public-key` so the Flutter PWA can call
//! `PushManager.subscribe({ applicationServerKey })`.
//!
//! ## Dispatch
//!
//! `PushDispatcher::send_to_all` queries all stored push subscriptions and
//! sends a VAPID-signed Web Push request to each endpoint.  Endpoints that
//! respond with `410 Gone` are deleted from the database automatically.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use p256::pkcs8::{EncodePrivateKey, LineEnding};
use tracing::{debug, info, warn};
use web_push::{
    ContentEncoding, HyperWebPushClient, SubscriptionInfo, VapidSignatureBuilder, WebPushClient,
    WebPushMessageBuilder,
};

use assistant_storage::PushSubscriptionStore;

// -- VAPID key provisioning --------------------------------------------------

/// Load VAPID keys from `config.toml`, or generate and persist them if absent.
///
/// Returns `(private_key_b64url, public_key_b64url)`.
pub async fn ensure_vapid_keys(
    config_path: &PathBuf,
    private_key: Option<&str>,
    public_key: Option<&str>,
) -> Result<(String, String)> {
    if let (Some(priv_k), Some(pub_k)) = (private_key, public_key) {
        if !priv_k.is_empty() && !pub_k.is_empty() {
            debug!("VAPID keys already present in config");
            return Ok((priv_k.to_string(), pub_k.to_string()));
        }
    }

    info!("Generating new VAPID key pair…");
    let (priv_b64, pub_b64) = generate_vapid_keypair()?;

    // Write back to config.toml
    persist_vapid_keys(config_path, &priv_b64, &pub_b64).await?;

    Ok((priv_b64, pub_b64))
}

/// Generate a fresh P-256 VAPID key pair.
///
/// Returns `(private_key_base64url, public_key_base64url)` where:
/// - The private key is PEM-encoded PKCS#8 (for `VapidSignatureBuilder::from_pem`)
/// - The public key is the raw uncompressed point (65 bytes), base64url-encoded
///   (the format expected by the browser's `PushManager.subscribe`)
fn generate_vapid_keypair() -> Result<(String, String)> {
    let signing_key = p256::ecdsa::SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
    let verifying_key = signing_key.verifying_key();

    // Private key: PEM (PKCS#8) for web-push crate
    let pem = signing_key
        .to_pkcs8_pem(LineEnding::LF)
        .context("Failed to encode VAPID private key to PEM")?;
    let priv_b64 = URL_SAFE_NO_PAD.encode(pem.as_bytes());

    // Public key: uncompressed EC point (04 || x || y), base64url (no padding)
    // This is the applicationServerKey format expected by the browser.
    let point_bytes = verifying_key.to_encoded_point(false);
    let pub_b64 = URL_SAFE_NO_PAD.encode(point_bytes.as_bytes());

    Ok((priv_b64, pub_b64))
}

/// Append/update `[notifications]` section in `config.toml` with the new keys.
async fn persist_vapid_keys(
    config_path: &PathBuf,
    private_key_b64: &str,
    public_key_b64: &str,
) -> Result<()> {
    // Read existing config as raw string (preserve user comments/structure).
    let raw = tokio::fs::read_to_string(config_path)
        .await
        .unwrap_or_default();
    let mut doc: toml_edit::DocumentMut = raw.parse().unwrap_or_default();

    if !doc.contains_key("notifications") {
        doc["notifications"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    doc["notifications"]["vapid_private_key"] = toml_edit::value(private_key_b64);
    doc["notifications"]["vapid_public_key"] = toml_edit::value(public_key_b64);

    tokio::fs::write(config_path, doc.to_string())
        .await
        .with_context(|| format!("Failed to write VAPID keys to {}", config_path.display()))?;

    info!("VAPID keys written to {}", config_path.display());
    Ok(())
}

// -- PushDispatcher ----------------------------------------------------------

/// Sends VAPID-signed Web Push notifications to all stored subscriptions.
#[derive(Clone)]
pub struct PushDispatcher {
    /// PEM bytes of the P-256 private key, base64url-encoded.
    vapid_private_key_b64: Arc<String>,
    store: Arc<PushSubscriptionStore>,
}

impl PushDispatcher {
    /// Create a new dispatcher.
    ///
    /// `vapid_private_key_b64` is the base64url-encoded PEM bytes of the P-256
    /// private key (as produced by `ensure_vapid_keys`).
    pub fn new(vapid_private_key_b64: String, store: Arc<PushSubscriptionStore>) -> Self {
        Self {
            vapid_private_key_b64: Arc::new(vapid_private_key_b64),
            store,
        }
    }

    /// Decode the stored base64url-encoded PEM private key back to PEM bytes.
    fn pem_bytes(&self) -> Result<Vec<u8>> {
        URL_SAFE_NO_PAD
            .decode(self.vapid_private_key_b64.as_bytes())
            .context("Failed to base64url-decode VAPID private key")
    }

    /// Send a push notification to every stored subscription.
    ///
    /// Subscriptions that respond with `410 Gone` are deleted automatically.
    pub async fn send_to_all(
        &self,
        title: &str,
        body: &str,
        conversation_id: Option<&str>,
    ) -> Result<()> {
        let subscriptions = self.store.list_all().await?;
        if subscriptions.is_empty() {
            return Ok(());
        }

        let pem = self.pem_bytes()?;
        let client = HyperWebPushClient::new();

        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "conversationId": conversation_id,
        })
        .to_string();

        for sub in &subscriptions {
            let info = SubscriptionInfo::new(&sub.endpoint, &sub.p256dh, &sub.auth);

            let sig = VapidSignatureBuilder::from_pem(std::io::Cursor::new(&pem), &info)
                .context("Failed to build VAPID signature builder")?
                .build()
                .context("Failed to build VAPID signature")?;

            let mut builder = WebPushMessageBuilder::new(&info);
            builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
            builder.set_vapid_signature(sig);
            builder.set_ttl(86400); // 24 h

            let msg = match builder.build() {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to build push message for {}: {e}", sub.endpoint);
                    continue;
                }
            };

            match client.send(msg).await {
                Ok(()) => {
                    debug!("Push sent to {}", sub.endpoint);
                }
                Err(web_push::WebPushError::EndpointNotValid(_))
                | Err(web_push::WebPushError::EndpointNotFound(_)) => {
                    warn!("Deleting stale push subscription: {}", sub.endpoint);
                    if let Err(e) = self.store.delete(&sub.endpoint).await {
                        warn!("Failed to delete stale subscription: {e}");
                    }
                }
                Err(e) => {
                    warn!("Push delivery failed for {}: {e}", sub.endpoint);
                }
            }
        }

        Ok(())
    }
}
