use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone)]
pub struct PersonaRecord {
    pub id: String,
    pub name: String,
    pub is_default: bool,
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
            "SELECT id, name, is_default, created_at, updated_at
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
            "SELECT id, name, is_default, created_at, updated_at
             FROM personas
             WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_record).transpose()
    }
}

fn row_to_record(row: sqlx::sqlite::SqliteRow) -> Result<PersonaRecord> {
    Ok(PersonaRecord {
        id: row.get("id"),
        name: row.get("name"),
        is_default: row.get::<i64, _>("is_default") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
