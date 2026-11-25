## Context

A tool call travels through four places. Structured output survives none of them:

1. **Tool** returns `ToolOutput { content, success, data, attachments }` (`crates/core/src/tool.rs:50-62`). Only `bash` (`crates/tool-executor/src/builtins/bash.rs:297-309`), `web-search` (`web_search.rs:148-156`), `file-glob`, `process`, `agent-spawn` and `voice-response` set `data`. `ToolHandler::output_schema()` exists (`core/src/tool.rs:136-141`) and four built-ins implement it (`bash.rs:178`, `web_search.rs:54`, `file_glob.rs:53`, `voice_response.rs:57`), but nothing calls it.
2. **Dispatch** (`crates/runtime/src/orchestrator/dispatch.rs:121-300`) reads `output.data` only for the `voice-response` `audio_id` (`:164-177`). It turns the output into an `observation: String`, appends that to LLM history (`:249`), persists a tool-role `Message` containing only that string (`:250-254`, built by `make_tool_result_message`, `:72-82`), and sends `OrchestratorEvent::ToolResult { result: Some(observation[..512]) }` (`:274-292`), which panics on a multibyte boundary. `OrchestratorEvent::ToolResult` has no data field (`crates/runtime/src/orchestrator/stream_event.rs:31-43`). The denied path (`:440-459`) works the same way. `make_tool_result_message` has five call sites: `dispatch.rs:251`, `dispatch.rs:441`, and the three `end_turn` sites in `turn_control.rs:185,212,226`.
3. **Status**: `tool_status` starts as `"ok"` (`dispatch.rs:141`) and becomes `"error"` only in the `Err(err)` arm (`:236`). `ToolOutput::error(..)` returns go through `Ok(output)` (`:181-227`) and stay `"ok"`: web-fetch non-2xx (`web_fetch.rs:93-98`), the bash timeout (`bash.rs:256-262`), file-edit "old_string not found", MCP `is_error`, and confinement refusals. The webhook payload (`:255-260`) copies the same status.
4. **Projection**: `SseProjector` maps `ToolResult` to `tool_result` (`crates/runtime/src/projection/mod.rs:77-97`) and `project_subagent_inner` to `subagent_tool_result` (`:194-210`). The web handler persists each frame's `log_payload` to the 24 h event log (`crates/web-ui/src/api/messages.rs:333-340`; terminal events at `:452-456`). The projection tests have totality and name checks but no byte-level golden.
5. **History**: `get_conversation` rebuilds `ToolCallSummary { name, arguments, result, status }` by joining tool-role rows to assistant `tool_calls_json` on `(turn, name)` in a `HashMap` (`crates/web-ui/src/api/conversations.rs:443-500`), so two same-named calls in one turn both show the last result. Status is inferred from the `"Error executing"` prefix (`:476-486`). `r[..TOOL_RESULT_HISTORY_LIMIT]` (`:469`) panics on a multibyte boundary.
6. **Compaction**: `persist_compacted` (`crates/runtime/src/compaction.rs:355-369`) calls `replace_history`, which deletes all messages (`crates/storage/src/conversations.rs:369-376`), and re-inserts rows rebuilt from `ChatHistoryMessage` (`compaction.rs:343-347`), which carry only content and name, with `turn = i` per message. Any per-row metadata, and the turn grouping that the history join relies on, is lost.

On the client, `ToolResultEvent` (`app/lib/api/models/stream_event.dart:133-170`) and `ToolCallRecord` (`app/lib/features/chat/chat_models.dart:24-56`; `chat-element-library` PR 4 moves it to `assistant_ui`) have no `data`. `_toolDetails` renders arguments and result as text (`streaming_timeline_entry.dart:324-380`); `chat-element-library` PR 6 lifts it into `GenericToolResultView`.

## Goals / Non-Goals

