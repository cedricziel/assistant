//! Durable conversation event log.
//!
//! Every orchestrator event emitted during a streaming turn is written here so
//! clients can reconnect and replay missed events from a sequence cursor.
//! Events are ephemeral: `expires_at` drives TTL-based pruning.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use tokio::sync::{RwLock, broadcast};
use uuid::Uuid;

/// Default TTL for conversation events: 24 hours.
pub const DEFAULT_EVENT_TTL_HOURS: i64 = 24;

/// A single row from the `conversation_events` table.
#[derive(Debug, Clone)]
pub struct ConversationEventRow {
    pub id: i64,
    pub run_id: String,
    pub conversation_id: String,
    pub sequence: i64,
    pub event_type: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// SQLite-backed store for conversation event log entries.
#[derive(Clone)]
pub struct ConversationEventStore {
    pool: SqlitePool,
    /// How long events are retained before pruning.
    ttl: Duration,
}

impl ConversationEventStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            ttl: Duration::hours(DEFAULT_EVENT_TTL_HOURS),
        }
    }

    pub fn with_ttl(pool: SqlitePool, ttl: Duration) -> Self {
        Self { pool, ttl }
    }

    /// Append a single event to the log.
    ///
    /// `sequence` must be monotonically increasing within a `run_id`.
    /// The unique index on `(run_id, sequence)` enforces this at the DB level.
    pub async fn append_event(
        &self,
        run_id: &str,
        conversation_id: &str,
        sequence: i64,
        event_type: &str,
        payload: &Value,
    ) -> Result<()> {
        let now = Utc::now();
        let expires_at = now + self.ttl;
        sqlx::query(
            "INSERT INTO conversation_events \
             (run_id, conversation_id, sequence, event_type, payload, created_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(run_id)
        .bind(conversation_id)
        .bind(sequence)
        .bind(event_type)
        .bind(payload.to_string())
        .bind(now)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Return all events for `run_id` with `sequence >= since`, ordered by sequence.
    pub async fn list_events_since(
        &self,
        run_id: &str,
        since: i64,
    ) -> Result<Vec<ConversationEventRow>> {
        let rows = sqlx::query(
            "SELECT id, run_id, conversation_id, sequence, event_type, payload, created_at, expires_at \
             FROM conversation_events \
             WHERE run_id = ?1 AND sequence >= ?2 \
             ORDER BY sequence ASC",
        )
        .bind(run_id)
        .bind(since)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| parse_row(&r)).collect()
    }

    /// Return `true` if any unexpired events exist for `run_id`.
    ///
    /// Used to distinguish "run not found" (no rows at all) from "run expired"
    /// (rows existed but TTL elapsed and were pruned).
    pub async fn has_events(&self, run_id: &str) -> Result<bool> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM conversation_events WHERE run_id = ?1")
                .bind(run_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(count > 0)
    }

    /// Return `true` if the run has completed (a `done` or `agent_error` event was persisted).
    pub async fn is_run_complete(&self, run_id: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM conversation_events \
             WHERE run_id = ?1 AND (event_type = 'done' OR event_type = 'agent_error')",
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count > 0)
    }

    /// Delete all rows where `expires_at < now`. Returns the number of rows deleted.
    pub async fn prune_expired(&self) -> Result<u64> {
        let now = Utc::now();
        let result = sqlx::query("DELETE FROM conversation_events WHERE expires_at < ?1")
            .bind(now)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Find runs that have a `run_started` event but no terminal event
    /// (`done` or `agent_error`), and whose `run_started` was created more than
    /// `older_than` ago.
    ///
    /// Returns `(run_id, conversation_id)` pairs for orphaned runs that likely
    /// died mid-stream (e.g. server restart).
    pub async fn find_incomplete_runs(
        &self,
        older_than: Duration,
    ) -> Result<Vec<(String, String)>> {
        let cutoff = Utc::now() - older_than;
        let rows = sqlx::query(
            "SELECT DISTINCT ce.run_id, ce.conversation_id \
             FROM conversation_events ce \
             WHERE ce.event_type = 'run_started' \
               AND ce.created_at < ?1 \
               AND NOT EXISTS ( \
                   SELECT 1 FROM conversation_events ce2 \
                   WHERE ce2.run_id = ce.run_id \
                     AND ce2.event_type IN ('done', 'agent_error') \
               )",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .iter()
            .map(|r| {
                let run_id: String = r.try_get("run_id").unwrap_or_default();
                let conversation_id: String = r.try_get("conversation_id").unwrap_or_default();
                (run_id, conversation_id)
            })
            .collect();
        Ok(results)
    }

    /// Append a synthetic `agent_error` terminal event to an orphaned run.
    ///
    /// Auto-determines the next sequence number. The payload includes
    /// `"synthetic": true` so clients can distinguish crash recovery from real
    /// errors.
    pub async fn append_synthetic_terminal(
        &self,
        run_id: &str,
        conversation_id: &str,
        message: &str,
    ) -> Result<i64> {
        let next_seq: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence), -1) + 1 \
             FROM conversation_events WHERE run_id = ?1",
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;

        let payload = serde_json::json!({
            "message": message,
            "synthetic": true,
        });

        self.append_event(run_id, conversation_id, next_seq, "agent_error", &payload)
            .await?;

        Ok(next_seq)
    }
}

