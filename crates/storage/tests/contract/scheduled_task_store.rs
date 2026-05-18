//! Contract tests for [`ScheduledTaskStore`].

use assistant_storage::{
    InMemoryScheduledTaskStore, ScheduledTaskStore, SqliteScheduledTaskStore, StorageLayer,
};
use chrono::{Duration, Utc};

async fn sqlite() -> Box<dyn ScheduledTaskStore> {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    Box::new(SqliteScheduledTaskStore::new(storage.pool))
}

fn memory() -> Box<dyn ScheduledTaskStore> {
    Box::new(InMemoryScheduledTaskStore::new())
}

async fn insert_and_list(store: Box<dyn ScheduledTaskStore>) {
    let next = Utc::now() + Duration::hours(1);
    store
        .insert("daily", "0 0 9 * * *", "do thing", false, Some(next))
        .await
        .unwrap();
    let tasks = store.list_all().await.unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].name, "daily");
    assert!(tasks[0].enabled);
}

async fn due_tasks_only_returns_overdue(store: Box<dyn ScheduledTaskStore>) {
    let past = Utc::now() - Duration::hours(1);
    let future = Utc::now() + Duration::hours(1);
    store
        .insert("overdue", "*", "p", false, Some(past))
        .await
        .unwrap();
    store
        .insert("future", "*", "p", false, Some(future))
        .await
        .unwrap();
    let due = store.due_tasks(Utc::now()).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].name, "overdue");
}

async fn disable_marks_disabled(store: Box<dyn ScheduledTaskStore>) {
    let id = store.insert("t", "*", "p", false, None).await.unwrap();
    assert!(store.disable(id).await.unwrap());
    let tasks = store.list_all().await.unwrap();
    assert!(!tasks[0].enabled);
    // Already disabled: second disable returns false.
    assert!(!store.disable(id).await.unwrap());
}

async fn delete_removes(store: Box<dyn ScheduledTaskStore>) {
    let id = store.insert("t", "*", "p", false, None).await.unwrap();
    assert!(store.delete(id).await.unwrap());
    assert!(store.list_all().await.unwrap().is_empty());
}

async fn find_by_name_case_insensitive(store: Box<dyn ScheduledTaskStore>) {
    store
        .insert("DailyReport", "*", "p", false, None)
        .await
        .unwrap();
    assert!(store.find_by_name("dailyreport").await.unwrap().is_some());
    assert!(store.find_by_name("DAILYREPORT").await.unwrap().is_some());
    assert!(store.find_by_name("other").await.unwrap().is_none());
}

async fn record_run_updates_timestamps(store: Box<dyn ScheduledTaskStore>) {
    let id = store.insert("t", "*", "p", false, None).await.unwrap();
    let now = Utc::now();
    let next = now + Duration::hours(1);
    store.record_run(id, now, Some(next)).await.unwrap();
    let tasks = store.list_all().await.unwrap();
    assert_eq!(tasks[0].last_run, Some(now));
    assert_eq!(tasks[0].next_run, Some(next));
}

macro_rules! contract_tests {
    ($name:ident, $ctor:expr) => {
        mod $name {
            use super::*;

            #[tokio::test]
            async fn t_insert_and_list() {
                insert_and_list($ctor).await;
            }

            #[tokio::test]
            async fn t_due_tasks_only_returns_overdue() {
                due_tasks_only_returns_overdue($ctor).await;
            }

            #[tokio::test]
            async fn t_disable_marks_disabled() {
                disable_marks_disabled($ctor).await;
            }

            #[tokio::test]
            async fn t_delete_removes() {
                delete_removes($ctor).await;
            }

            #[tokio::test]
            async fn t_find_by_name_case_insensitive() {
                find_by_name_case_insensitive($ctor).await;
            }

            #[tokio::test]
            async fn t_record_run_updates_timestamps() {
                record_run_updates_timestamps($ctor).await;
            }
        }
    };
}

contract_tests!(sqlite_impl, sqlite().await);
contract_tests!(memory_impl, memory());
