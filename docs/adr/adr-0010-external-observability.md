# ADR 0010: Observability Lives Outside the Assistant

**Status**: Accepted
**Date**: 2026-08-07

## Context

The assistant shipped two local OTel exporter backends: SQLite and Apache
Iceberg. The Iceberg path wrote Parquet files into a warehouse directory
(optionally registered in a REST catalog) and the web UI read them back by
scanning Parquet directly, so the binary carried a full columnar analytics
stack:

- `iceberg`, `iceberg-catalog-rest`
- `arrow-array`, `arrow-schema` (and the ten transitive `arrow-*` crates)
- `parquet`, `thrift`, `apache-avro`, plus five compression codecs
  (`zstd`, `lz4_flex`, `snap`, `brotli`, `flatbuffers`)
- a second, duplicate `reqwest` major version pulled in by `opendal`

That is **81 crates** in service of a feature that duplicates what every
real observability backend already does — and does far better, with
retention policies, cross-service correlation, and a query language.

Two problems followed from it:

1. **Mixed concerns.** Storing, partitioning, compacting, and querying
   telemetry is a database's job. Building one into an assistant runtime
   means owning schema evolution, partition specs, and Parquet scan
   performance forever.
2. **Build weight.** The columnar stack dominated cold-build time and
   binary size, and it was the workspace's largest single source of
   dependency churn (its own Dependabot group).

## Decision

**Telemetry leaves the process over OTLP. The assistant does not own an
analytics store.**

Concretely:

- The `opentelemetry-exporter-iceberg` crate is removed, along with the
  `IcebergTraceBackend` / `IcebergLogBackend` / `IcebergMetricsBackend`
  read-side backends in `web-ui` and all `arrow`/`parquet`/`iceberg`
  workspace dependencies.
- `[observability] exporter` narrows from `sqlite | iceberg | both | none`
  to `sqlite | none`. The removed values are **rejected** at config parse
  time rather than silently downgraded, so a stale config fails loudly.
- **OTLP is the supported export path.** It is enabled by setting any
  non-empty `OTEL_EXPORTER_OTLP_*` environment variable and runs alongside
  the local SQLite exporter.
- The local SQLite exporter stays, and stays small. Its job is to power the
  built-in trace/log/metric viewers for a single-node install — a debugging
  convenience, not a warehouse. Operators who want retention or aggregation
  set `exporter = "none"` and point OTLP at their stack.

### OTLP transport

Only the HTTP transports are compiled in, and specifically only
`http-proto` (OTLP/HTTP with protobuf payloads) — the OTel default protocol,
accepted by every collector. Endpoints are therefore the `:4318`-style HTTP
ones, not `:4317`.

`grpc-tonic` is deliberately not enabled: it would re-introduce the
tonic/hyper stack this ADR exists to avoid. `http-json` is also not enabled,
because the crate's feature priority is `http-json > http-proto` — turning it
on would silently make JSON the default wire format.

`gzip-http` **is** enabled, so `OTEL_EXPORTER_OTLP_COMPRESSION=gzip` works.
Without it the exporter fails to build at startup with "gzip compression
requested but gzip-http feature not enabled" — a hard error from a variable
the documentation advertised as supported.

#### The HTTP client must be blocking

The single most important constraint here, and the one that silently
invalidated OTLP export before this change:

The SDK's default `BatchSpanProcessor`, `BatchLogProcessor`, and
`PeriodicReader` all run exports on a **plain background thread** using
`futures_executor::block_on`. That thread has no Tokio reactor. Pairing them
with an async HTTP client (`reqwest-client`, `hyper-client`) makes every
export panic on the exporter thread with:

```
thread 'OpenTelemetry.Traces.BatchProcessor' panicked:
there is no reactor running, must be called from the context of a Tokio 1.x runtime
```

Nothing surfaces in the application: the panic is confined to the exporter
thread, `init_tracing` returns a healthy-looking guard, and **not one span
reaches the collector**. The workspace shipped `reqwest-client` and was in
exactly this state.

The fix is `reqwest-blocking-client`, the combination the SDK documents as
supported alongside `tokio::main`. Client selection priority inside
`opentelemetry-otlp` is `reqwest-client > hyper-client >
reqwest-blocking-client`, so re-enabling `reqwest-client` anywhere in the
feature-unified graph would silently reintroduce the bug. The alternative —
the `*_with_async_runtime` processor variants — is still marked experimental
and was not worth the risk for this.

