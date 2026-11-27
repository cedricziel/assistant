# Tool result views

A tool call in the chat is a header plus a one-line summary. When the tool
attached structured data, opening the entry shows a dedicated view — a
terminal, a source list, a diff, a table, a plan — instead of one monospace
blob. This page covers the data path, the contract shapes, how a view is
chosen, and what changed for tool failures. The wire shapes live in
[`schemas/tool-results/`](../schemas/tool-results/README.md) and the reasons
behind them in [ADR-0020](adr/adr-0020-tool-result-render-contracts.md).

## From tool output to the client

`ToolHandler::run` returns `ToolOutput { content, success, data, attachments }`.
`content` is what the model sees. `data` is a JSON object for the client only.

1. **Dispatch takes `data` before building the observation.** The observation,
   the LLM history entry, the webhook payload and the messenger adapters never
   see it. Serialized `data` over 32 KiB (`TOOL_RESULT_DATA_LIMIT_BYTES`) is
   dropped whole with a warning; it is never cut, because a half-valid JSON
   value does not parse.
2. **The `tool_result` SSE frame carries `data` only when present.** Without
   data the frame is byte-identical to before. Replay
   (`GET /api/conversations/{id}/runs/{run}/events/stream?since=N`) delivers the
   same payload, because the web handler stores each frame's payload in the
   conversation event log.
3. **The tool row persists it.** Migration
   `add_tool_result_metadata_to_messages` adds nullable `tool_call_id`,
   `tool_status` and `tool_data_json` to `messages`.
   `GET /api/conversations/{id}` rebuilds `ToolCallSummary.data` from
   `tool_data_json` and joins each tool call to its row by `tool_call_id`,
   then by ordinal among same-named calls in the turn. A turn with two `bash`
   calls used to show the last result twice.
4. **The client carries it.** `ToolResultEvent.data` and `ToolCallRecord.data`
   hold the map; the SSE parser treats a non-object `data` as absent without
   dropping the event, and history mapping copies `data` and `tool_call_id`.
5. **Compaction keeps it.** When context compaction rewrites history, the rows
   of the retained recent turns stay as they were, including their turn number
   and the three metadata columns. Rows of compacted turns are gone, as before.
   Rows written before the migration have `NULL` metadata: they show no data
   and fall back to the old `Error executing` prefix rule for status.

Subagent tool calls are the exception. `subagent_tool_result` frames carry no
`data`, and the subagent timeline keeps its compact rows; no result view runs
inside a subagent. A tool call that carries an MCP App also leaves the body to
the app's own frame.

## The seven contracts

A payload is tagged by `"kind"`. Clients choose a renderer from `kind`, never
from the tool name, because MCP tool names are not known ahead of time.

| `kind`         | Produced by                              | View                    |
| -------------- | ---------------------------------------- | ----------------------- |
| `terminal`     | `bash`                                   | `TerminalResultView`    |
| `sources`      | `web-search`                             | `SourcesResultView`     |
| `link_preview` | `web-fetch` (including non-2xx)          | `LinkPreviewResultView` |
| `code_file`    | `file-read`                              | `CodeFileResultView`    |
| `code_diff`    | `file-edit`, `file-write`                | `CodeDiffResultView`    |
| `data_table`   | `list-tasks`, `process` (`action: list`) | `DataTableResultView`   |
| `plan`         | `plan-update`, any MCP tool by opt-in    | `PlanResultView`        |

Each contract has a JSON Schema (draft 2020-12) at
`schemas/tool-results/{kind}.schema.json` and shared valid/invalid fixtures
beside it. Rust (`assistant-core`'s `ToolRenderData`) and Dart
(`assistant_ui`) both check the same files. Producer tests validate real tool
output against the same schema and built-ins return it from `output_schema()`;
`process` is the exception, because its poll and log actions carry untyped data
and only `action: list` produces a table.

Data without a `kind` — `file-glob`, `process` poll, `voice-response`,
`agent-spawn`, most MCP servers — still travels unchanged and renders in the
generic view, which shows it as a collapsible JSON tree under the result.

### Producer budgets

The LLM-facing limits are far too large for a browser payload, so each
producer has its own render budget. Every cut lands on a character boundary
and sets the contract's truncation flag.

| Contract       | Budget                                                        |
| -------------- | ------------------------------------------------------------- |
| `terminal`     | `stdout` and `stderr` tails ≤ 12 KiB each                     |
| `code_file`    | `content` ≤ min(`limit`, 24 KiB)                              |
| `code_diff`    | ≤ 400 lines, each ≤ 512 characters                            |
| `data_table`   | ≤ 200 rows, each cell ≤ 256 characters                        |
| `sources`      | ≤ 20 results, `snippet` ≤ 300 characters                      |
| `link_preview` | `title` ≤ 200 characters, `excerpt` ≤ 280 characters          |
| `plan`         | ≤ 50 steps, step `title` ≤ 200 characters, plan `title` ≤ 120 |

## Which view renders

`ToolResultRendererRegistry.build` resolves in this order:

1. A renderer registered for the record's exact tool name (an escape hatch; none
   ship).
