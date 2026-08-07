# OpenTelemetry

The assistant emits traces, logs, and metrics via the OpenTelemetry SDK.

**Observability belongs outside the assistant.** The supported path for
anything real — retention, aggregation, dashboards, alerting, cross-service
correlation — is OTLP export to your own stack, queried there with TraceQL,
LogQL, PromQL, or whatever your backend speaks. See
[ADR-0010](adr/adr-0010-external-observability.md).

A small local SQLite store runs by default alongside OTLP. It exists to power
the built-in trace/log/metric viewers on a single-node install — a debugging
convenience, not a warehouse. Turn it off with `exporter = "none"`.

## Quick start

OTLP export turns on as soon as **any** non-empty `OTEL_EXPORTER_OTLP_*`
variable is set.

```sh
# Local SQLite viewers only (default — no OTLP)
assistant

# Send all signals to an OTLP collector
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 assistant

# Per-signal endpoints (traces to Tempo, logs to Loki, metrics to Mimir)
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://tempo:4318/v1/traces \
OTEL_EXPORTER_OTLP_LOGS_ENDPOINT=http://loki:4318/v1/logs \
OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://mimir:4318/v1/metrics \
  assistant

# Auth header for a managed backend (e.g. Grafana Cloud, Honeycomb)
OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp.example.com \
OTEL_EXPORTER_OTLP_HEADERS="Authorization=Bearer my-token" \
  assistant

# Ship everything off-box and keep nothing locally
OTEL_EXPORTER_OTLP_ENDPOINT=http://collector:4318 assistant   # + exporter = "none"

# Kill switch — no providers, no exporters, console logs only
OTEL_SDK_DISABLED=true assistant
```

### Transport

Only **OTLP over HTTP with protobuf payloads** (`http/protobuf`, the OTel
default) is compiled in. Use the HTTP port — `4318` on a standard collector —
not the gRPC port `4317`.

gRPC is deliberately not built: it would pull the entire tonic/hyper stack
into the binary. If you need gRPC ingest, run a collector locally and let it
forward.

Note that the generic `OTEL_EXPORTER_OTLP_ENDPOINT` gets the standard
`/v1/traces`, `/v1/logs`, `/v1/metrics` suffixes appended automatically;
per-signal endpoint variables are used **verbatim** and need the full path.

## Environment variables

The `opentelemetry-otlp` and `opentelemetry_sdk` crates read these
automatically. Every generic exporter variable has a per-signal override that
takes precedence (signal-specific > generic > default).

### Exporter configuration

| Generic                          | Traces                | Logs                | Metrics                | Default                 |
| -------------------------------- | --------------------- | ------------------- | ---------------------- | ----------------------- |
| `OTEL_EXPORTER_OTLP_ENDPOINT`    | `_TRACES_ENDPOINT`    | `_LOGS_ENDPOINT`    | `_METRICS_ENDPOINT`    | `http://localhost:4318` |
| `OTEL_EXPORTER_OTLP_PROTOCOL`    | `_TRACES_PROTOCOL`    | `_LOGS_PROTOCOL`    | `_METRICS_PROTOCOL`    | `http/protobuf`         |
| `OTEL_EXPORTER_OTLP_HEADERS`     | `_TRACES_HEADERS`     | `_LOGS_HEADERS`     | `_METRICS_HEADERS`     | _(none)_                |
| `OTEL_EXPORTER_OTLP_TIMEOUT`     | `_TRACES_TIMEOUT`     | `_LOGS_TIMEOUT`     | `_METRICS_TIMEOUT`     | `10s`                   |
| `OTEL_EXPORTER_OTLP_COMPRESSION` | `_TRACES_COMPRESSION` | `_LOGS_COMPRESSION` | `_METRICS_COMPRESSION` | `none`                  |

Headers use `key=value` pairs separated by commas:
`OTEL_EXPORTER_OTLP_HEADERS="api-key=secret,tenant=prod"`.

Compression values: `gzip` or `none`. (`zstd` is not compiled in.)

`OTEL_EXPORTER_OTLP_PROTOCOL` accepts only `http/protobuf` here — `grpc` and
`http/json` are not compiled in and setting them is an error at startup.

### Resource / SDK configuration

| Variable                      | Purpose                                           | Default     |
| ----------------------------- | ------------------------------------------------- | ----------- |
| `OTEL_SDK_DISABLED`           | `true` disables the SDK entirely                  | `false`     |
| `OTEL_SERVICE_NAME`           | `service.name` resource attribute                 | `assistant` |
| `OTEL_RESOURCE_ATTRIBUTES`    | Additional resource attributes (`k=v,k=v`)        | _(none)_    |
| `OTEL_METRIC_EXPORT_INTERVAL` | Metric export interval, milliseconds              | `60000`     |
| `RUST_LOG`                    | Console log filter (standard `tracing` EnvFilter) | `info`      |

