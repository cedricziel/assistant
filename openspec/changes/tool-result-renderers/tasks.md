# Tasks — tool-result-renderers

This change ships as a stack of PRs. Each one stays under roughly 500 changed lines (generated `assistant_api` code and golden PNGs don't count) and leaves `main` shippable. Every PR starts with a failing test (`.claude/skills/tdd/SKILL.md`) and ends with `make lint && make format` green. Tasks are sized at two hours or less.

Server PRs come first. Most need no client work and can start immediately.

Abbreviations: **CEL** = `chat-element-library`.

## Parallelism map

```
Server   S1a ── S1b
          ├──── S5   (S5–S8 need S1a; S6–S8 also need S1b for their kinds)
          ├──── S6
          ├──── S7
          └──── S8
         S2a ── S2b
          └──── S3 ── S3b
                 └─── S4
         S9            (independent; any time)
Client   C1 (needs S2a, S4, CEL PR 4)
         C2 (needs S1a, CEL PR 3, 4, 5)      ── C1 ∥ C2
         C3 (needs C1, C2, CEL PR 6, 7)
         C4 ∥ C5 ∥ C6 ∥ C7 (each needs C2; C4/C5 also CEL PR 7; C7 also chat-a11y CodeBlock)
         C8 (needs everything)
```

- S1a and S2a can run in parallel. S9 can run in parallel with everything.
- S5–S8 can run in parallel with each other and with S2a–S4.
- S3b and S4 can run in parallel once S3 merges.
- C1 and C2 can run in parallel. C4–C7 can run in parallel and can land before C3 (gallery-only until then).

## Cross-change ordering (binding)

- **Message struct lane:** message-branching PR 1 → **S3** → message-feedback PR 3 → message-branching PR 8.
- tool-approval-prompts PR 6 (denied path in `dispatch.rs`) merges before **S3**.
- generative-ui-a2ui PR 6 depends on **S2a** and **S3**; its surface hook `take()`s `output.data` before forwarding and persistence.
- **S4** lands before message-branching PR 6.
- tool-approval-prompts PR 17 and generative-ui-a2ui PR 11 depend on **S4** and **C1**.
- **Migrations:** S3's migration takes the number pre-allocated by the cross-change stack plan (otherwise the next free after `045_skill_executions.sql`). Register it only in `run_migrations` (`crates/storage/src/lib.rs:298ff`); `seed_database_through_016` (`:562`) is a test fixture and stays unchanged.
- **Playwright baselines:** never merge PNGs by hand. After rebasing, regenerate them (e2e-testing skill) and record the diff in the PR body.
- **Client stream lane:** C1 adds fields to existing `StreamEvent` subclasses only. It lands in the wave the stack plan assigns and updates every exhaustive switch it touches.

---

## S1a — `feat(core): tool render data contract harness with terminal and sources`

**Layer:** server · **Depends on:** none · **Parallel with:** S2a, S9

- [ ] S1a.1 Add `crates/core/tests/tool_render_contracts.rs`. For each kind in `ToolRenderData::KINDS` it asserts that `schemas/tool-results/{kind}.schema.json` exists, that at least one `valid-*.json` and one `invalid-*.json` fixture exist, that every valid fixture passes `jsonschema` and deserializes into the matching variant, and that every invalid fixture fails the schema. Confirm RED (module missing).
- [ ] S1a.2 Add `jsonschema = { workspace = true }` to `assistant-core` `[dev-dependencies]` (rust-dependencies skill).
- [ ] S1a.3 Implement `crates/core/src/tool_render.rs` with `ToolRenderData` (`#[serde(tag = "kind", rename_all = "snake_case")]`) and the `Terminal` and `Sources` variants, `ToolOutput::with_render` (`serde_json::to_value(..).ok()`), and `ToolRenderData::schema(kind) -> Option<Value>` (`include_str!` + `from_str(..).ok()`). Export from `lib.rs`.
- [ ] S1a.4 Write `terminal` and `sources` schemas (2020-12, top-level `additionalProperties: true`) and fixtures, each with an invalid type-error fixture (e.g. `exit_code: "one"`). Confirm S1a.1 GREEN.
- [ ] S1a.5 Add `'schemas/tool-results/**'` to the `flutter` paths filter in `.github/workflows/flutter.yml:36-40` (ci-organization skill).
- [ ] S1a.6 Add `schemas/tool-results/README.md`: the `kind` rule, evolve-in-place with client fallback, drop-not-truncate and producer budgets, MCP opt-in.

## S1b — `feat(core): remaining tool render data contracts`

**Layer:** server · **Depends on:** S1a · **Parallel with:** S2a–S4, S5, S9

- [ ] S1b.1 Add `link_preview` and `plan` fixtures. Confirm RED (no variants or schemas).
- [ ] S1b.2 Add the `LinkPreview` and `Plan` variants and schemas. Confirm GREEN.
- [ ] S1b.3 Add `code_file` and `code_diff` fixtures (including an over-long diff line fixture that is invalid). Confirm RED; add variants and schemas; GREEN.
- [ ] S1b.4 Add `data_table` fixtures (including `rows: []`). Confirm RED; add the variant and schema; GREEN.
- [ ] S1b.5 Write `docs/adr/adr-0020-tool-result-render-contracts.md` (design D10): contract directory, `kind` tag, no versioning, MCP opt-in. Confirm the number is still free.

## S2a — `feat(runtime): carry tool data and truthful status on ToolResult events`

**Layer:** server · **Depends on:** none · **Parallel with:** S1a, S5–S9

- [ ] S2a.1 Create a projection golden test in `crates/runtime/src/projection/mod.rs`: a representative frame sequence (token, tool_result without data, subagent_tool_result, done) serialized to a committed JSON file. It is GREEN on `main` before any other change.
- [ ] S2a.2 Add failing projection tests: (a) a `ToolResult` with `data: Some(json!({"kind":"terminal"}))` projects a `tool_result` payload with `data`; (b) a `SubagentEvent` wrapping such a `ToolResult` yields `subagent_tool_result` whose inner object has `data`. Confirm RED (compile failure).
- [ ] S2a.3 Add failing dispatch tests in `crates/runtime/src/orchestrator/tests/`. `Orchestrator` holds a concrete `Arc<ToolExecutor>` (`orchestrator/mod.rs:206`), so use a stub `ToolHandler` registered on a real `ToolExecutor`, plus `ScriptedLlmProvider`: (a) `with_data(..)` reaches the event; (b) 40 KiB data is dropped and `result` stays; (c) a multibyte char straddling byte 512 does not panic (regression for `dispatch.rs:275-276`); (d) the LLM history entry holds content, not data; (e) a denied call has `data: None`; (f) a handler returning `ToolOutput::error("boom")` yields event status `error` and webhook payload status `error`. Confirm RED.
- [ ] S2a.4 Add `data: Option<serde_json::Value>` to `OrchestratorEvent::ToolResult` (`stream_event.rs:31-43`). In `finalize_tool`, take `output.data` before the observation is built and apply `TOOL_RESULT_DATA_LIMIT_BYTES` with `warn!`. Map `Ok(output) if !output.success` to `tool_status = "error"` (design D11), also on the span. Replace byte slicing with a char-boundary helper. Set `data: None` on the denied path.
- [ ] S2a.5 Project `data` in `SseProjector` (`projection/mod.rs:77-97`) and `project_subagent_inner` (`:194-210`), only when `Some`. Update `all_variants()` fixtures in `projection/mod.rs` and `crates/web-ui/src/a2a/projection.rs` tests (the A2A projector itself uses `..` and is unchanged). Confirm S2a GREEN, golden unchanged.
- [ ] S2a.6 Regenerate Playwright baselines that show a tool-level failure (it now force-expands as an error); record the diff.

## S2b — `fix(web-ui): document tool data on SseToolResultEvent and verify replay`

**Layer:** server · **Depends on:** S2a · **Parallel with:** S3, S5–S9

- [ ] S2b.1 Add a web-ui replay test: a turn whose stub tool returns data, then `GET .../runs/{run}/events/stream?since=0`. The replayed `tool_result` frame has identical `data`. (Frames are persisted at `crates/web-ui/src/api/messages.rs:333-340`.) It should be GREEN after S2a; if RED, fix before continuing.
- [ ] S2b.2 Add a failing test that the generated OpenAPI document gives `SseToolResultEvent.arguments` and `.data` `type: object` and lists `tool_call_id`. Confirm RED.
- [ ] S2b.3 Fix `SseToolResultEvent` (`messages.rs:528-538`): `arguments` and `data` as `Option<serde_json::Value>` with `#[schema(value_type = Object)]`; add `tool_call_id: Option<String>`; describe `data` with a pointer to `schemas/tool-results/`. Confirm GREEN.
- [ ] S2b.4 `make dump-openapi && make generate-flutter-client`; commit `openapi.json` and `assistant_api`.

## S3 — `feat(storage): persist tool call id, status and data on tool rows`

**Layer:** server · **Depends on:** S2a, message-branching PR 1, tool-approval-prompts PR 6 · **Parallel with:** S2b, S5–S9

- [ ] S3.1 Add failing contract tests in `crates/storage/tests/contract/`: a tool-role `Message` with the three fields round-trips identically through `SqliteConversationStore` and `InMemoryConversationStore`. Confirm RED.
- [ ] S3.2 Add migration `NNN_add_tool_result_metadata_to_messages.sql` (number per the cross-change ordering above) with three `ALTER TABLE messages ADD COLUMN … TEXT`. Register it in `run_migrations` only. No backfill.
- [ ] S3.3 Add the three `Option<String>` fields to `Message` (`crates/core/src/types/conversation.rs:33-48`), `None` in `Message::new`. Update the INSERT and SELECTs in `crates/storage/src/conversations.rs` and the in-memory store. Confirm S3.1 GREEN.
- [ ] S3.4 Add a failing runtime test: an `ok` call with data and id `call_1` saves all three fields; a `ToolOutput::error` saves `tool_status = "error"`; an executor `Err` saves `error`; a denied call saves `denied`; each `end_turn` outcome saves `ok` with no data. Confirm RED.
- [ ] S3.5 Change `make_tool_result_message` (`dispatch.rs:72-82`) to take `ToolResultMeta { tool_call_id, status, data }` and update all five call sites: `dispatch.rs:251` (finalize), `dispatch.rs:441` (denied), `turn_control.rs:185`, `:212`, `:226` (end_turn: `ok`, no data). Update `orchestrator/tests/dispatch.rs:130`. Confirm GREEN.

## S3b — `fix(runtime): keep tool metadata and turn indices when compacting`

**Layer:** server · **Depends on:** S3 · **Parallel with:** S4, S5–S9

- [ ] S3b.1 Add a failing test in `crates/runtime/src/compaction.rs` using `InMemoryConversationStore`: seed six turns, the last calling `bash` twice with distinct `tool_call_id` and data; run `maybe_compact` with `keep_recent_turns = 1`; the stored kept rows equal the originals (ids, `turn`, `tool_calls_json`, three new columns), and the summary row's `turn` is below the first kept turn. Confirm RED.
- [ ] S3b.2 Rewrite `persist_compacted` (design D12): load stored rows, keep the rows of the retained turns unchanged, prepend one summary row, `replace_history`. Remove `chat_history_to_message` if unused. Confirm GREEN.
- [ ] S3b.3 Add a web-ui test: compact, then `GET /api/conversations/{id}`; both kept `bash` summaries carry their own data and join correctly. (Needs S4; if S4 has not merged, add this test in S4.)

## S4 — `feat(web-ui): expose tool data in history and fix the per-call join`

**Layer:** server · **Depends on:** S3 · **Parallel with:** S3b, S5–S9 · **Must land before:** message-branching PR 6

- [ ] S4.1 Add failing handler tests next to `get_conversation` (`crates/web-ui/src/api/conversations.rs`): (a) two `bash` calls in a turn return distinct results and data (regression for `:443-448`); (b) `tool_status = "error"` without the "Error executing" prefix gives `error`; (c) NULL metadata omits `data` and uses the prefix rule; (d) a multibyte observation at byte 512 does not panic (`:469`); (e) `tool_call_id` is surfaced; (f) the OpenAPI schema gives `ToolCallSummary.data` `type: object`. Confirm RED.
- [ ] S4.2 Add `data: Option<serde_json::Value>` (`#[schema(value_type = Object)]`) and `tool_call_id: Option<String>` to `ToolCallSummary` (`:87-98`), both `skip_serializing_if`. Join by `tool_call_id`, then per-turn ordinal per name. Status from the column. Char-safe truncation. Confirm GREEN.
- [ ] S4.3 `make dump-openapi && make generate-flutter-client`; commit both.

## S5 — `feat(tool-executor): terminal and sources render data for bash and web-search`

**Layer:** server · **Depends on:** S1a · **Parallel with:** S1b, S2a–S4, S6–S9

- [ ] S5.1 Add failing producer tests in `crates/tool-executor/tests/render_contracts.rs`: `bash` with `echo hi`, `false`, a 1 s timeout on `echo started; sleep 5`, and a command printing 100 KB; `web-search` against a `wiremock` DuckDuckGo fixture. Each `data` validates against `output_schema()`; `exit_code` 0 / 1 / null; the timeout case has `timed_out = true` and `stdout` containing `started`; the 100 KB case serializes under 32 KiB with `stdout_truncated = true`. Confirm RED.
- [ ] S5.2 Restructure bash reading (design D4): spawn the two `read_tail` readers as tasks; on timeout kill the child and await them with a 1 s grace. Confirm the timeout tail case GREEN.
- [ ] S5.3 Replace bash's ad-hoc `data` (`bash.rs:297-301`) and `output_schema` (`:178-189`) with `ToolRenderData::Terminal`: `command`, 12 KiB render tails cut on a char boundary, `*_truncated`, `duration_ms`, `timed_out`. Attach terminal data to the timeout error.
- [ ] S5.4 Replace web-search's data (`web_search.rs:148-156`) and `output_schema` (`:54`) with `ToolRenderData::Sources`: `query`, `domain` via `reqwest::Url` host, ≤ 20 results, snippets ≤ 300 chars, `results: []` when empty. Confirm GREEN.

## S6 — `feat(tool-executor): link preview and code file data for web-fetch and file-read`

**Layer:** server · **Depends on:** S1b · **Parallel with:** S2a–S5, S7–S9

- [ ] S6.1 Add failing tests: `web-fetch` via `wiremock` for 200 with `<title>`, 302 → 200, and 404 (an error result with `link_preview`, `status = 404`); `file-read` on a `.rs` file (`language = "rust"`), on an over-limit file (`truncated = true`), and on a 200 KB file with `limit = 1000000` (content ≤ 24 KiB, observation still honours `limit`). Each validates against `output_schema()`. Confirm RED.
- [ ] S6.2 Add `crates/tool-executor/src/builtins/render_lang.rs` (design D7) with unit tests.
- [ ] S6.3 Implement `web-fetch` data (`web_fetch.rs:89-145`, including the non-2xx branch at `:93-98`): title ≤ 200 chars, content type, excerpt ≤ 280 chars. Implement `file-read` data (`file_read.rs:103-118`) with content ≤ min(`limit`, 24 KiB). Add `output_schema` to both. Confirm GREEN.

## S7 — `feat(tool-executor): code diff data for file-edit and file-write`

**Layer:** server · **Depends on:** S1b (and S6 for `render_lang.rs`; if S6 has not merged, S7 adds the file and S6 rebases) · **Parallel with:** S2a–S6, S8, S9

- [ ] S7.1 Add failing tests: one-line `file-edit` (`edit`, `added: 1`, `removed: 1`, one hunk); `file-write` to a new path (`create`); overwrite (`overwrite`); a diff over 400 lines (`truncated`, ≤ 400 lines); a single 100,000-char changed line (each `text` ≤ 512 chars, `truncated`); overwriting a non-UTF-8 file (`data: None`). Each validates against `output_schema()`. Confirm RED.
- [ ] S7.2 Add `similar = { workspace = true }` to `assistant-tool-executor`. Implement `unified_hunks(old, new, max_lines, max_line_chars)` over `similar::TextDiff::from_lines(..)`. In `file-write`, read the prior content (≤ 256 KiB) first. Attach data in `file_edit.rs:97-101` and `file_write.rs:86-90`. Confirm GREEN.

## S8 — `feat(tool-executor): data table render data for list-tasks and process list`

**Layer:** server · **Depends on:** S1b · **Parallel with:** S2a–S7, S9

- [ ] S8.1 Add failing tests: `list-tasks` with two tasks yields `data_table` with columns `name, status, mode, schedule, next_run, last_run`; with none, `rows = []`, `total_rows = 0` and the text "No scheduled tasks found."; over 200 tasks, `truncated` with full `total_rows`; `process` `action: list` with zero and one session yields `data_table`. Each validates against `output_schema()`. Confirm RED.
- [ ] S8.2 Implement `list-tasks` (`list_tasks.rs:60-100`; the early return at `:68-70` now also carries the empty table) and `process` list (`process.rs:~549-571`; cells ≤ 256 chars). Update the existing process tests that assert an array shape (`process.rs:759`, `:783`) to read `rows`. Keep poll/log data unchanged. Confirm GREEN.

## S9 — `feat(mcp-client): forward MCP structuredContent as tool data`

**Layer:** server · **Depends on:** none (meaningful once S2a merges) · **Parallel with:** all server PRs

- [ ] S9.1 Add failing tests in `crates/mcp-client/src/bridge.rs` with a fake `McpClient`: an object `structured_content` becomes `ToolOutput.data` on success and `is_error`; a non-object value or `None` gives `None`. Confirm RED.
- [ ] S9.2 Set `data` from `result.structured_content` in `McpToolHandler::run` (`bridge.rs:122-150`). Confirm GREEN.

---

## C1 — `feat(app): carry tool result data from stream and history`

**Layer:** client (`app/lib`, `assistant_ui` model) · **Depends on:** S2a, S4, CEL PR 4 (`ToolCallRecord` in `assistant_ui`) · **Parallel with:** C2

- [ ] C1.1 Add failing tests: `app/test/unit/api/stream_parse_tool_data_test.dart` (object `data` → `ToolResultEvent.data`; string `data` → null and the event still emitted; `subagent_tool_result` inner `data.data` parsed); `chat_timeline_reducers_test.dart` (`applyToolResultEvent` copies `data`); `chat_history_mapping_test.dart` (`ToolCallSummary.data` and `toolCallId` reach `ToolCallRecord`). Confirm RED.
- [ ] C1.2 Add `data` (`Map<String, Object?>?`) to `ToolResultEvent`, `SubagentToolResultEvent` (`stream_event.dart`) and `ToolCallRecord`. Parse in `api_client.dart:632-645, 751-765`. Copy in `chat_timeline_reducers.dart:111-160` and `chat_history_mapping.dart:32-55`. No new `StreamEvent` subclass; update any exhaustive switch touched. Confirm GREEN.

## C2 — `feat(app): tool result renderer registry and JSON tree in the generic view`

**Layer:** client (`assistant_ui`) · **Depends on:** S1a (fixtures), CEL PR 3 (widgetbook, `kAssistantUiElements`), CEL PR 4 (models), CEL PR 5 (`ThreadDensityScope`), CEL PR 6 (`GenericToolResultView`, for C2.3 only; land C2.1–C2.2 first if PR 6 is late) · **Parallel with:** C1

- [ ] C2.1 Add failing tests in `app/packages/assistant_ui/test/tool_results/`: `registry_test.dart` (tool-name, then kind, then `null`; a throwing `parse` → `null`; unknown kind → `null`; `denied` → `null`; `pending` → `null`; under `ThreadDensityScope(compact)` `build` returns only the summary line); `contract_fixtures_test.dart` with a `fixturesDir()` helper resolving `../../../schemas/tool-results/fixtures` from the package root and failing if it is missing or empty, plus an explicit `pendingKinds` set that C4–C7 shrink to empty. Confirm RED.
- [ ] C2.2 Implement `ToolResultRenderer<T>`, `ToolResultRendererRegistry` with `Widget? build(BuildContext, ToolCallRecord)` and internal resolution, and `defaultToolResultRenderers()` (empty). Confirm GREEN.
- [ ] C2.3 Add a failing test that CEL's `GenericToolResultView` shows a `JsonTreeView` when `data` is present and is unchanged without it. Implement `JsonTreeView` and extend `GenericToolResultView`. Confirm GREEN.
- [ ] C2.4 Widgetbook use-cases for `JsonTreeView` and the generic view with data; add to `kAssistantUiElements` and `ELEMENTS.md`. Re-run the boundary test; `pubspec.yaml` dependencies unchanged. Check that CEL's `update-goldens` Docker target mounts the repository root (goldens read `schemas/`); if it mounts only `app/`, fix the mount in this PR.

## C3 — `feat(app): install the tool result renderer scope in the chat screen`

**Layer:** client (`app/lib`) · **Depends on:** C1, C2, CEL PR 6 (`ToolResultRendererScope`), CEL PR 7 (`ChatMarkdownScope`, `SubagentBody`)

- [ ] C3.1 Add failing widget tests in `app/test/widget/chat/`: pump the chat timeline with a `ToolCallRecord` carrying fake-kind data and a test registry; the renderer shows, and activating its link reaches the `MarkdownLinkHandler` seam via `ChatMarkdownScope.onTapLink`. A subagent's inner tool call with the same data also uses the renderer. Confirm RED.
- [ ] C3.2 Build the registry once in `ChatScreen` from `defaultToolResultRenderers()` and install `ToolResultRendererScope(resolve: registry.build)` (CEL task 6.4). No `ToolCallBody` constructor change, no Riverpod in the package. Confirm GREEN.

## C4 — `feat(app): terminal and sources result views`

**Layer:** client (`assistant_ui`) · **Depends on:** C2, CEL PR 7 (`ChatMarkdownScope.onTapLink`) · **Parallel with:** C3, C5, C6, C7

- [ ] C4.1 Add failing tests: fixture parse tests for `terminal`/`sources` (remove both from `pendingKinds`); widget tests for the timeout badge, stderr in the error colour, per-stream truncation notices, the `exit 2 · 14 lines stderr` compact summary, and a source row tap calling `onTapLink` with that URL; goldens from the valid fixtures, light and dark. Confirm RED.
- [ ] C4.2 Implement `TerminalResultView`/`TerminalRenderer` and `SourcesResultView`/`SourcesRenderer`; register them. Widgetbook use-cases (valid, invalid → fallback, error, three densities); `ELEMENTS.md` rows 13, 15, 16 → Adopt (have). Confirm GREEN.

## C5 — `feat(app): link preview and plan result views`

**Layer:** client (`assistant_ui`) · **Depends on:** C2, CEL PR 7 · **Parallel with:** C3, C4, C6, C7

- [ ] C5.1 Add failing tests: fixture parse tests for `link_preview` and `plan`; a 404 badge on an `error` record; a redirect note when `final_url ≠ url`; a tap calls `onTapLink(final_url)`; plan `3/5` progress and five step statuses with distinct icons and semantics labels; goldens. Confirm RED.
- [ ] C5.2 Implement both renderers, stories, and `ELEMENTS.md` row 21 → Adopt (scoped): read-only, no built-in producer. Confirm GREEN.

## C6 — `feat(app): data table result view`

**Layer:** client (`assistant_ui`) · **Depends on:** C2 · **Parallel with:** C3, C4, C5, C7

- [ ] C6.1 Add failing tests: fixture parse tests for `data_table` (including `rows: []`, which shows an empty-state line); header tap sorts locally and a second tap reverses; at 360 px the table scrolls horizontally without page overflow; the truncation notice shows `total_rows`; goldens. Confirm RED.
- [ ] C6.2 Implement the renderer, stories, `ELEMENTS.md` row 19 → Adopt (have). Confirm GREEN.

## C7 — `feat(app): code file and code diff result views`

**Layer:** client (`assistant_ui`) · **Depends on:** C2, the chat-a11y-and-code-blocks PR that adds `CodeBlock` to `assistant_ui` · **Parallel with:** C3–C6

- [ ] C7.1 Add failing tests: fixture parse tests for `code_file` and `code_diff`; `CodeBlock` receives the `language` hint; diff lines carry `+`/`−` gutter glyphs; `+added −removed` in the compact summary; an `operation: create` label; a truncation notice; goldens. Confirm RED.
- [ ] C7.2 Implement both renderers on `CodeBlock`. No package dependency added here. `pendingKinds` is now empty. Stories; `ELEMENTS.md` row 14 → Adopt (have). Confirm GREEN.

## C8 — `docs(app): tool result views, baselines and archive`

**Layer:** both · **Depends on:** S1a–S9, C1–C7

- [ ] C8.1 Write a `docs/` page, "Tool result views": the seven contracts, the registry order, the `error` status rule (ToolOutput failures are errors; non-zero bash exit is not), and MCP opt-in via `structuredContent` + `kind`. Link `schemas/tool-results/README.md` and ADR-0020.
- [ ] C8.2 Reconcile `ELEMENTS.md` end to end and recompute the tally.
- [ ] C8.3 Regenerate the Playwright baselines for tool-call entries after rebasing (never merge PNGs by hand); record the diff in the PR body.
- [ ] C8.4 `make lint && make format && make test`, then `make lint-flutter && make test-flutter`.
- [ ] C8.5 Manual smoke: a turn calling `bash` (success, non-zero exit, timeout), `web-search`, `web-fetch` (200 and 404), `file-read`, `file-edit`, `list-tasks`; reload and confirm identical views; disconnect mid-turn, reconnect, confirm replay; trigger compaction and confirm kept tool views survive.
- [ ] C8.6 `openspec archive tool-result-renderers`.