`crates/runtime/tests/otlp_env_config.rs` is the regression test: it asserts
a span actually arrives at an HTTP server, which is the only assertion that
catches this class of failure.

### Environment variables

The `opentelemetry-otlp` and `opentelemetry_sdk` crates resolve the standard
variables themselves; the runtime's job is mostly to not get in their way.
Several bugs fixed here were exactly that:

- `PeriodicReader` reads `OTEL_METRIC_EXPORT_INTERVAL`, but the runtime
  called `.with_interval(60s)` unconditionally, overriding it. The explicit
  interval is gone from the OTLP reader.
- The exporters were built with `.with_http()`, which pins the transport and
  bypasses `OTEL_EXPORTER_OTLP_PROTOCOL`. They are now built with a bare
  `.build()` so the env var is honoured.
- `build_resource()` unconditionally set `service.name` from
  `OTEL_SERVICE_NAME` or the `assistant` default, which clobbered a
  `service.name` supplied through `OTEL_RESOURCE_ATTRIBUTES`. The SDK
  detectors already implement the specified precedence, so the runtime now
  only substitutes its default when the SDK fell back to its
  `unknown_service` placeholder.

`OTEL_SDK_DISABLED=true` is now honoured as a hard kill switch (spec-defined:
only the literal `true`, case-insensitive, disables the SDK).

Sampling (`OTEL_TRACES_SAMPLER`, `OTEL_TRACES_SAMPLER_ARG`) and batch tuning
(`OTEL_BSP_*`, `OTEL_BLRP_*`) were verified to work through the SDK's own
`Config::default()` and processor builders — no runtime change needed, but
they are now documented in `docs/opentelemetry.md`.

## Self-learning

The autonomous skill-learning subsystem (`skill_improver`, `skill_learner`,
the `self-analyze` tool) reads skill statistics through the
`SkillStatsProvider` trait. `SqliteTraceStore` implements it and is the only
implementation ever wired up — `IcebergTraceBackend`'s implementation existed
but was never constructed, since `ToolExecutor::with_stats_provider` had no
callers.

**Self-learning is therefore unaffected by this change and is not being
mothballed.** It keeps working against the SQLite trace store.

`SkillStatsProvider` is retained precisely as the seam for the intended
future: sourcing the same statistics from an external backend via TraceQL or
PromQL queries, instead of from a local store. Doing that is a self-contained
follow-up — implement the trait against an HTTP query client and pass it to
`ToolExecutor::with_stats_provider`.

## Consequences

**Good**

- 81 crates removed from the lock file; the `arrow-iceberg` Dependabot group
  is gone. Duplicate `reqwest` major versions collapse to one.
- Substantially faster cold builds and a smaller release binary.
- Observability concerns leave the runtime. The assistant emits signals; the
  backend stores and queries them.
- OTLP configuration now actually follows the OTel specification.
- **OTLP export works at all.** It did not before: the async HTTP client was
  incompatible with the batch processors driving it, so every export panicked
  on a background thread and was silently discarded. Covered by a regression
  test that asserts a span reaches a real HTTP server.

**Bad / accepted**

- No backwards compatibility for `exporter = "iceberg"` / `"both"`. Configs
  using them fail to parse and must be changed to `"sqlite"` or `"none"`.
  Existing Parquet warehouses are not read, migrated, or deleted — they are
  simply no longer touched.
- The default OTLP wire format changes from `http/json` to `http/protobuf`.
  Collectors accept both on the same endpoint, so this should be transparent;
  `OTEL_EXPORTER_OTLP_PROTOCOL=http/json` is no longer available.
- gRPC OTLP (`:4317`) is not supported. Point at the HTTP port, or run a
  local collector that fans out.
- Telemetry beyond the local SQLite window requires an external backend. For
  a single-node install with no collector, the built-in viewers remain the
  only history.

## Alternatives considered

**Feature-gate Iceberg behind an off-by-default cargo feature.** Keeps the
capability at zero default cost, but leaves the code, the schema, and the
Dependabot churn in the tree, and a feature nobody builds is a feature nobody
maintains. Rejected.

**Keep Iceberg, drop only the read path.** Would still carry the whole
columnar write stack for data the assistant could no longer show. Rejected.

**Drop SQLite too, OTLP only.** Tempting for purity, but it removes the
built-in trace/log viewers — the fastest way to debug a local install with no
collector running. SQLite is cheap (`sqlx` is already a dependency) and
earns its place. Rejected, with `exporter = "none"` available for anyone who
disagrees.
