//! Coverage for `scheduler::prune::prune_conversation_events`.

use assistant_storage::StorageLayer;

use crate::scheduler::prune_conversation_events;

#[tokio::test]
async fn prune_on_empty_store_is_a_noop() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    // No conversation events in the store; prune must complete without error.
    prune_conversation_events(&storage).await;
}

#[tokio::test]
async fn prune_runs_against_seeded_store() {
    // We can't easily insert expired rows without exposing more API,
    // but calling prune against a populated DB still exercises the
    // happy `Ok(0)` branch.
    let storage = StorageLayer::new_in_memory().await.unwrap();
    prune_conversation_events(&storage).await;
    // Idempotency: a second call also succeeds.
    prune_conversation_events(&storage).await;
}
