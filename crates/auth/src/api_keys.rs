//! Scoped API key generation and resolution.
//!
//! API keys provide non-interactive authentication for scripts, CI pipelines,
//! and integrations. Each key is scoped to specific resources and actions,
//! carries an expiration, and can optionally restrict access to specific
//! resource IDs.
//!
//! Key format: `ask_live_{32 random base64url chars}` — the `ask_live_` prefix
//! enables identification and scanning. Only the SHA-256 hash is stored; the
//! plaintext is shown exactly once at creation time.

use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use tracing::debug;

use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, Role, Scope, SpaceId, UserId};

// -- Constants --

/// Prefix for all API keys (enables scanning / identification).
const KEY_PREFIX_TAG: &str = "ask_live_";

/// Number of random bytes in the key body (32 bytes → 43 base64url chars).
const KEY_RANDOM_BYTES: usize = 32;

// -- Types --

/// A newly generated API key. The `plaintext` field is shown once; only
/// `key_hash` and `key_prefix` are stored.
#[derive(Clone, Debug)]
pub struct GeneratedKey {
    /// The full key string (e.g. `ask_live_<base64url>`). Show once, never store.
    pub plaintext: String,
    /// SHA-256 hash of the full key (hex-encoded). Stored for lookup.
    pub key_hash: String,
    /// First 8 characters of the key (after prefix). Stored for display.
    pub key_prefix: String,
}

/// Metadata about a stored API key.
#[derive(Clone, Debug)]
pub struct ApiKeyRecord {
    /// Unique ID for this key.
    pub id: String,
    /// The user who owns this key.
    pub user_id: UserId,
    /// Human-readable name.
    pub name: String,
    /// SHA-256 hash of the full key (hex-encoded).
    pub key_hash: String,
    /// First 8 characters of the key body (for display).
    pub key_prefix: String,
    /// The organization this key belongs to.
    pub org_id: OrgId,
    /// Space → role mapping inherited from the user at creation time.
    pub space_roles: HashMap<SpaceId, Role>,
    /// Granted scopes (may be a subset of the user's full scopes).
    pub scopes: Vec<Scope>,
    /// When this key expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// When this key was created.
    pub created_at: DateTime<Utc>,
}

// -- Key generation --

/// Generate a new API key.
///
/// Returns the plaintext (show once), the hash (for storage), and the prefix
/// (for display in listings).
pub fn generate_key() -> GeneratedKey {
    use rand::Rng;

    let mut rng = rand::rng();
    let mut random_bytes = vec![0u8; KEY_RANDOM_BYTES];
    rng.fill(&mut random_bytes[..]);

    let body = URL_SAFE_NO_PAD.encode(&random_bytes);
    let plaintext = format!("{KEY_PREFIX_TAG}{body}");
    let key_hash = hash_key(&plaintext);
    let key_prefix = body[..8].to_string();

    GeneratedKey {
        plaintext,
        key_hash,
        key_prefix,
    }
}

/// Compute the SHA-256 hash of a key (hex-encoded).
pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

// -- Store trait --

/// Trait for API key persistence.
#[async_trait::async_trait]
pub trait ApiKeyStore: Send + Sync {
    /// Store a new API key record.
    async fn store_key(&self, record: ApiKeyRecord) -> Result<()>;
    /// Look up a key by its hash.
    async fn get_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyRecord>>;
    /// List all keys for a user (without secrets).
    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ApiKeyRecord>>;
    /// Revoke (delete) a key by ID.
    async fn revoke(&self, key_id: &str, user_id: &UserId) -> Result<bool>;
}

// -- Resolution --

/// Resolve an API key string to an [`AuthContext`].
///
/// Hashes the provided key, looks it up in the store, validates expiry,
/// and builds an AuthContext from the stored metadata.
pub async fn resolve_key(key_plaintext: &str, store: &dyn ApiKeyStore) -> Result<AuthContext> {
    // Must start with the expected prefix.
    if !key_plaintext.starts_with(KEY_PREFIX_TAG) {
        bail!("invalid API key format: missing prefix");
    }

    let key_hash = hash_key(key_plaintext);
    let record = store
        .get_by_hash(&key_hash)
        .await?
        .context("API key not found")?;

    // Check expiry.
    if let Some(expires_at) = record.expires_at
        && Utc::now() > expires_at
    {
        bail!("API key has expired");
    }

    debug!(user_id = %record.user_id, key_prefix = %record.key_prefix, "resolved API key");

    Ok(AuthContext {
        user_id: record.user_id,
        org_id: record.org_id,
        email: String::new(), // Not stored in the key — caller can enrich.
        space_roles: record.space_roles,
        scopes: record.scopes,
        client_id: "api_key".into(),
    })
}

