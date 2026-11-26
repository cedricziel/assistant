//! Per-message turn timing backed by the `message_timings` table.
//!
//! The orchestrator measures a [`TurnTiming`] for every answered turn and
//! puts it here against the assistant message id. Conversation history reads
//! the whole conversation's timings in one call.
//!
//! The table references `conversations` but not `messages`, so
//! `replace_history` (context compaction) keeps the timing of every message
//! it keeps.

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};
use tokio::sync::RwLock;
use uuid::Uuid;

use assistant_core::TurnTiming;

/// Persistence for [`TurnTiming`] values, one per assistant message.
#[async_trait]
pub trait MessageTimingStore: Send + Sync {
    /// Store the timing of `message_id`, replacing any earlier value.
    async fn put(&self, conversation_id: Uuid, message_id: Uuid, timing: &TurnTiming)
    -> Result<()>;

    /// Every stored timing of `conversation_id`, keyed by message id.
    async fn get_for_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<HashMap<Uuid, TurnTiming>>;
}

/// SQLite-backed [`MessageTimingStore`].
pub struct SqliteMessageTimingStore {
    pool: SqlitePool,
}

impl SqliteMessageTimingStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn to_db(value: u64) -> Result<i64> {
    i64::try_from(value).context("timing value does not fit a SQLite integer")
}

fn from_db(value: i64) -> Result<u64> {
    u64::try_from(value).context("negative timing value in message_timings")
}

#[async_trait]
impl MessageTimingStore for SqliteMessageTimingStore {
    async fn put(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        timing: &TurnTiming,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO message_timings \
             (message_id, conversation_id, time_to_first_token_ms, total_ms, output_tokens, decode_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(message_id) DO UPDATE SET \
                 conversation_id = excluded.conversation_id, \
                 time_to_first_token_ms = excluded.time_to_first_token_ms, \
                 total_ms = excluded.total_ms, \
                 output_tokens = excluded.output_tokens, \
                 decode_ms = excluded.decode_ms",
        )
        .bind(message_id.to_string())
        .bind(conversation_id.to_string())
        .bind(timing.time_to_first_token_ms.map(to_db).transpose()?)
        .bind(to_db(timing.total_ms)?)
        .bind(timing.output_tokens.map(to_db).transpose()?)
        .bind(timing.decode_ms.map(to_db).transpose()?)
        .execute(&self.pool)
        .await
        .with_context(|| format!("storing timing for message {message_id}"))?;
        Ok(())
    }

    async fn get_for_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<HashMap<Uuid, TurnTiming>> {
        let rows = sqlx::query(
            "SELECT message_id, time_to_first_token_ms, total_ms, output_tokens, decode_ms \
             FROM message_timings WHERE conversation_id = ?1",
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("loading timings for conversation {conversation_id}"))?;

        rows.iter()
            .map(|row| {
                let message_id: String = row.get("message_id");
                let optional = |column: &str| -> Result<Option<u64>> {
                    row.get::<Option<i64>, _>(column).map(from_db).transpose()
                };
                let timing = TurnTiming {
                    time_to_first_token_ms: optional("time_to_first_token_ms")?,
                    total_ms: from_db(row.get("total_ms"))?,
                    output_tokens: optional("output_tokens")?,
                    decode_ms: optional("decode_ms")?,
                };
                let id = Uuid::parse_str(&message_id)
                    .context("invalid message_id in message_timings")?;
                Ok((id, timing))
            })
            .collect()
    }
}

/// In-memory [`MessageTimingStore`] for tests.
#[derive(Default)]
pub struct InMemoryMessageTimingStore {
    rows: RwLock<HashMap<Uuid, (Uuid, TurnTiming)>>,
}

impl InMemoryMessageTimingStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MessageTimingStore for InMemoryMessageTimingStore {
    async fn put(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        timing: &TurnTiming,
    ) -> Result<()> {
        self.rows
            .write()
            .await
            .insert(message_id, (conversation_id, timing.clone()));
        Ok(())
    }

    async fn get_for_conversation(
        &self,
        conversation_id: Uuid,
    ) -> Result<HashMap<Uuid, TurnTiming>> {
        Ok(self
            .rows
            .read()
            .await
            .iter()
            .filter(|(_, (conv, _))| *conv == conversation_id)
            .map(|(message_id, (_, timing))| (*message_id, timing.clone()))
            .collect())
    }
}
