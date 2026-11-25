//! Contract tests for the [`MessageFeedbackStore`] trait.
//!
//! Referential checks (cascade, unknown message id) apply to the SQLite
//! implementation only: the in-memory store cannot see the `messages` table.

use std::sync::Arc;
use std::time::Duration;

use assistant_core::clock::{Clock, FakeClock, SystemClock};
use assistant_core::types::conversation::Message;
use assistant_storage::{
    ConversationStore, InMemoryMessageFeedbackStore, MessageFeedbackRating, MessageFeedbackStore,
    SqliteMessageFeedbackStore, StorageLayer,
};
use uuid::Uuid;

/// A store plus a conversation holding two assistant messages.
struct Env {
    store: Box<dyn MessageFeedbackStore>,
    conv: Uuid,
    msg_a: Uuid,
    msg_b: Uuid,
}

async fn sqlite(clock: Arc<dyn Clock>) -> Env {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conversations = storage.conversation_store();
    let conv = conversations
        .create_conversation(Some("t"))
        .await
        .unwrap()
        .id;
    let a = Message::assistant(conv, "first");
    let b = Message::assistant(conv, "second");
    conversations.save_message(&a).await.unwrap();
    conversations.save_message(&b).await.unwrap();
    Env {
        store: Box::new(SqliteMessageFeedbackStore::new(storage.pool.clone()).with_clock(clock)),
        conv,
        msg_a: a.id,
        msg_b: b.id,
    }
}

async fn memory(clock: Arc<dyn Clock>) -> Env {
    Env {
        store: Box::new(InMemoryMessageFeedbackStore::new().with_clock(clock)),
        conv: Uuid::new_v4(),
        msg_a: Uuid::new_v4(),
        msg_b: Uuid::new_v4(),
    }
}

async fn set_then_get(env: Env) {
    let set = env
        .store
        .set(
            env.conv,
            env.msg_a,
            "u1",
            MessageFeedbackRating::Negative,
            Some("wrong date"),
        )
        .await
        .unwrap();
    assert_eq!(set.rating, MessageFeedbackRating::Negative);
    assert_eq!(set.comment.as_deref(), Some("wrong date"));
    let got = env.store.get(env.msg_a, "u1").await.unwrap().unwrap();
    assert_eq!(got, set, "get returns what set stored");
    assert!(env.store.get(env.msg_a, "u2").await.unwrap().is_none());
    assert!(env.store.get(env.msg_b, "u1").await.unwrap().is_none());
}

async fn set_twice_keeps_created_at(clock: Arc<FakeClock>, env: Env) {
    let first = env
        .store
        .set(
            env.conv,
            env.msg_a,
            "u1",
            MessageFeedbackRating::Positive,
            None,
        )
        .await
        .unwrap();
    clock.advance(Duration::from_secs(60));
    let second = env
        .store
        .set(
            env.conv,
            env.msg_a,
            "u1",
            MessageFeedbackRating::Negative,
            Some("nope"),
        )
        .await
        .unwrap();
    assert_eq!(second.created_at, first.created_at, "created_at is kept");
    assert!(second.updated_at > first.updated_at, "updated_at advances");
    assert_eq!(second.rating, MessageFeedbackRating::Negative);
    assert_eq!(second.comment.as_deref(), Some("nope"));
    let listed = env
        .store
        .list_for_conversation(env.conv, "u1")
        .await
        .unwrap();
    assert_eq!(listed.len(), 1, "exactly one row per (message, user)");
    let replaced = env
        .store
        .set(
            env.conv,
            env.msg_a,
            "u1",
            MessageFeedbackRating::Negative,
            None,
        )
        .await
        .unwrap();
    assert_eq!(replaced.comment, None, "set is a full replace");
}

