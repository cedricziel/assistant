//! Contract tests for the [`WebhookStore`] trait.

use assistant_storage::{InMemoryWebhookStore, SqliteWebhookStore, StorageLayer, WebhookStore};

async fn sqlite() -> Box<dyn WebhookStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteWebhookStore::new(storage.pool))
}

fn memory() -> Box<dyn WebhookStore> {
    Box::new(InMemoryWebhookStore::new())
}

async fn create_and_get(store: Box<dyn WebhookStore>) {
    store
        .create(
            "wh1",
            "test",
            "https://example.com",
            "secret",
            &["a".to_string()],
        )
        .await
        .unwrap();
    let got = store.get("wh1").await.unwrap().unwrap();
    assert_eq!(got.id, "wh1");
    assert_eq!(got.name, "test");
    assert_eq!(got.url, "https://example.com");
    assert_eq!(got.event_types, vec!["a".to_string()]);
    assert!(got.active);
    assert!(got.verified_at.is_none());
}

async fn list_orders_by_recency(store: Box<dyn WebhookStore>) {
    store
        .create("wh1", "a", "https://example.com/a", "s", &[])
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store
        .create("wh2", "b", "https://example.com/b", "s", &[])
        .await
        .unwrap();
    let list = store.list().await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "wh2");
    assert_eq!(list[1].id, "wh1");
}

async fn update_fields(store: Box<dyn WebhookStore>) {
    store
        .create("wh1", "old", "https://example.com", "s", &[])
        .await
        .unwrap();
    let ok = store
        .update(
            "wh1",
            "new",
            "https://example.com/new",
            &["x".to_string()],
            false,
        )
        .await
        .unwrap();
    assert!(ok);
    let got = store.get("wh1").await.unwrap().unwrap();
    assert_eq!(got.name, "new");
    assert_eq!(got.url, "https://example.com/new");
    assert!(!got.active);
}

async fn mark_verified_clears_after_rotate(store: Box<dyn WebhookStore>) {
    store
        .create("wh1", "t", "https://example.com", "old", &[])
        .await
        .unwrap();
    assert!(store.mark_verified("wh1").await.unwrap());
    assert!(
        store
            .get("wh1")
            .await
            .unwrap()
            .unwrap()
            .verified_at
            .is_some()
    );
    assert!(store.rotate_secret("wh1", "new").await.unwrap());
    assert!(
        store
            .get("wh1")
            .await
            .unwrap()
            .unwrap()
            .verified_at
            .is_none()
    );
    assert_eq!(store.get("wh1").await.unwrap().unwrap().secret, "new");
}

async fn toggle_active(store: Box<dyn WebhookStore>) {
    store
        .create("wh1", "t", "https://example.com", "s", &[])
        .await
        .unwrap();
    assert!(store.get("wh1").await.unwrap().unwrap().active);
    store.toggle_active("wh1").await.unwrap();
    assert!(!store.get("wh1").await.unwrap().unwrap().active);
    store.toggle_active("wh1").await.unwrap();
    assert!(store.get("wh1").await.unwrap().unwrap().active);
}

async fn delete_removes(store: Box<dyn WebhookStore>) {
    store
        .create("wh1", "t", "https://example.com", "s", &[])
        .await
        .unwrap();
    assert!(store.delete("wh1").await.unwrap());
    assert!(store.get("wh1").await.unwrap().is_none());
    assert!(!store.delete("wh1").await.unwrap());
}

async fn list_active_for_event_filters(store: Box<dyn WebhookStore>) {
    store
        .create(
            "wh1",
            "active",
            "https://example.com",
            "s",
            &["turn.completed".to_string()],
        )
        .await
        .unwrap();
    store
        .create(
            "wh2",
            "other-event",
            "https://example.com",
            "s",
            &["other".to_string()],
        )
        .await
        .unwrap();
    let matches = store.list_active_for_event("turn.completed").await.unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].id, "wh1");
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_create_and_get() {
                create_and_get($ctor).await;
            }

            #[tokio::test]
            async fn t_list_orders_by_recency() {
                list_orders_by_recency($ctor).await;
            }

            #[tokio::test]
            async fn t_update_fields() {
                update_fields($ctor).await;
            }

            #[tokio::test]
            async fn t_mark_verified_clears_after_rotate() {
                mark_verified_clears_after_rotate($ctor).await;
            }

            #[tokio::test]
            async fn t_toggle_active() {
                toggle_active($ctor).await;
            }

            #[tokio::test]
            async fn t_delete_removes() {
                delete_removes($ctor).await;
            }

            #[tokio::test]
            async fn t_list_active_for_event_filters() {
                list_active_for_event_filters($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
