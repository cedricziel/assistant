## ADDED Requirements

### Requirement: Tool spans are rendered as dedicated cards in trace detail

The trace detail screen SHALL detect tool-call spans and render them with a `_ToolCallSpanCard` widget distinct from the generic `_SpanCard`. A span SHALL be classified as a tool call when its `name` starts with the literal prefix `"execute_tool "` OR, defensively, when `attributes['tool_name']` is non-null. The tool name SHALL be derived from `attributes['tool_name']` when present, otherwise from the suffix of the span name after `"execute_tool "`.

#### Scenario: Span with `execute_tool` prefix renders as tool card

- **GIVEN** a span with `name == "execute_tool file-read"` and attribute `tool_name == "file-read"`
- **WHEN** the trace detail screen renders
- **THEN** the entry for that span SHALL be a `_ToolCallSpanCard` AND the card title SHALL be `"file-read"`

#### Scenario: Generic span is unaffected

- **GIVEN** a span with `name == "chat anthropic/claude-haiku-4-5-20251001"`
- **WHEN** the trace detail screen renders
- **THEN** the entry SHALL be a `_SpanCard` (generic), not `_ToolCallSpanCard`

#### Scenario: Span with tool_name attribute but no prefix

- **GIVEN** a span with `name == "tool_invocation"` AND `attributes['tool_name'] == "bash"`
- **THEN** the entry SHALL be a `_ToolCallSpanCard` with title `"bash"`

### Requirement: Tool-call card surfaces status with an icon and theme-aware colour

The `_ToolCallSpanCard` header SHALL display a status icon and a coloured pill matching `attributes['tool_status']`:

| `tool_status`     | Icon                 | Colour                                                    |
| ----------------- | -------------------- | --------------------------------------------------------- |
| `ok`              | `Icons.check_circle` | `colorScheme.tertiary`                                    |
| `error`           | `Icons.error`        | `colorScheme.error`                                       |
| `denied`          | `Icons.block`        | Amber via `AssistantColors.warning` (`Color(0xFFB45309)`) |
| missing / unknown | `Icons.help_outline` | `colorScheme.onSurfaceVariant`                            |

The icon and pill SHALL be visible without expanding the card.

#### Scenario: ok status shows check icon and tertiary colour

- **GIVEN** a tool span with `tool_status == "ok"`
- **THEN** the card header SHALL render `Icons.check_circle` AND a pill labelled `"ok"` in `colorScheme.tertiary`

#### Scenario: error status shows error icon and error colour

- **GIVEN** a tool span with `tool_status == "error"`
- **THEN** the card header SHALL render `Icons.error` AND a pill labelled `"error"` in `colorScheme.error`

#### Scenario: denied status shows block icon

- **GIVEN** a tool span with `tool_status == "denied"`
- **THEN** the card header SHALL render `Icons.block` AND a pill labelled `"denied"`

#### Scenario: Missing status falls back to neutral

- **GIVEN** a tool span without a `tool_status` attribute
- **THEN** the card header SHALL render `Icons.help_outline` AND a pill labelled `"unknown"` in `colorScheme.onSurfaceVariant`

### Requirement: Expanded tool-call card shows params and output side by side

When the card is expanded, it SHALL render a two-pane body labelled `Params` and `Output`.

- `Params` SHALL show `attributes['tool_params']`, pretty-printed via `JsonEncoder.withIndent('  ')` when it parses as JSON, or verbatim when it does not.
- `Output` SHALL show:
  - `attributes['tool_observation']` for `tool_status == "ok"` (pretty-printed when JSON-parseable).
  - `attributes['tool_error']` for `tool_status` in {`error`, `denied`}, styled with `colorScheme.error` text.
  - The literal string `"(no output)"` when neither attribute is present.

The two panes SHALL lay out side-by-side at viewport widths >= 600 dp and stack vertically below that.

#### Scenario: Success — params + observation visible

- **GIVEN** a tool span with `tool_params == '{"path":"/tmp/foo.txt"}'` and `tool_observation == "ok, 42 bytes"` and `tool_status == "ok"`
- **WHEN** the card is expanded
- **THEN** the `Params` pane SHALL show the pretty-printed JSON AND the `Output` pane SHALL show `"ok, 42 bytes"`

#### Scenario: Error — error message styled red

- **GIVEN** a tool span with `tool_status == "error"` and `tool_error == "permission denied"` (no `tool_observation`)
- **WHEN** the card is expanded
- **THEN** the `Output` pane SHALL show `"permission denied"` in `colorScheme.error` text

#### Scenario: Narrow viewport stacks panes

- **GIVEN** the viewport is 480 dp wide
- **WHEN** a tool card is expanded
- **THEN** the `Params` pane SHALL render above the `Output` pane (vertical stack), not side-by-side

#### Scenario: Non-JSON params shown verbatim

- **GIVEN** a tool span with `tool_params == "not json {{"`
- **WHEN** the card is expanded
- **THEN** the `Params` pane SHALL show the literal string `"not json {{"`

### Requirement: "Show all attributes" toggle preserves the full attribute set

Other tool-span attributes (`iteration`, `turn`, `interface`, `active_skill`, `conversation_id`, etc.) SHALL remain accessible behind a `"Show all attributes"` toggle in the card footer, rendered as the existing key/value list when toggled on.

#### Scenario: Default view hides extra attributes

- **WHEN** the card is first expanded
- **THEN** the `iteration` / `turn` / `interface` attributes SHALL NOT be visible

#### Scenario: Toggle reveals the full attribute dump

- **WHEN** the user taps `"Show all attributes"`
- **THEN** the card SHALL render all attributes (including the four primary ones already shown) in the existing key/value list format
