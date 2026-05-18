//! Contract tests for the [`PushSubscriptionStore`] trait.

use assistant_storage::{
    InMemoryPushSubscriptionStore, PushSubscriptionStore, SqlitePushSubscriptionStore, StorageLayer,
};

async fn sqlite_store() -> Box<dyn PushSubscriptionStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqlitePushSubscriptionStore::new(storage.pool))
}

fn memory_store() -> Box<dyn PushSubscriptionStore> {
    Box::new(InMemoryPushSubscriptionStore::new())
}

async fn upsert_and_list(store: Box<dyn PushSubscriptionStore>) {
    store
        .upsert("https://push.example.com/abc", "p256", "auth")
        .await
        .unwrap();
    let subs = store.list_all().await.unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].endpoint, "https://push.example.com/abc");
}

async fn upsert_replaces_existing(store: Box<dyn PushSubscriptionStore>) {
    store
        .upsert("https://push.example.com/dup", "k1", "a1")
        .await
        .unwrap();
    store
        .upsert("https://push.example.com/dup", "k2", "a2")
        .await
        .unwrap();
    let subs = store.list_all().await.unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].p256dh, "k2");
}

async fn delete_removes_subscription(store: Box<dyn PushSubscriptionStore>) {
    store
        .upsert("https://push.example.com/del", "k", "a")
        .await
        .unwrap();
    store.delete("https://push.example.com/del").await.unwrap();
    let subs = store.list_all().await.unwrap();
    assert!(subs.is_empty());
}

async fn delete_missing_endpoint_is_ok(store: Box<dyn PushSubscriptionStore>) {
    store
        .delete("https://push.example.com/never-existed")
        .await
        .unwrap();
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_upsert_and_list() {
                upsert_and_list($ctor).await;
            }

            #[tokio::test]
            async fn t_upsert_replaces_existing() {
                upsert_replaces_existing($ctor).await;
            }

            #[tokio::test]
            async fn t_delete_removes_subscription() {
                delete_removes_subscription($ctor).await;
            }

            #[tokio::test]
            async fn t_delete_missing_endpoint_is_ok() {
                delete_missing_endpoint_is_ok($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite, sqlite_store().await);
contract_tests!(memory, memory_store());
