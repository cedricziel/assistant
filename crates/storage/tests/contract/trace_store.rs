//! Contract tests for the [`TraceStore`] trait.
//!
//! Each scenario inserts spans through the impl-specific write path
//! (SQLite via direct INSERT, InMemory via `insert()`) and then asserts
//! the trait read API returns consistent results.

use assistant_storage::{
    InMemoryTraceStore, RecordedSpan, SqliteTraceStore, StorageLayer, TraceStore,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::SqlitePool;

fn span(span_id: &str, trace_id: &str, tool: &str, start: DateTime<Utc>) -> RecordedSpan {
    RecordedSpan {
        span_id: span_id.into(),
        trace_id: trace_id.into(),
        parent_span_id: None,
        name: format!("span-{span_id}"),
        service_name: Some("test".into()),
        conversation_id: None,
        turn: Some(0),
        tool_name: Some(tool.into()),
        tool_status: Some("ok".into()),
        observation: None,
        error: None,
        duration_ms: 5,
        start_time: start,
        end_time: start + chrono::Duration::milliseconds(5),
        attributes: Value::Object(Default::default()),
        input_tokens: Some(10),
        output_tokens: Some(20),
    }
}

async fn insert_sqlite(pool: &SqlitePool, s: RecordedSpan) {
    let attrs = serde_json::to_string(&s.attributes).unwrap_or_else(|_| "{}".into());
    sqlx::query(
        "INSERT INTO distributed_traces \
            (span_id, trace_id, parent_span_id, name, conversation_id, turn, \
             service_name, tool_name, tool_status, tool_observation, tool_error, \
             duration_ms, start_time, end_time, attributes, input_tokens, output_tokens) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
    )
    .bind(&s.span_id)
    .bind(&s.trace_id)
    .bind(&s.parent_span_id)
    .bind(&s.name)
    .bind(s.conversation_id.map(|c| c.to_string()))
    .bind(s.turn)
    .bind(&s.service_name)
    .bind(&s.tool_name)
    .bind(&s.tool_status)
    .bind(&s.observation)
    .bind(&s.error)
    .bind(s.duration_ms)
    .bind(s.start_time)
    .bind(s.end_time)
    .bind(attrs)
    .bind(s.input_tokens)
    .bind(s.output_tokens)
    .execute(pool)
    .await
    .unwrap();
}

// -- SQLite scenarios ---------------------------------------------------------

#[tokio::test]
async fn sqlite_list_skills_collects_distinct_tools() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let pool = storage.pool.clone();
    let store = SqliteTraceStore::new(pool.clone());
    let t0 = Utc::now();
    insert_sqlite(&pool, span("a1", "trace-1", "echo", t0)).await;
    insert_sqlite(&pool, span("a2", "trace-1", "bash", t0)).await;
    insert_sqlite(
        &pool,
        span("a3", "trace-2", "echo", t0 + chrono::Duration::seconds(1)),
    )
    .await;
    let mut skills = store.list_skills().await.unwrap();
    skills.sort();
    assert_eq!(skills, vec!["bash".to_string(), "echo".to_string()]);
}

#[tokio::test]
async fn sqlite_get_trace_returns_spans_of_trace_id() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let pool = storage.pool.clone();
    let store = SqliteTraceStore::new(pool.clone());
    let t0 = Utc::now();
    insert_sqlite(&pool, span("a1", "trace-X", "foo", t0)).await;
    insert_sqlite(&pool, span("a2", "trace-X", "bar", t0)).await;
    insert_sqlite(&pool, span("a3", "trace-Y", "baz", t0)).await;
    let spans = store.get_trace("trace-X").await.unwrap();
    assert_eq!(spans.len(), 2);
    assert!(spans.iter().all(|s| s.trace_id == "trace-X"));
}

#[tokio::test]
async fn sqlite_list_recent_caps_to_limit() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let pool = storage.pool.clone();
    let store = SqliteTraceStore::new(pool.clone());
    let t0 = Utc::now();
    for i in 0..5 {
        insert_sqlite(
            &pool,
            span(
                &format!("s{i}"),
                &format!("t{i}"),
                "tool",
                t0 + chrono::Duration::seconds(i),
            ),
        )
        .await;
    }
    let recent = store.list_recent(3).await.unwrap();
    assert_eq!(recent.len(), 3);
}

#[tokio::test]
async fn sqlite_stats_for_skill_counts_total() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    let pool = storage.pool.clone();
    let store = SqliteTraceStore::new(pool.clone());
    let t0 = Utc::now();
    for i in 0..3 {
        insert_sqlite(&pool, span(&format!("a{i}"), &format!("t{i}"), "calc", t0)).await;
    }
    let stats = store.stats_for_skill("calc", 100).await.unwrap();
    assert_eq!(stats.total, 3);
}

// -- InMemory scenarios -------------------------------------------------------

#[tokio::test]
async fn memory_list_skills_collects_distinct_tools() {
    let store = InMemoryTraceStore::new();
    let t0 = Utc::now();
    store.insert(span("a1", "trace-1", "echo", t0));
    store.insert(span("a2", "trace-1", "bash", t0));
    store.insert(span("a3", "trace-2", "echo", t0));
    let mut skills = store.list_skills().await.unwrap();
    skills.sort();
    assert_eq!(skills, vec!["bash".to_string(), "echo".to_string()]);
}

#[tokio::test]
async fn memory_get_trace_returns_spans_of_trace_id() {
    let store = InMemoryTraceStore::new();
    let t0 = Utc::now();
    store.insert(span("a1", "trace-X", "foo", t0));
    store.insert(span("a2", "trace-X", "bar", t0));
    store.insert(span("a3", "trace-Y", "baz", t0));
    let spans = store.get_trace("trace-X").await.unwrap();
    assert_eq!(spans.len(), 2);
    assert!(spans.iter().all(|s| s.trace_id == "trace-X"));
}

#[tokio::test]
async fn memory_list_recent_caps_to_limit() {
    let store = InMemoryTraceStore::new();
    let t0 = Utc::now();
    for i in 0..5 {
        store.insert(span(
            &format!("s{i}"),
            &format!("t{i}"),
            "tool",
            t0 + chrono::Duration::seconds(i),
        ));
    }
    let recent = store.list_recent(3).await.unwrap();
    assert_eq!(recent.len(), 3);
}

#[tokio::test]
async fn memory_stats_for_skill_counts_total() {
    let store = InMemoryTraceStore::new();
    let t0 = Utc::now();
    for i in 0..3 {
        store.insert(span(&format!("a{i}"), &format!("t{i}"), "calc", t0));
    }
    let stats = store.stats_for_skill("calc", 100).await.unwrap();
    assert_eq!(stats.total, 3);
}
