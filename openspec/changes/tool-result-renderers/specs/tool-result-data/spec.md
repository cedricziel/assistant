## ADDED Requirements

### Requirement: Tool result events carry the tool's structured data

`OrchestratorEvent::ToolResult` SHALL carry `data: Option<serde_json::Value>` populated from `ToolOutput.data` of the executed tool. The SSE projector SHALL include it as a `data` field on the `tool_result` frame only when it is present. The `subagent_tool_result` frame SHALL NOT carry `data`: subagent tool calls render as compact rows. When `data` is absent, and for every `subagent_tool_result` frame, the frame payload SHALL be byte-identical to the payload produced before this change.

#### Scenario: Data flows into the tool_result frame

- **WHEN** a tool returns `ToolOutput` with `data = {"kind": "terminal", ...}` during a web turn
- **THEN** the emitted `tool_result` SSE frame SHALL contain `"data"` equal to that object
- **AND** the frame SHALL still contain `tool_name`, `status`, `arguments`, `result` and `tool_call_id` as before

#### Scenario: Absent data leaves the frame unchanged

- **WHEN** a tool returns `ToolOutput` with `data = None`
- **THEN** the projected `tool_result` payload SHALL NOT contain a `data` key
- **AND** the committed projection golden file for that frame sequence SHALL match byte for byte

#### Scenario: Subagent tool results carry no data

- **WHEN** a subagent's tool returns structured data
- **THEN** the `subagent_tool_result` frame's inner object SHALL NOT contain a `data` key
- **AND** the frame SHALL otherwise match the pre-change payload

#### Scenario: Denied calls carry no data

- **WHEN** a tool call is denied before execution
- **THEN** the `tool_result` frame SHALL have `status: "denied"` and no `data` key

### Requirement: Structured data is capped by dropping, not truncating

Dispatch SHALL measure the serialized size of `ToolOutput.data`. When it exceeds `TOOL_RESULT_DATA_LIMIT_BYTES` (32 KiB), dispatch SHALL drop the data entirely (emit and persist `None`) and log a warning naming the tool and the size. Dispatch SHALL NOT partially truncate a JSON value. Built-in producers SHALL keep their data under the cap with per-producer render budgets that are independent of the LLM-facing output limits: terminal tails at most 12 KiB each, `code_file.content` at most min(`limit`, 24 KiB), `code_diff` at most 400 lines of at most 512 chars, `data_table` at most 200 rows with cells of at most 256 chars, `sources` at most 20 results with snippets of at most 300 chars. Each cut SHALL fall on a character boundary and SHALL set the contract's truncation flag.

#### Scenario: Oversized data is dropped whole

- **GIVEN** a tool returns `data` whose serialization is 40 KiB
- **WHEN** the call completes
- **THEN** the `tool_result` frame SHALL have no `data` key
- **AND** the persisted tool row SHALL have `tool_data_json = NULL`
- **AND** the `result` string SHALL still be present

#### Scenario: Data at the limit is kept

- **GIVEN** a tool returns `data` whose serialization is exactly 32 KiB
- **THEN** the frame SHALL contain it unchanged

#### Scenario: Large bash output stays within the render budget

- **WHEN** `bash` runs a command printing 100 KB to stdout
- **THEN** the terminal `data` SHALL serialize to less than 32 KiB
- **AND** `stdout_truncated` SHALL be `true` and `stdout` SHALL be the last at most 12 KiB of the output

#### Scenario: file-read with a huge limit stays within the budget

- **WHEN** `file-read` reads a 200 KB file with `limit = 1000000`
- **THEN** `code_file.content` SHALL be at most 24 KiB and `truncated` SHALL be `true`
- **AND** the observation sent to the model SHALL still honour the requested `limit`

#### Scenario: A minified line in a diff

- **WHEN** `file-edit` changes a single 100,000-char line
- **THEN** each `code_diff` line's `text` SHALL be at most 512 chars and `truncated` SHALL be `true`

### Requirement: Structured data never enters LLM context

Structured tool data SHALL be visible to clients only. The observation appended to the model's history SHALL remain `ToolOutput.content` (or the error string), never the serialized `data`.

#### Scenario: History sent to the model excludes data

- **GIVEN** `bash` returns content `"stdout:\nhi"` and terminal `data`
- **WHEN** the next LLM request is built
- **THEN** the tool message in that request SHALL contain `"stdout:\nhi"` and SHALL NOT contain the serialized `data`