**Goals:** carry `ToolOutput.data` end to end, live, after reload and after compaction; report tool-level failures truthfully; define a small set of render contracts; render each with a dedicated widget that refuses invalid input and falls back; keep `assistant_ui` on its dependency allowlist.

**Non-Goals:** interactive tool UIs (`tool-approval-prompts`, `generative-ui-a2ui`); syntax highlighting itself (`chat-a11y-and-code-blocks`); sending data to the LLM, webhooks or messenger adapters.

## Decisions

### D1. One `data` field on the existing event, not a new event type

`OrchestratorEvent::ToolResult` gains `data: Option<Value>`. Both SSE frames add `"data": {...}` only when the value is `Some`. S2a adds a projection golden test (a representative frame sequence serialized to a committed JSON file) so that "without data the frame is unchanged" is checked from then on.

_Alternative:_ a separate `tool_result_data` SSE event. Rejected: the client would correlate two frames per call and replay ordering would matter. No new `StreamEvent` subclass is added, so no sibling's exhaustive switch breaks; C1 only adds a field.

### D2. Render contracts are tagged by `kind` inside `data`

Contract payloads carry `"kind": "terminal" | "sources" | "link_preview" | "code_file" | "code_diff" | "data_table" | "plan"`. On the Rust side this is `enum ToolRenderData` with `#[serde(tag = "kind", rename_all = "snake_case")]` in `crates/core/src/tool_render.rs`. `ToolOutput::with_render(ToolRenderData)` serializes it with `serde_json::to_value(..).ok()` (panic-free).

The client registry resolves: (1) a renderer registered for the exact tool name (escape hatch; none ship), (2) a renderer for `data['kind']`, (3) the generic view. Step 2 is the main key because MCP tool names are `mcp__{server}__{tool}` (`crates/mcp-client/src/bridge.rs:19-21`) and unknown ahead of time.

_Alternative:_ a separate `render_kind` field. Rejected: two sources of truth, and MCP `structuredContent` could not set it.

Data without a `kind` keeps flowing (`file-glob`, `process` poll, `voice-response`, `agent-spawn`) and renders in the generic view's JSON tree. No version field: contracts evolve in place and an old client meeting a new shape fails its parse and falls back. ADR-0020 records this, because third-party MCP servers build against it (D10).

### D3. JSON Schema files are the contract; Rust and Dart check them against shared fixtures

- `schemas/tool-results/{kind}.schema.json` (JSON Schema 2020-12, top-level `"additionalProperties": true`), with `fixtures/{kind}/valid-*.json` and `invalid-*.json` beside them.
- Rust: an `assistant-core` test validates every fixture with `jsonschema` (workspace dependency, `Cargo.toml:150`), deserializes valid ones into `ToolRenderData`, and asserts one schema per variant. An `assistant-tool-executor` test validates each producer's real output against `handler.output_schema()`, which returns the same file via `include_str!` + `serde_json::from_str(..).ok()`.
- Dart: `app/packages/assistant_ui/test/tool_results/contract_fixtures_test.dart` reads the fixtures with `dart:io` (test code only; the boundary test scans `lib/`). The directory is resolved by one helper, `fixturesDir()`, as `../../../schemas/tool-results/fixtures` relative to the package root (the cwd of `flutter test`), and the helper asserts the directory exists and is non-empty, so a wrong path fails loudly.
- CI: `.github/workflows/flutter.yml` runs Flutter tests only when the `flutter` paths filter matches (`'app/**'`, `'openapi.json'`, the workflow file; `flutter.yml:36-40`). S1a adds `'schemas/tool-results/**'` so a schema-only PR runs the Dart contract test. The Docker-based `update-goldens` target that `chat-element-library` PR 1.4 adds must mount the repository root, not only `app/`, because goldens decode the shared fixtures; C2 checks this.

_Alternative:_ emit contracts into `openapi.json` and consume generated Dart models. Rejected: `assistant_ui` must not import `assistant_api`, and `assistant-core` has no utoipa dependency. `openapi.json` documents `data` as an object via `#[schema(value_type = Object)]` (a bare `Option<serde_json::Value>` produces an untyped schema, as `ToolCallSummary.arguments` shows today).

