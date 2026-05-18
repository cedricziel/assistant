//! Contract tests for [`RefinementsStore`].

use assistant_storage::{
    InMemoryRefinementsStore, RefinementStatus, RefinementsStore, SqliteRefinementsStore,
    StorageLayer,
};

async fn sqlite() -> Box<dyn RefinementsStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteRefinementsStore::new(storage.pool))
}

fn memory() -> Box<dyn RefinementsStore> {
    Box::new(InMemoryRefinementsStore::new())
}

async fn insert_creates_pending(store: Box<dyn RefinementsStore>) {
    let _id = store
        .insert("skill-x", "## proposal", "rationale")
        .await
        .unwrap();
    let pending = store
        .list_by_status(&RefinementStatus::Pending)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].target_skill, "skill-x");
    assert_eq!(pending[0].status, RefinementStatus::Pending);
}

async fn review_accepts(store: Box<dyn RefinementsStore>) {
    let id = store
        .insert("skill-x", "## proposal", "rationale")
        .await
        .unwrap();
    store.review(id, true, Some("looks good")).await.unwrap();
    let accepted = store
        .list_by_status(&RefinementStatus::Accepted)
        .await
        .unwrap();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].id, id);
    assert_eq!(accepted[0].review_note.as_deref(), Some("looks good"));
}

async fn review_rejects(store: Box<dyn RefinementsStore>) {
    let id = store
        .insert("skill-x", "## proposal", "rationale")
        .await
        .unwrap();
    store.review(id, false, Some("no thanks")).await.unwrap();
    let rejected = store
        .list_by_status(&RefinementStatus::Rejected)
        .await
        .unwrap();
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].id, id);
}

async fn insert_with_previous_is_auto_accepted(store: Box<dyn RefinementsStore>) {
    let id = store
        .insert_with_previous("skill-x", "## new", "rationale", "## old")
        .await
        .unwrap();
    let accepted = store
        .list_by_status(&RefinementStatus::Accepted)
        .await
        .unwrap();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].id, id);
    assert_eq!(accepted[0].previous_skill_md.as_deref(), Some("## old"));
}

async fn set_status_transitions(store: Box<dyn RefinementsStore>) {
    let id = store
        .insert("skill-x", "## proposal", "rationale")
        .await
        .unwrap();
    store
        .set_status(id, &RefinementStatus::Confirmed)
        .await
        .unwrap();
    let confirmed = store
        .list_by_status(&RefinementStatus::Confirmed)
        .await
        .unwrap();
    assert_eq!(confirmed.len(), 1);
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_insert_creates_pending() {
                insert_creates_pending($ctor).await;
            }

            #[tokio::test]
            async fn t_review_accepts() {
                review_accepts($ctor).await;
            }

            #[tokio::test]
            async fn t_review_rejects() {
                review_rejects($ctor).await;
            }

            #[tokio::test]
            async fn t_insert_with_previous_is_auto_accepted() {
                insert_with_previous_is_auto_accepted($ctor).await;
            }

            #[tokio::test]
            async fn t_set_status_transitions() {
                set_status_transitions($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
