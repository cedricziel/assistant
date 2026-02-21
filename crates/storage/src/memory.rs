//! Key-value memory store backed by the `memory_entries` SQLite table.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

/// A single memory entry persisted to SQLite.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    /// Who set this value: `"user"`, `"assistant"`, or a skill name.
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// SQLite-backed key-value memory store.
pub struct MemoryStore {
    pool: SqlitePool,
}

impl MemoryStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Retrieve a memory entry by its key. Returns `None` if the key does not exist.
    pub async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let row = sqlx::query(
            "SELECT key, value, source, created_at, updated_at \
             FROM memory_entries \
             WHERE key = ?1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| MemoryEntry {
            key: r.get("key"),
            value: r.get("value"),
            source: r.get("source"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    /// Insert or update a memory entry (upsert by key).
    pub async fn set(&self, key: &str, value: &str, source: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO memory_entries (key, value, source, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?4) \
             ON CONFLICT(key) DO UPDATE SET \
                 value      = excluded.value, \
                 source     = excluded.source, \
                 updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(source)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Simple full-text search over keys and values.
    /// Returns all entries where the key or value contains `query` (case-insensitive).
    pub async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        let pattern = format!("%{}%", query);
        let rows = sqlx::query(
            "SELECT key, value, source, created_at, updated_at \
             FROM memory_entries \
             WHERE key   LIKE ?1 ESCAPE '\\' \
                OR value LIKE ?1 ESCAPE '\\' \
             ORDER BY updated_at DESC",
        )
        .bind(&pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MemoryEntry {
                key: r.get("key"),
                value: r.get("value"),
                source: r.get("source"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Return all stored memory entries, ordered by most-recently updated first.
    pub async fn list_all(&self) -> Result<Vec<MemoryEntry>> {
        let rows = sqlx::query(
            "SELECT key, value, source, created_at, updated_at \
             FROM memory_entries \
             ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MemoryEntry {
                key: r.get("key"),
                value: r.get("value"),
                source: r.get("source"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    /// Delete a memory entry by key. No-op if the key does not exist.
    pub async fn delete(&self, key: &str) -> Result<()> {
        sqlx::query("DELETE FROM memory_entries WHERE key = ?1")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::StorageLayer;

    #[tokio::test]
    async fn test_set_get_delete() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.memory_store();

        assert!(store.get("foo").await.unwrap().is_none());

        store.set("foo", "bar", "user").await.unwrap();
        let entry = store.get("foo").await.unwrap().expect("entry should exist");
        assert_eq!(entry.value, "bar");
        assert_eq!(entry.source, "user");

        // Update
        store.set("foo", "baz", "assistant").await.unwrap();
        let entry = store.get("foo").await.unwrap().unwrap();
        assert_eq!(entry.value, "baz");

        store.delete("foo").await.unwrap();
        assert!(store.get("foo").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_search() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.memory_store();

        store.set("github_token", "ghp_xxx", "user").await.unwrap();
        store.set("openai_key", "sk-xxx", "user").await.unwrap();
        store
            .set("greeting", "hello world", "assistant")
            .await
            .unwrap();

        let results = store.search("xxx").await.unwrap();
        assert_eq!(results.len(), 2);

        let results = store.search("hello").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "greeting");
    }

    #[tokio::test]
    async fn test_list_all() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.memory_store();

        store.set("a", "1", "user").await.unwrap();
        store.set("b", "2", "user").await.unwrap();

        let all = store.list_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
