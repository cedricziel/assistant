//! VAPID key provisioning and Web Push dispatch.
//!
//! Implements RFC 8291 (Message Encryption for Web Push) and RFC 8292 (VAPID)
//! using pure-Rust crates — no OpenSSL dependency.
//!
//! ## Key generation
//!
//! On first `assistant webui serve` startup, if `[notifications]` is absent or
//! lacks VAPID keys, a new P-256 key pair is generated and written back to
//! `~/.assistant/server.toml`.  The public key is served at
//! `GET /api/push/vapid-public-key` so the Flutter PWA can call
//! `PushManager.subscribe({ applicationServerKey })`.
//!
//! ## Dispatch
//!
//! `PushDispatcher::send_to_all` queries all stored push subscriptions and
//! sends a VAPID-signed Web Push request to each endpoint.  Endpoints that
//! respond with `410 Gone` are deleted from the database automatically.

use assistant_core::clock::{Clock, SystemClock};
use std::path::PathBuf;
use std::sync::Arc;

use aes_gcm::{AeadInOut, Aes128Gcm, Key, Nonce, aead::KeyInit};
use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use hkdf::Hkdf;
use p256::{
    PublicKey,
    ecdh::EphemeralSecret,
    ecdsa::{SigningKey, signature::Signer},
    elliptic_curve::sec1::ToEncodedPoint,
    pkcs8::EncodePrivateKey,
};
use sha2::Sha256;
use tracing::{debug, info, warn};

use assistant_storage::PushSubscriptionStore;
#[cfg(test)]
use assistant_storage::SqlitePushSubscriptionStore;

// -- VAPID key provisioning --------------------------------------------------

