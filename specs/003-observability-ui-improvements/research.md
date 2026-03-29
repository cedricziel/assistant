# Research: Observability UI Improvements

## 1. Interface attribute storage

**Decision**: Extract `interface` from span `attributes` JSON in SQL.

**Finding**: In `crates/runtime/src/orchestrator/mod.rs:1003`, every turn span receives:

```rust
otel_turn.set_attribute(KeyValue::new("interface", format!("{interface:?}")));
```

Values observed in production: `"Slack"`, `"Scheduler"`, `"Cli"`.

The attribute lands in the `attributes` JSON column of `distributed_traces`.
`list_recent_traces_for_agent` aggregates with `GROUP BY trace_id`, so extraction is:

```sql
MAX(CASE WHEN dt.parent_span_id IS NULL
    THEN json_extract(dt.attributes, '$.interface')
    ELSE NULL END) AS interface
```

**Alternatives considered**: Adding a dedicated `interface` column to the table — rejected
because the attribute is already stored and a schema migration adds friction; JSON extraction
is fast enough for the UI query volume.

---

## 2. "Replied" detection

**Decision**: Add `has_reply` boolean to `TraceSummary` via SQL aggregate.

**Finding**: The reply tools are named `reply` and `slack-post`. The `tool_name` column in
`distributed_traces` holds this value. Within a `GROUP BY trace_id` aggregate:

```sql
SUM(CASE WHEN dt.tool_name IN ('reply', 'slack-post') THEN 1 ELSE 0 END) > 0 AS has_reply
```

SQLite returns `0`/`1`; map to `bool` in Rust.

**Alternatives considered**: Detecting "no reply" via a NOT EXISTS subquery — more complex
and no advantage since we need the positive case for display anyway.

---

## 3. Time range filtering

**Decision**: Add optional `since` / `until` `DateTime<Utc>` parameters to
`list_recent_traces_for_agent` and `list_recent_for_agent` in storage, wired to query params
`?since=` and `?until=` (ISO-8601 strings) in the web UI.

**Finding**: Both queries already have `ORDER BY trace_start DESC`. Adding:

```sql
AND (?N IS NULL OR dt.start_time >= ?N)
AND (?M IS NULL OR dt.start_time <= ?M)
```

is straightforward. The UI can render a pair of `<input type="datetime-local">` fields in
the sidebar using HTMX `hx-get` / `hx-push-url`.

**Alternatives considered**: Named time window shortcuts (1h, 6h, 24h) like the analytics
page — useful but can coexist; the datetime inputs are more precise for incident investigation.

---

## 4. Conversation ID filter visibility

**Decision**: Add an explicit `<input>` for `conversation_id` in both traces and logs sidebars.

**Finding**: `crates/web-ui/src/traces.rs:177` calls `list_recent_traces_for_agent` but
passes the `conversation` query param only to a sub-filter, not surfaced in the sidebar
template. The logs page has the same gap. Zero new storage or backend changes needed —
just add the input and wire it through the existing query-param handling.

---

## 5. Logs for conversation (cross-turn)

**Decision**: Add `list_recent_for_agent_by_conversation` to `LogStore` that joins via
`distributed_traces.conversation_id`, then add a `/logs?conversation=<uuid>` route that
uses it.

**Finding**: `list_recent_for_agent` in `storage/src/logs.rs:81` already does an EXISTS
subquery on `distributed_traces` to scope logs to an agent. Extending this to filter by
`conversation_id` is:

```sql
AND (?N IS NULL OR dt.conversation_id = ?N)
```

The trace detail page in `crates/web-ui/templates/traces/detail.html` already has access
to `conversation_id` via the trace summary; adding a "View conversation logs →" link costs
one template line.

---

## 6. Worker failure → otel error status

**Decision**: Set `Status::Error` on the root turn span in `orchestrator/mod.rs` when
`run_turn` returns `Err`.

**Finding**: In `crates/runtime/src/orchestrator/mod.rs`:

- `init_turn_context()` at line ~985 creates `otel_turn` and returns `turn_cx`.
- The orchestrator's main run function calls `run_turn(...)` inside the `turn_cx`.
- The worker (`worker.rs:328`) catches `Err(e)` but only logs a WARN; it does not have
  direct access to the `otel_turn` span by that point.

The fix is inside `orchestrator/mod.rs` before the turn result is returned — the turn span
is still in scope there. On `Err`, call:

```rust
otel_turn.set_status(opentelemetry::trace::Status::Error {
    description: Cow::Owned(e.to_string()),
});
```

This causes the SQLite exporter to write `otel.status_code = "Error"` into `attributes`,
which the web UI already reads via `error_count` in the aggregate.

**Alternatives considered**: Setting error in the worker after receiving `Err` — not
possible cleanly because the span is already ended when the async context drops after the
orchestrator returns.

---

## Summary of Changes by Crate

| Crate               | Changes                                                                                                                                                                                                |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `assistant-storage` | Add `interface` + `has_reply` to `TraceSummary`; extend `list_recent_traces_for_agent` with time range + interface filter; extend `list_recent_for_agent` (logs) with time range + conversation filter |
| `assistant-runtime` | Set `Status::Error` on turn span when turn returns `Err`                                                                                                                                               |
| `assistant-web-ui`  | Surface interface facet, replied badge, conversation input, time range picker, and conversation-logs link in traces + logs pages                                                                       |
