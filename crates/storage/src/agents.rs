//! Subagent lifecycle persistence.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;
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
/// Trait-based interface for subagent process persistence.
#[async_trait]
pub trait AgentStore: Send + Sync {
    /// Insert a new agent record in `running` status.
    async fn create(
        &self,
        id: &str,
        parent_agent_id: Option<&str>,
        parent_conversation_id: &str,
        conversation_id: &str,
        task: &str,
        depth: u32,
    ) -> Result<()>;

    /// Mark an agent as completed/failed/cancelled.
    async fn complete(
        &self,
        id: &str,
        status: AgentStatus,
        result_summary: Option<&str>,
    ) -> Result<()>;

    /// Fetch a single agent record by ID.
    async fn get(&self, id: &str) -> Result<Option<AgentRecord>>;

    /// List all subagents spawned within a parent conversation.
    async fn list_by_parent_conversation(
        &self,
        parent_conversation_id: &str,
    ) -> Result<Vec<AgentRecord>>;
}

pub struct SqliteAgentStore {
    pool: SqlitePool,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl SqliteAgentStore {
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
}

#[async_trait]
impl AgentStore for SqliteAgentStore {
    /// Insert a new agent record in `running` status.
    async fn create(
        &self,
        id: &str,
        parent_agent_id: Option<&str>,
        parent_conversation_id: &str,
        conversation_id: &str,
        task: &str,
        depth: u32,
    ) -> Result<()> {
        let now = self.clock.now();
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
    async fn complete(
        &self,
        id: &str,
        status: AgentStatus,
        result_summary: Option<&str>,
    ) -> Result<()> {
        let now = self.clock.now();
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
    async fn get(&self, id: &str) -> Result<Option<AgentRecord>> {
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
    async fn list_by_parent_conversation(
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

// ---------------------------------------------------------------------------
// In-memory implementation
// ---------------------------------------------------------------------------

/// In-memory [`AgentStore`]. HashMap<id, AgentRecord>.
pub struct InMemoryAgentStore {
    state: Arc<Mutex<HashMap<String, AgentRecord>>>,
    clock: Arc<dyn Clock>,
}

impl InMemoryAgentStore {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, AgentRecord>> {
        match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

impl Default for InMemoryAgentStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentStore for InMemoryAgentStore {
    async fn create(
        &self,
        id: &str,
        parent_agent_id: Option<&str>,
        parent_conversation_id: &str,
        conversation_id: &str,
        task: &str,
        depth: u32,
    ) -> Result<()> {
        let now = self.clock.now();
        let mut state = self.lock();
        state.insert(
            id.to_string(),
            AgentRecord {
                id: id.to_string(),
                parent_agent_id: parent_agent_id.map(str::to_string),
                parent_conversation_id: parent_conversation_id.to_string(),
                conversation_id: conversation_id.to_string(),
                task: task.to_string(),
                status: AgentStatus::Running,
                depth: depth as i64,
                created_at: now,
                completed_at: None,
                result_summary: None,
            },
        );
        Ok(())
    }

    async fn complete(
        &self,
        id: &str,
        status: AgentStatus,
        result_summary: Option<&str>,
    ) -> Result<()> {
        let now = self.clock.now();
        let mut state = self.lock();
        if let Some(a) = state.get_mut(id) {
            a.status = status;
            a.completed_at = Some(now);
            a.result_summary = result_summary.map(str::to_string);
        }
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<AgentRecord>> {
        let state = self.lock();
        Ok(state.get(id).cloned())
    }

    async fn list_by_parent_conversation(
        &self,
        parent_conversation_id: &str,
    ) -> Result<Vec<AgentRecord>> {
        let state = self.lock();
        let mut out: Vec<AgentRecord> = state
            .values()
            .filter(|a| a.parent_conversation_id == parent_conversation_id)
            .cloned()
            .collect();
        out.sort_by_key(|a| a.created_at);
        Ok(out)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn store() -> SqliteAgentStore {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        SqliteAgentStore::new(storage.pool)
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
