//! Contract tests for [`SlackActiveThreadStore`].

use assistant_storage::{
    InMemorySlackActiveThreadStore, SlackActiveThreadStore, SqliteSlackActiveThreadStore,
    StorageLayer,
};

async fn sqlite() -> Box<dyn SlackActiveThreadStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteSlackActiveThreadStore::new(storage.pool))
}

fn memory() -> Box<dyn SlackActiveThreadStore> {
    Box::new(InMemorySlackActiveThreadStore::new())
}

async fn upsert_and_contains(store: Box<dyn SlackActiveThreadStore>) {
    assert!(!store.contains("C123", "1.2").await.unwrap());
    store.upsert("C123", "1.2").await.unwrap();
    assert!(store.contains("C123", "1.2").await.unwrap());
}

async fn upsert_idempotent(store: Box<dyn SlackActiveThreadStore>) {
    store.upsert("C1", "t1").await.unwrap();
    store.upsert("C1", "t1").await.unwrap();
    assert!(store.contains("C1", "t1").await.unwrap());
}

async fn prune_preserves_recent_rows(store: Box<dyn SlackActiveThreadStore>) {
    // With older_than_days = 7, a just-upserted row's last_seen_at is well
    // within the window and must be preserved.
    store.upsert("C1", "t1").await.unwrap();
    let pruned = store.prune(7).await.unwrap();
    assert_eq!(pruned, 0);
    assert!(store.contains("C1", "t1").await.unwrap());
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_upsert_and_contains() {
                upsert_and_contains($ctor).await;
            }

            #[tokio::test]
            async fn t_upsert_idempotent() {
                upsert_idempotent($ctor).await;
            }

            #[tokio::test]
            async fn t_prune_preserves_recent_rows() {
                prune_preserves_recent_rows($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
