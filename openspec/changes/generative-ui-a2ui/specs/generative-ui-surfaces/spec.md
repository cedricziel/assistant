## ADDED Requirements

### Requirement: Surfaces use A2UI v1.0 with the assistant catalog

The server SHALL emit A2UI messages of protocol version `1.0` only, and every `createSurface` SHALL name the assistant catalog id `https://assistant.invalid/a2ui/catalogs/assistant/v1/catalog.json`. The assistant catalog SHALL contain exactly these components: Text, Image, Icon, Row, Column, List, Card, Tabs, Divider, Button, CheckBox, TextField, DateTimeInput, ChoicePicker, Slider. It SHALL also contain exactly these functions: required, regex, length, numeric, email, formatString, formatNumber, formatCurrency, formatDate, pluralize, and, or, not, openUrl. The catalog's component list SHALL be recorded in a shared fixture that the server and client tests both assert against. The server SHALL NOT emit `deleteSurface`.

#### Scenario: Built messages carry version and catalog

- **WHEN** a valid `render-surface` call is turned into A2UI messages
- **THEN** each message has `"version": "1.0"`
- **AND** the `createSurface` message has `catalogId` equal to the assistant catalog id

#### Scenario: Excluded basic-catalog components are rejected

- **WHEN** a draft contains a component of type `Video`, `AudioPlayer` or `Modal`
- **THEN** validation fails with an issue whose `path` points at that component

### Requirement: Server validates every surface before it reaches a client

The server SHALL validate every surface draft, and every merged surface after an update, before it emits or persists it. Validation SHALL check each component against its catalog JSON Schema (draft 2020-12). It SHALL check structure: unique ids, exactly one `root`, all child references resolving, no cycles, depth at most 12, at most 100 components. It SHALL enforce size limits: serialized surface at most 64 KiB, string literals at most 4 KiB, `fallback_text` between 1 and 2000 characters. It SHALL reject unknown `@`-prefixed keys. Failures SHALL return all issues at once, each with `code`, `path` (JSON Pointer) and `message`. The validator SHALL be a pure synchronous function; any data that needs I/O (attachment ownership) SHALL be resolved by the caller and passed in.

#### Scenario: Unknown component type

- **WHEN** a draft contains `{"id":"x","component":"Script"}`
- **THEN** validation fails with an issue at `/components/<index>` naming the unknown type
- **AND** nothing is emitted or persisted

#### Scenario: Dangling child reference

- **WHEN** a Column lists a child id that no component has
- **THEN** validation fails with an issue that names the missing id

#### Scenario: Multiple issues are reported together

- **WHEN** a draft has both a missing `root` and an over-long string
- **THEN** the returned issue list contains both issues

#### Scenario: Oversized surface

- **WHEN** the serialized draft exceeds 64 KiB
- **THEN** validation fails with a size-limit issue

#### Scenario: Empty fallback text

- **WHEN** `fallback_text` is empty or longer than 2000 characters
- **THEN** validation fails with an issue at `/fallback_text`

### Requirement: Regex patterns are safe for a backtracking client engine

Every `regex` function pattern SHALL be at most 200 characters, SHALL compile under the Rust `regex` crate, and SHALL pass a backtracking-safety check on its parsed syntax tree: no quantified group whose body contains another quantifier, and no quantified group that contains an alternation.

#### Scenario: Nested quantifier

- **WHEN** a TextField check uses `regex` with pattern `(a+)+$`
- **THEN** validation fails with a regex-safety issue at that check's path

#### Scenario: Quantified alternation

- **WHEN** a pattern is `(a|aa)+`
- **THEN** validation fails with a regex-safety issue

#### Scenario: Simple pattern passes

- **WHEN** a pattern is `^[0-9]{5}$`
- **THEN** the pattern is accepted

### Requirement: URL policy forbids unsafe targets

Literal `Image.url` values, and initial `data_model` values at paths bound to `Image.url`, SHALL be `https:` URLs or same-origin `/api/attachments/{id}` paths whose attachment belongs to the same conversation. Literal `openUrl` targets, and initial `data_model` values at paths bound to an `openUrl` argument, SHALL be `https:` or `mailto:` URLs. All other schemes, including `javascript:`, `data:`, `file:` and `http:`, SHALL be rejected. When the org configures `[ui] image_hosts` in `org.toml`, `https:` image hosts outside that list SHALL be rejected. The tool handler SHALL collect attachment ids from the draft, resolve their ownership asynchronously, and pass the owned set and the host list to the validator.

#### Scenario: javascript URL in openUrl

