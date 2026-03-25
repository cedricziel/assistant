---
name: observability
description: >
  Practical observability guide for this workspace. Includes where traces, logs,
  and metrics are persisted in SQLite and quick query patterns for debugging.
  Use when investigating runtime behavior, incidents, or telemetry quality.
license: MIT
---

# Observability

Use this skill when you need to diagnose behavior via telemetry in this
repository.

## Telemetry Storage Tables

- Traces: `distributed_traces`
  - Source: OpenTelemetry spans
  - Key columns: `span_id`, `trace_id`, `parent_span_id`, `name`,
    `service_name`, `start_time`, `end_time`, `duration_ms`, `attributes`
  - Tool-oriented columns: `tool_name`, `tool_status`, `tool_observation`,
    `tool_error`, `conversation_id`, `turn`
  - Decoupling note: `conversation_id` is intentionally not a foreign key to
    `conversations` (telemetry must remain standalone)

- Logs: `logs`
  - Source: OpenTelemetry log records
  - Key columns: `id`, `timestamp`, `severity_number`, `severity_text`, `body`,
    `service_name`, `target`, `attributes`
  - Trace correlation columns: `trace_id`, `span_id`

- Metrics: `metric_points` (with `resources` and `metric_scopes`)
  - `metric_points` stores datapoints for counters, gauges, histograms
  - Key columns: `metric_name`, `metric_kind`, `value`, `count`, `sum`, `min`,
    `max`, `recorded_at`, `attributes`, plus denormalized
    `model`/`provider`/`operation`/`skill`/`interface`/`agent_id`
  - `resources` stores deduplicated resource attributes via fingerprint
  - `metric_scopes` stores instrumentation scope `(name, version)`

## Quick Query Hints

- Recent trace failures by tool

```sql
SELECT start_time, service_name, tool_name, tool_status, tool_error, trace_id, span_id
FROM distributed_traces
WHERE tool_status = 'error'
ORDER BY start_time DESC
LIMIT 50;
```

- Logs for a given trace

```sql
SELECT timestamp, severity_text, target, body
FROM logs
WHERE trace_id = ?
ORDER BY timestamp ASC;
```

- Latest metric series for one metric name

```sql
SELECT recorded_at, metric_kind, value, count, sum, model, provider, operation
FROM metric_points
WHERE metric_name = ?
ORDER BY recorded_at DESC
LIMIT 200;
```

## Join Patterns

- Metric metadata joins

```sql
SELECT mp.recorded_at, mp.metric_name, mp.metric_kind, mp.value,
       r.attributes AS resource_attrs, ms.name AS scope_name, ms.version AS scope_version
FROM metric_points mp
JOIN resources r ON r.id = mp.resource_id
JOIN metric_scopes ms ON ms.id = mp.scope_id
WHERE mp.recorded_at >= datetime('now', '-1 day')
ORDER BY mp.recorded_at DESC
LIMIT 200;
```

- Trace + log correlation

```sql
SELECT t.start_time, t.name AS span_name, t.tool_name, t.tool_status,
       l.timestamp, l.severity_text, l.body
FROM distributed_traces t
LEFT JOIN logs l ON l.trace_id = t.trace_id AND (l.span_id = t.span_id OR l.span_id IS NULL)
WHERE t.trace_id = ?
ORDER BY l.timestamp ASC;
```

## Operational Tips

- Prefer filtering by indexed columns first (`start_time`, `timestamp`,
  `recorded_at`, `trace_id`, `metric_name`, `service_name`).
- Keep high-cardinality details in JSON `attributes`; use denormalized columns
  for dashboards and fast predicates.
- Use `trace_id` as the primary pivot when moving between spans and logs.
- For metrics, inspect both `value` (scalar) and histogram fields
  (`count`/`sum`/`bounds`/`bucket_counts`) before concluding behavior.

## Known Limits

- `u64` metrics are currently skipped by the SQLite metrics exporter; if you
  need those series in SQLite dashboards, emit compatible numeric types
  (`i64`/`f64`) or add `u64` handling in the exporter.
