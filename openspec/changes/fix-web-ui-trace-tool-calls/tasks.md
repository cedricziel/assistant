## Tasks

### Phase 1 — Failing tests

- [ ] Add `app/test/widget/traces/trace_detail_tool_call_test.dart` that pumps `_TraceDetailBody` with a synthetic `TraceDetailResponse` containing: - one span `execute_tool file-read` with `tool_status == "ok"`, parseable `tool_params` JSON, and a short `tool_observation` - one span `execute_tool bash` with `tool_status == "error"` and a `tool_error` message - one span `execute_tool web-fetch` with `tool_status == "denied"` and a `tool_error` - one generic `chat anthropic/...` span
- [ ] Assertions: - finds three `_ToolCallSpanCard` widgets and one `_SpanCard` - each tool card surfaces the correct status icon and pill colour - expanding the `ok` card reveals `Params` / `Output` panes - expanding the `error` card shows the error message styled with `colorScheme.error` - `Show all attributes` toggle reveals `iteration` / `turn` / `interface`
- [ ] Add a narrow-width variant of the same test (480 dp) asserting vertical stacking.
- [ ] Run `flutter test` and confirm RED.

### Phase 2 — Detection helper + widget

- [ ] Add `bool isToolSpan(OtelSpanRecord span)` in `app/lib/features/traces/span_classifier.dart`. Cover with a unit test.
- [ ] Build `_ToolCallSpanCard` in `app/lib/features/traces/tool_call_span_card.dart` with: - Header: tool name, duration, status icon + pill - Expandable body with `LayoutBuilder` switching between row / column at 600 dp - JSON pretty-printer with parse fallback - `Show all attributes` toggle reusing the existing key/value list
- [ ] Add a widget test for `_ToolCallSpanCard` in isolation covering each status and the parse-fallback branch.
- [ ] Run `flutter test`; confirm Phase-1 widget-card assertions GREEN.

### Phase 3 — Wire into `_TraceDetailBody`

- [ ] In `trace_detail_screen.dart`, branch the list builder on `isToolSpan` and emit `_ToolCallSpanCard` for matches.
- [ ] Keep `_SpanCard` for non-tool spans.
- [ ] Run `flutter test`; confirm the full Phase-1 trace-detail test is GREEN.

### Phase 4 — Playwright + screenshot baselines

- [ ] Add a Playwright spec capturing the trace detail page with at least one tool call (success and failure).
- [ ] Update existing trace-detail screenshot baselines; document the intentional diff in the PR.

### Phase 5 — Wrap-up

- [ ] `make lint-flutter && make test-flutter && make lint && make format`.
- [ ] Update `docs/operations/observability.md` with a short section describing the tool-call card and which attributes it surfaces.
- [ ] Manual smoke: run `assistant webui serve`, trigger a turn with both a successful and a failing tool, open `/traces/{id}` and verify the cards.
