//! Per-skill execution log backed by the `skill_executions` table.
//!
//! The orchestrator records one row for every tool call finalised while a
//! skill is active. The same rows back [`SkillStatsProvider`], which feeds
//! skill self-improvement.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::skill_stats::{SkillStats, SkillStatsProvider};

/// Whether a skill-scoped tool call succeeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillOutcome {
    Success,
    Error,
}

impl SkillOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Error => "error",
        }
    }

    fn parse(s: &str) -> Result<Self> {
        match s {
            "success" => Ok(Self::Success),
            "error" => Ok(Self::Error),
            other => anyhow::bail!("unknown skill outcome '{other}'"),
        }
    }
}

/// One tool call made while a skill was active.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillExecution {
    pub skill_name: String,
    pub agent_id: String,
    pub conversation_id: Uuid,
    pub tool_name: String,
    pub outcome: SkillOutcome,
    /// Error text when `outcome` is [`SkillOutcome::Error`].
    pub error: Option<String>,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
}

/// Persistence for [`SkillExecution`] rows.
///
/// Every implementation is also a [`SkillStatsProvider`] over its rows.
#[async_trait]
pub trait SkillExecutionStore: Send + Sync {
    /// Append one execution.
    async fn record(&self, execution: &SkillExecution) -> Result<()>;

    /// The `window` most recent executions of `skill_name`, newest first.
    async fn recent(&self, skill_name: &str, window: i64) -> Result<Vec<SkillExecution>>;
}

#[async_trait]
impl<S: SkillExecutionStore + ?Sized> SkillStatsProvider for S {
    async fn stats_for_active_skill(&self, skill_name: &str, window: i64) -> Result<SkillStats> {
        let executions = self.recent(skill_name, window).await?;
        Ok(SkillStats::summarize(skill_name, &executions))
    }
}

/// SQLite-backed [`SkillExecutionStore`].
pub struct SqliteSkillExecutionStore {
    pool: SqlitePool,
}

impl SqliteSkillExecutionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SkillExecutionStore for SqliteSkillExecutionStore {
    async fn record(&self, execution: &SkillExecution) -> Result<()> {
        sqlx::query(
            "INSERT INTO skill_executions \
             (id, skill_name, agent_id, conversation_id, tool_name, outcome, error, duration_ms, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&execution.skill_name)
        .bind(&execution.agent_id)
        .bind(execution.conversation_id.to_string())
        .bind(&execution.tool_name)
        .bind(execution.outcome.as_str())
        .bind(&execution.error)
        .bind(execution.duration_ms)
        .bind(
            execution
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn recent(&self, skill_name: &str, window: i64) -> Result<Vec<SkillExecution>> {
        let rows = sqlx::query(
            "SELECT skill_name, agent_id, conversation_id, tool_name, outcome, error, duration_ms, created_at \
             FROM skill_executions \
             WHERE skill_name = ?1 \
             ORDER BY created_at DESC, rowid DESC \
             LIMIT ?2",
        )
        .bind(skill_name)
        .bind(window)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let conversation_id: String = row.get("conversation_id");
                let outcome: String = row.get("outcome");
                let created_at: String = row.get("created_at");
                Ok(SkillExecution {
                    skill_name: row.get("skill_name"),
                    agent_id: row.get("agent_id"),
                    conversation_id: Uuid::parse_str(&conversation_id)
                        .context("invalid conversation_id in skill_executions")?,
                    tool_name: row.get("tool_name"),
                    outcome: SkillOutcome::parse(&outcome)?,
                    error: row.get("error"),
                    duration_ms: row.get("duration_ms"),
                    created_at: DateTime::parse_from_rfc3339(&created_at)?.with_timezone(&Utc),
                })
            })
            .collect()
    }
}

/// In-memory [`SkillExecutionStore`] for tests.
#[derive(Default)]
pub struct InMemorySkillExecutionStore {
    rows: Arc<Mutex<Vec<SkillExecution>>>,
}

impl InMemorySkillExecutionStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<SkillExecution>> {
        match self.rows.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }
}

