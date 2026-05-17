//! Slash-command event persistence backed by the `command_events` table.
//!
//! Records slash-command invocations so they appear in the conversation
//! timeline.  These records are never included in LLM context — the
//! orchestrator only loads the `messages` table.

use std::sync::Arc;

use anyhow::Result;
use assistant_core::clock::{Clock, SystemClock};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// A row from the `command_events` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEventRow {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub event_type: String,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_text: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// SQLite-backed store for slash-command events.
#[derive(Clone)]
pub struct CommandEventStore {
    pool: SqlitePool,
    /// Clock for row timestamps. Default `Arc::new(SystemClock)`.
    clock: Arc<dyn Clock>,
}

impl CommandEventStore {
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

    /// Persist a command event.
    pub async fn save_event(
        &self,
        conversation_id: Uuid,
        command: &str,
        payload: Option<&serde_json::Value>,
        ack_text: &str,
    ) -> Result<CommandEventRow> {
        let id = Uuid::new_v4();
        let now = self.clock.now();

        let id_str = id.to_string();
        let conv_str = conversation_id.to_string();
        let payload_str = payload.map(|p| serde_json::to_string(p).unwrap_or_default());

        sqlx::query(
            "INSERT INTO command_events (id, conversation_id, event_type, command, payload, ack_text, created_at) \
             VALUES (?1, ?2, 'command', ?3, ?4, ?5, ?6)",
        )
        .bind(&id_str)
        .bind(&conv_str)
        .bind(command)
        .bind(&payload_str)
        .bind(ack_text)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(CommandEventRow {
            id,
            conversation_id,
            event_type: "command".into(),
            command: command.into(),
            payload: payload.cloned(),
            ack_text: Some(ack_text.into()),
            created_at: now,
        })
    }

    /// List all command events for a conversation, ordered by `created_at` ascending.
    pub async fn list_events(&self, conversation_id: Uuid) -> Result<Vec<CommandEventRow>> {
        let conv_str = conversation_id.to_string();
        let rows = sqlx::query(
            "SELECT id, conversation_id, event_type, command, payload, ack_text, created_at \
             FROM command_events \
             WHERE conversation_id = ?1 \
             ORDER BY created_at ASC",
        )
        .bind(&conv_str)
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String = row.get("id");
            let conv_id_str: String = row.get("conversation_id");
            let payload_str: Option<String> = row.get("payload");
            let created_str: String = row.get("created_at");

            events.push(CommandEventRow {
                id: Uuid::parse_str(&id_str)?,
                conversation_id: Uuid::parse_str(&conv_id_str)?,
                event_type: row.get("event_type"),
                command: row.get("command"),
                payload: payload_str
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok()),
                ack_text: row.get("ack_text"),
                created_at: DateTime::parse_from_rfc3339(&created_str)?.with_timezone(&Utc),
            });
        }

        Ok(events)
    }
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    #[tokio::test]
    async fn save_and_list_events() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = CommandEventStore::new(storage.pool.clone());

        let conv_id = Uuid::new_v4();
        let payload = serde_json::json!({"model_name": "claude-opus-4"});

        let event = store
            .save_event(
                conv_id,
                "model",
                Some(&payload),
                "Model switched to claude-opus-4.",
            )
            .await
            .unwrap();

        assert_eq!(event.command, "model");
        assert_eq!(event.conversation_id, conv_id);
        assert_eq!(
            event.ack_text.as_deref(),
            Some("Model switched to claude-opus-4.")
        );

        let events = store.list_events(conv_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].command, "model");
        assert_eq!(
            events[0]
                .payload
                .as_ref()
                .and_then(|p| p.get("model_name"))
                .and_then(|v| v.as_str()),
            Some("claude-opus-4")
        );
    }

    #[tokio::test]
    async fn list_events_empty_for_unknown_conversation() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = CommandEventStore::new(storage.pool.clone());

        let events = store.list_events(Uuid::new_v4()).await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn events_ordered_by_created_at() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = CommandEventStore::new(storage.pool.clone());
        let conv_id = Uuid::new_v4();

        store
            .save_event(conv_id, "model", None, "Model switched to a.")
            .await
            .unwrap();
        store
            .save_event(conv_id, "status", None, "Status info.")
            .await
            .unwrap();

        let events = store.list_events(conv_id).await.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].command, "model");
        assert_eq!(events[1].command, "status");
        assert!(events[0].created_at <= events[1].created_at);
    }
}
