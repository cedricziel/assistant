//! Subagent lifecycle persistence.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

/// Status of a subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "cancelled" => Self::Cancelled,
            _ => Self::Running,
        }
    }
}

/// A single agent record.
#[derive(Debug, Clone)]
pub struct AgentRecord {
    pub id: String,
    pub parent_agent_id: Option<String>,
    pub parent_conversation_id: String,
    pub conversation_id: String,
    pub task: String,
    pub status: AgentStatus,
    pub depth: i64,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result_summary: Option<String>,
}

/// SQLite-backed store for subagent lifecycle records.
pub struct AgentStore {
    pool: SqlitePool,
}

impl AgentStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new agent record in `running` status.
    pub async fn create(
        &self,
        id: &str,
        parent_agent_id: Option<&str>,
        parent_conversation_id: &str,
        conversation_id: &str,
        task: &str,
        depth: u32,
    ) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "INSERT INTO agents \
                (id, parent_agent_id, parent_conversation_id, conversation_id, task, status, depth, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7)",
        )
        .bind(id)
        .bind(parent_agent_id)
        .bind(parent_conversation_id)
        .bind(conversation_id)
        .bind(task)
        .bind(depth)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark an agent as completed or failed and record a result summary.
    pub async fn complete(
        &self,
        id: &str,
        status: AgentStatus,
        result_summary: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            "UPDATE agents SET status = ?1, completed_at = ?2, result_summary = ?3 WHERE id = ?4",
        )
        .bind(status.as_str())
        .bind(now)
        .bind(result_summary)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Fetch a single agent by ID.
    pub async fn get(&self, id: &str) -> Result<Option<AgentRecord>> {
        let row = sqlx::query(
            "SELECT id, parent_agent_id, parent_conversation_id, conversation_id, \
                    task, status, depth, created_at, completed_at, result_summary \
             FROM agents WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(parse_row(r)?)),
            None => Ok(None),
        }
    }

    /// List agents spawned within a given parent conversation.
    pub async fn list_by_parent_conversation(
        &self,
        parent_conversation_id: &str,
    ) -> Result<Vec<AgentRecord>> {
        let rows = sqlx::query(
            "SELECT id, parent_agent_id, parent_conversation_id, conversation_id, \
                    task, status, depth, created_at, completed_at, result_summary \
             FROM agents WHERE parent_conversation_id = ?1 \
             ORDER BY created_at ASC",
        )
        .bind(parent_conversation_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(parse_row).collect()
    }
}

fn parse_row(row: sqlx::sqlite::SqliteRow) -> Result<AgentRecord> {
    Ok(AgentRecord {
        id: row.try_get("id")?,
        parent_agent_id: row.try_get("parent_agent_id")?,
        parent_conversation_id: row.try_get("parent_conversation_id")?,
        conversation_id: row.try_get("conversation_id")?,
        task: row.try_get("task")?,
        status: AgentStatus::parse(row.try_get::<String, _>("status")?.as_str()),
        depth: row.try_get("depth")?,
        created_at: row.try_get("created_at")?,
        completed_at: row.try_get("completed_at")?,
        result_summary: row.try_get("result_summary")?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn store() -> AgentStore {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        AgentStore::new(storage.pool)
    }

    #[tokio::test]
    async fn create_and_get() {
        let s = store().await;
        s.create("a1", None, "conv-parent", "conv-child", "do stuff", 1)
            .await
            .unwrap();

        let agent = s.get("a1").await.unwrap().expect("agent should exist");
        assert_eq!(agent.id, "a1");
        assert_eq!(agent.status, AgentStatus::Running);
        assert_eq!(agent.depth, 1);
        assert_eq!(agent.task, "do stuff");
        assert!(agent.completed_at.is_none());
    }

    #[tokio::test]
    async fn complete_sets_status_and_timestamp() {
        let s = store().await;
        s.create("a2", None, "conv-p", "conv-c", "task", 0)
            .await
            .unwrap();

        s.complete("a2", AgentStatus::Completed, Some("all done"))
            .await
            .unwrap();

        let agent = s.get("a2").await.unwrap().unwrap();
        assert_eq!(agent.status, AgentStatus::Completed);
        assert!(agent.completed_at.is_some());
        assert_eq!(agent.result_summary.as_deref(), Some("all done"));
    }

    #[tokio::test]
    async fn complete_failed() {
        let s = store().await;
        s.create("a3", None, "conv-p", "conv-c", "task", 0)
            .await
            .unwrap();

        s.complete("a3", AgentStatus::Failed, Some("timeout"))
            .await
            .unwrap();

        let agent = s.get("a3").await.unwrap().unwrap();
        assert_eq!(agent.status, AgentStatus::Failed);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let s = store().await;
        assert!(s.get("nonexistent").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_by_parent_conversation() {
        let s = store().await;
        s.create("a4", None, "conv-p1", "conv-c1", "task1", 1)
            .await
            .unwrap();
        s.create("a5", None, "conv-p1", "conv-c2", "task2", 1)
            .await
            .unwrap();
        s.create("a6", None, "conv-p2", "conv-c3", "task3", 1)
            .await
            .unwrap();

        let agents = s.list_by_parent_conversation("conv-p1").await.unwrap();
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].id, "a4");
        assert_eq!(agents[1].id, "a5");
    }

    #[tokio::test]
    async fn parent_agent_id_stored() {
        let s = store().await;
        s.create("child", Some("parent"), "conv-p", "conv-c", "task", 2)
            .await
            .unwrap();

        let agent = s.get("child").await.unwrap().unwrap();
        assert_eq!(agent.parent_agent_id.as_deref(), Some("parent"));
    }
}