async fn concurrent_sets_keep_first_created_at(env: Env) {
    let (a, b) = tokio::join!(
        env.store.set(
            env.conv,
            env.msg_a,
            "u1",
            MessageFeedbackRating::Positive,
            None
        ),
        env.store.set(
            env.conv,
            env.msg_a,
            "u1",
            MessageFeedbackRating::Negative,
            None
        ),
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    let stored = env.store.get(env.msg_a, "u1").await.unwrap().unwrap();
    assert_eq!(a.created_at, stored.created_at);
    assert_eq!(b.created_at, stored.created_at);
    let listed = env
        .store
        .list_for_conversation(env.conv, "u1")
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
}

async fn clear_reports_removal(env: Env) {
    env.store
        .set(
            env.conv,
            env.msg_a,
            "u1",
            MessageFeedbackRating::Positive,
            None,
        )
        .await
        .unwrap();
    assert!(env.store.clear(env.msg_a, "u1").await.unwrap());
    assert!(env.store.get(env.msg_a, "u1").await.unwrap().is_none());
    assert!(!env.store.clear(env.msg_a, "u1").await.unwrap());
}

async fn list_is_per_user_and_per_conversation(env: Env) {
    for (msg, user, rating) in [
        (env.msg_a, "u1", MessageFeedbackRating::Positive),
        (env.msg_b, "u1", MessageFeedbackRating::Negative),
        (env.msg_a, "u2", MessageFeedbackRating::Negative),
    ] {
        env.store
            .set(env.conv, msg, user, rating, None)
            .await
            .unwrap();
    }
    let mine = env
        .store
        .list_for_conversation(env.conv, "u1")
        .await
        .unwrap();
    assert_eq!(mine.len(), 2);
    assert!(mine.iter().all(|r| r.user_id == "u1"));
    let theirs = env
        .store
        .list_for_conversation(env.conv, "u2")
        .await
        .unwrap();
    assert_eq!(theirs.len(), 1);
    assert_eq!(theirs[0].message_id, env.msg_a);
    let other_conv = env
        .store
        .list_for_conversation(Uuid::new_v4(), "u1")
        .await
        .unwrap();
    assert!(other_conv.is_empty());
}

macro_rules! contract_tests {
    ($name:ident, $ctor:ident) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_set_then_get() {
                set_then_get($ctor(Arc::new(SystemClock)).await).await;
            }

            #[tokio::test]
            async fn t_set_twice_keeps_created_at() {
                let clock = Arc::new(FakeClock::new_at_seed());
                set_twice_keeps_created_at(clock.clone(), $ctor(clock).await).await;
            }

            #[tokio::test]
            async fn t_concurrent_sets_keep_first_created_at() {
                concurrent_sets_keep_first_created_at($ctor(Arc::new(SystemClock)).await).await;
            }

            #[tokio::test]
            async fn t_clear_reports_removal() {
                clear_reports_removal($ctor(Arc::new(SystemClock)).await).await;
            }

            #[tokio::test]
            async fn t_list_is_per_user_and_per_conversation() {
                list_is_per_user_and_per_conversation($ctor(Arc::new(SystemClock)).await).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite);
contract_tests!(memory_impl, memory);

#[tokio::test]
async fn sqlite_conversation_delete_cascades() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let conversations = storage.conversation_store();
    let conv = conversations
        .create_conversation(Some("t"))
        .await
        .unwrap()
        .id;
    let msg = Message::assistant(conv, "answer");
    conversations.save_message(&msg).await.unwrap();
    let store = SqliteMessageFeedbackStore::new(storage.pool.clone());
    store
        .set(conv, msg.id, "u1", MessageFeedbackRating::Positive, None)
        .await
        .unwrap();

    conversations.delete_conversation(conv).await.unwrap();

    assert!(store.get(msg.id, "u1").await.unwrap().is_none());
}

#[tokio::test]
async fn sqlite_set_on_unknown_message_errors() {
    let env = sqlite(Arc::new(SystemClock)).await;
    let result = env
        .store
        .set(
            env.conv,
            Uuid::new_v4(),
            "u1",
            MessageFeedbackRating::Positive,
            None,
        )
        .await;
    assert!(result.is_err(), "unknown message id must be rejected");
}

#[tokio::test]
async fn sqlite_set_on_message_of_other_conversation_errors() {
    let env = sqlite(Arc::new(SystemClock)).await;
    let result = env
        .store
        .set(
            Uuid::new_v4(),
            env.msg_a,
            "u1",
            MessageFeedbackRating::Positive,
            None,
        )
        .await;
    assert!(result.is_err(), "message must belong to the conversation");
}
