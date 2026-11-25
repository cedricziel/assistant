## Why

Every tool call in the chat renders the same way: arguments as pretty-printed JSON and the result as a monospace string cut at 512 bytes (`app/lib/features/chat/streaming_timeline_entry.dart:231-380`). A failing `bash` run looks like a successful `web-search`, and a file edit shows only `Edited 'x.rs'`. assistant-ui's tool-ui solves this with one component per tool behind a schema check: "Parse tool output, render when valid, fail safely when not" ([tool-ui](https://github.com/assistant-ui/tool-ui)).

The structured data partly exists on the server (`ToolOutput.data`, `crates/core/src/tool.rs:50-62`) but is discarded in dispatch, never persisted, and never reaches the client. Tool-level failures (`ToolOutput::error`) are also reported as status `ok`, and so is a non-zero `bash` exit. design.md "Context" has the code evidence.

## What Changes

**Server (lands first; needs no client work)**

- `OrchestratorEvent::ToolResult` and the top-level `tool_result` frame gain optional `data`, capped at 32 KiB and dropped whole when over. Replay delivers it unchanged. Data never enters LLM context, webhooks or messenger adapters. `subagent_tool_result` frames do not carry `data`: subagent tool calls stay compact rows (decision 6 in design.md).
- Status becomes truthful: `Ok(output)` with `success == false` is `error` in the event, the persisted row and the webhook payload. `bash` reports `success = false` for a non-zero exit code, so a failing command is an `error` and force-expands.
- Tool rows persist `tool_call_id`, `tool_status` and `tool_data_json`. `ToolCallSummary` gains `data` and `tool_call_id`. History joins by id, then by per-turn ordinal, fixing the `(turn, name)` collision. Compaction keeps this metadata for the turns it retains.
- Kind-tagged render contracts (`terminal`, `sources`, `link_preview`, `code_file`, `code_diff`, `data_table`, `plan`), each with a JSON Schema and shared fixtures under `schemas/tool-results/`, checked by Rust and Dart tests. Built-ins produce them within per-producer data budgets. A new `plan-update` built-in lets a Persona publish a `plan` without MCP. MCP `structuredContent` is forwarded unchanged, so third-party servers can opt in.
- Char-safe truncation fixes two existing multibyte panics. ADR for the contract directory and the MCP opt-in.

**Client (on top of `chat-element-library`)**

- `ToolResultEvent` and `ToolCallRecord` carry `data`.
- `assistant_ui` gets `ToolResultRendererRegistry`, whose `build` is installed through `chat-element-library`'s `ToolResultRendererScope`. Top-level tool calls resolve through it; subagent timelines keep their compact rows. Resolution: tool-name override, then `data['kind']`, then the generic view (extended with a JSON tree).
- Seven result views with one-line summaries, widgetbook stories and goldens. At compact (phone) density the body is the summary; tapping it expands the full view. At normal density terminal and diff views stay collapsed behind their summary and open on error. No new package dependency: `assistant_ui` stays on the allowlist defined by `chat-element-library` (and amended by `chat-a11y-and-code-blocks`). Code views reuse that sibling's `CodeBlock`.

## Capabilities

### New Capabilities

- `tool-result-data`: structured tool output on the wire and in history: event field, cap, truthful status, persistence, join, compaction, contracts, producers, MCP forwarding.
- `tool-result-renderers`: client registry, parse-or-fallback, one view per contract, density, gallery and goldens.

### Modified Capabilities

None. `tool-call-display` still holds; renderers fill the tool-call body.

## Impact

- **Crates**: `assistant-core`, `assistant-runtime`, `assistant-storage` (one `space.db` migration, three nullable columns), `assistant-tool-executor` (adds workspace dep `similar`), `assistant-mcp-client`, `assistant-web-ui`.
- **Wire**: optional `data` on two frames and `ToolCallSummary`; regenerated `openapi.json` and `assistant_api`.
- **Visible**: tool-level failures and non-zero `bash` exits now show as errors and force-expand; `skill_executions` also records a non-zero `bash` exit as a failure; Playwright baselines move.
- **Non-goals**: rich renderers inside subagent timelines; interactive tool UIs (`tool-approval-prompts`, `generative-ui-a2ui`); inline citation markers; fetching images for link previews; server-side validation of MCP payloads; data in the trace viewer; backfilling history.