- **WHEN** a Button action calls `openUrl` with `javascript:alert(1)`
- **THEN** validation fails with a URL-policy issue at that action's path

#### Scenario: Attachment from another conversation

- **WHEN** an Image references `/api/attachments/{id}` for an attachment that belongs to a different conversation
- **THEN** validation fails with a URL-policy issue

#### Scenario: Bound initial value with an unsafe scheme

- **WHEN** an Image `url` is `{"@path": "/img"}` and the initial `data_model` sets `/img` to `data:image/png;base64,AAAA`
- **THEN** validation fails with a URL-policy issue at `/data_model/img`

#### Scenario: Image host outside the org allowlist

- **WHEN** the org sets `[ui] image_hosts = ["images.example.com"]`
- **AND** an Image uses `https://tracker.example.net/p.png`
- **THEN** validation fails with a URL-policy issue naming the host

#### Scenario: No allowlist configured

- **WHEN** the org has no `[ui]` section
- **AND** an Image uses any `https:` URL
- **THEN** the URL passes the policy

### Requirement: render-surface tool creates and updates surfaces

A built-in tool named `render-surface` SHALL support two modes. Create (no `surface_id`) SHALL require `title`, `fallback_text` and `components`, and SHALL accept `data_model`, `send_data_model` (default true) and `wait_for_action` (default false). Update (with `surface_id`) SHALL require at least one of `components` (upserted by component id) or `data_updates` (an array of `{path, value}` with a JSON Pointer `path`; a `null` value deletes), and SHALL accept `title` and `fallback_text` as replacements. An update SHALL target an `active` surface of the same conversation. Validation limits SHALL apply to the merged surface. Every successful call, create or update, SHALL count toward a limit of 5 per turn. Validation failures SHALL return `Ok(ToolOutput::error)` carrying the issue list as JSON. A successful call SHALL return an observation that names the `surface_id`. The handler SHALL return the validated surface in `ToolOutput.data`, and the runtime SHALL take that data before generic tool-data forwarding, so it never appears in the `tool_result` frame or the persisted tool row.

#### Scenario: Valid create

- **WHEN** the model calls `render-surface` with a valid draft
- **THEN** the observation contains the new `surface_id`
- **AND** a `surface_update` event with status `active` is emitted

#### Scenario: Invalid draft lets the model retry

- **WHEN** the model calls `render-surface` with an invalid draft
- **THEN** the tool returns an error observation listing the issues
- **AND** no surface row is created

#### Scenario: Data-model update

- **WHEN** the model calls `render-surface` with an `active` `surface_id` and `data_updates: [{"path": "/name", "value": "Ada"}]`
- **THEN** one `updateDataModel` message is emitted for `/name`
- **AND** the stored data model has `/name` set to `Ada`

#### Scenario: Update that exceeds limits after merge

- **WHEN** an update adds components so that the merged surface has 101 components
- **THEN** the tool returns an error observation and the stored surface is unchanged

#### Scenario: Update without changes

- **WHEN** the model passes `surface_id` with neither `components` nor `data_updates`
- **THEN** the tool returns an error observation

#### Scenario: Sixth call in a turn

- **WHEN** a turn has already made 5 successful `render-surface` calls, creates and updates combined
- **THEN** the next call returns an error observation and nothing is emitted

#### Scenario: Update of a surface from another conversation

- **WHEN** `surface_id` names a surface that belongs to a different conversation, or one that is not `active`
- **THEN** the tool returns an error observation and changes nothing

#### Scenario: Surface data stays out of the tool-result frame

- **WHEN** a valid create runs
- **THEN** the `tool_result` frame and the persisted tool-result row carry no surface data

### Requirement: render-surface is offered only to interactive web turns

`ToolHandler` and `ToolSpec` SHALL expose `requires_rich_client`, defaulting to false. The orchestrator SHALL drop rich-client tools from the tool list unless the turn's `ExecutionContext` has `interactive = true`, its interface is `Web`, and its subagent depth is 0. The MCP server SHALL omit rich-client tools from `tools/list`. If `render-surface` is executed in any other context, it SHALL succeed with an observation telling the model to present `fallback_text` as text, and it SHALL NOT create a surface or emit an event.

#### Scenario: Slack turn

- **WHEN** a turn runs on `Interface::Slack`
- **THEN** the tool specs sent to the LLM do not include `render-surface`

#### Scenario: quick-message turn does not see render-surface

- **WHEN** `POST /api/quick-message` runs a turn on `Interface::Web` with `interactive = false`
- **THEN** the tool specs sent to the LLM do not include `render-surface`

