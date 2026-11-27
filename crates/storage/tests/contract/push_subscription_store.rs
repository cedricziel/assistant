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
        .upsert("https://push.example.com/abc", "p256", "auth", None)
        .await
        .unwrap();
    let subs = store.list_all().await.unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].endpoint, "https://push.example.com/abc");
}

async fn upsert_replaces_existing(store: Box<dyn PushSubscriptionStore>) {
    store
        .upsert("https://push.example.com/dup", "k1", "a1", None)
        .await
        .unwrap();
    store
        .upsert("https://push.example.com/dup", "k2", "a2", None)
        .await
        .unwrap();
    let subs = store.list_all().await.unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].p256dh, "k2");
}

async fn delete_removes_subscription(store: Box<dyn PushSubscriptionStore>) {
    store
        .upsert("https://push.example.com/del", "k", "a", None)
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

async fn list_for_user_returns_only_that_users_subscriptions(
    store: Box<dyn PushSubscriptionStore>,
) {
    store
        .upsert("https://push.example.com/alice-1", "k", "a", Some("alice"))
        .await
        .unwrap();
    store
        .upsert("https://push.example.com/alice-2", "k", "a", Some("alice"))
        .await
        .unwrap();
    store
        .upsert("https://push.example.com/bob", "k", "a", Some("bob"))
        .await
        .unwrap();
    store
        .upsert("https://push.example.com/legacy", "k", "a", None)
        .await
        .unwrap();

    let alice = store.list_for_user("alice").await.unwrap();
    let endpoints: Vec<&str> = alice.iter().map(|s| s.endpoint.as_str()).collect();
    assert_eq!(
        endpoints,
        [
            "https://push.example.com/alice-1",
            "https://push.example.com/alice-2"
        ]
    );
    assert_eq!(alice[0].user_id.as_deref(), Some("alice"));
    assert!(store.list_for_user("carol").await.unwrap().is_empty());
    // Rows without a user still reach the broadcast.
    assert_eq!(store.list_all().await.unwrap().len(), 4);
}

async fn resubscribing_moves_the_endpoint_to_the_new_user(store: Box<dyn PushSubscriptionStore>) {
    let endpoint = "https://push.example.com/shared-browser";
    store
        .upsert(endpoint, "k", "a", Some("alice"))
        .await
        .unwrap();
    store.upsert(endpoint, "k", "a", Some("bob")).await.unwrap();

    assert!(store.list_for_user("alice").await.unwrap().is_empty());
    assert_eq!(store.list_for_user("bob").await.unwrap().len(), 1);
}

async fn get_by_endpoint_returns_the_current_owner(store: Box<dyn PushSubscriptionStore>) {
    store
        .upsert("https://push.example.com/get", "k", "a", Some("alice"))
        .await
        .unwrap();

    let sub = store
        .get_by_endpoint("https://push.example.com/get")
        .await
        .unwrap()
        .expect("the stored row");
    assert_eq!(sub.user_id.as_deref(), Some("alice"));

    // A rebind is visible immediately: this is what send-time rechecks read.
    store
        .upsert("https://push.example.com/get", "k", "a", Some("bob"))
        .await
        .unwrap();
    let sub = store
        .get_by_endpoint("https://push.example.com/get")
        .await
        .unwrap()
        .expect("the rebound row");
    assert_eq!(sub.user_id.as_deref(), Some("bob"));

    assert!(
        store
            .get_by_endpoint("https://push.example.com/missing")
            .await
            .unwrap()
            .is_none()
    );
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
            async fn t_list_for_user_returns_only_that_users_subscriptions() {
                list_for_user_returns_only_that_users_subscriptions($ctor).await;
            }

            #[tokio::test]
            async fn t_resubscribing_moves_the_endpoint_to_the_new_user() {
                resubscribing_moves_the_endpoint_to_the_new_user($ctor).await;
            }

            #[tokio::test]
            async fn t_get_by_endpoint_returns_the_current_owner() {
                get_by_endpoint_returns_the_current_owner($ctor).await;
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