### Requirement: Tool result display truncation is character-safe

Truncating a tool observation to the display limit for the `result` field, both live (dispatch) and in history (`get_conversation`), SHALL cut on a UTF-8 character boundary and SHALL NOT panic.

#### Scenario: Multibyte observation at the limit

- **GIVEN** an observation whose byte 512 falls inside a multibyte character
- **WHEN** the `tool_result` frame and the history summary are built
- **THEN** neither SHALL panic
- **AND** each `result` SHALL be a valid UTF-8 prefix of the observation followed by `…`

### Requirement: Tool status reflects tool-level failure

Dispatch SHALL report status `error` when the executor returns `Ok(output)` with `output.success == false`, in the `tool_result` event, in the persisted `tool_status` and in the `tool.result` webhook payload, exactly as for an executor `Err`. A denied call SHALL remain `denied`. A `bash` command that exits with a non-zero code SHALL return `success = false`, and is therefore reported as `error`; its observation text and terminal `data` SHALL be unchanged apart from that flag. A `bash` command that exits with code 0 SHALL be `ok`.

#### Scenario: ToolOutput::error yields status error

- **GIVEN** a tool handler that returns `ToolOutput::error("boom")`
- **WHEN** the call completes
- **THEN** the `tool_result` frame SHALL have `status: "error"`
- **AND** the persisted tool row SHALL have `tool_status = "error"`
- **AND** the webhook payload SHALL have `status: "error"`

#### Scenario: MCP error result

- **WHEN** an MCP tool returns a `CallToolResult` with `is_error = true`
- **THEN** the `tool_result` frame SHALL have `status: "error"`

#### Scenario: Non-zero bash exit is a tool error

- **WHEN** `bash` runs `false`
- **THEN** the `tool_result` frame SHALL have `status: "error"` and terminal `data` with `exit_code = 1`
- **AND** the persisted tool row SHALL have `tool_status = "error"`
- **AND** the text observation sent to the LLM SHALL be identical to the observation produced for the same exit before this change

#### Scenario: Zero bash exit is ok

- **WHEN** `bash` runs `true`
- **THEN** the `tool_result` frame SHALL have `status: "ok"`

#### Scenario: Non-zero bash exit counts against an active skill

- **GIVEN** a skill is active
- **WHEN** `bash` exits non-zero
- **THEN** the skill execution outcome SHALL be recorded as a failure, as for any tool returning `success = false`

### Requirement: Tool rows persist call id, status and data

A tool-role message SHALL persist `tool_call_id` (when the provider supplied one), `tool_status` (`ok` | `error` | `denied`) and `tool_data_json` (the capped data, serialized) in new nullable `messages` columns. Both the SQLite and in-memory conversation stores SHALL round-trip them. `end_turn` tool rows SHALL persist `tool_status = "ok"` and no data.

#### Scenario: Successful call is persisted with metadata

- **WHEN** a `web-search` call with provider id `call_7` succeeds
- **THEN** the saved tool row SHALL have `tool_call_id = "call_7"`, `tool_status = "ok"` and `tool_data_json` equal to the serialized sources data

#### Scenario: In-memory store parity

- **WHEN** the same message is saved to `InMemoryConversationStore` and `SqliteConversationStore` and loaded back
- **THEN** both SHALL return identical `tool_call_id`, `tool_status` and `tool_data_json`

### Requirement: Compaction preserves tool metadata for retained turns

When context compaction rewrites a conversation's stored history, the rows of the retained recent turns SHALL be kept unchanged, including `turn`, `tool_calls_json`, `tool_call_id`, `tool_status` and `tool_data_json`. Compaction SHALL NOT renumber the `turn` of retained rows.

#### Scenario: History after compaction

- **GIVEN** a conversation whose last retained turn called `bash` twice with distinct data
- **WHEN** compaction runs and the conversation is then loaded via `GET /api/conversations/{id}`
- **THEN** both `bash` summaries SHALL carry their own `data`, `tool_call_id` and status
- **AND** they SHALL be joined to the correct entries of that turn's `tool_calls_json`

### Requirement: History exposes per-call data with a correct join

`ToolCallSummary` SHALL include optional `data` (JSON object) and `tool_call_id`. `get_conversation` SHALL join each entry of an assistant row's `tool_calls_json` to its tool row by `tool_call_id` when both sides have one. Otherwise it SHALL join by ordinal position among same-named tool rows within the turn. Status SHALL come from `tool_status` when present.