### D4. Cap by dropping; producers stay under it with their own budgets

`TOOL_RESULT_DATA_LIMIT_BYTES = 32 * 1024` applies to the serialized `data`. Over the cap, dispatch logs `warn!(tool, size)` and emits and persists `None`. Cutting JSON generically would produce half-valid shapes.

The LLM-facing caps are much larger (`bash_max_output_chars` defaults to 200,000 per stream, `crates/core/src/types/permissions.rs:118`; file-read's `limit` has no maximum, `file_read.rs:60-64`), so producers must not reuse them. Each producer has its own render budget, cuts on a char boundary and sets a truncation flag:

| Producer       | Budget                                                                                                                                                                 |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `terminal`     | `stdout` and `stderr` tails ≤ 12 KiB each (last bytes kept); `*_truncated = true` when the render tail is shorter than the captured text or the capture itself was cut |
| `code_file`    | `content` ≤ min(`limit`, 24 KiB)                                                                                                                                       |
| `code_diff`    | ≤ 400 diff lines, each line ≤ 512 chars (`…` appended); `truncated = true` if either applies                                                                           |
| `data_table`   | ≤ 200 rows, each cell string ≤ 256 chars                                                                                                                               |
| `sources`      | ≤ 20 results, `snippet` ≤ 300 chars                                                                                                                                    |
| `link_preview` | `excerpt` ≤ 280 chars, `title` ≤ 200 chars                                                                                                                             |

With these budgets every built-in stays under 32 KiB; the dispatch cap is a backstop for MCP payloads.

**Bash timeout.** Today the read futures are dropped with the `tokio::time::timeout` (`bash.rs:245-262`), so nothing is captured. S5 spawns the two `read_tail` readers as tasks, kills the child on timeout, then awaits the readers for a 1 s grace period. The terminal data carries whatever tails they captured, or empty strings if a grandchild keeps a pipe open past the grace.

### D5. Persist on the tool row; join by id, then by ordinal

One migration, `add_tool_result_metadata_to_messages`, adds `tool_call_id TEXT`, `tool_status TEXT` and `tool_data_json TEXT` to `messages`. It is registered only in `run_migrations`'s list (`crates/storage/src/lib.rs:298ff`); `seed_database_through_016` (`:562`) is a test fixture and must not change. The number comes from the cross-change pre-allocation; without one, the next free after `045_skill_executions.sql`. `Message` gains three `Option<String>` fields, `None` in `Message::new`. Both stores round-trip them (ADR-0009 pair).

`make_tool_result_message` takes `ToolResultMeta { tool_call_id, status, data }`. Call sites:

| Site                                      | `tool_status`        | data        |
| ----------------------------------------- | -------------------- | ----------- |
| `dispatch.rs:251` (finalize)              | `ok` / `error` (D11) | capped data |
| `dispatch.rs:441` (denied)                | `denied`             | none        |
| `turn_control.rs:185` (end_turn deferred) | `ok`                 | none        |
| `turn_control.rs:212` (end_turn rejected) | `ok`                 | none        |
| `turn_control.rs:226` (end_turn accepted) | `ok`                 | none        |

`get_conversation` joins `tool_calls_json` entries to tool rows by `tool_call_id`, then per turn by ordinal among same-named rows (dispatch is sequential, so order matches). Status comes from `tool_status`; the prefix rule stays only for `NULL` rows. The event log alone is not enough because it expires after 24 h.

### D6. Diffs are computed on the server

`file-edit` holds `content` and `patched` in memory (`file_edit.rs:97-101`). `file-write` reads the prior file (≤ 256 KiB) before writing and treats a missing file as `create`. Hunks come from `similar::TextDiff::from_lines(..).unified_diff().context_radius(3)` (`similar` 3.x is in the workspace, `Cargo.toml:204`, lock `3.2.1`). Non-UTF-8 content yields no `data`.

_Alternative:_ diff on the client. Rejected: twice the payload and a Dart diff dependency outside the allowlist.

### D7. Language is a server-side hint from the extension

`crates/tool-executor/src/builtins/render_lang.rs` maps extensions to ids (`rs, dart, ts, tsx, js, py, go, rb, java, kt, swift, c, h, cpp, cs, sh, bash, zsh, json, yaml, yml, toml, md, html, css, sql, xml`). The client passes the id to `chat-a11y-and-code-blocks`' `CodeBlock`, which owns the highlighter and any allowlist amendment.

### D8. MCP: forward `structuredContent` unchanged

`McpToolHandler::run` (`bridge.rs:122-150`) sets `output.data = result.structured_content` when it is a JSON object (rmcp 3.11.0, `CallToolResult.structured_content: Option<Value>`, `model.rs:4187`), for success and `is_error`. The client parse check is the safety boundary.

### D9. Client renderer API: the registry feeds `chat-element-library`'s scope

`chat-element-library` Decision 3 defines `ToolResultRendererScope(resolve: ToolResultBuilder)` with `typedef ToolResultBuilder = Widget? Function(BuildContext, ToolCallRecord)`, an `InheritedWidget` so `SubagentBody`'s inner tool calls resolve too. This change plugs into it and adds no constructor parameter to `ToolCallBody`.

```dart
abstract class ToolResultRenderer<T extends Object> {
  const ToolResultRenderer();
  String get kind;
  T? parse(Map<String, Object?> data);            // total by contract
  Widget summary(BuildContext context, T model);  // one line, compact density
  Widget build(BuildContext context, T model, ToolCallStatus status);
}

class ToolResultRendererRegistry {
  const ToolResultRendererRegistry({List<ToolResultRenderer> byKind, Map<String, ToolResultRenderer> byToolName});
  Widget? build(BuildContext context, ToolCallRecord record);  // the ToolResultBuilder
}
```

- `build` resolves internally (tool name, kind, parse inside try/catch) and returns `null` for the fallback, which makes `ToolCallBody` render `GenericToolResultView`.
- Density comes from `ThreadDensityScope` (CEL PR 5); links leave through `ChatMarkdownScope.onTapLink` (CEL PR 7, a `String` URL). No separate context object duplicates them.
- **Compact summary.** At `TimelineDensity.compact`, `build` returns the renderer's one-line `summary` widget instead of the full view. It renders inside the body, under the shell's existing label, so `ToolResultBuilder` keeps its signature and `chat-element-library` needs no change. This is the agreed interface with that change.
- Status: `denied` returns `null`; `pending` returns `null` (the body shows the spinner as today); `error` runs the renderer when data parses, and CEL's force-expand-on-error stays.
- **Fallback.** C2 extends CEL's `GenericToolResultView` with a collapsible `JsonTreeView` of `data` when present, rather than defining a second fallback.
- `ChatScreen` (C3) installs `ToolResultRendererScope(resolve: registry.build)`, as CEL task 6.4 anticipates, with the registry built once from `defaultToolResultRenderers()`. `onTapLink` is already the `MarkdownLinkHandler` installed by CEL PR 7.2.

### D10. ADR-0020 for the render contracts

The change adds a cross-language contract directory and a public extension point: third-party MCP servers opt in by returning `structuredContent` with a `kind`, and the no-version, evolve-in-place policy binds them. `docs/adr/adr-0020-tool-result-render-contracts.md` records: the `schemas/tool-results/` directory as source of truth, the `kind` tag, no versioning with parse-or-fallback, drop-not-truncate, and MCP opt-in. 0017–0019 are reserved for siblings; 0020 is the next free number at writing and is confirmed at implementation.

### D11. Tool status reflects `ToolOutput.success`

Dispatch maps `Ok(output) if !output.success` to `"error"` in the event, the persisted `tool_status` and the webhook payload, and sets the span's `tool_status` the same way. `Err` stays `"error"`, denial stays `"denied"`.

A non-zero bash exit stays `ok`: `bash` returns `ToolOutput::success` for any exit code (`bash.rs:309`), and many non-zero exits are expected (`grep` with no match). The terminal view's exit badge shows the failure. A bash timeout or spawn failure is `ToolOutput::error` and becomes `error`.

Visible change: more entries force-expand on error (`streaming_timeline_entry.dart:257`), so Playwright baselines with failing tools move in S2a.

_Alternative:_ keep the status and key error scenarios on payload fields (`link_preview.status >= 400`). Rejected: the wire would keep lying to clients and webhooks.

### D12. Compaction keeps tool metadata for retained turns

`persist_compacted` stops rebuilding rows from `ChatHistoryMessage`. It loads the stored rows, keeps the rows of the `keep_recent_turns` most recent turns unchanged (ids, `turn`, the three new columns, `tool_calls_json`), and writes one summary row with a `turn` below the first kept turn, then the kept rows, via `replace_history`. Identifying the kept rows uses the same turn boundary as `find_split_for_turns`, applied to stored rows. Rows of compacted turns are gone, as today.

_Alternative:_ add the three fields to `ChatHistoryMessage::ToolResult`. Rejected: it puts render data on the LLM-facing type, and it does not fix the per-message `turn` renumbering, which breaks the history join on its own.

## Cross-change sequencing

- **Message struct lane** (`Message` and the INSERT/SELECT lists in `crates/storage/src/conversations.rs`): message-branching PR 1, then this change's S3, then message-feedback PR 3, then message-branching PR 8.
- **dispatch.rs denied path**: tool-approval-prompts PR 6 (`authorize_tool_call` signature) merges before S3.
- **`finalize_tool`**: generative-ui-a2ui PR 6 depends on S2a and S3; its surface hook `take()`s `output.data` before forwarding and persistence, so surfaces are not duplicated into the event log and `tool_data_json`.
- **`get_conversation`**: S4 lands before message-branching PR 6, which then uses the `tool_call_id`/ordinal join.
- **Consumers of `tool_call_id`**: tool-approval-prompts PR 17 and generative-ui-a2ui PR 11 depend on S4 and C1.
- **Client stream lane**: C1 adds a field to existing `StreamEvent` subclasses only; it lands in the wave the stack plan assigns and updates any exhaustive switch it touches.
- **Playwright baselines**: never merge PNGs by hand. After rebasing, regenerate them (e2e-testing skill) and record the diff in the PR body.

## Risks / Trade-offs

- **Secrets in `data`.** More bash output reaches the browser, the event log and `space.db`. _Mitigation:_ it is a subset of what the LLM already received, under per-producer budgets.
- **Fixture drift.** _Mitigation:_ the producer test validates real outputs, a schema-per-variant test catches missing fixtures, and the Flutter paths filter includes `schemas/tool-results/**`.
- **Status change ripples.** Webhook consumers now see `error` for tool-level failures. _Mitigation:_ this is a correction; documented in the PR and the docs page.
- **Compaction rewrite.** D12 touches a path that deletes all messages. _Mitigation:_ a test compacts, reloads and checks kept turns row by row.
- **Migration number collisions.** _Mitigation:_ pre-allocated numbers; one registration list.
- **Larger history payloads.** Up to 32 KiB per tool call. _Mitigation:_ lazy loading is a follow-up (open question 3).

## Open Questions

1. **Plan producer.** No built-in emits `plan`. Should Personas get a `plan-update` built-in so the plan view appears without MCP? The contract and renderer ship either way.
2. **Default expansion.** At `normal` density, should terminal and diff views open expanded on completion, or stay collapsed as tool calls do today? This proposal keeps today's behaviour.
3. **Lazy history data.** Add `GET /api/conversations/{id}/messages/{mid}/tool-data` if payloads grow? Deferred.
