//! Contract tests for [`ConversationSummaryStore`].
//!
//! The shared scenarios run against `SqliteConversationSummaryStore` and
//! `InMemoryConversationSummaryStore`; the cascade scenario only makes sense
//! for SQLite and sits below the macro.

use assistant_storage::{
    ConversationStore, ConversationSummaryStore, InMemoryConversationSummaryStore,
    SqliteConversationStore, StorageLayer,
};
use uuid::Uuid;

/// SQLite enforces the `conversations(id)` foreign key, so summaries need a
/// real conversation. The in-memory store accepts any id.
async fn sqlite_with_conversation() -> (Box<dyn ConversationSummaryStore>, Uuid) {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conv = SqliteConversationStore::new(storage.pool.clone())
        .create_conversation(None)
        .await
        .unwrap();
    (Box::new(storage.conversation_summary_store()), conv.id)
}

fn memory_with_conversation() -> (Box<dyn ConversationSummaryStore>, Uuid) {
    (
        Box::new(InMemoryConversationSummaryStore::new()),
        Uuid::new_v4(),
    )
}

async fn insert_returns_the_stored_summary(store: Box<dyn ConversationSummaryStore>, conv: Uuid) {
    let through = Uuid::new_v4();
    let stored = store
        .insert(conv, through, "earlier context")
        .await
        .unwrap();

    assert_eq!(stored.conversation_id, conv);
    assert_eq!(stored.through_message_id, through);
    assert_eq!(stored.content, "earlier context");

    let found = store.newest_on_path(conv, &[through]).await.unwrap();
    assert_eq!(found.map(|s| s.id), Some(stored.id), "round trip by path");
}

async fn newest_on_path_picks_the_newest_summary_on_the_path(
    store: Box<dyn ConversationSummaryStore>,
    conv: Uuid,
) {
    let (m2, m5, m8) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    store.insert(conv, m2, "first").await.unwrap();
    let second = store.insert(conv, m5, "second").await.unwrap();
    store.insert(conv, m8, "off the path").await.unwrap();

    let found = store
        .newest_on_path(conv, &[m2, m5, Uuid::new_v4()])
        .await
        .unwrap();

    assert_eq!(
        found.map(|s| s.id),
        Some(second.id),
        "the newest summary whose through message is on the path wins"
    );
}

async fn newest_on_path_is_none_for_a_diverged_path(
    store: Box<dyn ConversationSummaryStore>,
    conv: Uuid,
) {
    store.insert(conv, Uuid::new_v4(), "summary").await.unwrap();

    let found = store
        .newest_on_path(conv, &[Uuid::new_v4(), Uuid::new_v4()])
        .await
        .unwrap();

    assert!(found.is_none(), "no summary through a message off the path");
}

async fn newest_on_path_is_none_for_an_empty_path(
    store: Box<dyn ConversationSummaryStore>,
    conv: Uuid,
) {
    store.insert(conv, Uuid::new_v4(), "summary").await.unwrap();

    assert!(store.newest_on_path(conv, &[]).await.unwrap().is_none());
}

async fn summaries_are_scoped_to_their_conversation(
    store: Box<dyn ConversationSummaryStore>,
    conv: Uuid,
) {
    let through = Uuid::new_v4();
    store.insert(conv, through, "summary").await.unwrap();

    let found = store
        .newest_on_path(Uuid::new_v4(), &[through])
        .await
        .unwrap();

    assert!(found.is_none(), "another conversation sees nothing");
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_insert_returns_the_stored_summary() {
                let (store, conv) = $ctor;
                insert_returns_the_stored_summary(store, conv).await;
            }

            #[tokio::test]
            async fn t_newest_on_path_picks_the_newest_summary_on_the_path() {
                let (store, conv) = $ctor;
                newest_on_path_picks_the_newest_summary_on_the_path(store, conv).await;
            }

            #[tokio::test]
            async fn t_newest_on_path_is_none_for_a_diverged_path() {
                let (store, conv) = $ctor;
                newest_on_path_is_none_for_a_diverged_path(store, conv).await;
            }

            #[tokio::test]
            async fn t_newest_on_path_is_none_for_an_empty_path() {
                let (store, conv) = $ctor;
                newest_on_path_is_none_for_an_empty_path(store, conv).await;
            }

            #[tokio::test]
            async fn t_summaries_are_scoped_to_their_conversation() {
                let (store, conv) = $ctor;
                summaries_are_scoped_to_their_conversation(store, conv).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite_with_conversation().await);
contract_tests!(memory_impl, memory_with_conversation());

#[tokio::test]
async fn sqlite_conversation_delete_removes_summaries() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conversations = SqliteConversationStore::new(storage.pool.clone());
    let summaries = storage.conversation_summary_store();
    let conv = conversations.create_conversation(None).await.unwrap();
    let through = Uuid::new_v4();
    summaries.insert(conv.id, through, "summary").await.unwrap();

    conversations.delete_conversation(conv.id).await.unwrap();

    let found = summaries.newest_on_path(conv.id, &[through]).await.unwrap();
    assert!(found.is_none(), "cascade delete removes the summary rows");
}