#### Scenario: Voice turn

- **WHEN** a voice message turn runs through the shared runner with `interactive = false`
- **THEN** the tool specs sent to the LLM do not include `render-surface`

#### Scenario: Streaming chat turn

- **WHEN** a turn is submitted by `POST /api/conversations/{id}/messages`, edit or regenerate
- **THEN** the tool specs include `render-surface`

#### Scenario: CLI REPL turn

- **WHEN** an interactive CLI REPL turn assembles its tools
- **THEN** `render-surface` is absent

#### Scenario: Subagent turn

- **WHEN** a subagent at depth 1 assembles its tools
- **THEN** `render-surface` is absent

#### Scenario: Forced invocation off web

- **WHEN** `render-surface` executes with `ExecutionContext.interface` set to `Matrix`
- **THEN** the observation instructs the model to send `fallback_text` as a text reply
- **AND** no `surface_update` event is emitted

#### Scenario: Web prompt advertises surfaces

- **WHEN** the system prompt is built for an interactive `Interface::Web` turn
- **THEN** the output-capabilities block describes `render-surface` and when to use it

### Requirement: Surfaces stream as a replayable surface_update event

The orchestrator SHALL emit `OrchestratorEvent::SurfaceUpdate` with `surface_id`, optional `tool_call_id`, `anchor_message_id`, `anchor_call_index`, `title`, `fallback_text`, `status`, and `messages` (A2UI v1.0 messages). The SSE projector SHALL map it to an event named `surface_update` whose payload has exactly those snake_case fields. The event SHALL be persisted with a sequence id in the conversation event log, so `GET .../runs/{run}/events/stream?since=N` replays it. The CLI and A2A projectors SHALL render `fallback_text` as text; these arms exist so projection stays total. A status change SHALL emit `surface_update` with an empty `messages` list. The streamed-turn runner SHALL keep persisting and broadcasting frames after the client's SSE connection fails, until the turn ends.

#### Scenario: Replay after reconnect

- **WHEN** a client replays a run with `since` lower than the surface event's sequence id
- **THEN** the replay contains the `surface_update` frame with the same payload as the live frame

#### Scenario: Client disconnects during a wait

- **WHEN** the client disconnects while the turn waits on a surface interaction
- **AND** it reconnects via `events/stream`, answers the interaction and keeps tailing
- **THEN** it receives the status `surface_update` with `submitted` and the rest of the turn's frames

#### Scenario: CLI projector stays total

- **WHEN** the CLI projector receives a `SurfaceUpdate`
- **THEN** it prints `fallback_text`

### Requirement: Surfaces persist and reload with conversation history

Surfaces SHALL be stored in a `ui_surfaces` table in space.db through a `SurfaceStore` trait that has `InMemorySurfaceStore` and `SqliteSurfaceStore` implementations. Each surface SHALL be anchored to the tool-call message id and the call's index in it, and SHALL record the provider `tool_call_id` when one exists. Deleting a conversation SHALL delete its surfaces. `GET /api/conversations/{id}` SHALL include `surfaces`, a list of `SurfaceSummary` objects ordered by `(turn, created_at)`, containing only surfaces whose anchor message is among the returned messages. Each SHALL carry `surface_id` (uuid), `turn`, `anchor_message_id`, `anchor_call_index`, an optional `tool_call_id`, `title`, `fallback_text`, `status` (`active` or `submitted`), `created_at`, an optional `submitted_action`, an optional `action_message_id`, and `messages`: a single synthesized `createSurface` holding the current components and data model.

#### Scenario: Reload after the event log expired

- **WHEN** a conversation with a surface is loaded more than 24 hours after the turn
- **THEN** `ConversationDetail.surfaces` contains the surface with its latest components and data model

#### Scenario: Validation retry keeps anchors exact

- **WHEN** a turn's first `render-surface` call fails validation and the retry succeeds
- **THEN** the only surface is anchored to the retry's tool call, not the failed one

#### Scenario: Cross-turn update

- **WHEN** a later turn updates a surface created in an earlier turn
- **THEN** the surface keeps its original anchor and shows the updated components

#### Scenario: Abandoned branch

- **WHEN** the returned messages are the active branch and a surface's anchor message is on another branch
- **THEN** that surface is not in `surfaces`

#### Scenario: Cascade delete

- **WHEN** a conversation is deleted from the SQLite store
- **THEN** its `ui_surfaces` rows are removed

### Requirement: In-turn actions resolve through the surface_action interaction kind