2. The renderer for `data['kind']`.
3. The generic view.

Any other outcome — no data, an unknown `kind`, a payload that fails the
renderer's parse or throws — returns `null` and the generic view renders. The
parse is the safety boundary: a malformed payload never breaks the timeline.
A `pending` record keeps the header spinner and never gets a result view; the
generic body still shows its arguments. `denied` and `cancelled` records always
use the generic view.

`ChatScreen` builds the registry once from `defaultToolResultRenderers()` and
installs it as `ToolResultRendererScope(resolve: registry.build)`. Renderers
read density from `ThreadDensityScope` and open links only through
`ChatMarkdownScope.onTapLink`; they fetch nothing themselves.

## Failures are errors

Dispatch reports status `error` when the executor returns an error **or**
returns `Ok` with `output.success == false`. A denied call stays `denied`.

`bash` returns `success = false` for a non-zero exit code. The observation
text the model sees and the terminal data (`exit_code = n`) are unchanged; only
the status changes. Consequences you will see:

- A failing command opens expanded on its terminal view instead of showing a
  green row. So does any tool that returns `ToolOutput::error` with data, such
  as a `web-fetch` 404 or a timeout.
- Webhook `tool.result` consumers receive `status: "error"` for those calls.
- A non-zero `bash` exit during an active skill counts as a failed skill
  execution, because skill outcomes already derive from `output.success`.

Commands where a non-zero exit is an expected answer (`grep` with no match,
`test -f`) also read as errors. That is the accepted trade-off; the badge still
shows the plain exit code.

## Collapse, density and links

The entry stays collapsed behind its header at every density, for every kind.
It opens when the user taps it or when its status is `error`; a successful
call never auto-expands. This keeps long runs of tool calls from pushing the
answer off screen.

When open at `normal` or `expanded` density, the body is the renderer's full
view. At `compact` density the body is the renderer's one-line summary — for
example `exit 2 · 14 lines stderr` or `3/5` — with a **Show full result**
button that swaps in the full view and a **Hide full result** button that
restores the summary. Both are ordinary buttons with semantics labels.

Links leave through `ChatMarkdownScope.onTapLink`, the same handler as a link
in message markdown, so source rows and link previews open in the app's
browser. The views never navigate on their own.

## `plan-update`

`plan-update` lets a Persona publish its working plan without an MCP server. It
takes an optional `title` and a complete `steps` array of
`{title, status}`, where `status` is one of `pending`, `in_progress`,
`completed`, `failed` or `skipped`. Input beyond the contract budget (more than
50 steps, a plan title over 120 characters, a step title over 200 characters)
or an unknown status returns a tool error naming the field, with no data.

The tool is stateless: every call replaces the whole plan, so the latest call in
the timeline is current and earlier calls remain as history. It returns the
`plan` data plus the text observation `Plan updated: 3/5 steps completed`. It
has no side effects and is classified as read-only, so it needs no approval.

## MCP servers opt in

The MCP bridge forwards a `CallToolResult.structuredContent` object unchanged as
`ToolOutput.data`, on success and on `is_error`. A non-object value, an absent
`structuredContent` or a text-only result yields no data. The bridge does not
validate or rewrite the payload.

An MCP tool that returns `structuredContent` with a `kind` from this directory
therefore gets the same rich view. Anything else — including a payload whose
shape does not match the contract — falls back to the generic view, and the
client treats a text-only result as before. That parse-or-fallback rule is why
contracts are never versioned: they evolve in place, and a breaking change uses
a new `kind`.

## Related

- [Render contracts and fixtures](../schemas/tool-results/README.md)
- [ADR-0020: Tool Result Render Contracts](adr/adr-0020-tool-result-render-contracts.md)
- [Element index](../app/packages/assistant_ui/ELEMENTS.md)
- [Web UI](web-ui.md)
