## Tasks

### Phase 1 — Failing tests

- [x] Add `app/test/widget/features/traces/trace_detail_tool_calls_test.dart` that pumps `TraceDetailScreen` with a synthetic `TraceDetailResponse` containing: - one span `execute_tool file-read` with `tool_status == "ok"`, parseable `tool_params` JSON, and a short `tool_observation` - one span `execute_tool bash` with `tool_status == "error"` and a `tool_error` message - one span `execute_tool web-fetch` with `tool_status == "denied"` and a `tool_error` - one generic `chat anthropic/...` span
- [x] Assertions: - finds three `ToolCallSpanCard` widgets and a generic span renders the `chat` span name - each tool card surfaces the correct status icon - generic chat span is still rendered
- [x] Unit-test `isToolSpan` / `toolNameFromSpan` in `span_classifier_test.dart`.
- [x] Widget-test `ToolCallSpanCard` in isolation: each status (`ok`, `error`, `denied`, missing), expanded params + output panes, error styling, `Show all attributes` toggle, non-JSON params verbatim.
- [x] Run `flutter test` and confirm RED before implementation.

### Phase 2 — Detection helper + widget

- [x] Add `bool isToolSpan(SpanEntryResponse span)` and `String toolNameFromSpan(SpanEntryResponse span)` in `app/lib/features/traces/span_classifier.dart`.
- [x] Build `ToolCallSpanCard` in `app/lib/features/traces/tool_call_span_card.dart` with: - header: status icon + tool name + status pill + duration - expandable body with `LayoutBuilder` switching between row / column at 600 dp - JSON pretty-printer with parse fallback - `Show all attributes` toggle reusing a simple key/value list
- [x] Phase-1 widget-card assertions GREEN.

### Phase 3 — Wire into `_TraceDetailBody`

- [x] In `trace_detail_screen.dart`, branch the list builder on `isToolSpan` and emit `ToolCallSpanCard` for matches.
- [x] Keep `_SpanCard` for non-tool spans.
- [x] Phase-1 trace-detail test GREEN.

### Phase 4 — Playwright + screenshot baselines

- [x] Add `crates/web-ui/e2e/tests/trace-tool-call-rendering.spec.ts` with a stubbed trace response covering one `ok`, one `error`, and one generic chat span.
- [ ] Screenshot baselines for the trace detail page move when this lands — CI regenerates with `--update-snapshots` on first run; document the intentional diff in the PR.

### Phase 5 — Wrap-up

- [ ] `make lint-flutter && make test-flutter && make lint && make format`.
- [x] Add a "Trace detail — tool call cards" section to `docs/web-ui.md`.
- [ ] Manual smoke: run `assistant webui serve`, trigger a turn with both a successful and a failing tool, open `/traces/{id}` and verify the cards.
