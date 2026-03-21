use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone)]
pub struct AssistantAgentRecord {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct AssistantAgentStore {
    pool: SqlitePool,
}

impl AssistantAgentStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn ensure_default(&self) -> Result<AssistantAgentRecord> {
        sqlx::query(
            "INSERT INTO assistant_agents (id, name, is_default) VALUES ('default', 'Default', 1) \
             ON CONFLICT(id) DO NOTHING",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE assistant_agents
             SET is_default = CASE WHEN id = 'default' THEN 1 ELSE 0 END,
                 updated_at = CURRENT_TIMESTAMP
             WHERE NOT EXISTS (
                 SELECT 1 FROM assistant_agents WHERE is_default = 1
             )",
        )
        .execute(&self.pool)
        .await?;

        self.get("default")
            .await?
            .ok_or_else(|| anyhow::anyhow!("default agent missing after ensure_default"))
    }

    pub async fn ensure_exists(&self, id: &str) -> Result<AssistantAgentRecord> {
        if let Some(existing) = self.get(id).await? {
            return Ok(existing);
        }

        let inferred_name = id.replace(['_', '-'], " ");
        sqlx::query(
            "INSERT INTO assistant_agents (id, name, is_default) VALUES (?1, ?2, 0)
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
            .ok_or_else(|| anyhow::anyhow!("agent '{}' missing after ensure_exists", id))
    }

    pub async fn get(&self, id: &str) -> Result<Option<AssistantAgentRecord>> {
        let row = sqlx::query(
            "SELECT id, name, is_default, created_at, updated_at
             FROM assistant_agents
             WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_record).transpose()
    }
}

fn row_to_record(row: sqlx::sqlite::SqliteRow) -> Result<AssistantAgentRecord> {
    Ok(AssistantAgentRecord {
        id: row.get("id"),
        name: row.get("name"),
        is_default: row.get::<i64, _>("is_default") != 0,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}
