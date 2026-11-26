//! Per-user feedback on assistant messages, backed by the `message_feedback` table.
//!
//! Each (message, user) pair holds at most one rating with an optional
//! comment. Feedback is product data owned by the space database: deleting a
//! message deletes its feedback, and the comment text never leaves it.

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result, bail};
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool, sqlite::SqliteRow};
use uuid::Uuid;

/// A user's verdict on an assistant message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageFeedbackRating {
    Positive,
    Negative,
}

impl MessageFeedbackRating {
    /// The value stored in the `rating` column.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }
}

impl fmt::Display for MessageFeedbackRating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MessageFeedbackRating {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "positive" => Ok(Self::Positive),
            "negative" => Ok(Self::Negative),
            other => bail!("unknown message feedback rating: {other}"),
        }
    }
}

/// A row from the `message_feedback` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageFeedbackRow {
    pub message_id: Uuid,
    pub user_id: String,
    pub rating: MessageFeedbackRating,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trait-based interface for message feedback persistence.
#[async_trait]
pub trait MessageFeedbackStore: Send + Sync {
    /// Replace the user's feedback on a message and return the stored row.
    ///
    /// A single atomic upsert: `created_at` of an existing row is kept and
    /// `updated_at` advances. The SQLite implementation errors when
    /// `message_id` does not belong to `conversation_id`.
    async fn set(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        user_id: &str,
        rating: MessageFeedbackRating,
        comment: Option<&str>,
    ) -> Result<MessageFeedbackRow>;

    /// Remove the user's feedback. Returns `true` when a row was removed.
    async fn clear(&self, message_id: Uuid, user_id: &str) -> Result<bool>;

    /// Fetch the user's feedback on one message.
    async fn get(&self, message_id: Uuid, user_id: &str) -> Result<Option<MessageFeedbackRow>>;

    /// List the user's feedback across a conversation's messages.
    async fn list_for_conversation(
        &self,
        conversation_id: Uuid,
        user_id: &str,
    ) -> Result<Vec<MessageFeedbackRow>>;
}

// -- SQLite --

/// SQLite-backed [`MessageFeedbackStore`].
pub struct SqliteMessageFeedbackStore {
    pool: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl SqliteMessageFeedbackStore {
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

fn parse_timestamp(s: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(s)?.with_timezone(&Utc))
}

fn row_to_feedback(row: &SqliteRow) -> Result<MessageFeedbackRow> {
    let message_id: String = row.try_get("message_id")?;
    let rating: String = row.try_get("rating")?;
    let created_at: String = row.try_get("created_at")?;
    let updated_at: String = row.try_get("updated_at")?;
    Ok(MessageFeedbackRow {
        message_id: Uuid::parse_str(&message_id)?,
        user_id: row.try_get("user_id")?,
        rating: rating.parse()?,
        comment: row.try_get("comment")?,
        created_at: parse_timestamp(&created_at)?,
        updated_at: parse_timestamp(&updated_at)?,
    })
}

#[async_trait]
impl MessageFeedbackStore for SqliteMessageFeedbackStore {
    async fn set(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        user_id: &str,
        rating: MessageFeedbackRating,
        comment: Option<&str>,
    ) -> Result<MessageFeedbackRow> {
        let now = self.clock.now().to_rfc3339();
        let row = sqlx::query(
            "INSERT INTO message_feedback \
             (message_id, user_id, rating, comment, created_at, updated_at) \
             SELECT m.id, ?3, ?4, ?5, ?6, ?6 FROM messages m \
             WHERE m.id = ?1 AND m.conversation_id = ?2 \
             ON CONFLICT (message_id, user_id) DO UPDATE SET \
                 rating = excluded.rating, \
                 comment = excluded.comment, \
                 updated_at = excluded.updated_at \
             RETURNING message_id, user_id, rating, comment, created_at, updated_at",
        )
        .bind(message_id.to_string())
        .bind(conversation_id.to_string())
        .bind(user_id)
        .bind(rating.as_str())
        .bind(comment)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => row_to_feedback(&row),
            None => bail!("message {message_id} not found in conversation {conversation_id}"),
        }
    }

