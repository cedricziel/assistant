# tool-result-renderers Specification

## Purpose

The client side of structured tool output: the renderer registry, parse-or-fallback resolution, one view per render contract, density and collapse behaviour, and gallery and golden coverage.

## Requirements

### Requirement: The client carries tool data from stream and history to the tool-call record

`ToolResultEvent` and `ToolCallRecord` SHALL expose an optional `data` map. The SSE parser SHALL read it from `tool_result` frames (`subagent_tool_result` frames carry none and `SubagentToolResultEvent` SHALL NOT gain a `data` field), the timeline reducer SHALL copy it onto the resolved record, and history mapping SHALL copy `ToolCallSummary.data` and `tool_call_id` onto the reconstructed record. A frame whose `data` is not a JSON object SHALL be handled as if `data` were absent; the event itself SHALL NOT be dropped.

#### Scenario: Live frame populates the record

- **WHEN** a `tool_result` frame with `data = {"kind": "terminal", ...}` resolves a pending tool-call entry
- **THEN** that entry's `ToolCallRecord.data` SHALL equal the frame's `data`

#### Scenario: Reloaded history populates the record

- **WHEN** a conversation whose history contains a `ToolCallSummary` with `data` is opened
- **THEN** the reconstructed tool-call entry SHALL carry the same `data` and `toolCallId`

#### Scenario: Malformed data does not drop the event

- **WHEN** a `tool_result` frame has `"data": "oops"`
- **THEN** the tool-call entry SHALL resolve with its status and result
- **AND** `ToolCallRecord.data` SHALL be null

### Requirement: A pure registry resolves a renderer or falls back

`assistant_ui` SHALL export `ToolResultRenderer<T>`, `ToolResultRendererRegistry` and `defaultToolResultRenderers()`. Each renderer declares a `kind`, a total `parse` from the data map to a typed model or null, a one-line `summary`, and a `build`. The registry SHALL expose `Widget? build(BuildContext, ToolCallRecord)`, matching `chat-element-library`'s `ToolResultBuilder` typedef, and SHALL return `null` whenever the generic view applies. Resolution SHALL be internal to the registry, in this order:

1. A renderer registered for the record's exact tool name.
2. The renderer registered for `data['kind']`.
3. The generic fallback.

A `parse` that returns null or throws SHALL produce the fallback. The registry SHALL read density from `ThreadDensityScope` and SHALL send links only through `ChatMarkdownScope.onTapLink`; it SHALL NOT define its own density or link-callback inputs. The registry and renderers SHALL be consistent with the `chat-element-library` purity rule. This change SHALL NOT add any dependency to `assistant_ui`.

#### Scenario: Kind selects the renderer

- **GIVEN** a completed `bash` record whose data is a valid `terminal` payload
- **WHEN** the tool-call body is built
- **THEN** `TerminalResultView` SHALL render

#### Scenario: Invalid payload falls back

- **GIVEN** a record whose data has `kind: "terminal"` but `exit_code: "one"`
- **WHEN** the tool-call body is built
- **THEN** the generic fallback SHALL render
- **AND** no exception SHALL surface

#### Scenario: Unknown kind falls back

- **GIVEN** data with `kind: "weather"`
- **THEN** the generic fallback SHALL render

#### Scenario: Tool-name override wins

- **GIVEN** a registry with a renderer registered for tool name `"mcp__acme__deploy"` and data with `kind: "terminal"`
- **WHEN** a record for `mcp__acme__deploy` is resolved
- **THEN** the tool-name renderer SHALL be chosen

#### Scenario: MCP payload with a known kind

- **GIVEN** a record for `mcp__notes__outline` whose data is a valid `plan` payload
- **THEN** `PlanResultView` SHALL render

#### Scenario: Package dependencies unchanged

- **WHEN** `app/packages/assistant_ui/pubspec.yaml` is inspected after this change
- **THEN** its `dependencies` SHALL be exactly those allowed by the `chat-element-library` allowlist at that time

### Requirement: The generic fallback shows structured data as a tree

When no renderer applies, `chat-element-library`'s `GenericToolResultView` SHALL show the arguments and result as today. This change SHALL extend that view, not add a second fallback: when `data` is present it SHALL also show `data` as a collapsible JSON tree, so structure is never flattened into a string.

