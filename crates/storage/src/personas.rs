use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone)]
pub struct PersonaRecord {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    /// Skill access mode: "all", "whitelist", or "blacklist". Defaults to "all".
    pub skill_access_mode: String,
    /// Per-persona turn timeout in seconds. `None` means use the compiled-in
    /// default (10 800 s / 3 h).
    pub turn_timeout_secs: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct PersonaStore {
    pool: SqlitePool,
}

impl PersonaStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn ensure_default(&self) -> Result<PersonaRecord> {
        sqlx::query(
            "INSERT INTO personas (id, name, is_default) VALUES ('default', 'Default', 1) \
             ON CONFLICT(id) DO NOTHING",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE personas
             SET is_default = CASE WHEN id = 'default' THEN 1 ELSE 0 END,
                 updated_at = CURRENT_TIMESTAMP
             WHERE NOT EXISTS (
                 SELECT 1 FROM personas WHERE is_default = 1
             )",
        )
        .execute(&self.pool)
        .await?;

        self.get("default")
            .await?
            .ok_or_else(|| anyhow::anyhow!("default persona missing after ensure_default"))
    }

    pub async fn ensure_exists(&self, id: &str) -> Result<PersonaRecord> {
        if let Some(existing) = self.get(id).await? {
            return Ok(existing);
        }

        let inferred_name = id.replace(['_', '-'], " ");
        sqlx::query(
            "INSERT INTO personas (id, name, is_default) VALUES (?1, ?2, 0)
             ON CONFLICT(id) DO NOTHING",
        )
        .bind(id)
        .bind(if inferred_name.is_empty() {
            id.to_string()
        } else {
            inferred_name
        })
        .execute(&self.pool)
        .await?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("persona '{}' missing after ensure_exists", id))
    }

    pub async fn list(&self) -> Result<Vec<PersonaRecord>> {
        let rows = sqlx::query(
            "SELECT id, name, is_default, skill_access_mode, turn_timeout_secs, created_at, updated_at
             FROM personas
             ORDER BY is_default DESC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_record).collect()
    }

    pub async fn set_default(&self, id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM personas WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        if exists == 0 {
            anyhow::bail!("persona '{}' does not exist", id);
        }

        sqlx::query("UPDATE personas SET is_default = 0, updated_at = CURRENT_TIMESTAMP")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE personas
             SET is_default = 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn default_id(&self) -> Result<String> {
        let id = sqlx::query_scalar::<_, String>(
            "SELECT id
             FROM personas
             WHERE is_default = 1
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_else(|| "default".to_string());
        Ok(id)
    }

    pub async fn get(&self, id: &str) -> Result<Option<PersonaRecord>> {
        let row = sqlx::query(
            "SELECT id, name, is_default, skill_access_mode, turn_timeout_secs, created_at, updated_at
             FROM personas
             WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_record).transpose()
    }

    pub async fn create(&self, id: &str, name: &str) -> Result<PersonaRecord> {
        sqlx::query("INSERT INTO personas (id, name, is_default) VALUES (?1, ?2, 0)")
            .bind(id)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    anyhow::anyhow!(
                        "persona with id '{}' already exists (UNIQUE constraint)",
                        id
                    )
                } else {
                    anyhow::anyhow!("failed to create persona '{}': {}", id, e)
                }
            })?;

        self.get(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("persona '{}' missing after create", id))
    }

    /// Set a per-persona turn timeout. `secs` must be > 0.
    pub async fn set_turn_timeout(&self, id: &str, secs: u64) -> Result<()> {
        anyhow::ensure!(secs > 0, "turn_timeout_secs must be greater than 0");
        let rows = sqlx::query(
            "UPDATE personas
             SET turn_timeout_secs = ?1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?2",
        )
        .bind(secs as i64)
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        anyhow::ensure!(rows > 0, "persona '{}' not found", id);
        Ok(())
    }

    /// Clear the per-persona turn timeout, reverting to the compiled-in default.
    pub async fn clear_turn_timeout(&self, id: &str) -> Result<()> {
        let rows = sqlx::query(
            "UPDATE personas
             SET turn_timeout_secs = NULL, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        anyhow::ensure!(rows > 0, "persona '{}' not found", id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::StorageLayer;

    #[tokio::test]
    async fn create_returns_error_on_duplicate_id() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = super::PersonaStore::new(storage.pool.clone());

        store.create("foo", "Foo").await.unwrap();
        let result = store.create("foo", "Foo Again").await;
        assert!(result.is_err(), "expected Err on duplicate id, got Ok");
    }
}

fn row_to_record(row: sqlx::sqlite::SqliteRow) -> Result<PersonaRecord> {
    Ok(PersonaRecord {
        id: row.get("id"),
        name: row.get("name"),
        is_default: row.get::<i64, _>("is_default") != 0,
        skill_access_mode: row
            .try_get("skill_access_mode")
            .unwrap_or_else(|_| "all".to_string()),
        turn_timeout_secs: row
            .try_get::<Option<i64>, _>("turn_timeout_secs")
            .unwrap_or(None)
            .map(|v| v as u64),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
