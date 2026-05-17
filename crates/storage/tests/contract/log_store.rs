//! Contract tests for the [`LogStore`] trait.

use assistant_storage::{InMemoryLogStore, LogStore, RecordedLog, SqliteLogStore, StorageLayer};
use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;

fn log_record(id: &str, target: &str, severity: i32, body: &str) -> RecordedLog {
    RecordedLog {
        id: id.into(),
        timestamp: Utc::now(),
        observed_timestamp: None,
        severity_number: Some(severity),
        severity_text: Some("INFO".into()),
        body: Some(body.into()),
        trace_id: None,
        span_id: None,
        span_name: None,
        service_name: Some("test".into()),
        target: Some(target.into()),
        attributes: Value::Object(Default::default()),
    }
}

async fn insert_sqlite(pool: &SqlitePool, log: RecordedLog) {
    let attrs = serde_json::to_string(&log.attributes).unwrap_or_else(|_| "{}".into());
    sqlx::query(
        "INSERT INTO logs \
            (id, timestamp, observed_timestamp, severity_number, severity_text, body, \
             trace_id, span_id, service_name, target, attributes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )
    .bind(&log.id)
    .bind(log.timestamp)
    .bind(log.observed_timestamp)
    .bind(log.severity_number)
    .bind(&log.severity_text)
    .bind(&log.body)
    .bind(&log.trace_id)
    .bind(&log.span_id)
    .bind(&log.service_name)
    .bind(&log.target)
    .bind(attrs)
    .execute(pool)
    .await
    .unwrap();
}

// -- SQLite --

#[tokio::test]
async fn sqlite_list_recent_caps_to_limit() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let pool = storage.pool.clone();
    let store = SqliteLogStore::new(pool.clone());
    for i in 0..5 {
        insert_sqlite(&pool, log_record(&format!("l{i}"), "app", 9, "hello")).await;
    }
    let recent = store.list_recent(3, None, None, None, None).await.unwrap();
    assert_eq!(recent.len(), 3);
}

#[tokio::test]
async fn sqlite_get_log_by_id() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let pool = storage.pool.clone();
    let store = SqliteLogStore::new(pool.clone());
    insert_sqlite(&pool, log_record("target-id", "app", 9, "find me")).await;
    let got = store.get_log("target-id").await.unwrap().unwrap();
    assert_eq!(got.body.as_deref(), Some("find me"));
}

#[tokio::test]
async fn sqlite_list_targets_distinct() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let pool = storage.pool.clone();
    let store = SqliteLogStore::new(pool.clone());
    insert_sqlite(&pool, log_record("a", "x", 9, "")).await;
    insert_sqlite(&pool, log_record("b", "y", 9, "")).await;
    insert_sqlite(&pool, log_record("c", "x", 9, "")).await;
    let mut targets = store.list_targets().await.unwrap();
    targets.sort();
    assert_eq!(targets, vec!["x".to_string(), "y".to_string()]);
}

// -- InMemory --

#[tokio::test]
async fn memory_list_recent_caps_to_limit() {
    let store = InMemoryLogStore::new();
    for i in 0..5 {
        store.insert(log_record(&format!("l{i}"), "app", 9, "hello"));
    }
    let recent = store.list_recent(3, None, None, None, None).await.unwrap();
    assert_eq!(recent.len(), 3);
}

#[tokio::test]
async fn memory_get_log_by_id() {
    let store = InMemoryLogStore::new();
    store.insert(log_record("target-id", "app", 9, "find me"));
    let got = store.get_log("target-id").await.unwrap().unwrap();
    assert_eq!(got.body.as_deref(), Some("find me"));
}

#[tokio::test]
async fn memory_list_targets_distinct() {
    let store = InMemoryLogStore::new();
    store.insert(log_record("a", "x", 9, ""));
    store.insert(log_record("b", "y", 9, ""));
    store.insert(log_record("c", "x", 9, ""));
    let mut targets = store.list_targets().await.unwrap();
    targets.sort();
    assert_eq!(targets, vec!["x".to_string(), "y".to_string()]);
}

#[tokio::test]
async fn memory_filter_by_min_severity() {
    let store = InMemoryLogStore::new();
    store.insert(log_record("a", "app", 9, "info"));
    store.insert(log_record("b", "app", 17, "error"));
    let errs = store
        .list_recent(10, Some(13), None, None, None)
        .await
        .unwrap();
    assert_eq!(errs.len(), 1);
    assert_eq!(errs[0].id, "b");
}