Per the OTel specification, only the literal `true` (case-insensitive)
disables the SDK; any other value leaves it enabled.

`service.name` follows the specified precedence: `OTEL_SERVICE_NAME`, then
`service.name` inside `OTEL_RESOURCE_ATTRIBUTES`, then the built-in
`assistant` default. Setting either variable wins over the default.

### Sampling

| Variable                  | Purpose                       | Default                 |
| ------------------------- | ----------------------------- | ----------------------- |
| `OTEL_TRACES_SAMPLER`     | Sampler selection             | `parentbased_always_on` |
| `OTEL_TRACES_SAMPLER_ARG` | Sampler argument (e.g. ratio) | _(none)_                |

Supported samplers: `always_on`, `always_off`, `traceidratio`,
`parentbased_always_on`, `parentbased_always_off`,
`parentbased_traceidratio`. Sample 10% of traces with:

```sh
OTEL_TRACES_SAMPLER=parentbased_traceidratio \
OTEL_TRACES_SAMPLER_ARG=0.1 \
  assistant
```

### Batching

Spans and logs are buffered and exported in batches. Defaults follow the
OTel specification; tune them if you are dropping telemetry under load.

| Spans                            | Logs                              | Default   |
| -------------------------------- | --------------------------------- | --------- |
| `OTEL_BSP_SCHEDULE_DELAY`        | `OTEL_BLRP_SCHEDULE_DELAY`        | `5000`ms  |
| `OTEL_BSP_MAX_QUEUE_SIZE`        | `OTEL_BLRP_MAX_QUEUE_SIZE`        | `2048`    |
| `OTEL_BSP_MAX_EXPORT_BATCH_SIZE` | `OTEL_BLRP_MAX_EXPORT_BATCH_SIZE` | `512`     |
| `OTEL_BSP_EXPORT_TIMEOUT`        | `OTEL_BLRP_EXPORT_TIMEOUT`        | `30000`ms |

### Test coverage

`crates/runtime/tests/otlp_env_config.rs` exercises the endpoint, headers,
protocol, compression, and resource-attribute variables end to end against a
real HTTP server; `otlp_sdk_disabled.rs` covers the kill switch. Both are
integration tests because `init_tracing` installs global state and the tests
mutate the process environment.

### Config file

In `~/.assistant/config.toml`:

```toml
[observability]
exporter = "sqlite"    # "sqlite" (default) or "none"
trace_content = false  # Capture full LLM message content in spans (default: false)
```

`exporter` controls **only** the local SQLite store. OTLP export is
independent and always additive — the two run side by side.

> The removed `"iceberg"` and `"both"` values are rejected at parse time
> rather than silently downgraded, so a stale config fails loudly. See
> [ADR-0010](adr/adr-0010-external-observability.md).

`trace_content = true` records `gen_ai.input.messages`,
`gen_ai.output.messages`, `gen_ai.system_instructions`, and
`gen_ai.tool.definitions` on LLM spans. Off by default for PII concerns.

## Traces

Tracer name: `assistant.orchestrator`

### Span hierarchy

```
conversation                          # root span per conversation
  chat {model}                        # one per LLM call
  execute_tool {name}                 # one per tool invocation
```

### `conversation` span

| Attribute         | Type   | Description                                   |
| ----------------- | ------ | --------------------------------------------- |
| `conversation_id` | string | UUID of the conversation                      |
| `interface`       | string | `Cli`, `Slack`, `Mattermost`, `Signal`, `A2A` |

### `chat {model}` span (GenAI semantic conventions)

| Attribute                        | Type   | Description                                |
| -------------------------------- | ------ | ------------------------------------------ |
| `gen_ai.system`                  | string | Provider name (e.g. `ollama`, `anthropic`) |
| `gen_ai.request.model`           | string | Requested model name                       |
| `gen_ai.operation.name`          | string | Always `chat`                              |
| `server.address`                 | string | Provider endpoint URL                      |
| `iteration`                      | int    | ReAct loop iteration index                 |
| `gen_ai.response.model`          | string | Model used in response (may differ)        |
| `gen_ai.response.id`             | string | Provider-assigned response ID              |
| `gen_ai.response.finish_reasons` | string | Stop reason (`stop`, `tool_use`, etc.)     |
| `gen_ai.usage.input_tokens`      | int    | Input token count                          |
| `gen_ai.usage.output_tokens`     | int    | Output token count                         |

When `trace_content = true`:

| Attribute                    | Type   | Description                          |
| ---------------------------- | ------ | ------------------------------------ |
| `gen_ai.system_instructions` | string | Full system prompt                   |
| `gen_ai.input.messages`      | string | Serialised chat history (JSON)       |
| `gen_ai.output.messages`     | string | Serialised assistant response (JSON) |
| `gen_ai.tool.definitions`    | string | Serialised tool specs (JSON)         |

### `execute_tool {name}` span

