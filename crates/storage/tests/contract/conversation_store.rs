//! Contract tests for the [`ConversationStore`] trait.
//!
//! The same test scenarios run against every implementation
//! (`SqliteConversationStore` + `InMemoryConversationStore`).  Any
//! observable divergence between impls fails CI.

use assistant_core::types::conversation::{Message, MessageRole};
use assistant_storage::{
    ConversationStore, InMemoryConversationStore, SqliteConversationStore, StorageLayer,
};
use uuid::Uuid;

/// Construct a SQLite-backed store for tests.
async fn sqlite_store() -> Box<dyn ConversationStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteConversationStore::new(storage.pool.clone()))
}

/// Construct an in-memory store for tests.
fn memory_store() -> Box<dyn ConversationStore> {
    Box::new(InMemoryConversationStore::new())
}

fn user_msg(conversation_id: Uuid, body: &str, turn: i64) -> Message {
    let mut m = Message::user(conversation_id, body);
    m.turn = turn;
    m
}

fn assistant_msg(conversation_id: Uuid, body: &str, turn: i64) -> Message {
    let mut m = Message::assistant(conversation_id, body);
    m.turn = turn;
    m
}

async fn create_and_get_roundtrip(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("Hello")).await.unwrap();
    assert_eq!(conv.title.as_deref(), Some("Hello"));
    assert!(conv.title_locked, "explicit title must lock");

    let loaded = store.get_conversation(conv.id).await.unwrap();
    let loaded = loaded.expect("must round-trip");
    assert_eq!(loaded.id, conv.id);
    assert_eq!(loaded.title.as_deref(), Some("Hello"));
}

async fn create_untitled_does_not_lock(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    assert!(conv.title.is_none());
    assert!(!conv.title_locked, "untitled conversations stay unlocked");
}

async fn create_with_id_idempotent(store: Box<dyn ConversationStore>) {
    let id = Uuid::new_v4();
    let first = store
        .create_conversation_with_id(id, Some("First"))
        .await
        .unwrap();
    let second = store
        .create_conversation_with_id(id, Some("Second"))
        .await
        .unwrap();
    assert_eq!(
        first.id, second.id,
        "create_with_id is idempotent for the same UUID"
    );
    // The contract is: if a row already exists, return it unchanged.
    assert_eq!(
        second.title.as_deref(),
        Some("First"),
        "existing row's title survives a second create_with_id"
    );
}

async fn list_orders_by_recency(store: Box<dyn ConversationStore>) {
    let a = store.create_conversation(Some("A")).await.unwrap();
    let b = store.create_conversation(Some("B")).await.unwrap();
    let _c = store.create_conversation(Some("C")).await.unwrap();
    // Touch `a` last so it becomes the most-recent.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    store.update_title(a.id, "A-updated").await.unwrap();

    let list = store.list_conversations().await.unwrap();
    assert!(!list.is_empty());
    // The first element should be `a` (just-updated).
    assert_eq!(list[0].id, a.id);
    let _ = b;
}

async fn update_title_locks(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(None).await.unwrap();
    store.update_title(conv.id, "renamed").await.unwrap();
    let loaded = store.get_conversation(conv.id).await.unwrap().unwrap();
    assert_eq!(loaded.title.as_deref(), Some("renamed"));
    assert!(loaded.title_locked, "explicit update_title must lock");
}

async fn save_and_load_history(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("History")).await.unwrap();
    let u = user_msg(conv.id, "hi", 0);
    let a = assistant_msg(conv.id, "hello!", 0);
    store.save_message(&u).await.unwrap();
    store.save_message(&a).await.unwrap();

    let history = store.load_history(conv.id).await.unwrap();
    assert_eq!(history.len(), 2);
    assert!(matches!(history[0].role, MessageRole::User));
    assert!(matches!(history[1].role, MessageRole::Assistant));
    assert_eq!(history[0].content, "hi");
    assert_eq!(history[1].content, "hello!");
}

async fn get_message_by_id(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("M")).await.unwrap();
    let m = user_msg(conv.id, "find me", 0);
    let id = m.id;
    store.save_message(&m).await.unwrap();

    let got = store.get_message(id).await.unwrap();
    let got = got.expect("must find the saved message by id");
    assert_eq!(got.content, "find me");

    let missing = store.get_message(Uuid::new_v4()).await.unwrap();
    assert!(missing.is_none(), "unknown id returns None");
}

async fn last_messages_caps_to_limit(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("L")).await.unwrap();
    for i in 0..5 {
        store
            .save_message(&user_msg(conv.id, &format!("m{i}"), i))
            .await
            .unwrap();
    }
    let last3 = store.last_messages(conv.id, 3).await.unwrap();
    assert_eq!(last3.len(), 3);
    assert_eq!(last3[0].content, "m2");
    assert_eq!(last3[2].content, "m4");
}

async fn delete_conversation_removes_history(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("D")).await.unwrap();
    store
        .save_message(&user_msg(conv.id, "hello", 0))
        .await
        .unwrap();
    store.delete_conversation(conv.id).await.unwrap();
    let gone = store.get_conversation(conv.id).await.unwrap();
    assert!(gone.is_none(), "conversation is gone after delete");
    let history = store.load_history(conv.id).await.unwrap();
    assert!(history.is_empty(), "history is cascaded on delete");
}

async fn replace_history_overwrites(store: Box<dyn ConversationStore>) {
    let conv = store.create_conversation(Some("R")).await.unwrap();
    store
        .save_message(&user_msg(conv.id, "old", 0))
        .await
        .unwrap();
    let replacement = vec![
        user_msg(conv.id, "new1", 0),
        assistant_msg(conv.id, "new2", 0),
    ];
    store.replace_history(conv.id, &replacement).await.unwrap();
    let loaded = store.load_history(conv.id).await.unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].content, "new1");
    assert_eq!(loaded[1].content, "new2");
}

macro_rules! contract_tests {
    ($store_name:ident, $constructor:expr) => {
        mod $store_name {
            use super::*;

            #[tokio::test]
            async fn t_create_and_get_roundtrip() {
                create_and_get_roundtrip($constructor).await;
            }

            #[tokio::test]
            async fn t_create_untitled_does_not_lock() {
                create_untitled_does_not_lock($constructor).await;
            }

            #[tokio::test]
            async fn t_create_with_id_idempotent() {
                create_with_id_idempotent($constructor).await;
            }

            #[tokio::test]
            async fn t_list_orders_by_recency() {
                list_orders_by_recency($constructor).await;
            }

            #[tokio::test]
            async fn t_update_title_locks() {
                update_title_locks($constructor).await;
            }

            #[tokio::test]
            async fn t_save_and_load_history() {
                save_and_load_history($constructor).await;
            }

            #[tokio::test]
            async fn t_get_message_by_id() {
                get_message_by_id($constructor).await;
            }

            #[tokio::test]
            async fn t_last_messages_caps_to_limit() {
                last_messages_caps_to_limit($constructor).await;
            }

            #[tokio::test]
            async fn t_delete_conversation_removes_history() {
                delete_conversation_removes_history($constructor).await;
            }

            #[tokio::test]
            async fn t_replace_history_overwrites() {
                replace_history_overwrites($constructor).await;
            }
        }
    };
}

contract_tests!(sqlite, sqlite_store().await);
contract_tests!(memory, memory_store());
