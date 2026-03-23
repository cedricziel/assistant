---
name: opentelemetry-rust-sdk
description: >
  Best-practice guide for instrumenting Rust apps with the OpenTelemetry SDK.
  Use when setting up traces, metrics, logs, exporters, and lifecycle handling.
  For GenAI attributes and semantics, pair with the opentelemetry-genai-spec
  skill.
license: MIT
---

# opentelemetry-rust-sdk

Use this skill as a **best-practice reference** for OpenTelemetry Rust SDK
instrumentation. It is intentionally normative and may be stricter than the
current project implementation.

For GenAI semantic conventions (`gen_ai.*`), use:

- `opentelemetry-genai-spec`

## Core setup pattern (best-practice)

1. Build a shared `Resource` (`service.name`, `service.version`, env metadata).
2. Build providers per signal:
   - `SdkTracerProvider`
   - `SdkLoggerProvider`
   - `SdkMeterProvider`
3. Attach processors/readers/exporters:
   - traces/logs: batch processors for production
   - metrics: `PeriodicReader`
4. Register globally (`global::set_tracer_provider`, `global::set_meter_provider`,
   log bridge setup).
5. Flush/shutdown on exit to avoid telemetry loss.

## Exporter strategy

- Prefer OTLP for production (`opentelemetry-otlp`) and let standard
  `OTEL_EXPORTER_OTLP_*` env vars drive endpoints/headers/timeouts.
- Use stdout exporters for local debugging and quick verification.
- If dual-exporting (for example local + remote), keep one provider per signal
  and attach multiple processors/readers instead of duplicating providers.

## Tracing + logging integration

- Use `tracing-opentelemetry` when you want `tracing` spans exported as OTel traces.
- Use `opentelemetry-appender-tracing` when you want `tracing` events exported as OTel logs.
- Prevent recursive logging loops (for example DB/exporter internals) by adding
  explicit target filters on the log bridge.

## Metrics recommendations

- Use explicit instrument naming and units.
- Keep attribute cardinality bounded; avoid unbounded IDs in metric labels.
- Configure export interval intentionally (`PeriodicReader`) for your cost/latency
  goals.
- Always shutdown/flush meter providers in long-running services and tests.

## Performance defaults

- Production: batch processors for traces/logs.
- Debug/test: simple processors or in-memory exporters for deterministic checks.
- Avoid large payload attributes unless required and policy-approved.

## Safety and data policy

- Treat prompts, outputs, tool args/results, and retrieval payloads as sensitive.
- Default to metadata-only telemetry in production.
- Add truncation/redaction toggles before enabling full content capture.

## Versioning and compatibility

- Keep `opentelemetry`, `opentelemetry_sdk`, and `opentelemetry-otlp` versions
  aligned.
- Re-check feature flags (`trace`, `logs`, `metrics`, runtime/exporter features)
  on upgrades.
- Read changelogs for SDK/exporter crates before bumping versions.

## Review checklist

1. Resource fields present and consistent across all signals.
2. Providers registered once and shutdown cleanly.
3. Batch/reader configuration fits workload.
4. Cardinality and payload-size risks addressed.
5. OTLP env-based config works in target deploy environment.
6. GenAI attributes validated against `opentelemetry-genai-spec`.
