//! Contract tests for [`AgentStore`].

use assistant_storage::{
    AgentRecord, AgentStatus, AgentStore, InMemoryAgentStore, SqliteAgentStore, StorageLayer,
};

async fn sqlite() -> Box<dyn AgentStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteAgentStore::new(storage.pool))
}

fn memory() -> Box<dyn AgentStore> {
    Box::new(InMemoryAgentStore::new())
}

async fn create_and_get(store: Box<dyn AgentStore>) {
    store
        .create("a1", None, "parent-conv", "child-conv", "do thing", 1)
        .await
        .unwrap();
    let got: AgentRecord = store.get("a1").await.unwrap().unwrap();
    assert_eq!(got.id, "a1");
    assert_eq!(got.status, AgentStatus::Running);
    assert_eq!(got.depth, 1);
}

async fn complete_sets_status(store: Box<dyn AgentStore>) {
    store
        .create("a1", None, "parent-conv", "child-conv", "task", 0)
        .await
        .unwrap();
    store
        .complete("a1", AgentStatus::Completed, Some("done"))
        .await
        .unwrap();
    let got = store.get("a1").await.unwrap().unwrap();
    assert_eq!(got.status, AgentStatus::Completed);
    assert_eq!(got.result_summary.as_deref(), Some("done"));
    assert!(got.completed_at.is_some());
}

async fn list_by_parent_conversation(store: Box<dyn AgentStore>) {
    store
        .create("a1", None, "conv-A", "c1", "t", 0)
        .await
        .unwrap();
    store
        .create("a2", None, "conv-A", "c2", "t", 0)
        .await
        .unwrap();
    store
        .create("a3", None, "conv-B", "c3", "t", 0)
        .await
        .unwrap();
    let list = store.list_by_parent_conversation("conv-A").await.unwrap();
    assert_eq!(list.len(), 2);
    assert!(list.iter().all(|a| a.parent_conversation_id == "conv-A"));
}

async fn get_missing_returns_none(store: Box<dyn AgentStore>) {
    assert!(store.get("nope").await.unwrap().is_none());
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
            async fn t_complete_sets_status() {
                complete_sets_status($ctor).await;
            }

            #[tokio::test]
            async fn t_list_by_parent_conversation() {
                list_by_parent_conversation($ctor).await;
            }

            #[tokio::test]
            async fn t_get_missing_returns_none() {
                get_missing_returns_none($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