#### Scenario: Two calls to the same tool in one turn

- **GIVEN** a turn that called `bash` twice, with results "A" then "B"
- **WHEN** the conversation is loaded
- **THEN** the first `bash` summary SHALL carry result "A" and its own data
- **AND** the second SHALL carry result "B" and its own data

#### Scenario: Status comes from the column

- **GIVEN** a tool row with `tool_status = "error"` whose content does not start with `"Error executing"`
- **WHEN** the conversation is loaded
- **THEN** the summary status SHALL be `"error"`

#### Scenario: Rows from before the migration

- **GIVEN** a tool row with all three new columns `NULL`
- **WHEN** the conversation is loaded
- **THEN** the summary SHALL omit `data` and derive status by the previous prefix rule

### Requirement: Render data contracts are kind-tagged and schema-checked

Structured data intended for a rich client view SHALL be a JSON object with a string `kind` naming one of the contracts `terminal`, `sources`, `link_preview`, `code_file`, `code_diff`, `data_table` or `plan`. Each contract SHALL have a JSON Schema (2020-12) at `schemas/tool-results/{kind}.schema.json`, at least one valid and one invalid fixture under `schemas/tool-results/fixtures/{kind}/`, and a variant of the `ToolRenderData` enum in `assistant-core` serialized with `kind` as the tag. Data without a `kind` SHALL remain permitted and SHALL be transported unchanged.

#### Scenario: Every variant has a schema and fixtures

- **WHEN** the contract test enumerates `ToolRenderData` variants
- **THEN** each SHALL have a schema file and at least one `valid-*.json` and one `invalid-*.json` fixture

#### Scenario: Fixtures agree with the schema and the Rust type

- **WHEN** every fixture is validated against its schema and deserialized into `ToolRenderData`
- **THEN** every `valid-*` fixture SHALL pass both
- **AND** every `invalid-*` fixture SHALL fail schema validation

#### Scenario: Built-in output matches its declared schema

- **WHEN** a built-in tool that produces a contract runs in the producer test
- **THEN** its `ToolOutput.data` SHALL validate against that tool's `output_schema()`

### Requirement: Built-in tools produce render data

The following built-ins SHALL populate `ToolOutput.data` with the named contract, on success and on failures where the data is meaningful. Each SHALL return that contract's schema from `output_schema()`.

