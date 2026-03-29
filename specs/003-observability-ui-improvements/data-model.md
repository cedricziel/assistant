# Data Model: Observability UI Improvements

## No schema migrations required

All new data is derived from existing columns. No `ALTER TABLE` or new migrations needed.

---

## TraceSummary (storage crate — `crates/storage/src/traces.rs`)

Two new fields added to the existing struct:

```rust
pub struct TraceSummary {
    // --- existing fields ---
    pub trace_id: String,
    pub conversation_id: Option<Uuid>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub span_count: i64,
    pub tool_span_count: i64,
    pub error_count: i64,
    pub tool_names: Vec<String>,
    pub root_span_name: Option<String>,
    pub root_service_name: Option<String>,
    // --- new fields ---
    pub interface: Option<String>,  // "Slack" | "Scheduler" | "Cli" | None
    pub has_reply: bool,            // true if reply/slack-post tool was called
}
```

### SQL additions to `list_recent_traces_for_agent`

```sql
-- new SELECT columns
MAX(CASE WHEN dt.parent_span_id IS NULL
    THEN json_extract(dt.attributes, '$.interface')
    ELSE NULL END) AS interface,
SUM(CASE WHEN dt.tool_name IN ('reply', 'slack-post') THEN 1 ELSE 0 END) > 0
    AS has_reply,

-- new HAVING / WHERE clauses (added as optional filters)
AND (?since IS NULL OR dt.start_time >= ?since)
AND (?until IS NULL OR dt.start_time <= ?until)
AND (?interface IS NULL OR
    json_extract(dt.attributes, '$.interface') = ?interface)
```

---

## TraceFilter (new struct — `crates/web-ui/src/traces.rs` or `crates/storage/src/traces.rs`)

Centralises filter parameters to avoid growing function arity unboundedly:

```rust
pub struct TraceFilter {
    pub skill: Option<String>,
    pub status: Option<String>,      // "ok" | "error"
    pub conversation: Option<Uuid>,
    pub min_duration_ms: Option<i64>,
    pub interface: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
}
```

Currently `list_recent_traces_for_agent` takes `skill_filter: Option<&str>`. Replace with
`filter: &TraceFilter`. This is an internal API change only (no public crate boundary).

---

## LogStore — extended filter

`list_recent_for_agent` signature extension:

```rust
pub async fn list_recent_for_agent(
    &self,
    limit: i64,
    min_severity: Option<i32>,
    target_filter: Option<&str>,
    search: Option<&str>,
    trace_id: Option<&str>,
    conversation_id: Option<&str>,   // NEW — cross-turn conversation scope
    since: Option<DateTime<Utc>>,    // NEW
    until: Option<DateTime<Utc>>,    // NEW
    agent_id: &str,
) -> Result<Vec<RecordedLog>>
```

SQL additions:

```sql
AND (?conv IS NULL OR dt.conversation_id = ?conv)
AND (?since IS NULL OR l.timestamp >= ?since)
AND (?until IS NULL OR l.timestamp <= ?until)
```

---

## OTel turn span — error status

In `crates/runtime/src/orchestrator/mod.rs`, the `run_turn` result handling:

```rust
// BEFORE (no error status set on failure)
let result = run_turn(...).await;

// AFTER
let result = run_turn(...).await;
if let Err(ref e) = result {
    otel_turn.set_status(opentelemetry::trace::Status::Error {
        description: std::borrow::Cow::Owned(e.to_string()),
    });
}
```

The SQLite exporter writes `otel.status_code = "Error"` to `attributes`, which the existing
`error_count` aggregate already reads for the Status ✓/✗ column.

---

## UI query parameters (no breaking changes)

All new params are additive and optional:

| Page   | New param      | Type              | Example                       |
| ------ | -------------- | ----------------- | ----------------------------- |
| traces | `interface`    | string            | `?interface=Slack`            |
| traces | `since`        | ISO-8601 datetime | `?since=2026-03-27T07:00:00Z` |
| traces | `until`        | ISO-8601 datetime | `?until=2026-03-27T10:00:00Z` |
| logs   | `conversation` | UUID string       | `?conversation=06effb79...`   |
| logs   | `since`        | ISO-8601 datetime | `?since=2026-03-27T07:00:00Z` |
| logs   | `until`        | ISO-8601 datetime | `?until=2026-03-27T10:00:00Z` |

The existing `?conversation=` on traces and `?trace_id=` on logs are unchanged.