// -- In-memory store (for tests and single-process deployments) --

/// An in-memory API key store.
pub struct InMemoryApiKeyStore {
    records: Mutex<Vec<ApiKeyRecord>>,
}

impl InMemoryApiKeyStore {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryApiKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ApiKeyStore for InMemoryApiKeyStore {
    async fn store_key(&self, record: ApiKeyRecord) -> Result<()> {
        self.records.lock().unwrap().push(record);
        Ok(())
    }

    async fn get_by_hash(&self, key_hash: &str) -> Result<Option<ApiKeyRecord>> {
        let records = self.records.lock().unwrap();
        Ok(records.iter().find(|r| r.key_hash == key_hash).cloned())
    }

    async fn list_for_user(&self, user_id: &UserId) -> Result<Vec<ApiKeyRecord>> {
        let records = self.records.lock().unwrap();
        Ok(records
            .iter()
            .filter(|r| r.user_id == *user_id)
            .cloned()
            .collect())
    }

    async fn revoke(&self, key_id: &str, user_id: &UserId) -> Result<bool> {
        let mut records = self.records.lock().unwrap();
        let len_before = records.len();
        records.retain(|r| !(r.id == key_id && r.user_id == *user_id));
        Ok(records.len() < len_before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use assistant_core::identity::{Action, ResourceKind};

    fn make_test_record(key: &GeneratedKey, scopes: Vec<Scope>) -> ApiKeyRecord {
        let mut space_roles = HashMap::new();
        space_roles.insert(SpaceId::from("eng"), Role::Member);

        ApiKeyRecord {
            id: "key_1".into(),
            user_id: UserId::from("usr_alice"),
            name: "Test Key".into(),
            key_hash: key.key_hash.clone(),
            key_prefix: key.key_prefix.clone(),
            org_id: OrgId::from("org_acme"),
            space_roles,
            scopes,
            expires_at: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn generate_key_has_correct_format() {
        let key = generate_key();
        assert!(
            key.plaintext.starts_with("ask_live_"),
            "key should start with ask_live_ prefix"
        );
        // 9 chars prefix + 43 chars base64url body = 52 total
        assert!(
            key.plaintext.len() >= 50,
            "key should be at least 50 chars, got {}",
            key.plaintext.len()
        );
        assert_eq!(key.key_prefix.len(), 8, "prefix should be 8 chars");
        assert!(!key.key_hash.is_empty(), "hash should not be empty");
    }

    #[test]
    fn generate_key_produces_unique_keys() {
        let k1 = generate_key();
        let k2 = generate_key();
        assert_ne!(k1.plaintext, k2.plaintext, "keys should be unique");
        assert_ne!(k1.key_hash, k2.key_hash, "hashes should be unique");
    }

    #[test]
    fn hash_key_is_deterministic() {
        let key = "ask_live_test1234567890abcdef";
        let h1 = hash_key(key);
        let h2 = hash_key(key);
        assert_eq!(h1, h2, "hashing the same key should produce the same hash");
    }

    #[test]
    fn hash_key_produces_hex_sha256() {
        let hash = hash_key("test");
        // SHA-256 hex = 64 chars
        assert_eq!(hash.len(), 64, "SHA-256 hex should be 64 chars");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "hash should be hex-encoded"
        );
    }

    #[tokio::test]
    async fn resolve_key_returns_auth_context() {
        let store = InMemoryApiKeyStore::new();
        let key = generate_key();
        let scopes = vec![
            Scope::new(ResourceKind::Personas, Action::Read),
            Scope::new(ResourceKind::Conversations, Action::Read),
        ];
        let record = make_test_record(&key, scopes);
        store.store_key(record).await.unwrap();

        let ctx = resolve_key(&key.plaintext, &store).await.unwrap();
        assert_eq!(ctx.user_id, UserId::from("usr_alice"));
        assert_eq!(ctx.org_id, OrgId::from("org_acme"));
        assert_eq!(ctx.client_id, "api_key");
        assert_eq!(ctx.scopes.len(), 2);
        assert_eq!(ctx.role_in(&SpaceId::from("eng")), Some(&Role::Member));
    }

    #[tokio::test]
    async fn resolve_key_rejects_unknown_key() {
        let store = InMemoryApiKeyStore::new();
        let err = resolve_key("ask_live_unknown_key_12345678901234", &store).await;
        assert!(err.is_err(), "should reject unknown key");
    }

    #[tokio::test]
    async fn resolve_key_rejects_invalid_prefix() {
        let store = InMemoryApiKeyStore::new();
        let err = resolve_key("invalid_prefix_key", &store).await;
        assert!(err.is_err(), "should reject key without prefix");
        let msg = format!("{:?}", err.unwrap_err());
        assert!(msg.contains("missing prefix"));
    }

    #[tokio::test]
    async fn resolve_key_rejects_expired() {
        let store = InMemoryApiKeyStore::new();
        let key = generate_key();
        let mut record = make_test_record(&key, vec![]);
        record.expires_at = Some(Utc::now() - chrono::Duration::hours(1));
        store.store_key(record).await.unwrap();

        let err = resolve_key(&key.plaintext, &store).await;
        assert!(err.is_err(), "should reject expired key");
        let msg = format!("{:?}", err.unwrap_err());
        assert!(msg.contains("expired"));
    }

    #[tokio::test]
    async fn resolve_key_with_resource_restrictions() {
        let store = InMemoryApiKeyStore::new();
        let key = generate_key();
        let scopes = vec![Scope::restricted(
            ResourceKind::Personas,
            Action::Read,
            vec!["deploy-bot".into()],
        )];
        let record = make_test_record(&key, scopes);
        store.store_key(record).await.unwrap();

        let ctx = resolve_key(&key.plaintext, &store).await.unwrap();
        let space = SpaceId::from("eng");

        // Can read the specific restricted persona.
        assert!(ctx.can(
            &space,
            &ResourceKind::Personas,
            &Action::Read,
            Some("deploy-bot"),
        ));
        // Cannot read a different persona.
        assert!(!ctx.can(
            &space,
            &ResourceKind::Personas,
            &Action::Read,
            Some("other-bot"),
        ));
    }

    #[tokio::test]
    async fn list_keys_for_user() {
        let store = InMemoryApiKeyStore::new();
        let k1 = generate_key();
        let k2 = generate_key();

        let mut r1 = make_test_record(&k1, vec![]);
        r1.id = "key_1".into();
        r1.name = "Key 1".into();

        let mut r2 = make_test_record(&k2, vec![]);
        r2.id = "key_2".into();
        r2.name = "Key 2".into();

        // Different user's key.
        let k3 = generate_key();
        let mut r3 = make_test_record(&k3, vec![]);
        r3.id = "key_3".into();
        r3.user_id = UserId::from("usr_bob");

        store.store_key(r1).await.unwrap();
        store.store_key(r2).await.unwrap();
        store.store_key(r3).await.unwrap();

        let alice_keys = store
            .list_for_user(&UserId::from("usr_alice"))
            .await
            .unwrap();
        assert_eq!(alice_keys.len(), 2);

        let bob_keys = store.list_for_user(&UserId::from("usr_bob")).await.unwrap();
        assert_eq!(bob_keys.len(), 1);
    }

    #[tokio::test]
    async fn revoke_key_removes_it() {
        let store = InMemoryApiKeyStore::new();
        let key = generate_key();
        let record = make_test_record(&key, vec![]);
        store.store_key(record).await.unwrap();

        // Revoke succeeds.
        let revoked = store
            .revoke("key_1", &UserId::from("usr_alice"))
            .await
            .unwrap();
        assert!(revoked, "revoke should return true");

        // Key no longer resolvable.
        let err = resolve_key(&key.plaintext, &store).await;
        assert!(err.is_err(), "revoked key should not resolve");
    }

    #[tokio::test]
    async fn revoke_wrong_user_fails() {
        let store = InMemoryApiKeyStore::new();
        let key = generate_key();
        let record = make_test_record(&key, vec![]);
        store.store_key(record).await.unwrap();

        // Bob cannot revoke Alice's key.
        let revoked = store
            .revoke("key_1", &UserId::from("usr_bob"))
            .await
            .unwrap();
        assert!(!revoked, "should not revoke another user's key");

        // Key still resolvable.
        let ctx = resolve_key(&key.plaintext, &store).await.unwrap();
        assert_eq!(ctx.user_id, UserId::from("usr_alice"));
    }
}
