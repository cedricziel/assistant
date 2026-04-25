//! Device authorization grant (RFC 8628).
//!
//! Enables CLI and other input-constrained clients to authenticate by
//! displaying a user code that the user enters in a browser.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, bail};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// `DeviceCodeStore` and `DeviceState` live in `assistant_core::auth`; re-export
// here for back-compat.
pub use assistant_core::auth::{DeviceCodeStore, DeviceState};

// -- Types --

/// Response to a device authorization request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    /// Opaque device code used for polling.
    pub device_code: String,
    /// Short human-readable code the user enters in a browser.
    pub user_code: String,
    /// URL where the user should go to enter the code.
    pub verification_uri: String,
    /// Recommended polling interval in seconds.
    pub interval: u64,
    /// When this device code expires.
    pub expires_in: u64,
}

/// The result of polling for device authorization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PollResult {
    /// The user hasn't authorized yet — keep polling.
    Pending,
    /// The user authorized — here are the tokens.
    Authorized {
        user_id: String,
        scopes: Vec<String>,
    },
    /// The device code has expired.
    Expired,
}

// -- In-memory implementation --

/// In-memory device code store for testing and single-server deployments.
#[derive(Clone, Default)]
pub struct InMemoryDeviceCodeStore {
    entries: Arc<RwLock<HashMap<String, DeviceState>>>,
}

#[async_trait::async_trait]
impl DeviceCodeStore for InMemoryDeviceCodeStore {
    async fn store(&self, device_code: &str, state: DeviceState) -> Result<()> {
        self.entries
            .write()
            .await
            .insert(device_code.to_owned(), state);
        Ok(())
    }

    async fn get_by_device_code(&self, device_code: &str) -> Result<Option<DeviceState>> {
        Ok(self.entries.read().await.get(device_code).cloned())
    }

    async fn get_by_user_code(&self, user_code: &str) -> Result<Option<DeviceState>> {
        Ok(self
            .entries
            .read()
            .await
            .values()
            .find(|s| s.user_code == user_code)
            .cloned())
    }

    async fn update(&self, device_code: &str, state: DeviceState) -> Result<()> {
        self.entries
            .write()
            .await
            .insert(device_code.to_owned(), state);
        Ok(())
    }

    async fn remove(&self, device_code: &str) -> Result<()> {
        self.entries.write().await.remove(device_code);
        Ok(())
    }
}

// -- Device code manager --

/// Manages the device authorization grant flow.
pub struct DeviceCodeManager {
    store: Arc<dyn DeviceCodeStore>,
    verification_uri: String,
    code_ttl: Duration,
    poll_interval: u64,
}

impl DeviceCodeManager {
    /// Create a new device code manager.
    pub fn new(store: Arc<dyn DeviceCodeStore>, verification_uri: String) -> Self {
        Self {
            store,
            verification_uri,
            code_ttl: Duration::minutes(15),
            poll_interval: 5,
        }
    }

    /// Override the device code TTL.
    pub fn with_code_ttl(mut self, ttl: Duration) -> Self {
        self.code_ttl = ttl;
        self
    }

    /// Initiate a new device authorization flow.
    pub async fn initiate(
        &self,
        client_id: &str,
        scopes: Vec<String>,
    ) -> Result<DeviceCodeResponse> {
        let device_code = generate_random_token();
        let user_code = generate_user_code();
        let expires_at = Utc::now() + self.code_ttl;

        let state = DeviceState {
            device_code: device_code.clone(),
            user_code: user_code.clone(),
            client_id: client_id.to_owned(),
            scopes,
            expires_at,
            authorized_user: None,
        };

        self.store.store(&device_code, state).await?;

        Ok(DeviceCodeResponse {
            device_code,
            user_code,
            verification_uri: self.verification_uri.clone(),
            interval: self.poll_interval,
            expires_in: self.code_ttl.num_seconds() as u64,
        })
    }

    /// Poll for the result of a device authorization.
    pub async fn poll(&self, device_code: &str) -> Result<PollResult> {
        let state = self
            .store
            .get_by_device_code(device_code)
            .await?
            .ok_or_else(|| anyhow::anyhow!("unknown device code"))?;

        if Utc::now() > state.expires_at {
            self.store.remove(device_code).await?;
            return Ok(PollResult::Expired);
        }

        match state.authorized_user {
            Some(user_id) => {
                self.store.remove(device_code).await?;
                Ok(PollResult::Authorized {
                    user_id,
                    scopes: state.scopes,
                })
            }
            None => Ok(PollResult::Pending),
        }
    }

