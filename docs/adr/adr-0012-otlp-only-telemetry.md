# ADR 0012: Telemetry Is Emitted Over OTLP Only

**Status**: Accepted
**Date**: 2026-11-25
**Supersedes**: the SQLite exporter and built-in viewers kept by [ADR-0010](adr-0010-external-observability.md)

## Context

[ADR-0010](adr-0010-external-observability.md) removed the Iceberg backend
but kept a small local SQLite exporter so the built-in trace, log and
analytics viewers worked on a single-node install without a collector.

That store still carried real weight:

- a `opentelemetry-exporter-sqlite` crate with span, log and metric exporters,
- three storage modules (about 3,200 lines) and five tables,
- REST endpoints, query backends and three Flutter screens on top,
- a log-bridge feedback-loop guard, because the exporter's own `INSERT`
  statements produced tracing events.

The owner decision is to emit OpenTelemetry and drop everything else. A
collector or hosted backend does retention, correlation and querying better
than a per-install SQLite file ever will.

## Decision

**OTLP is the only telemetry output. The assistant keeps no local telemetry
store, API or viewer.**

- `opentelemetry-exporter-sqlite` is deleted.
- `[observability] exporter` and the `OtelExporter` type are removed.
  `[observability] trace_content` remains: it controls what goes into spans,
  not where they go. A stale `exporter` key in an existing config file is
  ignored.
- `init_tracing` takes no database pool. OTLP export is installed when any
  non-empty `OTEL_EXPORTER_OTLP_*` variable is set and `OTEL_SDK_DISABLED` is
  not `true`. Otherwise telemetry is a no-op and only console logging runs.
  Transport, environment variables and the blocking-client constraint are
  unchanged from ADR-0010.
- The storage layer drops the trace, log and metrics stores. Migration
  `043_drop_local_telemetry.sql` drops `distributed_traces`, `logs`,
  `metric_points`, `metric_scopes` and `resources`. Earlier migrations are
  untouched. Rows already written are deleted by the migration.
- `GET /api/traces`, `/api/traces/{id}`, `/api/logs` and `/api/analytics`,
  the web-ui query backends, the `--trace-limit` and `--log-limit` flags, and
  the Flutter traces, logs and analytics screens are removed.
- The conversation event log (`conversation_events`), command events and all
  conversation data are not telemetry and stay.

## Impact on skill self-improvement

`skill_improver` and the `self-analyze` tool read per-skill execution
statistics through `SkillStatsProvider`. The only implementation aggregated
the `active_skill`, `tool_status`, `tool_error` and `duration_ms` columns of
`distributed_traces`. With that table gone, **skill self-improvement has no
data source and is inactive.**

What changed in the code:

- `SkillStatsProvider` and its result type, now `SkillStats` (was
  `TraceStats`), moved to `assistant_storage::skill_stats`.
- `UnavailableSkillStats` is the provider wired into `ToolExecutor`. It
  returns an error naming the skill instead of zero executions, so
  `self-analyze` reports the gap and the improvement cycle skips the skill
  rather than concluding it is healthy.
- The CLI no longer registers the `skill-self-improve` scheduled task, and
  logs a warning at startup when `[learning] enabled = true`.
- Autonomous skill creation (`skill_learner`) does not read these statistics
  and is unaffected.

### Proposed replacement

Record skill executions as product data, not telemetry:

1. A `skill_executions` table in the space database: skill name, persona,
   conversation, outcome, error text, duration, and tokens, with an index on
   `(skill_name, created_at DESC)` and a retention prune.
2. The orchestrator writes one row where it already finalises a tool result
   (`finalize_tool_result`), using the `active_skill` it already tracks.
3. `SqliteSkillStats` implements `SkillStatsProvider` over that table, and
   `ToolExecutor` receives it instead of `UnavailableSkillStats`.
4. The CLI registers the improvement task again.

This keeps the learning loop independent of whether an OTLP backend is
configured. The alternative, querying the external backend through TraceQL or
PromQL, ties a core feature to optional infrastructure and to a vendor's
query language. It stays possible behind the same trait.

This was not done here because step 2 touches
`crates/runtime/src/orchestrator/dispatch.rs`, which another change owns.

## Consequences

**Good**

- One telemetry path. No exporter crate, no local schema to evolve, no
  feedback-loop special case beyond the log-bridge filter.
- About 14,000 fewer lines across Rust, Dart and generated client code.
- Telemetry cost is zero unless an endpoint is configured.

**Bad / accepted**

- A single-node install with no collector has no built-in trace, log or
  metric history. Run a local collector or a single-binary backend if you
  need one.
- Skill self-improvement is off until the replacement above lands.
- Existing local telemetry rows are deleted on upgrade.
- No backward compatibility: nothing reads the old tables, endpoints or
  config key.

## Alternatives considered

**Keep the SQLite store but drop the viewers.** Leaves the exporter crate,
schema and write cost with no reader. Rejected.

**Keep the SQLite store only for skill statistics.** Needs a tracer provider
installed even with no OTLP endpoint, which contradicts a no-op default, and
still makes learning depend on span attributes. Rejected in favour of the
product-owned table above.
