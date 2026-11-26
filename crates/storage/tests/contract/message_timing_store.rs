//! Contract tests for [`MessageTimingStore`].
//!
//! The shared scenarios run against `SqliteMessageTimingStore` and
//! `InMemoryMessageTimingStore`; the cascade and compaction scenarios only
//! make sense for SQLite and sit below the macro.

use assistant_core::TurnTiming;
use assistant_core::types::conversation::Message;
use assistant_storage::{
    ConversationStore, InMemoryMessageTimingStore, MessageTimingStore, SqliteConversationStore,
    SqliteMessageTimingStore, StorageLayer,
};
use uuid::Uuid;

/// SQLite enforces the `conversations(id)` foreign key, so timings need a
/// real conversation. The in-memory store accepts any id.
async fn sqlite_with_conversation() -> (Box<dyn MessageTimingStore>, Uuid) {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conv = SqliteConversationStore::new(storage.pool.clone())
        .create_conversation(None)
        .await
        .unwrap();
    (Box::new(storage.message_timing_store()), conv.id)
}

fn memory_with_conversation() -> (Box<dyn MessageTimingStore>, Uuid) {
    (Box::new(InMemoryMessageTimingStore::new()), Uuid::new_v4())
}

fn full_timing() -> TurnTiming {
    TurnTiming {
        time_to_first_token_ms: Some(800),
        total_ms: 3000,
        output_tokens: Some(120),
        decode_ms: Some(2200),
    }
}

fn sparse_timing() -> TurnTiming {
    TurnTiming {
        time_to_first_token_ms: None,
        total_ms: 450,
        output_tokens: None,
        decode_ms: None,
    }
}

async fn round_trips_all_fields(store: Box<dyn MessageTimingStore>, conv: Uuid) {
    let (full_id, sparse_id) = (Uuid::new_v4(), Uuid::new_v4());
    store.put(conv, full_id, &full_timing()).await.unwrap();
    store.put(conv, sparse_id, &sparse_timing()).await.unwrap();

    let got = store.get_for_conversation(conv).await.unwrap();
    assert_eq!(got.len(), 2, "both timings are returned");
    assert_eq!(got.get(&full_id), Some(&full_timing()));
    assert_eq!(
        got.get(&sparse_id),
        Some(&sparse_timing()),
        "absent fields stay None"
    );
}

async fn empty_conversation_returns_empty_map(store: Box<dyn MessageTimingStore>, conv: Uuid) {
    let got = store.get_for_conversation(conv).await.unwrap();
    assert!(got.is_empty(), "no timings stored yet");
}

async fn put_twice_overwrites(store: Box<dyn MessageTimingStore>, conv: Uuid) {
    let id = Uuid::new_v4();
    store.put(conv, id, &full_timing()).await.unwrap();
    store.put(conv, id, &sparse_timing()).await.unwrap();

    let got = store.get_for_conversation(conv).await.unwrap();
    assert_eq!(got.len(), 1, "one row per message");
    assert_eq!(got.get(&id), Some(&sparse_timing()), "last write wins");
}

async fn timings_are_scoped_to_their_conversation(store: Box<dyn MessageTimingStore>, conv: Uuid) {
    store
        .put(conv, Uuid::new_v4(), &full_timing())
        .await
        .unwrap();

    let got = store.get_for_conversation(Uuid::new_v4()).await.unwrap();
    assert!(got.is_empty(), "another conversation sees nothing");
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_round_trips_all_fields() {
                let (store, conv) = $ctor;
                round_trips_all_fields(store, conv).await;
            }

            #[tokio::test]
            async fn t_empty_conversation_returns_empty_map() {
                let (store, conv) = $ctor;
                empty_conversation_returns_empty_map(store, conv).await;
            }

            #[tokio::test]
            async fn t_put_twice_overwrites() {
                let (store, conv) = $ctor;
                put_twice_overwrites(store, conv).await;
            }

            #[tokio::test]
            async fn t_timings_are_scoped_to_their_conversation() {
                let (store, conv) = $ctor;
                timings_are_scoped_to_their_conversation(store, conv).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite_with_conversation().await);
contract_tests!(memory_impl, memory_with_conversation());

#[tokio::test]
async fn sqlite_conversation_delete_removes_timings() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conversations = SqliteConversationStore::new(storage.pool.clone());
    let timings = SqliteMessageTimingStore::new(storage.pool.clone());
    let conv = conversations.create_conversation(None).await.unwrap();
    timings
        .put(conv.id, Uuid::new_v4(), &full_timing())
        .await
        .unwrap();

    conversations.delete_conversation(conv.id).await.unwrap();

    let got = timings.get_for_conversation(conv.id).await.unwrap();
    assert!(got.is_empty(), "cascade delete removes the timing rows");
}

#[tokio::test]
async fn sqlite_replace_history_keeps_timing_of_kept_message() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conversations = SqliteConversationStore::new(storage.pool.clone());
    let timings = SqliteMessageTimingStore::new(storage.pool.clone());
    let conv = conversations.create_conversation(None).await.unwrap();

    let question = Message::user(conv.id, "hi");
    let mut answer = Message::assistant(conv.id, "hello");
    answer.turn = 1;
    conversations.save_message(&question).await.unwrap();
    conversations.save_message(&answer).await.unwrap();
    timings
        .put(conv.id, answer.id, &full_timing())
        .await
        .unwrap();

    conversations
        .replace_history(conv.id, std::slice::from_ref(&answer))
        .await
        .unwrap();

    let got = timings.get_for_conversation(conv.id).await.unwrap();
    assert_eq!(
        got.get(&answer.id),
        Some(&full_timing()),
        "compaction rewrites messages but must not drop the timing"
    );
}