    /// Complete the device flow — called when the user approves in the browser.
    pub async fn complete(&self, user_code: &str, user_id: &str) -> Result<()> {
        let state = self
            .store
            .get_by_user_code(user_code)
            .await?
            .ok_or_else(|| anyhow::anyhow!("unknown user code"))?;

        if Utc::now() > state.expires_at {
            self.store.remove(&state.device_code).await?;
            bail!("device code expired");
        }

        let device_code = state.device_code.clone();
        let updated = DeviceState {
            authorized_user: Some(user_id.to_owned()),
            ..state
        };
        self.store.update(&device_code, updated).await?;
        Ok(())
    }
}

/// Generate a URL-safe random token.
fn generate_random_token() -> String {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use rand::Rng;

    let mut rng = rand::rng();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Generate a short, human-friendly user code (e.g., "ABCD-EFGH").
fn generate_user_code() -> String {
    use rand::Rng;

    // Use uppercase letters excluding easily confused chars (0/O, 1/I/L).
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ";
    let mut rng = rand::rng();
    let left: String = (0..4)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect();
    let right: String = (0..4)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect();
    format!("{left}-{right}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> DeviceCodeManager {
        DeviceCodeManager::new(
            Arc::new(InMemoryDeviceCodeStore::default()),
            "https://example.com/device".into(),
        )
    }

    #[tokio::test]
    async fn initiate_returns_valid_response() {
        let mgr = make_manager();
        let resp = mgr
            .initiate("cli", vec!["personas:read".into()])
            .await
            .unwrap();

        assert!(!resp.device_code.is_empty(), "should have device code");
        assert_eq!(resp.user_code.len(), 9, "user code should be XXXX-XXXX");
        assert!(resp.user_code.contains('-'), "user code should have dash");
        assert_eq!(resp.verification_uri, "https://example.com/device");
        assert_eq!(resp.interval, 5);
    }

    #[tokio::test]
    async fn poll_returns_pending_before_approval() {
        let mgr = make_manager();
        let resp = mgr.initiate("cli", vec![]).await.unwrap();

        let result = mgr.poll(&resp.device_code).await.unwrap();
        assert_eq!(result, PollResult::Pending, "should be pending");
    }

    #[tokio::test]
    async fn complete_then_poll_returns_authorized() {
        let mgr = make_manager();
        let resp = mgr
            .initiate("cli", vec!["personas:read".into()])
            .await
            .unwrap();

        mgr.complete(&resp.user_code, "usr_alice").await.unwrap();

        let result = mgr.poll(&resp.device_code).await.unwrap();
        assert_eq!(
            result,
            PollResult::Authorized {
                user_id: "usr_alice".into(),
                scopes: vec!["personas:read".into()],
            },
            "should be authorized after completion"
        );
    }

    #[tokio::test]
    async fn poll_after_authorized_removes_entry() {
        let mgr = make_manager();
        let resp = mgr.initiate("cli", vec![]).await.unwrap();

        mgr.complete(&resp.user_code, "usr_alice").await.unwrap();
        mgr.poll(&resp.device_code).await.unwrap(); // consumes

        let err = mgr.poll(&resp.device_code).await;
        assert!(err.is_err(), "entry should be removed after authorization");
    }

    #[tokio::test]
    async fn expired_code_returns_expired() {
        let mgr = make_manager().with_code_ttl(Duration::seconds(-10));
        let resp = mgr.initiate("cli", vec![]).await.unwrap();

        let result = mgr.poll(&resp.device_code).await.unwrap();
        assert_eq!(result, PollResult::Expired, "should be expired");
    }

    #[tokio::test]
    async fn complete_with_unknown_user_code_fails() {
        let mgr = make_manager();
        let err = mgr.complete("XXXX-YYYY", "usr_alice").await;
        assert!(err.is_err(), "unknown user code should fail");
    }

    #[tokio::test]
    async fn user_codes_are_unique() {
        let mgr = make_manager();
        let r1 = mgr.initiate("cli", vec![]).await.unwrap();
        let r2 = mgr.initiate("cli", vec![]).await.unwrap();
        assert_ne!(r1.user_code, r2.user_code, "user codes should be unique");
    }
}