// -- Live broadcast registry -------------------------------------------------

/// A live event broadcast over the in-memory registry.
///
/// Sent to all active tail subscribers while an orchestrator run is in progress.
#[derive(Debug, Clone)]
pub struct LiveEvent {
    pub sequence: i64,
    pub event_type: String,
    pub payload: Value,
}

/// In-memory registry of active run broadcast channels.
///
/// The orchestrator publishes to the sender; SSE tail handlers subscribe.
/// When a run completes (or errors), the sender is removed and all receivers
/// observe channel closure, signalling end-of-stream.
#[derive(Clone, Default)]
pub struct RunBroadcaster {
    /// Map of `run_id` → broadcast sender for that run's live events.
    senders: Arc<RwLock<HashMap<Uuid, broadcast::Sender<LiveEvent>>>>,
}

impl RunBroadcaster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new broadcast channel for `run_id` and return a sender.
    ///
    /// Call this before the first event is emitted; drop the returned sender
    /// (via `finish_run`) when the run completes.
    pub async fn start_run(&self, run_id: Uuid) -> broadcast::Sender<LiveEvent> {
        let (tx, _) = broadcast::channel(256);
        self.senders.write().await.insert(run_id, tx.clone());
        tx
    }

    /// Remove the sender for `run_id`, signalling completion to all subscribers.
    pub async fn finish_run(&self, run_id: &Uuid) {
        self.senders.write().await.remove(run_id);
    }

    /// Subscribe to live events for `run_id`.
    ///
    /// Returns `None` if the run is not currently active (already completed or
    /// never started — caller should fall back to DB replay only).
    pub async fn subscribe(&self, run_id: &Uuid) -> Option<broadcast::Receiver<LiveEvent>> {
        self.senders
            .read()
            .await
            .get(run_id)
            .map(|tx| tx.subscribe())
    }
}