| Attribute          | Type   | Description                                      |
| ------------------ | ------ | ------------------------------------------------ |
| `conversation_id`  | string | UUID                                             |
| `iteration`        | int    | ReAct loop iteration index                       |
| `turn`             | int    | Turn number                                      |
| `interface`        | string | Interface name                                   |
| `tool_name`        | string | Tool identifier (e.g. `file-read`)               |
| `tool_params`      | string | Serialised parameters (JSON)                     |
| `tool_status`      | string | `ok`, `error`, `deferred`, `rejected`, `blocked` |
| `tool_observation` | string | Tool output (on success)                         |
| `tool_error`       | string | Error message (on failure)                       |

## Logs

Tracing events from all `assistant_*` crates are bridged into OTel log
records via `OpenTelemetryTracingBridge`. Each log record carries:

- Timestamp and observed timestamp
- Severity (mapped from tracing level: TRACE=1, DEBUG=5, INFO=9, WARN=13, ERROR=17)
- Body (the formatted tracing event message)
- `target` (Rust module path, e.g. `assistant_runtime::orchestrator`)
- Trace/span context (when emitted inside an active span)
- Structured attributes from tracing fields

`sqlx` targets are suppressed to prevent a feedback loop (SQLite exporter
INSERT -> tracing event -> bridge -> exporter -> INSERT -> ...).

## Metrics

Meter name: `assistant-runtime`

### GenAI metrics (OTel semantic conventions)

| Metric                                | Type      | Unit      | Key Attributes                                                                                                  |
| ------------------------------------- | --------- | --------- | --------------------------------------------------------------------------------------------------------------- |
| `gen_ai.client.token.usage`           | Histogram | `{token}` | `gen_ai.request.model`, `gen_ai.provider.name`, `gen_ai.operation.name`, `gen_ai.token.type` (`input`/`output`) |
| `gen_ai.client.operation.duration`    | Histogram | `s`       | `gen_ai.request.model`, `gen_ai.provider.name`, `gen_ai.operation.name`, `error.type`                           |
| `gen_ai.server.time_to_first_token`   | Histogram | `s`       | `gen_ai.request.model`, `gen_ai.provider.name`                                                                  |
| `gen_ai.server.time_per_output_token` | Histogram | `s`       | `gen_ai.request.model`, `gen_ai.provider.name`                                                                  |

### Operational metrics

| Metric                         | Type      | Unit             | Key Attributes         |
| ------------------------------ | --------- | ---------------- | ---------------------- |
| `assistant.turn.count`         | Counter   | `{turn}`         | `skill`, `interface`   |
| `assistant.turn.duration`      | Histogram | `s`              | `skill`, `interface`   |
| `assistant.tool.invocations`   | Counter   | `{invocation}`   | `tool.name`            |
| `assistant.tool.duration`      | Histogram | `s`              | `tool.name`            |
| `assistant.error.count`        | Counter   | `{error}`        | `error.type`, `source` |
| `assistant.conversation.count` | Counter   | `{conversation}` | _(none)_               |
| `assistant.agent.spawn.count`  | Counter   | `{agent}`        | _(none)_               |

Metrics are exported every 60 seconds via `PeriodicReader`. Override with
`OTEL_METRIC_EXPORT_INTERVAL` (milliseconds).

## Resource attributes

Every signal carries a shared OTel `Resource`:

| Attribute                | Source                                 |
| ------------------------ | -------------------------------------- |
| `service.name`           | `OTEL_SERVICE_NAME` or `"assistant"`   |
| `service.version`        | Crate version from `Cargo.toml`        |
| `os.type`                | Compile-time OS                        |
| `host.arch`              | Compile-time architecture              |
| `process.pid`            | Runtime PID                            |
| `process.runtime.name`   | `"rust"`                               |
| `telemetry.sdk.name`     | `"opentelemetry"`                      |
| `telemetry.sdk.language` | `"rust"`                               |
| _(user-defined)_         | `OTEL_RESOURCE_ATTRIBUTES` (`k=v,k=v`) |

## Architecture

```
                    ┌──────────────────┐
                    │   Orchestrator   │
                    │  (spans + metrics│
                    │   + tracing logs)│
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        TracerProvider  LoggerProvider  MeterProvider
          │      │        │      │       │      │
          ▼      ▼        ▼      ▼       ▼      ▼
       SQLite  OTLP    SQLite  OTLP   SQLite  OTLP
       export  HTTP    export  HTTP   export  HTTP
          │      │        │      │       │      │
          ▼      ▼        ▼      ▼       ▼      ▼
       Web UI  Tempo/   Web UI  Loki/  Web UI  Mimir/
       viewers Jaeger   viewers  ...   viewers  ...
```

Both destinations run side by side when OTLP env vars are set. The SQLite
exporters power the built-in web UI viewers; the OTLP exporters send data
to your collector of choice — which is where anything beyond local
debugging should live.