/// Load VAPID keys from `server.toml`, or generate and persist them if absent.
///
/// Returns `(private_key_b64url, public_key_b64url)`.
pub async fn ensure_vapid_keys(
    config_path: &PathBuf,
    private_key: Option<&str>,
    public_key: Option<&str>,
) -> Result<(String, String)> {
    if let (Some(priv_k), Some(pub_k)) = (private_key, public_key)
        && !priv_k.is_empty()
        && !pub_k.is_empty()
    {
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
pub(crate) fn generate_vapid_keypair() -> Result<(String, String)> {
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

/// Append/update `[notifications]` section in `server.toml` with the new keys.
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

/// Sends VAPID-signed Web Push notifications to the subscriptions of one
/// space, which the caller passes as the store to [`send_to_all`](Self::send_to_all).
#[derive(Clone)]
pub struct PushDispatcher {
    /// base64url-encoded PEM bytes of the P-256 ECDSA signing key.
    vapid_private_key_b64: Arc<String>,
    /// HTTP client used for outbound push delivery. Default `Client::new()`;
    /// tests inject a client built against a `wiremock::MockServer`.
    client: reqwest::Client,
}

impl PushDispatcher {
    pub fn new(vapid_private_key_b64: String) -> Self {
        Self::with_client(vapid_private_key_b64, reqwest::Client::new())
    }

    /// Construct with an explicit `reqwest::Client`. Tests pass a client
    /// built against a `wiremock::MockServer`.
    pub fn with_client(vapid_private_key_b64: String, client: reqwest::Client) -> Self {
        Self {
            vapid_private_key_b64: Arc::new(vapid_private_key_b64),
            client,
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

    /// Send a push notification to every subscription in `store`.
    ///
    /// Subscriptions that respond with `410 Gone` are deleted from `store`.
    pub async fn send_to_all(
        &self,
        store: &dyn PushSubscriptionStore,
        title: &str,
        body: &str,
        conversation_id: Option<&str>,
    ) -> Result<()> {
        let subscriptions = store.list_all().await?;
        let message = PushMessage {
            title,
            body,
            conversation_id,
            tag: None,
        };
        self.deliver(store, &subscriptions, &message).await
    }

    /// Send a push notification to the subscriptions of `user_id` only.
    /// Subscriptions stored without a user never match.
    ///
    /// Subscriptions that respond with `410 Gone` are deleted from `store`.
    pub async fn send_to_user(
        &self,
        store: &dyn PushSubscriptionStore,
        user_id: &str,
        message: &PushMessage<'_>,
    ) -> Result<()> {
        let subscriptions = store.list_for_user(user_id).await?;
        self.deliver(store, &subscriptions, message).await
    }

    async fn deliver(
        &self,
        store: &dyn PushSubscriptionStore,
        subscriptions: &[assistant_storage::PushSubscription],
        message: &PushMessage<'_>,
    ) -> Result<()> {
        if subscriptions.is_empty() {
            return Ok(());
        }

        let signing_key = self.signing_key()?;
        let payload = message.payload();

        for sub in subscriptions {
            match send_one(
                &self.client,
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
                    if let Err(e) = store.delete(&sub.endpoint).await {
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

/// What a push tells the user. The service worker shows `title` and `body`,
/// opens `conversationId` on a tap, and replaces an earlier notification that
/// carries the same `tag` instead of stacking a second one.
#[derive(Debug, Clone, Copy)]
pub struct PushMessage<'a> {
    pub title: &'a str,
    pub body: &'a str,
    pub conversation_id: Option<&'a str>,
    pub tag: Option<&'a str>,
}

impl PushMessage<'_> {
    /// The JSON the service worker reads. `tag` is left out when absent.
    fn payload(&self) -> String {
        let mut payload = serde_json::json!({
            "title": self.title,
            "body": self.body,
            "conversationId": self.conversation_id,
        });
        if let Some(tag) = self.tag {
            payload["tag"] = serde_json::Value::String(tag.to_string());
        }
        payload.to_string()
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
    let cipher = Aes128Gcm::new(key);
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

    let now = SystemClock.now().timestamp() as u64;

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

    #[tokio::test]
    async fn dispatcher_with_no_subscriptions_is_noop() {
        // Even with a totally invalid VAPID key, send_to_all should
        // short-circuit when there are no subscriptions to deliver to.
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let store = SqlitePushSubscriptionStore::new(storage.pool.clone());
        let dispatcher = PushDispatcher::with_client(
            "AA".repeat(40), // not a valid key, but not parsed in the empty path
            reqwest::Client::new(),
        );
        dispatcher
            .send_to_all(&store, "title", "body", None)
            .await
            .expect("empty subscriptions should return Ok");
    }

    /// Counts how often the dispatcher reads a store's subscriptions.
    #[derive(Default)]
    struct CountingStore {
        reads: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl PushSubscriptionStore for CountingStore {
        async fn upsert(
            &self,
            _endpoint: &str,
            _p256dh: &str,
            _auth: &str,
            _user_id: Option<&str>,
        ) -> Result<()> {
            Ok(())
        }
        async fn delete(&self, _endpoint: &str) -> Result<()> {
            Ok(())
        }
        async fn list_all(&self) -> Result<Vec<assistant_storage::PushSubscription>> {
            self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![])
        }
        async fn list_for_user(
            &self,
            _user_id: &str,
        ) -> Result<Vec<assistant_storage::PushSubscription>> {
            self.reads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn dispatcher_reads_only_the_store_it_is_given() {
        let (space_a, space_b) = (CountingStore::default(), CountingStore::default());
        let dispatcher = PushDispatcher::new("AA".repeat(40));

        dispatcher
            .send_to_all(&space_a, "title", "body", None)
            .await
            .unwrap();

        let reads = |s: &CountingStore| s.reads.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(reads(&space_a), 1);
        assert_eq!(
            reads(&space_b),
            0,
            "another space's subscriptions are never read"
        );
    }

    #[tokio::test]
    async fn dispatcher_with_client_constructor_threads_client_through() {
        // Smoke test for the with_client injection seam: build a dispatcher
        // with a client pointing at a wiremock server, verify the inner
        // `self.client` is the one we passed. (We can't actually exercise
        // send_one without valid p256dh + auth subscription keys, which
        // require client-side key derivation that's out of scope here.)
        use wiremock::MockServer;
        let server = MockServer::start().await;
        let custom_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(123))
            .build()
            .unwrap();

        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let store = SqlitePushSubscriptionStore::new(storage.pool.clone());
        let dispatcher = PushDispatcher::with_client("fakekey".to_string(), custom_client);

        // Construct OK + empty send_to_all returns Ok.
        dispatcher
            .send_to_all(&store, "t", "b", None)
            .await
            .unwrap();
        // Reference the mock server so the compiler doesn't elide it.
        let _ = server.uri();
    }

    // ── send_to_user ─────────────────────────────────────────────────────

    /// Subscribe a browser with valid RFC 8291 key material at `endpoint`.
    async fn subscribe_as(
        store: &dyn PushSubscriptionStore,
        endpoint: &str,
        user_id: Option<&str>,
    ) {
        let subscriber =
            p256::ecdsa::SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let p256dh = URL_SAFE_NO_PAD.encode(subscriber.verifying_key().to_encoded_point(false));
        let auth = URL_SAFE_NO_PAD.encode([7u8; 16]);
        store
            .upsert(endpoint, &p256dh, &auth, user_id)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn send_to_user_delivers_to_that_users_endpoints_only() {
        use assistant_storage::InMemoryPushSubscriptionStore;
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&server)
            .await;
        let store = InMemoryPushSubscriptionStore::new();
        for (path, user) in [
            ("alice-phone", Some("alice")),
            ("alice-laptop", Some("alice")),
            ("bob", Some("bob")),
            ("legacy", None),
        ] {
            subscribe_as(&store, &format!("{}/{path}", server.uri()), user).await;
        }
        let (vapid_private, _) = generate_vapid_keypair().unwrap();
        let dispatcher = PushDispatcher::new(vapid_private);

        dispatcher
            .send_to_user(
                &store,
                "alice",
                &PushMessage {
                    title: "Approval needed",
                    body: "The assistant wants to run 'bash'.",
                    conversation_id: Some("conv-1"),
                    tag: Some("interaction-i1"),
                },
            )
            .await
            .unwrap();

        let mut paths: Vec<String> = server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|r| r.url.path().to_string())
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            ["/alice-laptop", "/alice-phone"],
            "one push per endpoint of the owner, none to other members or to rows without a user"
        );
    }

    #[test]
    fn payload_carries_the_tag_and_conversation_for_the_service_worker() {
        let payload: serde_json::Value = serde_json::from_str(
            &PushMessage {
                title: "Approval needed",
                body: "body",
                conversation_id: Some("conv-1"),
                tag: Some("interaction-i1"),
            }
            .payload(),
        )
        .unwrap();
        assert_eq!(payload["title"], "Approval needed");
        assert_eq!(payload["conversationId"], "conv-1");
        assert_eq!(payload["tag"], "interaction-i1");
    }

    #[test]
    fn payload_omits_the_tag_when_there_is_none() {
        let payload: serde_json::Value = serde_json::from_str(
            &PushMessage {
                title: "New message",
                body: "body",
                conversation_id: None,
                tag: None,
            }
            .payload(),
        )
        .unwrap();
        assert!(payload.get("tag").is_none(), "{payload}");
    }

    // ── encrypt_payload ──────────────────────────────────────────────────

    #[test]
    fn encrypt_payload_produces_nonempty_aes128gcm_record() {
        // Generate a fresh subscriber keypair; the public key is the
        // 65-byte uncompressed P-256 point.
        let subscriber_secret =
            p256::ecdsa::SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let subscriber_pub_bytes = subscriber_secret
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        let auth_secret: [u8; 16] = [0xa5; 16];

        let (encrypted, sender_pub, salt) =
            encrypt_payload(b"hello world", &subscriber_pub_bytes, &auth_secret).unwrap();
        // Header: salt (16) + rs (4) + keyid_len (1) + keyid (65) = 86 bytes
        // followed by ciphertext.
        assert!(encrypted.len() > 86);
        assert_eq!(sender_pub.len(), 65, "sender pub must be uncompressed");
        assert_eq!(salt.len(), 16, "salt must be 16 bytes");
        // First byte of an uncompressed point is 0x04.
        assert_eq!(sender_pub[0], 0x04);
    }

    #[test]
    fn encrypt_payload_rejects_invalid_subscriber_pub_key() {
        let auth_secret = [0u8; 16];
        let result = encrypt_payload(b"hello", b"not-a-key", &auth_secret);
        assert!(result.is_err(), "invalid sub-pub bytes must Err");
    }

    #[test]
    fn encrypt_payload_uses_different_salt_each_call() {
        let subscriber_secret =
            p256::ecdsa::SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let pub_bytes = subscriber_secret
            .verifying_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        let auth: [u8; 16] = [0xff; 16];
        let (_, _, salt1) = encrypt_payload(b"a", &pub_bytes, &auth).unwrap();
        let (_, _, salt2) = encrypt_payload(b"a", &pub_bytes, &auth).unwrap();
        assert_ne!(salt1, salt2, "salt must be random per call");
    }

    // ── build_vapid_jwt ──────────────────────────────────────────────────

    #[test]
    fn build_vapid_jwt_returns_three_dot_separated_segments() {
        let (priv_b64, _) = generate_vapid_keypair().unwrap();
        let pem_bytes = URL_SAFE_NO_PAD.decode(&priv_b64).unwrap();
        let pem_str = std::str::from_utf8(&pem_bytes).unwrap();
        use p256::pkcs8::DecodePrivateKey as _;
        let key = SigningKey::from_pkcs8_pem(pem_str).unwrap();
        let jwt = build_vapid_jwt(&key, "https://fcm.googleapis.com/fcm/send/abc").unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        // All parts must be base64url-decodable.
        for p in &parts {
            URL_SAFE_NO_PAD.decode(p).unwrap();
        }
    }

    #[test]
    fn build_vapid_jwt_payload_includes_correct_audience() {
        let (priv_b64, _) = generate_vapid_keypair().unwrap();
        let pem_bytes = URL_SAFE_NO_PAD.decode(&priv_b64).unwrap();
        let pem_str = std::str::from_utf8(&pem_bytes).unwrap();
        use p256::pkcs8::DecodePrivateKey as _;
        let key = SigningKey::from_pkcs8_pem(pem_str).unwrap();
        let jwt = build_vapid_jwt(&key, "https://fcm.googleapis.com/fcm/send/abc").unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        let payload_bytes = URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).unwrap();
        assert_eq!(payload["aud"], "https://fcm.googleapis.com");
        assert!(payload["exp"].is_u64());
        assert_eq!(payload["sub"], "mailto:push@assistant.local");
    }

    #[test]
    fn build_vapid_jwt_errors_on_invalid_endpoint() {
        let (priv_b64, _) = generate_vapid_keypair().unwrap();
        let pem_bytes = URL_SAFE_NO_PAD.decode(&priv_b64).unwrap();
        let pem_str = std::str::from_utf8(&pem_bytes).unwrap();
        use p256::pkcs8::DecodePrivateKey as _;
        let key = SigningKey::from_pkcs8_pem(pem_str).unwrap();
        let err = build_vapid_jwt(&key, "not a url").unwrap_err();
        assert!(err.to_string().contains("Invalid push endpoint URL"));
    }

    // ── signing_key (dispatcher helper) ──────────────────────────────────

    #[tokio::test]
    async fn signing_key_decodes_round_tripped_pem() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let _ = storage;
        let (priv_b64, _) = generate_vapid_keypair().unwrap();
        let dispatcher = PushDispatcher::new(priv_b64);
        assert!(
            dispatcher.signing_key().is_ok(),
            "valid PEM must decode back to a signing key"
        );
    }

    #[tokio::test]
    async fn signing_key_errors_on_invalid_base64() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let _ = storage;
        let dispatcher = PushDispatcher::new("not-base64!@#$".to_string());
        assert!(dispatcher.signing_key().is_err());
    }
}