#[async_trait]
impl SkillExecutionStore for InMemorySkillExecutionStore {
    async fn record(&self, execution: &SkillExecution) -> Result<()> {
        self.lock().push(execution.clone());
        Ok(())
    }

    async fn recent(&self, skill_name: &str, window: i64) -> Result<Vec<SkillExecution>> {
        let mut rows: Vec<(usize, SkillExecution)> = self
            .lock()
            .iter()
            .filter(|e| e.skill_name == skill_name)
            .cloned()
            .enumerate()
            .collect();
        rows.sort_by(|(ia, a), (ib, b)| b.created_at.cmp(&a.created_at).then(ib.cmp(ia)));
        Ok(rows
            .into_iter()
            .take(usize::try_from(window).unwrap_or(0))
            .map(|(_, e)| e)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;
    use chrono::{Duration, TimeZone};

    fn execution(
        skill: &str,
        minute: i64,
        error: Option<&str>,
        duration_ms: i64,
    ) -> SkillExecution {
        SkillExecution {
            skill_name: skill.to_string(),
            agent_id: "default".to_string(),
            conversation_id: Uuid::nil(),
            tool_name: "bash".to_string(),
            outcome: if error.is_some() {
                SkillOutcome::Error
            } else {
                SkillOutcome::Success
            },
            error: error.map(str::to_string),
            duration_ms,
            created_at: Utc.with_ymd_and_hms(2026, 11, 25, 12, 0, 0).unwrap()
                + Duration::minutes(minute),
        }
    }

    /// Behaviour every store must share.
    async fn assert_store_contract(store: &impl SkillExecutionStore) {
        store
            .record(&execution("deploy", 0, Some("timeout"), 400))
            .await
            .unwrap();
        store
            .record(&execution("deploy", 1, None, 100))
            .await
            .unwrap();
        store
            .record(&execution("deploy", 2, Some("timeout"), 300))
            .await
            .unwrap();
        store
            .record(&execution("deploy", 3, Some("denied"), 200))
            .await
            .unwrap();
        store
            .record(&execution("other", 4, None, 999))
            .await
            .unwrap();

        let recent = store.recent("deploy", 2).await.unwrap();
        assert_eq!(
            recent.iter().map(|e| e.duration_ms).collect::<Vec<_>>(),
            vec![200, 300],
            "recent returns the newest executions of that skill first"
        );

        let all = store.stats_for_active_skill("deploy", 50).await.unwrap();
        assert_eq!(all.total, 4);
        assert_eq!(all.success_count, 1);
        assert_eq!(all.error_count, 3);
        assert_eq!(all.avg_duration_ms, 250.0);
        assert_eq!(
            all.common_errors,
            vec!["timeout".to_string(), "denied".to_string()],
            "errors are ordered by frequency"
        );

        let windowed = store.stats_for_active_skill("deploy", 2).await.unwrap();
        assert_eq!(windowed.total, 2, "stats cover only the window");
        assert_eq!(windowed.error_count, 2);

        let unseen = store.stats_for_active_skill("never-run", 50).await.unwrap();
        assert_eq!(unseen.total, 0, "an unknown skill has no executions");
    }

    #[tokio::test]
    async fn sqlite_store_records_and_summarizes() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        assert_store_contract(&storage.skill_execution_store()).await;
    }

    #[tokio::test]
    async fn in_memory_store_records_and_summarizes() {
        assert_store_contract(&InMemorySkillExecutionStore::new()).await;
    }

    #[tokio::test]
    async fn sqlite_store_round_trips_every_field() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.skill_execution_store();
        let original = SkillExecution {
            conversation_id: Uuid::new_v4(),
            tool_name: "file-read".to_string(),
            ..execution("deploy", 7, Some("no such file"), 12)
        };
        store.record(&original).await.unwrap();

        let stored = store.recent("deploy", 1).await.unwrap();
        assert_eq!(stored, vec![original]);
    }
}