#### Scenario: Data without a kind

- **GIVEN** a `file-glob` record with data `{"paths": [...], "truncated": false}`
- **WHEN** the tool-call body is expanded
- **THEN** a JSON tree with `paths` and `truncated` nodes SHALL render beneath the result

#### Scenario: No data

- **GIVEN** a record with no `data`
- **THEN** the body SHALL render arguments and result exactly as the pre-change generic view

### Requirement: Renderers honour status and density

The tool-call entry SHALL stay collapsed behind its header at every density, for every kind including `terminal` and `code_diff`, and SHALL open only when the user activates it or when its status is `error` (the existing force-expand-on-error behaviour). A completed `ok` call SHALL NOT auto-expand.

When the entry is open at `normal` or `expanded` density, the body SHALL be the resolved renderer's full view.

At `TimelineDensity.compact`, `registry.build` SHALL return a widget that shows the renderer's one-line `summary` with an expand affordance. Activating it SHALL replace the summary with the renderer's full view; activating again SHALL restore the summary. The affordance SHALL be a button with a semantics label ("Show full result" / "Hide full result"). The summary renders in the tool-call body, and the entry header label stays as `chat-element-library` defines it.

A `denied` record SHALL always use the fallback. An `error` record SHALL use the renderer when the data parses. A `pending` record SHALL render no body.

#### Scenario: Compact summary

- **GIVEN** a terminal payload with `exit_code = 2` and 14 stderr lines
- **WHEN** rendered at compact density
- **THEN** the body SHALL show a single line such as `exit 2 · 14 lines stderr` and an expand affordance, and not the terminal view

#### Scenario: Compact tap expands the full view

- **GIVEN** the compact summary from the previous scenario
- **WHEN** the user activates it
- **THEN** `TerminalResultView` SHALL replace the summary line
- **AND** activating the "Hide full result" control SHALL restore the summary line

#### Scenario: Successful terminal run stays collapsed

- **GIVEN** a completed `bash` record with status `ok` and a valid `terminal` payload, at normal density
- **THEN** the entry SHALL be collapsed behind its header and one-line summary
- **AND** it SHALL open to `TerminalResultView` only when the user activates it

#### Scenario: Diff stays collapsed

- **GIVEN** a completed `file-edit` record with a valid `code_diff` payload, at normal density
- **THEN** the entry SHALL be collapsed until the user activates it

#### Scenario: Non-zero bash exit opens expanded

- **GIVEN** a `bash` record with status `error` and terminal data with `exit_code = 1`, at normal density
- **THEN** the entry SHALL be open and `TerminalResultView` SHALL render with a failure badge

#### Scenario: Error with valid data

- **GIVEN** a `web-fetch` record with status `error` and a valid `link_preview` payload with `status = 404`
- **THEN** `LinkPreviewResultView` SHALL render expanded, showing the 404 status

#### Scenario: Denied

- **GIVEN** a record with status `denied`
- **THEN** the generic fallback SHALL render regardless of data

### Requirement: Each contract has a dedicated view

`assistant_ui` SHALL provide these renderers:

| Kind           | View                    | Must show                                                                                                                                                        |
| -------------- | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `terminal`     | `TerminalResultView`    | `$ command` line, exit-code badge (success / failure / timed out), stdout tail, stderr tail in the error colour, a truncation notice per stream, duration        |
| `sources`      | `SourcesResultView`     | one row per result with title, domain and snippet; activating a row calls `ChatMarkdownScope.onTapLink`                                                          |
| `link_preview` | `LinkPreviewResultView` | title (or URL), final URL's host, HTTP status badge, content type, excerpt; a redirect note when `final_url ≠ url`; activating calls `onTapLink(final_url)`      |
| `code_file`    | `CodeFileResultView`    | path header, content through the package `CodeBlock` element with the `language` hint, truncation notice                                                         |
| `code_diff`    | `CodeDiffResultView`    | path, operation, `+added −removed` counts, hunks with per-line add/remove/context styling that does not rely on colour alone (`+`/`−` gutter), truncation notice |
| `data_table`   | `DataTableResultView`   | header from `columns`, rows, horizontal scroll at narrow widths, tap-to-sort per column (local widget state), `truncated` / `total_rows` notice                  |
| `plan`         | `PlanResultView`        | optional title, completed/total progress, each step with a status icon and label for `pending`, `in_progress`, `completed`, `failed`, `skipped`                  |

