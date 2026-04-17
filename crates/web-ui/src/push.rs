//! VAPID key provisioning and Web Push dispatch.
//!
//! Implements RFC 8291 (Message Encryption for Web Push) and RFC 8292 (VAPID)
//! using pure-Rust crates — no OpenSSL dependency.
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
use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::{
    aead::{AeadMutInPlace, KeyInit},
    Aes128Gcm, Key, Nonce,
};
use anyhow::{Context, Result};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use hkdf::Hkdf;
use p256::{
    ecdh::EphemeralSecret,
    ecdsa::{signature::Signer, SigningKey},
    elliptic_curve::sec1::ToEncodedPoint,
    pkcs8::EncodePrivateKey,
    PublicKey,
};
use sha2::Sha256;
use tracing::{debug, info, warn};

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
    if let (Some(priv_k), Some(pub_k)) = (private_key, public_key)
        && !priv_k.is_empty() && !pub_k.is_empty() {
            debug!("VAPID keys already present in config");
            return Ok((priv_k.to_string(), pub_k.to_string()));
        }

    info!("Generating new VAPID key pair…");
    let (priv_b64, pub_b64) = generate_vapid_keypair()?;

    persist_vapid_keys(config_path, &priv_b64, &pub_b64).await?;

    Ok((priv_b64, pub_b64))
}

/// Generate a fresh P-256 VAPID key pair.
///
/// Returns `(private_key_base64url, public_key_base64url)` where:
/// - The private key is PEM-encoded PKCS#8, then base64url-encoded
/// - The public key is the raw uncompressed point (65 bytes), base64url-encoded
fn generate_vapid_keypair() -> Result<(String, String)> {
    let signing_key = SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
    let verifying_key = signing_key.verifying_key();

    let pem = signing_key
        .to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
        .context("Failed to encode VAPID private key to PEM")?;
    let priv_b64 = URL_SAFE_NO_PAD.encode(pem.as_bytes());

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
    /// base64url-encoded PEM bytes of the P-256 ECDSA signing key.
    vapid_private_key_b64: Arc<String>,
    store: Arc<PushSubscriptionStore>,
}

impl PushDispatcher {
    pub fn new(vapid_private_key_b64: String, store: Arc<PushSubscriptionStore>) -> Self {
        Self {
            vapid_private_key_b64: Arc::new(vapid_private_key_b64),
            store,
        }
    }

    fn signing_key(&self) -> Result<SigningKey> {
        let pem_bytes = URL_SAFE_NO_PAD
            .decode(self.vapid_private_key_b64.as_bytes())
            .context("Failed to base64url-decode VAPID private key")?;
        let pem_str =
            std::str::from_utf8(&pem_bytes).context("VAPID private key PEM is not valid UTF-8")?;
        use p256::pkcs8::DecodePrivateKey as _;
        SigningKey::from_pkcs8_pem(pem_str).context("Failed to parse VAPID PKCS#8 PEM")
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

        let signing_key = self.signing_key()?;

        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "conversationId": conversation_id,
        })
        .to_string();

        let client = reqwest::Client::new();

        for sub in &subscriptions {
            match send_one(
                &client,
                &signing_key,
                &sub.endpoint,
                &sub.p256dh,
                &sub.auth,
                &payload,
            )
            .await
            {
                Ok(()) => {
                    debug!("Push sent to {}", sub.endpoint);
                }
                Err(PushError::Gone) => {
                    warn!("Deleting stale push subscription: {}", sub.endpoint);
                    if let Err(e) = self.store.delete(&sub.endpoint).await {
                        warn!("Failed to delete stale subscription: {e}");
                    }
                }
                Err(PushError::Other(e)) => {
                    warn!("Push delivery failed for {}: {e}", sub.endpoint);
                }
            }
        }

        Ok(())
    }
}

// -- Internal error type -----------------------------------------------------

enum PushError {
    Gone,
    Other(anyhow::Error),
}

// -- Core Web Push send ------------------------------------------------------

