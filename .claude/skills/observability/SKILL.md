---
name: observability
description: >
  Practical observability guide for this workspace. Covers how the assistant
  emits traces, logs, and metrics over OTLP, how to turn export on, and what to
  query in the backend. Use when investigating runtime behavior, incidents, or
  telemetry quality.
license: MIT
---

# Observability

Use this skill when you need to diagnose behavior via telemetry in this
repository.

The assistant keeps **no local telemetry store**. Traces, logs, and metrics
leave the process over OTLP/HTTP (protobuf) only, and are queried in whatever
backend receives them (TraceQL, LogQL, PromQL, …). There are no
`distributed_traces`, `logs`, or `metric_points` tables, no `/api/traces`,
`/api/logs`, or `/api/analytics` endpoints, and no trace, log, or analytics
screens in the app. See `docs/opentelemetry.md` and
`docs/adr/adr-0012-otlp-only-telemetry.md`.

## Turn export on

Export is installed when any non-empty `OTEL_EXPORTER_OTLP_*` variable is set
and `OTEL_SDK_DISABLED` is not `true`. With neither, telemetry is a no-op and
only console logging (`RUST_LOG`) runs.

```sh
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 assistant
```

- Use the HTTP port (`4318`), not gRPC (`4317`).
- The generic endpoint gets `/v1/traces`, `/v1/logs`, `/v1/metrics` appended;
  per-signal endpoints are used verbatim.
- `assistant doctor` reports whether OTLP export is on.
- Set `[observability] trace_content = true` to record full LLM message
  content on spans. It may contain PII.

## What is emitted

- Traces (tracer `assistant.orchestrator`): `conversation` root span,
  `chat {model}` per LLM call, `execute_tool {name}` per tool call.
  Tool spans carry `tool_name`, `tool_status` (`ok`, `error`, `denied`, …),
  `tool_error`, `conversation_id`, `turn`, and `active_skill`.
- Logs: `tracing` events from `assistant_*` crates, bridged to OTel log
  records with trace and span context.
- User feedback: a `gen_ai.evaluation.result` log record per rating or
  withdrawal (`score.label` `positive`/`negative`/`cleared`, no
  `score.value` on `cleared`), parented to the reply's `chat {model}` span
  when a traceparent is stored. It carries `assistant.feedback.has_comment`,
  never the comment text.
- Metrics (meter `assistant-runtime`): GenAI token usage and operation
  duration, plus `assistant.turn.*`, `assistant.tool.*`,
  `assistant.error.count`, and `assistant.feedback.count{rating, agent_id}`
  (counts feedback events, not current votes).

The full attribute and metric reference is in `docs/opentelemetry.md`.

## Debugging hints

- Pivot on `trace_id`: find the failing span, then pull the logs with the
  same `trace_id`.
- Failing tools: filter spans on `tool_status = error` and group by
  `tool_name`.
- Nothing arrives: confirm the endpoint is the HTTP one, check for
  `OTEL_SDK_DISABLED=true`, and run with `RUST_LOG=opentelemetry=debug`.
  `crates/runtime/tests/otlp_env_config.rs` shows a working configuration.
- High cardinality: keep it in span attributes, not metric attributes.

## Not telemetry

`conversation_events` (the durable SSE replay log), `command_events`, and
conversation messages are product data in the space database. They stay.
