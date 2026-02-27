//! Outgoing webhook persistence — CRUD and verification tracking.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

/// A persisted webhook record.
#[derive(Debug, Clone)]
pub struct WebhookRecord {
    pub id: String,
    pub name: String,
    pub url: String,
    pub secret: String,
    pub event_types: Vec<String>,
    pub active: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SQLite-backed store for outgoing webhooks.
pub struct WebhookStore {
    pool: SqlitePool,
}

impl WebhookStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new webhook.
    pub async fn create(
        &self,
        id: &str,
        name: &str,
        url: &str,
        secret: &str,
        event_types: &[String],
    ) -> Result<()> {
        let now = Utc::now();
        let events_json = serde_json::to_string(event_types)?;
        sqlx::query(
            "INSERT INTO webhooks (id, name, url, secret, event_types, active, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
        )
        .bind(id)
        .bind(name)
        .bind(url)
        .bind(secret)
        .bind(&events_json)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Fetch a single webhook by ID.
    pub async fn get(&self, id: &str) -> Result<Option<WebhookRecord>> {
        let row = sqlx::query(
            "SELECT id, name, url, secret, event_types, active, verified_at, created_at, updated_at \
             FROM webhooks WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(parse_row(r)?)),
            None => Ok(None),
        }
    }

    /// List all webhooks ordered by creation time (newest first).
    pub async fn list(&self) -> Result<Vec<WebhookRecord>> {
        let rows = sqlx::query(
            "SELECT id, name, url, secret, event_types, active, verified_at, created_at, updated_at \
             FROM webhooks ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_row).collect()
    }

    /// Update an existing webhook's mutable fields.
    pub async fn update(
        &self,
        id: &str,
        name: &str,
        url: &str,
        event_types: &[String],
        active: bool,
    ) -> Result<bool> {
        let now = Utc::now();
        let events_json = serde_json::to_string(event_types)?;
        let result = sqlx::query(
            "UPDATE webhooks SET name = ?1, url = ?2, event_types = ?3, active = ?4, updated_at = ?5 \
             WHERE id = ?6",
        )
        .bind(name)
        .bind(url)
        .bind(&events_json)
        .bind(active)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a webhook by ID.
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM webhooks WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Record a successful verification timestamp.
    pub async fn mark_verified(&self, id: &str) -> Result<bool> {
        let now = Utc::now();
        let result =
            sqlx::query("UPDATE webhooks SET verified_at = ?1, updated_at = ?1 WHERE id = ?2")
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Toggle the active flag on a webhook.
    pub async fn toggle_active(&self, id: &str) -> Result<bool> {
        let now = Utc::now();
        let result =
            sqlx::query("UPDATE webhooks SET active = 1 - active, updated_at = ?1 WHERE id = ?2")
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Regenerate the HMAC secret for a webhook. Clears `verified_at` since the
    /// old secret is no longer valid.
    pub async fn rotate_secret(&self, id: &str, new_secret: &str) -> Result<bool> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE webhooks SET secret = ?1, verified_at = NULL, updated_at = ?2 WHERE id = ?3",
        )
        .bind(new_secret)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

fn parse_row(row: sqlx::sqlite::SqliteRow) -> Result<WebhookRecord> {
    let events_json: String = row.try_get("event_types")?;
    let event_types: Vec<String> = serde_json::from_str(&events_json)
        .with_context(|| format!("malformed event_types JSON: {events_json}"))?;
    let active_int: i32 = row.try_get("active")?;

    Ok(WebhookRecord {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        url: row.try_get("url")?,
        secret: row.try_get("secret")?,
        event_types,
        active: active_int != 0,
        verified_at: row.try_get("verified_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn store() -> WebhookStore {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        WebhookStore::new(storage.pool)
    }

    #[tokio::test]
    async fn create_and_get() {
        let s = store().await;
        s.create(
            "wh-1",
            "My Hook",
            "https://example.com/hook",
            "secret123",
            &["turn.result".to_string()],
        )
        .await
        .unwrap();

        let wh = s.get("wh-1").await.unwrap().expect("webhook should exist");
        assert_eq!(wh.name, "My Hook");
        assert_eq!(wh.url, "https://example.com/hook");
        assert_eq!(wh.secret, "secret123");
        assert_eq!(wh.event_types, vec!["turn.result".to_string()]);
        assert!(wh.active);
        assert!(wh.verified_at.is_none());
    }

    #[tokio::test]
    async fn list_returns_newest_first() {
        let s = store().await;
        s.create("wh-a", "First", "https://a.test", "s1", &[])
            .await
            .unwrap();
        // Nudge created_at so ordering is deterministic.
        sqlx::query(
            "UPDATE webhooks SET created_at = datetime('now', '-1 minute') WHERE id = 'wh-a'",
        )
        .execute(&s.pool)
        .await
        .unwrap();
        s.create("wh-b", "Second", "https://b.test", "s2", &[])
            .await
            .unwrap();

        let all = s.list().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, "wh-b", "newest should come first");
    }

    #[tokio::test]
    async fn update_fields() {
        let s = store().await;
        s.create("wh-u", "Old", "https://old.test", "s", &[])
            .await
            .unwrap();

        let ok = s
            .update(
                "wh-u",
                "New",
                "https://new.test",
                &["tool.result".to_string()],
                false,
            )
            .await
            .unwrap();
        assert!(ok);

        let wh = s.get("wh-u").await.unwrap().unwrap();
        assert_eq!(wh.name, "New");
        assert_eq!(wh.url, "https://new.test");
        assert!(!wh.active);
        assert_eq!(wh.event_types, vec!["tool.result".to_string()]);
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let s = store().await;
        s.create("wh-d", "Del", "https://d.test", "s", &[])
            .await
            .unwrap();

        assert!(s.delete("wh-d").await.unwrap());
        assert!(s.get("wh-d").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn mark_verified_sets_timestamp() {
        let s = store().await;
        s.create("wh-v", "Ver", "https://v.test", "s", &[])
            .await
            .unwrap();

        assert!(s.mark_verified("wh-v").await.unwrap());
        let wh = s.get("wh-v").await.unwrap().unwrap();
        assert!(wh.verified_at.is_some());
    }

    #[tokio::test]
    async fn toggle_active_flips_state() {
        let s = store().await;
        s.create("wh-t", "Tog", "https://t.test", "s", &[])
            .await
            .unwrap();
        assert!(s.get("wh-t").await.unwrap().unwrap().active);

        s.toggle_active("wh-t").await.unwrap();
        assert!(!s.get("wh-t").await.unwrap().unwrap().active);

        s.toggle_active("wh-t").await.unwrap();
        assert!(s.get("wh-t").await.unwrap().unwrap().active);
    }

    #[tokio::test]
    async fn rotate_secret_clears_verified() {
        let s = store().await;
        s.create("wh-r", "Rot", "https://r.test", "old-secret", &[])
            .await
            .unwrap();
        s.mark_verified("wh-r").await.unwrap();
        assert!(s.get("wh-r").await.unwrap().unwrap().verified_at.is_some());

        s.rotate_secret("wh-r", "new-secret").await.unwrap();
        let wh = s.get("wh-r").await.unwrap().unwrap();
        assert_eq!(wh.secret, "new-secret");
        assert!(
            wh.verified_at.is_none(),
            "verification should be cleared after secret rotation"
        );
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let s = store().await;
        assert!(s.get("nope").await.unwrap().is_none());
    }

    // -- Edge cases: operations on nonexistent IDs --

    #[tokio::test]
    async fn update_nonexistent_returns_false() {
        let s = store().await;
        let ok = s
            .update("nope", "name", "https://x.test", &[], true)
            .await
            .unwrap();
        assert!(!ok, "update on nonexistent ID should return false");
    }

    #[tokio::test]
    async fn delete_nonexistent_returns_false() {
        let s = store().await;
        assert!(
            !s.delete("nope").await.unwrap(),
            "delete on nonexistent ID should return false",
        );
    }

    #[tokio::test]
    async fn toggle_nonexistent_returns_false() {
        let s = store().await;
        assert!(
            !s.toggle_active("nope").await.unwrap(),
            "toggle on nonexistent ID should return false",
        );
    }

    #[tokio::test]
    async fn mark_verified_nonexistent_returns_false() {
        let s = store().await;
        assert!(
            !s.mark_verified("nope").await.unwrap(),
            "mark_verified on nonexistent ID should return false",
        );
    }

    #[tokio::test]
    async fn rotate_secret_nonexistent_returns_false() {
        let s = store().await;
        assert!(
            !s.rotate_secret("nope", "new").await.unwrap(),
            "rotate_secret on nonexistent ID should return false",
        );
    }

    // -- Edge case: multiple event types --

    #[tokio::test]
    async fn many_event_types_round_trip() {
        let s = store().await;
        let events = vec![
            "turn.request".to_string(),
            "turn.result".to_string(),
            "tool.execute".to_string(),
            "tool.result".to_string(),
        ];
        s.create("wh-m", "Multi", "https://m.test", "s", &events)
            .await
            .unwrap();
        let wh = s.get("wh-m").await.unwrap().unwrap();
        assert_eq!(wh.event_types, events);
    }

    #[tokio::test]
    async fn empty_event_types_round_trip() {
        let s = store().await;
        s.create("wh-e", "Empty", "https://e.test", "s", &[])
            .await
            .unwrap();
        let wh = s.get("wh-e").await.unwrap().unwrap();
        assert!(wh.event_types.is_empty());
    }

    // -- Edge case: list on empty table --

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let s = store().await;
        let all = s.list().await.unwrap();
        assert!(all.is_empty());
    }
}
