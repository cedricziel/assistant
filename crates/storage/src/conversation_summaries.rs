//! Conversation summaries backed by the `conversation_summaries` table.
//!
//! Context compaction records a summary of the messages up to a point instead
//! of deleting them. A summary applies to a path through the message tree only
//! when its `through_message_id` lies on that path, so a branch that diverged
//! before the summarised range never sees it.

use std::collections::HashSet;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use tokio::sync::RwLock;
use uuid::Uuid;

/// A summary of every message on a path up to `through_message_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSummary {
    pub id: Uuid,
    pub conversation_id: Uuid,
    /// The last message the summary covers.
    pub through_message_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Persistence for [`ConversationSummary`] rows.
#[async_trait]
pub trait ConversationSummaryStore: Send + Sync {
    /// Record a summary of `conversation_id` through `through_message_id`.
    async fn insert(
        &self,
        conversation_id: Uuid,
        through_message_id: Uuid,
        content: &str,
    ) -> Result<ConversationSummary>;

    /// The newest summary of `conversation_id` whose `through_message_id` is
    /// one of `path_ids`, or `None` when no summary applies to that path.
    async fn newest_on_path(
        &self,
        conversation_id: Uuid,
        path_ids: &[Uuid],
    ) -> Result<Option<ConversationSummary>>;
}

/// SQLite-backed [`ConversationSummaryStore`].
pub struct SqliteConversationSummaryStore {
    pool: SqlitePool,
}

impl SqliteConversationSummaryStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse_uuid(value: &str, column: &str) -> Result<Uuid> {
    Uuid::parse_str(value).with_context(|| format!("invalid {column} in conversation_summaries"))
}

fn row_to_summary(row: &sqlx::sqlite::SqliteRow) -> Result<ConversationSummary> {
    let id: String = row.try_get("id")?;
    let conversation_id: String = row.try_get("conversation_id")?;
    let through: String = row.try_get("through_message_id")?;
    let created_at: String = row.try_get("created_at")?;
    Ok(ConversationSummary {
        id: parse_uuid(&id, "id")?,
        conversation_id: parse_uuid(&conversation_id, "conversation_id")?,
        through_message_id: parse_uuid(&through, "through_message_id")?,
        content: row.try_get("content")?,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .context("invalid created_at in conversation_summaries")?
            .with_timezone(&Utc),
    })
}

#[async_trait]
impl ConversationSummaryStore for SqliteConversationSummaryStore {
    async fn insert(
        &self,
        conversation_id: Uuid,
        through_message_id: Uuid,
        content: &str,
    ) -> Result<ConversationSummary> {
        let summary = ConversationSummary {
            id: Uuid::new_v4(),
            conversation_id,
            through_message_id,
            content: content.to_string(),
            created_at: Utc::now(),
        };
        sqlx::query(
            "INSERT INTO conversation_summaries \
             (id, conversation_id, through_message_id, content, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(summary.id.to_string())
        .bind(conversation_id.to_string())
        .bind(through_message_id.to_string())
        .bind(content)
        .bind(summary.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .with_context(|| format!("storing summary for conversation {conversation_id}"))?;
        Ok(summary)
    }

    async fn newest_on_path(
        &self,
        conversation_id: Uuid,
        path_ids: &[Uuid],
    ) -> Result<Option<ConversationSummary>> {
        if path_ids.is_empty() {
            return Ok(None);
        }
        // A conversation has few summaries, so filter by path in Rust rather
        // than binding one parameter per path message.
        let rows = sqlx::query(
            "SELECT id, conversation_id, through_message_id, content, created_at \
             FROM conversation_summaries WHERE conversation_id = ?1 \
             ORDER BY created_at DESC, rowid DESC",
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("loading summaries for conversation {conversation_id}"))?;

        let on_path: HashSet<Uuid> = path_ids.iter().copied().collect();
        for row in &rows {
            let summary = row_to_summary(row)?;
            if on_path.contains(&summary.through_message_id) {
                return Ok(Some(summary));
            }
        }
        Ok(None)
    }
}

/// In-memory [`ConversationSummaryStore`] for tests.
#[derive(Default)]
pub struct InMemoryConversationSummaryStore {
    rows: RwLock<Vec<ConversationSummary>>,
}

impl InMemoryConversationSummaryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ConversationSummaryStore for InMemoryConversationSummaryStore {
    async fn insert(
        &self,
        conversation_id: Uuid,
        through_message_id: Uuid,
        content: &str,
    ) -> Result<ConversationSummary> {
        let summary = ConversationSummary {
            id: Uuid::new_v4(),
            conversation_id,
            through_message_id,
            content: content.to_string(),
            created_at: Utc::now(),
        };
        self.rows.write().await.push(summary.clone());
        Ok(summary)
    }

    async fn newest_on_path(
        &self,
        conversation_id: Uuid,
        path_ids: &[Uuid],
    ) -> Result<Option<ConversationSummary>> {
        let on_path: HashSet<Uuid> = path_ids.iter().copied().collect();
        // Rows are in insertion order, so the last match is the newest.
        Ok(self
            .rows
            .read()
            .await
            .iter()
            .rev()
            .find(|s| {
                s.conversation_id == conversation_id && on_path.contains(&s.through_message_id)
            })
            .cloned())
    }
}
