//! Contract tests for the [`CommandEventStore`] trait.

use assistant_storage::{
    CommandEventStore, InMemoryCommandEventStore, SqliteCommandEventStore, StorageLayer,
};
use uuid::Uuid;

async fn sqlite() -> Box<dyn CommandEventStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteCommandEventStore::new(storage.pool))
}

fn memory() -> Box<dyn CommandEventStore> {
    Box::new(InMemoryCommandEventStore::new())
}

async fn save_and_list(store: Box<dyn CommandEventStore>) {
    let conv = Uuid::new_v4();
    let payload = serde_json::json!({"k": "v"});
    let event = store
        .save_event(conv, "model", Some(&payload), "ack")
        .await
        .unwrap();
    assert_eq!(event.command, "model");
    assert_eq!(event.conversation_id, conv);
    let events = store.list_events(conv).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].ack_text.as_deref(), Some("ack"));
}

async fn list_empty_returns_empty(store: Box<dyn CommandEventStore>) {
    let events = store.list_events(Uuid::new_v4()).await.unwrap();
    assert!(events.is_empty());
}

async fn list_orders_by_created_at(store: Box<dyn CommandEventStore>) {
    let conv = Uuid::new_v4();
    store.save_event(conv, "first", None, "a1").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store.save_event(conv, "second", None, "a2").await.unwrap();
    let events = store.list_events(conv).await.unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].command, "first");
    assert_eq!(events[1].command, "second");
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_save_and_list() {
                save_and_list($ctor).await;
            }

            #[tokio::test]
            async fn t_list_empty() {
                list_empty_returns_empty($ctor).await;
            }

            #[tokio::test]
            async fn t_list_orders_by_created_at() {
                list_orders_by_created_at($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
