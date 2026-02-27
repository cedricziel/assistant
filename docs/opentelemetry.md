# OpenTelemetry

The assistant emits traces, logs, and metrics via the OpenTelemetry SDK.
All three signals are persisted to a local SQLite database (powering the
built-in web UI dashboards) and can optionally be exported to any
OTLP-compatible collector.

## Quick start

```sh
# Local SQLite only (default when mirror.trace_enabled = true)
assistant

# Send all signals to an OTLP collector
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 assistant

# Per-signal endpoints (traces to Tempo, logs to Loki, metrics to Prometheus)
OTEL_EXPORTER_OTLP_TRACES_ENDPOINT=http://tempo:4317 \
OTEL_EXPORTER_OTLP_LOGS_ENDPOINT=http://loki:4317 \
OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://prometheus:4317 \
  assistant

# Auth header for a managed backend (e.g. Grafana Cloud, Honeycomb)
OTEL_EXPORTER_OTLP_ENDPOINT=https://otlp.example.com:4317 \
OTEL_EXPORTER_OTLP_HEADERS="Authorization=Bearer my-token" \
  assistant
```

## Environment variables

The `opentelemetry-otlp` crate reads these env vars automatically at
exporter construction time.  Every generic variable has a per-signal
override that takes precedence (signal-specific > generic > default).

### Exporter configuration

| Generic | Traces | Logs | Metrics | Default |
|---------|--------|------|---------|---------|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `_TRACES_ENDPOINT` | `_LOGS_ENDPOINT` | `_METRICS_ENDPOINT` | `http://localhost:4317` |
| `OTEL_EXPORTER_OTLP_HEADERS` | `_TRACES_HEADERS` | `_LOGS_HEADERS` | `_METRICS_HEADERS` | *(none)* |
| `OTEL_EXPORTER_OTLP_TIMEOUT` | `_TRACES_TIMEOUT` | `_LOGS_TIMEOUT` | `_METRICS_TIMEOUT` | `10s` |
| `OTEL_EXPORTER_OTLP_COMPRESSION` | `_TRACES_COMPRESSION` | `_LOGS_COMPRESSION` | `_METRICS_COMPRESSION` | `none` |

Headers use `key=value` pairs separated by commas:
`OTEL_EXPORTER_OTLP_HEADERS="api-key=secret,tenant=prod"`.

Compression values: `gzip` or `none`.

### Resource / SDK configuration

| Variable | Purpose | Default |
|----------|---------|---------|
| `OTEL_SERVICE_NAME` | `service.name` resource attribute | `assistant` |
| `OTEL_RESOURCE_ATTRIBUTES` | Additional resource attributes (`k=v,k=v`) | *(none)* |
| `RUST_LOG` | Console log filter (standard `tracing` EnvFilter) | `info` |

### Config file

In `~/.assistant/config.toml`:

```toml
[mirror]
trace_enabled = true   # Enable SQLite telemetry export (default: true)
trace_content = false  # Capture full LLM message content in spans (default: false)
analysis_window = 50   # Number of recent traces for self-analysis
```

`trace_content = true` records `gen_ai.input.messages`,
`gen_ai.output.messages`, `gen_ai.system_instructions`, and
`gen_ai.tool.definitions` on LLM spans.  Off by default for PII concerns.

## Traces

Tracer name: `assistant.orchestrator`

### Span hierarchy

```
conversation                          # root span per conversation
  chat {model}                        # one per LLM call
  execute_tool {name}                 # one per tool invocation
```

### `conversation` span

| Attribute | Type | Description |
|-----------|------|-------------|
| `conversation_id` | string | UUID of the conversation |
| `interface` | string | `Cli`, `Slack`, `Mattermost`, `Signal`, `A2A` |

### `chat {model}` span (GenAI semantic conventions)

