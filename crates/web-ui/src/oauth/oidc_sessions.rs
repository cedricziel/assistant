//! In-memory store for pending OIDC authorization sessions.
//!
//! When a user is redirected to an external IdP we need to remember the
//! original OAuth2 authorize parameters (client_id, redirect_uri, etc.)
//! so we can complete the flow when the IdP calls us back.  Sessions are
//! keyed by a random `state` value and expire after a configurable TTL.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

/// Parameters captured from the original `/oauth/authorize` request before
/// redirecting to the external IdP.
#[derive(Clone, Debug)]
pub struct PendingOidcSession {
    /// The OAuth2 client_id that initiated the flow.
    pub client_id: String,
    /// The redirect_uri we should send the auth code to.
    pub redirect_uri: String,
    /// PKCE code_challenge (if supplied).
    pub code_challenge: Option<String>,
    /// The original `state` parameter from the client (forwarded back).
    pub client_state: Option<String>,
    /// Requested scopes.
    pub scope: Option<String>,
    /// When this session was created.
    created_at: Instant,
}

/// Thread-safe in-memory store for pending OIDC sessions.
pub struct PendingOidcSessionStore {
    sessions: RwLock<HashMap<String, PendingOidcSession>>,
    ttl: Duration,
}

impl PendingOidcSessionStore {
    /// Create a new store with the given session TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    /// Insert a pending session, returning the random `state` key.
    pub async fn insert(&self, session: PendingOidcSession) -> String {
        let state = uuid::Uuid::new_v4().to_string();
        let mut map = self.sessions.write().await;
        // Opportunistically prune expired entries.
        let now = Instant::now();
        map.retain(|_, s| now.duration_since(s.created_at) < self.ttl);
        map.insert(state.clone(), session);
        state
    }

    /// Remove and return a pending session by its `state` key.
    /// Returns `None` if the key is missing or expired.
    pub async fn take(&self, state: &str) -> Option<PendingOidcSession> {
        let mut map = self.sessions.write().await;
        let session = map.remove(state)?;
        if Instant::now().duration_since(session.created_at) >= self.ttl {
            return None; // Expired.
        }
        Some(session)
    }
}

impl PendingOidcSession {
    pub fn new(
        client_id: String,
        redirect_uri: String,
        code_challenge: Option<String>,
        client_state: Option<String>,
        scope: Option<String>,
    ) -> Self {
        Self {
            client_id,
            redirect_uri,
            code_challenge,
            client_state,
            scope,
            created_at: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_take() {
        let store = PendingOidcSessionStore::new(Duration::from_secs(300));
        let session = PendingOidcSession::new(
            "client-1".into(),
            "http://localhost/cb".into(),
            None,
            Some("abc".into()),
            None,
        );

        let state = store.insert(session).await;
        let recovered = store.take(&state).await;
        assert!(recovered.is_some());
        let s = recovered.unwrap();
        assert_eq!(s.client_id, "client-1");
        assert_eq!(s.client_state.as_deref(), Some("abc"));

        // Second take should return None (consumed).
        assert!(store.take(&state).await.is_none());
    }

    #[tokio::test]
    async fn expired_session_returns_none() {
        let store = PendingOidcSessionStore::new(Duration::from_millis(1));
        let session = PendingOidcSession::new(
            "client-2".into(),
            "http://localhost/cb".into(),
            None,
            None,
            None,
        );

        let state = store.insert(session).await;
        // Sleep long enough for expiry.
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(store.take(&state).await.is_none());
    }

    #[tokio::test]
    async fn unknown_state_returns_none() {
        let store = PendingOidcSessionStore::new(Duration::from_secs(300));
        assert!(store.take("no-such-state").await.is_none());
    }
}