No view SHALL fetch network resources. Links SHALL leave the package only through `ChatMarkdownScope.onTapLink`, called with the URL string.

#### Scenario: Terminal distinguishes timeout

- **GIVEN** terminal data with `timed_out = true`
- **THEN** the badge SHALL read "timed out", not an exit code

#### Scenario: Sources open links by callback

- **WHEN** the user activates the second source row
- **THEN** the `onTapLink` of the nearest `ChatMarkdownScope` SHALL be called with that row's URL
- **AND** the package SHALL perform no navigation itself

#### Scenario: Diff gutter is not colour-only

- **GIVEN** a diff with one added and one removed line
- **THEN** the added line SHALL be prefixed `+` and the removed line `−`, in addition to their colours

#### Scenario: Data table sorts locally

- **WHEN** the user activates the "Next run" column header
- **THEN** the rows SHALL reorder by that column without any callback to the app

#### Scenario: Plan progress

- **GIVEN** a plan with 5 steps, 3 `completed`
- **THEN** the view SHALL show `3/5`

### Requirement: Dart parsers agree with the shared contract fixtures

A test in `assistant_ui` SHALL load every fixture under `schemas/tool-results/fixtures/`, resolved by a single helper relative to the package root that fails when the directory is missing or empty, and assert that the renderer for that kind returns a model for every `valid-*` fixture and null for every `invalid-*` fixture. Every kind in the fixture directory SHALL have a renderer in `defaultToolResultRenderers()`.

#### Scenario: Wrong fixture path

- **GIVEN** the fixture helper points at a directory that does not exist
- **WHEN** the contract test runs
- **THEN** it SHALL fail, not pass with zero fixtures

#### Scenario: Schema-only change runs the Dart contract test

- **WHEN** a pull request changes only files under `schemas/tool-results/`
- **THEN** the Flutter CI workflow SHALL run the `assistant_ui` tests

#### Scenario: New contract without a renderer

- **GIVEN** a fixture directory `fixtures/chart/` with no registered `chart` renderer
- **WHEN** the contract test runs
- **THEN** it SHALL fail naming `chart`

### Requirement: Renderers are covered by gallery stories and goldens

Every renderer SHALL have widgetbook use-cases for a valid payload, an invalid payload (showing the fallback), an `error` status, and each `TimelineDensity`. It SHALL also have golden tests rendered from the shared valid fixtures in light and dark theme. Each renderer SHALL be listed in `kAssistantUiElements` and in `ELEMENTS.md`.

#### Scenario: Gallery coverage test includes renderers

- **WHEN** the `chat-element-library` gallery coverage test runs
- **THEN** each of the seven result views SHALL have at least one use-case

#### Scenario: Golden from fixture

- **WHEN** `TerminalResultView` golden tests run
- **THEN** their input SHALL be decoded from `schemas/tool-results/fixtures/terminal/valid-*.json`

### Requirement: The chat screen installs the registry through the resolver scope

`ChatScreen` SHALL construct the registry from `defaultToolResultRenderers()` once and SHALL install `ToolResultRendererScope(resolve: registry.build)` above the message list. It SHALL NOT pass the registry to `ToolCallBody` by constructor. Tool calls inside a `SubagentBody`'s inner timeline SHALL NOT resolve through the scope; they stay compact rows. Link handling SHALL come from the `ChatMarkdownScope` that `ChatScreen` already installs with `MarkdownLinkHandler`. No renderer SHALL read Riverpod state.

#### Scenario: Opening a source link from the chat

- **WHEN** the user activates a source row in a live chat
- **THEN** the link SHALL open through `MarkdownLinkHandler`, the same way as a link in message markdown

#### Scenario: Subagent tool call stays a compact row

- **GIVEN** a subagent whose inner `bash` call has a result
- **WHEN** the subagent entry is expanded in the chat
- **THEN** the inner tool call SHALL render as a compact row
- **AND** no `TerminalResultView` or other result view SHALL render inside the subagent timeline
