# Implementation Plan: Observability UI Improvements

**Branch**: `003-observability-ui-improvements` | **Date**: 2026-03-29
**Spec**: `specs/003-observability-ui-improvements/spec.md`

## Summary

Six targeted improvements to `/traces` and `/logs` to make silent turn failures diagnosable:
propagate worker errors to OTel span status, surface interface/replied state in the trace list,
add visible conversation-ID and time-range filters, and cross-link trace details to
conversation-scoped logs.

## Technical Context

**Language/Version**: Rust 2021 edition, workspace resolver 2
**Primary Dependencies**: `axum` (HTTP), `askama` (templates), `sqlx` + SQLite (storage),
`htmx` + Stimulus.js (frontend interactivity), `opentelemetry` (tracing)
**Storage**: SQLite — `distributed_traces` + `logs` tables; no schema migrations needed
**Testing**: `cargo test`, `#[tokio::test]`, `StorageLayer::new_in_memory()`
**Target Platform**: Linux server (deployed as `schorschvm`)
**Project Type**: web-service (web-ui crate) + library (storage + runtime crates)
**Performance Goals**: UI queries remain under 200 ms at current data volumes
**Constraints**: No breaking changes to public storage API; all query params additive/optional
**Scale/Scope**: Single-agent SQLite; no pagination needed yet

## Constitution Check

| Principle                             | Status | Notes                                                        |
| ------------------------------------- | ------ | ------------------------------------------------------------ |
| I. Crate-First Modularity             | PASS   | Changes span 3 existing crates with clear ownership          |
| II. Trait-Based DI                    | PASS   | No new cross-crate concrete types introduced                 |
| III. Test Discipline                  | PASS   | New query paths covered by in-memory SQLite unit tests       |
| IV. Observability                     | PASS   | Error status fix directly improves observability quality     |
| V. YAGNI                              | PASS   | Each change addresses a diagnosed production incident        |
| VI. Interface Parity via Orchestrator | PASS   | Runtime change applies regardless of interface               |
| VII. Code Quality Gate                | PASS   | `fmt` + `clippy` + `machete` required before commit          |
| VIII. Dual-Mode Parity                | PASS   | OTel span fix applies in both single-binary and worker modes |

## Project Structure

```text
crates/
├── runtime/src/orchestrator/mod.rs     # Fix: set Status::Error on failed turns
├── storage/src/traces.rs               # Add interface, has_reply to TraceSummary + TraceFilter
├── storage/src/logs.rs                 # Add conversation_id, since, until to list_recent_for_agent
└── web-ui/
    ├── src/traces.rs                   # Wire TraceFilter, interface facet, replied badge, time range
    ├── src/logs.rs                     # Wire conversation, since, until params
    ├── templates/traces/page.html      # Interface facet, replied column, time range inputs, conv input
    ├── templates/traces/detail.html    # "View conversation logs" link
    └── templates/logs/page.html        # Since/until inputs, conversation input
```

## Implementation Tasks

### Task 1 — `fix(runtime): set error status on failed turn spans`

**Crate**: `assistant-runtime`
**File**: `crates/runtime/src/orchestrator/mod.rs`

Find where `run_turn(...)` result is evaluated (around line 313 where `turn_span` is set up).
On `Err(ref e)`, call:

```rust
otel_turn.set_status(opentelemetry::trace::Status::Error {
    description: std::borrow::Cow::Owned(e.to_string()),
});
```

before the span context drops. The `otel_turn` variable is in scope at the `init_turn_context`
call site.

**Verification**: Run a turn that errors; confirm `attributes` in `distributed_traces` contains
`"otel.status_code":"Error"` for the root turn span. The existing `error_count` aggregate
will then count it, flipping the Status column to ✗.

**Test**: Add a unit test in `crates/runtime` that simulates a failed turn and asserts the
recorded span has `otel.status_code = "Error"`.

---

### Task 2 — `feat(storage): add interface + has_reply to TraceSummary`

**Crate**: `assistant-storage`
**File**: `crates/storage/src/traces.rs`

1. Add fields to `TraceSummary`:

   ```rust
   pub interface: Option<String>,
   pub has_reply: bool,
   ```