fn parse_row(r: &sqlx::sqlite::SqliteRow) -> Result<ConversationEventRow> {
    let payload_str: String = r.try_get("payload")?;
    let payload: Value = serde_json::from_str(&payload_str)?;
    Ok(ConversationEventRow {
        id: r.try_get("id")?,
        run_id: r.try_get("run_id")?,
        conversation_id: r.try_get("conversation_id")?,
        sequence: r.try_get("sequence")?,
        event_type: r.try_get("event_type")?,
        payload,
        created_at: r.try_get("created_at")?,
        expires_at: r.try_get("expires_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;

    async fn make_store() -> ConversationEventStore {
        let sl = StorageLayer::new_in_memory().await.unwrap();
        ConversationEventStore::new(sl.pool)
    }

    #[tokio::test]
    async fn append_and_list_events() {
        let store = make_store().await;
        let run_id = "run-001";
        let conv_id = "conv-001";

        store
            .append_event(
                run_id,
                conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();
        store
            .append_event(
                run_id,
                conv_id,
                1,
                "token",
                &serde_json::json!({"token": "hello"}),
            )
            .await
            .unwrap();
        store
            .append_event(
                run_id,
                conv_id,
                2,
                "token",
                &serde_json::json!({"token": " world"}),
            )
            .await
            .unwrap();

        let events = store.list_events_since(run_id, 0).await.unwrap();
        assert_eq!(events.len(), 3, "expected 3 events");
        assert_eq!(events[0].event_type, "run_started");
        assert_eq!(events[1].event_type, "token");
        assert_eq!(events[1].payload["token"], "hello");

        let from_1 = store.list_events_since(run_id, 1).await.unwrap();
        assert_eq!(from_1.len(), 2, "since=1 should return 2 events");
    }

    #[tokio::test]
    async fn has_events_returns_false_for_unknown_run() {
        let store = make_store().await;
        assert!(!store.has_events("nonexistent-run").await.unwrap());
    }

    #[tokio::test]
    async fn is_run_complete_detects_done_event() {
        let store = make_store().await;
        let run_id = "run-002";
        let conv_id = "conv-002";

        store
            .append_event(
                run_id,
                conv_id,
                0,
                "token",
                &serde_json::json!({"token": "hi"}),
            )
            .await
            .unwrap();
        assert!(!store.is_run_complete(run_id).await.unwrap());

        store
            .append_event(
                run_id,
                conv_id,
                1,
                "done",
                &serde_json::json!({"content": "hi"}),
            )
            .await
            .unwrap();
        assert!(store.is_run_complete(run_id).await.unwrap());
    }

    #[tokio::test]
    async fn prune_expired_removes_old_rows() {
        let sl = StorageLayer::new_in_memory().await.unwrap();
        let store = ConversationEventStore::with_ttl(
            sl.pool.clone(),
            Duration::seconds(-1), // already expired on insert
        );
        let run_id = "run-003";
        let conv_id = "conv-003";

        store
            .append_event(
                run_id,
                conv_id,
                0,
                "token",
                &serde_json::json!({"token": "x"}),
            )
            .await
            .unwrap();

        let deleted = store.prune_expired().await.unwrap();
        assert_eq!(deleted, 1, "one expired row should be pruned");

        let events = store.list_events_since(run_id, 0).await.unwrap();
        assert!(events.is_empty(), "pruned events should not be returned");
    }

    // -- RunBroadcaster tests --------------------------------------------------

    #[tokio::test]
    async fn broadcaster_subscribe_returns_none_before_start() {
        let broadcaster = RunBroadcaster::new();
        let run_id = Uuid::new_v4();
        assert!(
            broadcaster.subscribe(&run_id).await.is_none(),
            "no subscription before start_run"
        );
    }

    #[tokio::test]
    async fn broadcaster_start_subscribe_receive_finish() {
        let broadcaster = RunBroadcaster::new();
        let run_id = Uuid::new_v4();

        let tx = broadcaster.start_run(run_id).await;
        let mut rx = broadcaster
            .subscribe(&run_id)
            .await
            .expect("subscribe after start_run should succeed");

        // Send one event.
        tx.send(LiveEvent {
            sequence: 0,
            event_type: "token".to_string(),
            payload: serde_json::json!({"token": "hello"}),
        })
        .expect("send should succeed while receiver exists");

        let event = rx.recv().await.expect("recv should return the sent event");
        assert_eq!(event.event_type, "token");
        assert_eq!(event.sequence, 0);
        assert_eq!(event.payload["token"], "hello");

        // Finish the run: removing from registry drops the sender.
        broadcaster.finish_run(&run_id).await;
        drop(tx); // also drop our local handle so the channel closes

        assert!(
            matches!(
                rx.recv().await,
                Err(tokio::sync::broadcast::error::RecvError::Closed)
            ),
            "channel should be closed after finish_run"
        );
    }

    #[tokio::test]
    async fn broadcaster_concurrent_subscribers_both_receive() {
        let broadcaster = RunBroadcaster::new();
        let run_id = Uuid::new_v4();
        let tx = broadcaster.start_run(run_id).await;

        let mut rx1 = broadcaster.subscribe(&run_id).await.unwrap();
        let mut rx2 = broadcaster.subscribe(&run_id).await.unwrap();

        tx.send(LiveEvent {
            sequence: 7,
            event_type: "status".to_string(),
            payload: serde_json::json!({"message": "thinking"}),
        })
        .unwrap();

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();

        assert_eq!(e1.event_type, "status");
        assert_eq!(e2.event_type, "status");
        assert_eq!(e1.sequence, 7);
        assert_eq!(
            e2.sequence, e1.sequence,
            "both subscribers see the same sequence"
        );
    }

    #[tokio::test]
    async fn broadcaster_subscribe_returns_none_after_finish() {
        let broadcaster = RunBroadcaster::new();
        let run_id = Uuid::new_v4();
        let tx = broadcaster.start_run(run_id).await;
        broadcaster.finish_run(&run_id).await;
        drop(tx);

        assert!(
            broadcaster.subscribe(&run_id).await.is_none(),
            "no subscription after finish_run"
        );
    }

    #[tokio::test]
    async fn is_run_complete_detects_agent_error_event() {
        let store = make_store().await;
        let run_id = "run-004";
        let conv_id = "conv-004";

        store
            .append_event(
                run_id,
                conv_id,
                0,
                "token",
                &serde_json::json!({"token": "oops"}),
            )
            .await
            .unwrap();
        assert!(!store.is_run_complete(run_id).await.unwrap());

        store
            .append_event(
                run_id,
                conv_id,
                1,
                "agent_error",
                &serde_json::json!({"message": "LLM failed"}),
            )
            .await
            .unwrap();
        assert!(
            store.is_run_complete(run_id).await.unwrap(),
            "agent_error event should mark run as complete"
        );
    }

    #[tokio::test]
    async fn find_incomplete_runs_returns_orphans() {
        let store = make_store().await;

        // Create a run that started 10 minutes ago but has no terminal event.
        let orphan_run = "run-orphan";
        let orphan_conv = "conv-orphan";
        store
            .append_event(
                orphan_run,
                orphan_conv,
                0,
                "run_started",
                &serde_json::json!({"run_id": orphan_run}),
            )
            .await
            .unwrap();

        // Backdate the run_started event so it's older than the threshold.
        sqlx::query(
            "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
             WHERE run_id = ?1",
        )
        .bind(orphan_run)
        .execute(&store.pool)
        .await
        .unwrap();

        let orphans = store
            .find_incomplete_runs(Duration::minutes(5))
            .await
            .unwrap();
        assert_eq!(orphans.len(), 1);
        assert_eq!(
            orphans[0],
            (orphan_run.to_string(), orphan_conv.to_string())
        );
    }

    #[tokio::test]
    async fn find_incomplete_runs_excludes_completed() {
        let store = make_store().await;
        let run_id = "run-completed";
        let conv_id = "conv-completed";

        store
            .append_event(
                run_id,
                conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();
        store
            .append_event(
                run_id,
                conv_id,
                1,
                "done",
                &serde_json::json!({"content": "bye"}),
            )
            .await
            .unwrap();

        // Backdate so it's old enough.
        sqlx::query(
            "UPDATE conversation_events SET created_at = datetime('now', '-10 minutes') \
             WHERE run_id = ?1",
        )
        .bind(run_id)
        .execute(&store.pool)
        .await
        .unwrap();

        let orphans = store
            .find_incomplete_runs(Duration::minutes(5))
            .await
            .unwrap();
        assert!(
            orphans.is_empty(),
            "completed run should not appear as orphan"
        );
    }

    #[tokio::test]
    async fn find_incomplete_runs_respects_age_threshold() {
        let store = make_store().await;
        let run_id = "run-fresh";
        let conv_id = "conv-fresh";

        // Create a run that just started (no backdating — it's fresh).
        store
            .append_event(
                run_id,
                conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();

        let orphans = store
            .find_incomplete_runs(Duration::minutes(5))
            .await
            .unwrap();
        assert!(orphans.is_empty(), "fresh run should not appear as orphan");
    }

    #[tokio::test]
    async fn append_synthetic_terminal_correct_sequence() {
        let store = make_store().await;
        let run_id = "run-synth";
        let conv_id = "conv-synth";

        // Append some events first.
        store
            .append_event(
                run_id,
                conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();
        store
            .append_event(
                run_id,
                conv_id,
                1,
                "token",
                &serde_json::json!({"token": "hi"}),
            )
            .await
            .unwrap();

        let seq = store
            .append_synthetic_terminal(run_id, conv_id, "Server restarted")
            .await
            .unwrap();

        assert_eq!(seq, 2, "synthetic event should get next sequence");

        // Verify the event was persisted correctly.
        let events = store.list_events_since(run_id, 2).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "agent_error");
        assert_eq!(events[0].payload["message"], "Server restarted");
        assert_eq!(events[0].payload["synthetic"], true);

        // Run should now be complete.
        assert!(store.is_run_complete(run_id).await.unwrap());
    }

    #[tokio::test]
    async fn has_events_returns_true_after_append() {
        let store = make_store().await;
        let run_id = "run-005";
        let conv_id = "conv-005";

        assert!(!store.has_events(run_id).await.unwrap());

        store
            .append_event(
                run_id,
                conv_id,
                0,
                "run_started",
                &serde_json::json!({"run_id": run_id}),
            )
            .await
            .unwrap();

        assert!(store.has_events(run_id).await.unwrap());
    }
}
