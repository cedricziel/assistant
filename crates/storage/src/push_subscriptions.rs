//! PWA Web Push subscription persistence.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

/// A stored Web Push subscription.
#[derive(Debug, Clone)]
pub struct PushSubscription {
    pub id: i64,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub created_at: DateTime<Utc>,
}

/// Trait-based interface for PWA push subscription persistence.
#[async_trait]
pub trait PushSubscriptionStore: Send + Sync {
    /// Insert or update a subscription by `endpoint` (unique key).
    async fn upsert(&self, endpoint: &str, p256dh: &str, auth: &str) -> Result<()>;

    /// Delete a subscription by its push endpoint URL.
    async fn delete(&self, endpoint: &str) -> Result<()>;

    /// Return every stored subscription (used by the push dispatcher).
    async fn list_all(&self) -> Result<Vec<PushSubscription>>;
}

/// SQLite-backed store for PWA push subscriptions.
#[derive(Clone)]
pub struct SqlitePushSubscriptionStore {
    pool: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl SqlitePushSubscriptionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

#[async_trait]
impl PushSubscriptionStore for SqlitePushSubscriptionStore {
    async fn upsert(&self, endpoint: &str, p256dh: &str, auth: &str) -> Result<()> {
        let now = self.clock.now();
        sqlx::query(
            "INSERT INTO push_subscriptions (endpoint, p256dh, auth, created_at) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(endpoint) DO UPDATE SET p256dh = excluded.p256dh, auth = excluded.auth",
        )
        .bind(endpoint)
        .bind(p256dh)
        .bind(auth)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Delete a subscription by its push endpoint URL.
    async fn delete(&self, endpoint: &str) -> Result<()> {
        sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = ?1")
            .bind(endpoint)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Return every stored subscription (used by the push dispatcher).
    async fn list_all(&self) -> Result<Vec<PushSubscription>> {
        let rows = sqlx::query(
            "SELECT id, endpoint, p256dh, auth, created_at FROM push_subscriptions ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;

        let subs = rows
            .into_iter()
            .map(|r| PushSubscription {
                id: r.get("id"),
                endpoint: r.get("endpoint"),
                p256dh: r.get("p256dh"),
                auth: r.get("auth"),
                created_at: r.get("created_at"),
            })
            .collect();

        Ok(subs)
    }
}

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

/// In-memory [`PushSubscriptionStore`]. Vec-backed.
pub struct InMemoryPushSubscriptionStore {
    state: Arc<Mutex<MemoryPushState>>,
    clock: Arc<dyn Clock>,
}

#[derive(Default)]
struct MemoryPushState {
    next_id: i64,
    by_endpoint: HashMap<String, PushSubscription>,
}

impl InMemoryPushSubscriptionStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MemoryPushState::default())),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }
}

impl Default for InMemoryPushSubscriptionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PushSubscriptionStore for InMemoryPushSubscriptionStore {
    async fn upsert(&self, endpoint: &str, p256dh: &str, auth: &str) -> Result<()> {
        let now = self.clock.now();
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(sub) = state.by_endpoint.get_mut(endpoint) {
            sub.p256dh = p256dh.to_string();
            sub.auth = auth.to_string();
        } else {
            state.next_id += 1;
            let id = state.next_id;
            state.by_endpoint.insert(
                endpoint.to_string(),
                PushSubscription {
                    id,
                    endpoint: endpoint.to_string(),
                    p256dh: p256dh.to_string(),
                    auth: auth.to_string(),
                    created_at: now,
                },
            );
        }
        Ok(())
    }

    async fn delete(&self, endpoint: &str) -> Result<()> {
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        state.by_endpoint.remove(endpoint);
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<PushSubscription>> {
        let state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut subs: Vec<PushSubscription> = state.by_endpoint.values().cloned().collect();
        subs.sort_by_key(|s| s.id);
        Ok(subs)
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn store() -> SqlitePushSubscriptionStore {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        SqlitePushSubscriptionStore::new(storage.pool)
    }

    #[tokio::test]
    async fn upsert_and_list() {
        let store = store().await;

        store
            .upsert("https://push.example.com/123", "p256key", "authsecret")
            .await
            .unwrap();

        let subs = store.list_all().await.unwrap();
        assert_eq!(subs.len(), 1, "expected one subscription");
        assert_eq!(subs[0].endpoint, "https://push.example.com/123");
        assert_eq!(subs[0].p256dh, "p256key");
        assert_eq!(subs[0].auth, "authsecret");
    }

    #[tokio::test]
    async fn upsert_updates_existing() {
        let store = store().await;

        store
            .upsert("https://push.example.com/abc", "key1", "auth1")
            .await
            .unwrap();
        store
            .upsert("https://push.example.com/abc", "key2", "auth2")
            .await
            .unwrap();

        let subs = store.list_all().await.unwrap();
        assert_eq!(subs.len(), 1, "upsert should not create a duplicate row");
        assert_eq!(subs[0].p256dh, "key2");
        assert_eq!(subs[0].auth, "auth2");
    }

    #[tokio::test]
    async fn delete_removes_subscription() {
        let store = store().await;

        store
            .upsert("https://push.example.com/del", "key", "auth")
            .await
            .unwrap();
        store.delete("https://push.example.com/del").await.unwrap();

        let subs = store.list_all().await.unwrap();
        assert!(subs.is_empty(), "subscription should have been deleted");
    }

    #[tokio::test]
    async fn delete_nonexistent_is_ok() {
        let store = store().await;
        // Should not error even if the endpoint doesn't exist.
        store
            .delete("https://push.example.com/ghost")
            .await
            .unwrap();
    }
}