2. Add `TraceFilter` struct (lives in `traces.rs`, exported from `lib.rs`):

   ```rust
   #[derive(Default)]
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

3. Replace `skill_filter: Option<&str>` on `list_recent_traces_for_agent` with
   `filter: &TraceFilter`. Update query:

   ```sql
   -- new SELECT columns
   MAX(CASE WHEN dt.parent_span_id IS NULL
       THEN json_extract(dt.attributes, '$.interface') ELSE NULL END) AS interface,
   SUM(CASE WHEN dt.tool_name IN ('reply', 'slack-post') THEN 1 ELSE 0 END) > 0 AS has_reply,
   -- new WHERE clauses
   AND (?since IS NULL OR dt.start_time >= ?since)
   AND (?until IS NULL OR dt.start_time <= ?until)
   AND (?iface IS NULL OR json_extract(dt.attributes, '$.interface') = ?iface)
   ```

4. Map new columns in row mapper:
   ```rust
   let interface = row.try_get::<Option<String>, _>("interface").ok().flatten();
   let has_reply: bool = row.try_get::<i64, _>("has_reply").unwrap_or(0) != 0;
   ```

**Test**: Unit test — insert trace with `tool_name = 'reply'`, assert `has_reply = true`;
without reply tool, assert `has_reply = false`. Assert `interface` extracted from JSON attrs.

---

### Task 3 — `feat(storage): add conversation + time range to log queries`

**Crate**: `assistant-storage`
**File**: `crates/storage/src/logs.rs`

Extend `list_recent_for_agent` with three new optional parameters at positions before
`agent_id`:

```rust
conversation_id: Option<&str>,   // NEW — cross-turn scope
since: Option<DateTime<Utc>>,    // NEW
until: Option<DateTime<Utc>>,    // NEW
```

Add to WHERE clause:

```sql
AND (?conv IS NULL OR dt.conversation_id = ?conv)
AND (?since IS NULL OR l.timestamp >= ?since)
AND (?until IS NULL OR l.timestamp <= ?until)
```

Update the single call site in `crates/web-ui/src/logs.rs` — pass `None` for all three
until wired in Task 5.

**Test**: Insert logs for two conversations, filter by one, assert correct scoping.

---

### Task 4 — `feat(web-ui): interface facet + replied badge on traces page`

**Crate**: `assistant-web-ui`
**Files**: `crates/web-ui/src/traces.rs`, `crates/web-ui/templates/traces/page.html`

**Backend (`traces.rs`)**:

1. Parse `?interface=` from query params.
2. Derive interface facets (distinct values + counts) from loaded `TraceSummary` list.
3. Pass `interface` through `TraceFilter` to storage query.
4. Add `has_reply: bool` and `interface: Option<String>` to the `TraceRow` view model.

**Template (`page.html`)**:

1. Add "Interface" facet group in sidebar — radio buttons (All / Slack / Scheduler / Cli),
   HTMX-wired like the Status radios.
2. Add "Replied" column:
   - `✓` green if `has_reply`
   - `–` amber if `!has_reply && interface == Slack` (user-visible silent failure)
   - `–` grey if `!has_reply && interface != Slack` (scheduler jobs don't always reply)

---

### Task 5 — `feat(web-ui): conversation input + time range on traces + logs`

**Crate**: `assistant-web-ui`
**Files**:

- `crates/web-ui/src/traces.rs` + `templates/traces/page.html`
- `crates/web-ui/src/logs.rs` + `templates/logs/page.html`

**Traces sidebar** — add two new `<div class="facet-group">` sections:

1. **Conversation ID** text input (replaces hidden param):
   ```html
   <input
     type="text"
     name="conversation"
     value="{{ conversation_str }}"
     placeholder="paste UUID…"
   />
   ```
2. **Time Range** — two `<input type="datetime-local">` fields (`since`, `until`).

**Logs sidebar** — same time range inputs, same conversation UUID input.

**Backend**: Parse `since`/`until` as `DateTime<Utc>` from `%Y-%m-%dT%H:%M` format.
Wire through to storage calls from Tasks 2 and 3.

---

### Task 6 — `feat(web-ui): conversation logs link on trace detail`

**Crate**: `assistant-web-ui`
**File**: `crates/web-ui/templates/traces/detail.html`

The template context already has `conversation_id`. Add one link in the header bar:

```html
{% if let Some(conv) = conversation_id %}
<a class="btn-secondary" href="/logs?conversation={{ conv }}">
  View conversation logs →
</a>
{% endif %}
```

No backend changes needed — `/logs?conversation=<uuid>` is wired in Task 5.

---

## Commit Order

Tasks 1–3 are independent of each other and can be done in any order.
Tasks 4–6 depend on the corresponding storage task:

```text
Task 1  (runtime)    → no deps
Task 2  (storage)    → no deps
Task 3  (storage)    → no deps
Task 4  (web-ui)     → requires Task 2
Task 5  (web-ui)     → requires Task 3
Task 6  (web-ui)     → requires Task 5
```

## Complexity Tracking

No constitution violations. `TraceFilter` struct is justified: replaces a 3-parameter
function that needed 4 more params — struct grouping is simpler than a 7-arg function.

## Definition of Done

- [ ] `make lint` and `make format` pass with zero warnings
- [ ] All 6 tasks committed to `003-observability-ui-improvements`
- [ ] Unit tests for new storage query paths pass
- [ ] Silent-failure scenario (Slack turn, no reply tool called) shows amber `–` in Replied column
- [ ] `/logs?conversation=06effb79-9644-41f0-9e21-a3312c9d408c` returns logs spanning all turns
- [ ] Failed turns show ✗ in Status column on `/traces`
- [ ] Time range inputs allow filtering traces/logs to a specific incident window