| Attribute | Type | Description |
|-----------|------|-------------|
| `gen_ai.system` | string | Provider name (e.g. `ollama`, `anthropic`) |
| `gen_ai.request.model` | string | Requested model name |
| `gen_ai.operation.name` | string | Always `chat` |
| `server.address` | string | Provider endpoint URL |
| `iteration` | int | ReAct loop iteration index |
| `gen_ai.response.model` | string | Model used in response (may differ) |
| `gen_ai.response.id` | string | Provider-assigned response ID |
| `gen_ai.response.finish_reasons` | string | Stop reason (`stop`, `tool_use`, etc.) |
| `gen_ai.usage.input_tokens` | int | Input token count |
| `gen_ai.usage.output_tokens` | int | Output token count |

When `trace_content = true`:

| Attribute | Type | Description |
|-----------|------|-------------|
| `gen_ai.system_instructions` | string | Full system prompt |
| `gen_ai.input.messages` | string | Serialised chat history (JSON) |
| `gen_ai.output.messages` | string | Serialised assistant response (JSON) |
| `gen_ai.tool.definitions` | string | Serialised tool specs (JSON) |

### `execute_tool {name}` span

| Attribute | Type | Description |
|-----------|------|-------------|
| `conversation_id` | string | UUID |
| `iteration` | int | ReAct loop iteration index |
| `turn` | int | Turn number |
| `interface` | string | Interface name |
| `tool_name` | string | Tool identifier (e.g. `file-read`) |
| `tool_params` | string | Serialised parameters (JSON) |
| `tool_status` | string | `ok`, `error`, `deferred`, `rejected`, `blocked` |
| `tool_observation` | string | Tool output (on success) |
| `tool_error` | string | Error message (on failure) |

## Logs

Tracing events from all `assistant_*` crates are bridged into OTel log
records via `OpenTelemetryTracingBridge`.  Each log record carries:

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

| Metric | Type | Unit | Key Attributes |
|--------|------|------|----------------|
| `gen_ai.client.token.usage` | Histogram | `{token}` | `gen_ai.request.model`, `gen_ai.provider.name`, `gen_ai.operation.name`, `gen_ai.token.type` (`input`/`output`) |
| `gen_ai.client.operation.duration` | Histogram | `s` | `gen_ai.request.model`, `gen_ai.provider.name`, `gen_ai.operation.name`, `error.type` |
| `gen_ai.server.time_to_first_token` | Histogram | `s` | `gen_ai.request.model`, `gen_ai.provider.name` |
| `gen_ai.server.time_per_output_token` | Histogram | `s` | `gen_ai.request.model`, `gen_ai.provider.name` |

### Operational metrics

| Metric | Type | Unit | Key Attributes |
|--------|------|------|----------------|
| `assistant.turn.count` | Counter | `{turn}` | `skill`, `interface` |
| `assistant.turn.duration` | Histogram | `s` | `skill`, `interface` |
| `assistant.tool.invocations` | Counter | `{invocation}` | `tool.name` |
| `assistant.tool.duration` | Histogram | `s` | `tool.name` |
| `assistant.error.count` | Counter | `{error}` | `error.type`, `source` |
| `assistant.conversation.count` | Counter | `{conversation}` | *(none)* |
| `assistant.agent.spawn.count` | Counter | `{agent}` | *(none)* |

Metrics are exported every 60 seconds via `PeriodicReader`.

## Resource attributes

Every signal carries a shared OTel `Resource`:

| Attribute | Source |
|-----------|--------|
| `service.name` | `OTEL_SERVICE_NAME` or `"assistant"` |
| `service.version` | Crate version from `Cargo.toml` |
| `os.type` | Compile-time OS |
| `host.arch` | Compile-time architecture |
| `process.pid` | Runtime PID |
| `process.runtime.name` | `"rust"` |
| `telemetry.sdk.name` | `"opentelemetry"` |
| `telemetry.sdk.language` | `"rust"` |
| *(user-defined)* | `OTEL_RESOURCE_ATTRIBUTES` (`k=v,k=v`) |

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
       export  gRPC    export  gRPC   export  gRPC
          │      │        │      │       │      │
          ▼      ▼        ▼      ▼       ▼      ▼
       Web UI  Jaeger/  Web UI  Loki/  Web UI  Prom/
       dashboard Tempo  dashboard ...  dashboard ...
```

Both backends run side-by-side when OTLP env vars are set.  The SQLite
exporters power the built-in web UI; the OTLP exporters send data to
your collector of choice.