/// Build and deliver one encrypted, VAPID-signed push message.
///
/// Implements RFC 8291 (aesgcm → aes128gcm content encoding) and RFC 8292
/// (Voluntary Application Server Identification).
async fn send_one(
    client: &reqwest::Client,
    signing_key: &SigningKey,
    endpoint: &str,
    p256dh_b64: &str,
    auth_b64: &str,
    payload: &str,
) -> Result<(), PushError> {
    // Decrypt subscription key material.
    let p256dh = URL_SAFE_NO_PAD
        .decode(p256dh_b64)
        .or_else(|_| STANDARD.decode(p256dh_b64))
        .map_err(|e| PushError::Other(anyhow::anyhow!("bad p256dh: {e}")))?;
    let auth = URL_SAFE_NO_PAD
        .decode(auth_b64)
        .or_else(|_| STANDARD.decode(auth_b64))
        .map_err(|e| PushError::Other(anyhow::anyhow!("bad auth: {e}")))?;

    // Encrypt payload (RFC 8291 aes128gcm).
    let (ciphertext, server_public_key, salt) = encrypt_payload(payload.as_bytes(), &p256dh, &auth)
        .map_err(|e| PushError::Other(e.context("RFC 8291 encryption failed")))?;

    // Build VAPID JWT (RFC 8292).
    let vapid_token = build_vapid_jwt(signing_key, endpoint)
        .map_err(|e| PushError::Other(e.context("VAPID JWT failed")))?;

    let vapid_public_key = URL_SAFE_NO_PAD.encode(
        signing_key
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes(),
    );

    // POST to push endpoint.
    let resp = client
        .post(endpoint)
        .header("Content-Type", "application/octet-stream")
        .header("Content-Encoding", "aes128gcm")
        .header("TTL", "86400")
        .header(
            "Authorization",
            format!("vapid t={vapid_token},k={vapid_public_key}"),
        )
        .header(
            "Crypto-Key",
            format!("dh={}", URL_SAFE_NO_PAD.encode(&server_public_key)),
        )
        .header(
            "Encryption",
            format!("salt={}", URL_SAFE_NO_PAD.encode(&salt)),
        )
        .body(ciphertext)
        .send()
        .await
        .map_err(|e| PushError::Other(anyhow::anyhow!("HTTP request failed: {e}")))?;

    match resp.status().as_u16() {
        200..=204 => Ok(()),
        410 | 404 => Err(PushError::Gone),
        s => Err(PushError::Other(anyhow::anyhow!(
            "Push endpoint returned {s}"
        ))),
    }
}

// -- RFC 8291 encryption -----------------------------------------------------

/// Encrypt `plaintext` for a push subscription.
///
/// Returns `(ciphertext, server_public_key_uncompressed, salt)`.
fn encrypt_payload(
    plaintext: &[u8],
    receiver_pub_bytes: &[u8],
    auth_secret: &[u8],
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    // Parse the receiver's P-256 public key.
    let receiver_pub = PublicKey::from_sec1_bytes(receiver_pub_bytes)
        .context("Invalid receiver P-256 public key")?;

    // Generate an ephemeral sender key pair.
    let sender_secret = EphemeralSecret::random(&mut p256::elliptic_curve::rand_core::OsRng);
    let sender_pub = sender_secret.public_key();
    let sender_pub_bytes = sender_pub.to_encoded_point(false).as_bytes().to_vec();

    // ECDH shared secret.
    let shared_secret = sender_secret.diffie_hellman(&receiver_pub);
    let shared_bytes = shared_secret.raw_secret_bytes();

    // Random 16-byte salt.
    let mut salt = [0u8; 16];
    use p256::elliptic_curve::rand_core::RngCore as _;
    p256::elliptic_curve::rand_core::OsRng.fill_bytes(&mut salt);

    // RFC 8291 §3.3 — derive PRK via HKDF-SHA-256 with the auth secret.
    // ikm = ECDH output, salt = auth_secret, info = "WebPush: info\0" || ua_pub || as_pub
    let mut ikm_info = b"WebPush: info\x00".to_vec();
    ikm_info.extend_from_slice(receiver_pub_bytes);
    ikm_info.extend_from_slice(&sender_pub_bytes);

    let hk_auth = Hkdf::<Sha256>::new(Some(auth_secret), shared_bytes.as_ref());
    let mut ikm = [0u8; 32];
    hk_auth
        .expand(&ikm_info, &mut ikm)
        .map_err(|_| anyhow::anyhow!("HKDF-auth expand failed"))?;

    // Derive content encryption key (16 bytes) and nonce (12 bytes).
    let hk = Hkdf::<Sha256>::new(Some(&salt), &ikm);

    let mut cek = [0u8; 16];
    hk.expand(b"Content-Encoding: aes128gcm\x00", &mut cek)
        .map_err(|_| anyhow::anyhow!("HKDF CEK expand failed"))?;

    let mut nonce_bytes = [0u8; 12];
    hk.expand(b"Content-Encoding: nonce\x00", &mut nonce_bytes)
        .map_err(|_| anyhow::anyhow!("HKDF nonce expand failed"))?;

    // AES-128-GCM encrypt.  RFC 8291 §4: add a \x02 padding delimiter byte.
    let mut buf = plaintext.to_vec();
    buf.push(0x02); // record delimiter

    let key: &Key<Aes128Gcm> = (&cek).into();
    let mut cipher = Aes128Gcm::new(key);
    let nonce = Nonce::from(nonce_bytes);

    cipher
        .encrypt_in_place(&nonce, b"", &mut buf)
        .map_err(|_| anyhow::anyhow!("AES-GCM encryption failed"))?;

    // Build the aes128gcm content-encoding header (RFC 8188 §2.1):
    // salt (16) || rs (4, big-endian) || keyid_len (1) || keyid (sender pub, 65 bytes)
    let rs: u32 = (plaintext.len() + 1 + 16 + 1) as u32; // record size
    let mut header = Vec::with_capacity(16 + 4 + 1 + sender_pub_bytes.len());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&rs.to_be_bytes());
    header.push(sender_pub_bytes.len() as u8);
    header.extend_from_slice(&sender_pub_bytes);
    header.extend_from_slice(&buf); // ciphertext follows header

    Ok((header, sender_pub_bytes, salt.to_vec()))
}

