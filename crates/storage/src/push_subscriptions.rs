//! PWA Web Push subscription persistence.
//!
//! Each row represents one browser push subscription endpoint.  Subscriptions
//! are upserted by endpoint URL (unique) and deleted automatically when the
//! push service returns `410 Gone`.

use std::sync::Arc;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
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

/// SQLite-backed store for PWA push subscriptions.
#[derive(Clone)]
pub struct PushSubscriptionStore {
    pool: SqlitePool,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl PushSubscriptionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            clock: Arc::new(SystemClock),
        }
    }

    /// Inject a [`Clock`] implementation.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Insert or update a subscription by `endpoint` (unique key).
    pub async fn upsert(&self, endpoint: &str, p256dh: &str, auth: &str) -> Result<()> {
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
    pub async fn delete(&self, endpoint: &str) -> Result<()> {
        sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = ?1")
            .bind(endpoint)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Return every stored subscription (used by the push dispatcher).
    pub async fn list_all(&self) -> Result<Vec<PushSubscription>> {
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

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn store() -> PushSubscriptionStore {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        PushSubscriptionStore::new(storage.pool)
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
