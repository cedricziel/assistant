//! Contract tests for [`ConversationEventStore`].

use assistant_storage::{
    ConversationEventStore, InMemoryConversationEventStore, SqliteConversationEventStore,
    StorageLayer,
};
use chrono::Duration;
use serde_json::json;

async fn sqlite() -> Box<dyn ConversationEventStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteConversationEventStore::new(storage.pool))
}

fn memory() -> Box<dyn ConversationEventStore> {
    Box::new(InMemoryConversationEventStore::new())
}

async fn append_and_list_events(store: Box<dyn ConversationEventStore>) {
    store
        .append_event("r1", "c1", 0, "run_started", &json!({}))
        .await
        .unwrap();
    store
        .append_event("r1", "c1", 1, "chunk", &json!({"text": "hi"}))
        .await
        .unwrap();
    let events = store.list_events_since("r1", 0).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "run_started");
    assert_eq!(events[1].event_type, "chunk");
}

async fn list_events_since_cursor(store: Box<dyn ConversationEventStore>) {
    store
        .append_event("r1", "c1", 0, "run_started", &json!({}))
        .await
        .unwrap();
    store
        .append_event("r1", "c1", 1, "chunk", &json!({}))
        .await
        .unwrap();
    store
        .append_event("r1", "c1", 2, "done", &json!({}))
        .await
        .unwrap();
    let later = store.list_events_since("r1", 1).await.unwrap();
    assert_eq!(later.len(), 2);
    assert_eq!(later[0].sequence, 1);
    assert_eq!(later[1].sequence, 2);
}

async fn has_events_distinguishes_missing_run(store: Box<dyn ConversationEventStore>) {
    assert!(!store.has_events("unknown").await.unwrap());
    store
        .append_event("r1", "c1", 0, "run_started", &json!({}))
        .await
        .unwrap();
    assert!(store.has_events("r1").await.unwrap());
}

async fn is_run_complete_detects_done(store: Box<dyn ConversationEventStore>) {
    store
        .append_event("r1", "c1", 0, "run_started", &json!({}))
        .await
        .unwrap();
    assert!(!store.is_run_complete("r1").await.unwrap());
    store
        .append_event("r1", "c1", 1, "done", &json!({}))
        .await
        .unwrap();
    assert!(store.is_run_complete("r1").await.unwrap());
}

async fn is_run_complete_detects_agent_error(store: Box<dyn ConversationEventStore>) {
    store
        .append_event("r1", "c1", 0, "run_started", &json!({}))
        .await
        .unwrap();
    store
        .append_event("r1", "c1", 1, "agent_error", &json!({}))
        .await
        .unwrap();
    assert!(store.is_run_complete("r1").await.unwrap());
}

async fn append_synthetic_terminal_assigns_sequence(store: Box<dyn ConversationEventStore>) {
    store
        .append_event("r1", "c1", 0, "run_started", &json!({}))
        .await
        .unwrap();
    let seq = store
        .append_synthetic_terminal("r1", "c1", "crashed")
        .await
        .unwrap();
    assert_eq!(seq, 1);
    let events = store.list_events_since("r1", 0).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "agent_error");
    assert_eq!(events[1].sequence, 1);
    assert_eq!(
        events[1].payload.get("synthetic").and_then(|v| v.as_bool()),
        Some(true)
    );
}

async fn find_incomplete_runs_filters_by_age(store: Box<dyn ConversationEventStore>) {
    // Append a run_started with a created_at far in the past would require
    // a clock-injected impl; here we just verify the function returns empty
    // for recently-started runs.
    store
        .append_event("r1", "c1", 0, "run_started", &json!({}))
        .await
        .unwrap();
    let incomplete = store
        .find_incomplete_runs(Duration::hours(1))
        .await
        .unwrap();
    // Just-started run is NOT older than 1 hour → not flagged.
    assert!(incomplete.is_empty());
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_append_and_list_events() {
                append_and_list_events($ctor).await;
            }

            #[tokio::test]
            async fn t_list_events_since_cursor() {
                list_events_since_cursor($ctor).await;
            }

            #[tokio::test]
            async fn t_has_events_distinguishes_missing_run() {
                has_events_distinguishes_missing_run($ctor).await;
            }

            #[tokio::test]
            async fn t_is_run_complete_detects_done() {
                is_run_complete_detects_done($ctor).await;
            }

            #[tokio::test]
            async fn t_is_run_complete_detects_agent_error() {
                is_run_complete_detects_agent_error($ctor).await;
            }

            #[tokio::test]
            async fn t_append_synthetic_terminal_assigns_sequence() {
                append_synthetic_terminal_assigns_sequence($ctor).await;
            }

            #[tokio::test]
            async fn t_find_incomplete_runs_filters_by_age() {
                find_incomplete_runs_filters_by_age($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