// -- RFC 8292 VAPID JWT ------------------------------------------------------

/// Build a VAPID JWT for the given push endpoint (RFC 8292).
fn build_vapid_jwt(signing_key: &SigningKey, endpoint: &str) -> Result<String> {
    // audience = scheme + "://" + host
    let url = reqwest::Url::parse(endpoint).context("Invalid push endpoint URL")?;
    let audience = format!(
        "{}://{}",
        url.scheme(),
        url.host_str().context("Push endpoint has no host")?
    );

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Header + payload, base64url-encoded.
    let header = URL_SAFE_NO_PAD.encode(r#"{"typ":"JWT","alg":"ES256"}"#);
    let claims = serde_json::json!({
        "aud": audience,
        "exp": now + 43200, // 12 h
        "sub": "mailto:push@assistant.local",
    });
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string().as_bytes());

    let signing_input = format!("{header}.{payload}");

    // ES256 signature (RFC 7518 §3.4): fixed-size r||s (64 bytes).
    let sig: p256::ecdsa::Signature = signing_key.sign(signing_input.as_bytes());
    let sig_bytes = sig.to_bytes();
    let signature = URL_SAFE_NO_PAD.encode(&sig_bytes[..]);

    Ok(format!("{signing_input}.{signature}"))
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_roundtrip() {
        let (priv_b64, pub_b64) = generate_vapid_keypair().unwrap();
        assert!(!priv_b64.is_empty());
        assert!(!pub_b64.is_empty());

        // Public key should be 65 uncompressed bytes → 87 base64url chars (no padding).
        let pub_bytes = URL_SAFE_NO_PAD.decode(&pub_b64).unwrap();
        assert_eq!(pub_bytes.len(), 65);
        assert_eq!(pub_bytes[0], 0x04); // uncompressed point prefix
    }

    #[test]
    fn vapid_jwt_is_three_parts() {
        let (priv_b64, _) = generate_vapid_keypair().unwrap();
        let pem_bytes = URL_SAFE_NO_PAD.decode(&priv_b64).unwrap();
        let pem_str = std::str::from_utf8(&pem_bytes).unwrap();
        use p256::pkcs8::DecodePrivateKey as _;
        let key = SigningKey::from_pkcs8_pem(pem_str).unwrap();
        let jwt = build_vapid_jwt(&key, "https://fcm.googleapis.com/fcm/send/abc").unwrap();
        assert_eq!(jwt.split('.').count(), 3);
    }
}