| Tool                                     | Contract       | Required content                                                                                                                                                              |
| ---------------------------------------- | -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bash`                                   | `terminal`     | `command`, `exit_code` (null when timed out), `stdout`, `stderr`, `stdout_truncated`, `stderr_truncated`, `duration_ms`, `timed_out`                                          |
| `web-search`                             | `sources`      | `query`, `results[{title, url, domain, snippet?}]`                                                                                                                            |
| `web-fetch`                              | `link_preview` | `url`, `final_url`, `status` (HTTP), `title?`, `content_type?`, `excerpt?` (≤ 280 chars)                                                                                      |
| `file-read`                              | `code_file`    | `path`, `language?`, `content`, `truncated`, `total_chars`                                                                                                                    |
| `file-edit`, `file-write`                | `code_diff`    | `path`, `language?`, `operation` (`edit` / `create` / `overwrite`), `hunks[{old_start, old_lines, new_start, new_lines, lines[{op, text}]}]`, `added`, `removed`, `truncated` |
| `list-tasks`, `process` (`action: list`) | `data_table`   | `columns[{key, label, type?}]`, `rows[]`, `truncated`, `total_rows`                                                                                                           |
| `plan-update`                            | `plan`         | `title?` (≤ 120 chars), `steps[{title, status}]` (≤ 50 steps, `title` ≤ 200 chars, `status` one of `pending`, `in_progress`, `completed`, `failed`, `skipped`)                  |

#### Scenario: Failing bash command

- **WHEN** `bash` runs `false`
- **THEN** `data.kind` SHALL be `"terminal"` with `exit_code = 1` and `timed_out = false`
- **AND** the result SHALL be an error (wire status `error`)

#### Scenario: Timed-out bash command

- **WHEN** a `bash` command that printed `started` exceeds its timeout
- **THEN** the result SHALL be an error (wire status `error`) carrying terminal data with `timed_out = true` and `exit_code = null`
- **AND** `stdout` SHALL contain `started` when the output readers finish within the 1 s grace period after the kill, and SHALL be empty otherwise

#### Scenario: web-fetch on HTTP 404

- **WHEN** `web-fetch` receives a 404 response
- **THEN** the result SHALL be an error (wire status `error`) carrying `link_preview` data with `status = 404` and `final_url` set

#### Scenario: plan-update publishes a plan

- **WHEN** `plan-update` is called with a title and five steps, three `completed`
- **THEN** `data` SHALL be a `plan` with those steps and statuses
- **AND** the text observation SHALL read `Plan updated: 3/5 steps completed`
- **AND** the call SHALL need no approval and have no side effect

#### Scenario: plan-update rejects invalid input

- **WHEN** `plan-update` is called with a step whose `status` is `"done"`, or with more than 50 steps
- **THEN** the result SHALL be an error naming the problem (wire status `error`) and `data` SHALL be `None`

#### Scenario: No scheduled tasks

- **WHEN** `list-tasks` runs and there are no tasks
- **THEN** `data` SHALL be a `data_table` with the task columns, `rows = []` and `total_rows = 0`
- **AND** the content SHALL still read "No scheduled tasks found."

#### Scenario: Empty process list

- **WHEN** `process` runs with `action: list` and there are no sessions
- **THEN** `data` SHALL be a `data_table` with `rows = []` and `total_rows = 0`

#### Scenario: file-edit produces a diff

- **WHEN** `file-edit` replaces one line in a Rust file
- **THEN** `data` SHALL be a `code_diff` with `operation = "edit"`, `language = "rust"`, `added = 1`, `removed = 1` and one hunk

#### Scenario: file-write creates a file

- **WHEN** `file-write` writes a 3-line file to a path that did not exist
- **THEN** `data` SHALL be a `code_diff` with `operation = "create"` and `added = 3`

#### Scenario: Large diff is truncated semantically

- **WHEN** a diff exceeds 400 lines
- **THEN** `data.truncated` SHALL be `true` and the hunks SHALL contain at most 400 lines

#### Scenario: Non-UTF-8 file yields no diff data

- **WHEN** `file-write` overwrites a binary file
- **THEN** `ToolOutput.data` SHALL be `None`

### Requirement: MCP structuredContent is forwarded as tool data

The MCP tool bridge SHALL set `ToolOutput.data` to the `structuredContent` of the MCP `CallToolResult` when that value is a JSON object, for both success and error results. Non-object values and absent `structuredContent` SHALL yield `data = None`. The bridge SHALL NOT rewrite or validate the payload.

#### Scenario: MCP tool returns a plan

- **WHEN** an MCP tool returns `structuredContent = {"kind": "plan", "steps": [...]}`
- **THEN** the `tool_result` frame for `mcp__{server}__{tool}` SHALL carry that object as `data`

#### Scenario: MCP tool returns structured content without a kind

- **WHEN** an MCP tool returns `structuredContent = {"temperature": 21}`
- **THEN** `data` SHALL be `{"temperature": 21}` unchanged

#### Scenario: MCP tool returns no structured content

- **WHEN** an MCP tool returns only text content
- **THEN** `data` SHALL be absent

### Requirement: Tool data survives replay and reconnect

Because `tool_result` frames are persisted to the conversation event log with their full payload, replaying a run via `GET /api/conversations/{id}/runs/{run_id}/events/stream?since=N` SHALL deliver the same `data` that the live stream delivered.

#### Scenario: Reconnect mid-turn

- **GIVEN** a run whose `tool_result` frame at sequence 7 carried terminal data
- **WHEN** a client replays with `since=6`
- **THEN** the replayed sequence-7 frame SHALL contain the identical `data` object

### Requirement: The OpenAPI document describes tool data

`openapi.json` SHALL document `SseToolResultEvent.data` and `ToolCallSummary.data` as optional objects (`#[schema(value_type = Object)]`, since a bare `Option<serde_json::Value>` is emitted untyped), `SseToolResultEvent.arguments` as an object, and `tool_call_id` on both. The description SHALL reference `schemas/tool-results/`. The generated `assistant_api` client SHALL be regenerated from it.

#### Scenario: Spec matches the projector

- **WHEN** `make dump-openapi` is run after this change
- **THEN** the committed `openapi.json` SHALL be unchanged