    async fn clear(&self, message_id: Uuid, user_id: &str) -> Result<bool> {
        let result =
            sqlx::query("DELETE FROM message_feedback WHERE message_id = ?1 AND user_id = ?2")
                .bind(message_id.to_string())
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn get(&self, message_id: Uuid, user_id: &str) -> Result<Option<MessageFeedbackRow>> {
        let row = sqlx::query(
            "SELECT message_id, user_id, rating, comment, created_at, updated_at \
             FROM message_feedback \
             WHERE message_id = ?1 AND user_id = ?2",
        )
        .bind(message_id.to_string())
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(row_to_feedback).transpose()
    }

    async fn list_for_conversation(
        &self,
        conversation_id: Uuid,
        user_id: &str,
    ) -> Result<Vec<MessageFeedbackRow>> {
        let rows = sqlx::query(
            "SELECT f.message_id, f.user_id, f.rating, f.comment, f.created_at, f.updated_at \
             FROM message_feedback f \
             JOIN messages m ON m.id = f.message_id \
             WHERE m.conversation_id = ?1 AND f.user_id = ?2 \
             ORDER BY f.updated_at ASC, f.message_id ASC",
        )
        .bind(conversation_id.to_string())
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("listing message feedback")?;
        rows.iter().map(row_to_feedback).collect()
    }
}

// -- In-memory --

type FeedbackKey = (Uuid, String);

/// In-memory [`MessageFeedbackStore`].
///
/// It cannot see the `messages` table, so it accepts any message id and
/// remembers the conversation each row was set under.
pub struct InMemoryMessageFeedbackStore {
    state: Mutex<HashMap<FeedbackKey, (Uuid, MessageFeedbackRow)>>,
    clock: Arc<dyn Clock>,
}

impl InMemoryMessageFeedbackStore {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
            clock: Arc::new(SystemClock),
        }
    }

    /// Inject a [`Clock`] implementation.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<FeedbackKey, (Uuid, MessageFeedbackRow)>> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl Default for InMemoryMessageFeedbackStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageFeedbackStore for InMemoryMessageFeedbackStore {
    async fn set(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        user_id: &str,
        rating: MessageFeedbackRating,
        comment: Option<&str>,
    ) -> Result<MessageFeedbackRow> {
        let now = self.clock.now();
        let mut state = self.lock();
        let key = (message_id, user_id.to_string());
        let created_at = state
            .get(&key)
            .map_or(now, |(_, existing)| existing.created_at);
        let row = MessageFeedbackRow {
            message_id,
            user_id: user_id.to_string(),
            rating,
            comment: comment.map(str::to_string),
            created_at,
            updated_at: now,
        };
        state.insert(key, (conversation_id, row.clone()));
        Ok(row)
    }

    async fn clear(&self, message_id: Uuid, user_id: &str) -> Result<bool> {
        Ok(self
            .lock()
            .remove(&(message_id, user_id.to_string()))
            .is_some())
    }

    async fn get(&self, message_id: Uuid, user_id: &str) -> Result<Option<MessageFeedbackRow>> {
        Ok(self
            .lock()
            .get(&(message_id, user_id.to_string()))
            .map(|(_, row)| row.clone()))
    }

    async fn list_for_conversation(
        &self,
        conversation_id: Uuid,
        user_id: &str,
    ) -> Result<Vec<MessageFeedbackRow>> {
        let mut rows: Vec<MessageFeedbackRow> = self
            .lock()
            .values()
            .filter(|(conv, row)| *conv == conversation_id && row.user_id == user_id)
            .map(|(_, row)| row.clone())
            .collect();
        rows.sort_by_key(|row| (row.updated_at, row.message_id));
        Ok(rows)
    }
}