When `wait_for_action` is true, the server SHALL emit the surface's `surface_update`, then request an interaction of kind `surface_action` with request `{surface_id, tool_call_id}` on the turn-interaction channel defined by `tool-approval-prompts`, and await it. The `surface_action` kind SHALL be registered through the interaction-kind handler registry with `renders_card = false`, progress label "Waiting for your input", push copy "Your input is needed", and the same expiry setting as other kinds. Its response validator SHALL accept only `{kind: "surface_action", action, data_model?}` for an `active` surface whose Buttons declare `action.name`. On `resolved`, the server SHALL mark the surface `submitted`, store the action, emit a status `surface_update`, and return the action (plus the data model when `send_data_model`) as the observation. On a generic `decline`, the observation SHALL say the user dismissed the form. On `expired`, it SHALL say the user has not answered yet. On `decline`, `expired` and `cancelled`, the surface SHALL stay `active`.

#### Scenario: User submits while the turn waits

- **WHEN** the client answers the interaction with an `action` named `submit_form`
- **THEN** the tool observation contains that action's `name` and `context`
- **AND** the surface status becomes `submitted`

#### Scenario: Interaction is announced after the surface

- **WHEN** a `wait_for_action` surface is created
- **THEN** the persisted log has the `surface_update` frame before the `interaction_requested` frame whose payload carries the same `surface_id`

#### Scenario: User declines

- **WHEN** the client responds with `{kind: "decline"}`
- **THEN** the observation says the user dismissed the form
- **AND** the surface stays `active`

#### Scenario: Turn cancelled while waiting

- **WHEN** the turn is cancelled during the wait
- **THEN** the interaction resolves as `cancelled`
- **AND** the surface remains `active` and can be answered post-turn

#### Scenario: Undeclared action name

- **WHEN** the response carries an action `name` that no Button in the surface declares
- **THEN** the response is rejected with 400 and the surface is unchanged

#### Scenario: Turn status while waiting

- **WHEN** `GET .../turns/{id}/status` is called during the wait
- **THEN** the state is `awaiting_input`

### Requirement: Post-turn actions start a user turn with a synthesized block

`SendMessageRequest` SHALL accept an optional `surface_action` containing `surface_id` (uuid), `name`, `source_component_id`, `context` (object) and an optional `data_model` (object). When it is present, `message` SHALL be empty. The server SHALL check, in order: the surface belongs to the conversation, is `active`, and declares `name` on a Button (else 400); `context` plus `data_model` serialize to at most 64 KiB (else 413); no turn is in flight for the conversation (else 409). It SHALL then mark the surface `submitted` and record the action and the pre-assigned user message id in one compare-and-set; a lost race SHALL return 409. It SHALL persist the user message with the server-built content `Submitted “<title>”`, and stream the turn exactly like any other `send_message` response, with a status `surface_update` (`submitted`) as the first frame after `run_started`. If the turn fails to start, the server SHALL roll the surface back to `active` and clear the action. Whenever LLM history is built, user messages linked to a surface action SHALL be prefixed with a structured block identifying the surface, the action name, the context JSON and the data model JSON; the stored content SHALL stay the visible text.

#### Scenario: Submitting a form after the turn ended

- **WHEN** the client posts an empty `message` with a valid `surface_action` for an `active` surface
- **THEN** the response is an SSE stream that begins with `run_started` followed by a `surface_update` with status `submitted`
- **AND** the stored user message content is `Submitted “<title>”`

#### Scenario: LLM sees the structured block now and later

- **WHEN** the turn started by a surface action runs, and later another turn in the same conversation runs
- **THEN** both turns' LLM history contain the `[surface_action …]` block before that user message's text

#### Scenario: Message text with an action

- **WHEN** a request carries both a non-empty `message` and `surface_action`
- **THEN** the server responds 400

#### Scenario: Oversized action payload

- **WHEN** `context` plus `data_model` exceed 64 KiB
- **THEN** the server responds 413 and the surface is unchanged

#### Scenario: Turn in flight

- **WHEN** a turn is running in the conversation, including one waiting on this surface's interaction
- **THEN** the server responds 409 and the surface is unchanged

#### Scenario: Turn fails to start

- **WHEN** submitting the turn fails before `run_started`
- **THEN** the surface is `active` again with no stored action

#### Scenario: Double submit

- **WHEN** a second `surface_action` is posted for a `submitted` surface
- **THEN** the server responds 409 with an `{"error": ...}` body

#### Scenario: Caller without write scope

- **WHEN** the caller lacks conversation write access
- **THEN** the server responds 403 and the surface is unchanged
