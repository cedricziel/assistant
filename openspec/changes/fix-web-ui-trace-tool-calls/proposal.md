## Why

Trace detail views in the web UI render every span identically — name, duration bar, and a flat key/value dump of attributes (`app/lib/features/traces/trace_detail_screen.dart:174`). Tool calls _are_ recorded as OpenTelemetry spans named `execute_tool <tool_name>` with attributes `tool_name`, `tool_params`, `tool_status` (`ok` / `error` / `denied`), `tool_observation` (output) or `tool_error`, and `duration_ms` (`crates/runtime/src/otel_spans.rs:170`, `crates/runtime/src/orchestrator/dispatch.rs:169-215`). But because the UI doesn't single out tool spans:

- A failed tool execution looks identical to a successful one until you expand it.
- The most diagnostically useful attributes (`tool_status`, `tool_error`, `tool_observation`) are buried in the alphabetical attribute dump.
- The relationship between an LLM span and its child tool spans is invisible — they're flattened by start-time sort.
- Tool input parameters are stored as a JSON string inside a single attribute — readable only after expanding and squinting.

The result is that the trace viewer, our primary debugging surface for the ReAct loop, is much less useful than it should be.

## What Changes

- Detect tool spans (`span.name` starts with `execute_tool ` or `tool_name` attribute is present) and render them with a dedicated `_ToolCallSpanCard` instead of the generic `_SpanCard`.
- Surface status as a colored badge (`ok` green, `error` red, `denied` amber) matching the chat `tool-call-display` chip palette.
- Show `tool_name` as the card title, `duration_ms` and `tool_status` in the header, and a two-pane "Params / Output" body that pretty-prints the JSON.
- Add a regression test that builds a synthetic trace with one successful and one failed tool call and asserts the rendered output for each.

## Non-goals

- Changing how spans are stored (`opentelemetry-exporter-sqlite` schema unchanged).
- Adding span-tree / parent-child visualization for non-tool spans (separate, larger work).
- Streaming live trace updates (out of scope).
- Capturing extra attributes — the current `tool_params` / `tool_observation` / `tool_status` set is sufficient for this fix.

## Capabilities

### Added Capabilities

- `trace-tool-call-rendering` (new spec) — how tool spans are surfaced in the web UI trace detail.

## Impact

- `app/lib/features/traces/trace_detail_screen.dart` — add detection helper and new `_ToolCallSpanCard` widget. Wire the list builder to switch on span type.
- `app/test/widget/traces/trace_detail_tool_call_test.dart` — golden / widget test covering the three statuses (ok, error, denied) and the params + output panes.
- No backend changes. The required attributes are already emitted; we only render them better.

## Visual / UI change

Yes — the trace detail page gains a distinct card style for tool-call spans. Status colors share the chat tool-call chip palette to keep the trace ↔ chat mental model unified. Playwright trace-detail screenshot baselines will move.

## User-facing documentation

Brief addition to `docs/operations/observability.md` describing the new tool-call card and which attributes it surfaces.
