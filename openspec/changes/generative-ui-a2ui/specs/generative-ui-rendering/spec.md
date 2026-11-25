## ADDED Requirements

### Requirement: Client parses surface_update into the sealed StreamEvent

The client SHALL parse the SSE event `surface_update` into a `SurfaceUpdateEvent` subclass of the sealed `StreamEvent`. The subclass SHALL carry `surfaceId`, `toolCallId` (optional), `anchorMessageId`, `anchorCallIndex`, `title`, `fallbackText`, `status`, `messages` (a list of JSON maps), and the sequence id. Every exhaustive `switch` over `StreamEvent` SHALL handle it. A malformed payload SHALL be ignored, as other malformed events are. Unknown event names SHALL continue to be ignored.

#### Scenario: Live surface event

- **WHEN** the stream delivers `event: surface_update` with a valid payload
- **THEN** the parser yields a `SurfaceUpdateEvent` with the decoded fields and sequence id

#### Scenario: Replay yields the same timeline

- **WHEN** a run is replayed from `since=0`
- **THEN** the resulting timeline contains the same surface entry as the live stream produced

### Requirement: Surfaces render as timeline entries inside a pure frame

`assistant_ui` SHALL provide `TimelineEntryType.surface`, a pure `SurfaceRecord` model, and a `SurfaceFrame` element rendered in the timeline entry shell's body slot. `SurfaceFrame` SHALL take only constructor data (title, status, fallback text, error flag, pending flag, awaiting-input flag, an optional content widget) and callbacks, and SHALL perform no I/O. With no content widget, or when the error flag is set, the frame SHALL show the fallback text. A `submitted` surface SHALL be rendered read-only, labelled as submitted. A surface whose turn waits on it SHALL use `EntryState.awaitingInput`. `SurfaceFrame` SHALL have widgetbook stories for active, awaiting input, pending, submitted, error and fallback-only states, plus golden tests in light and dark themes.

#### Scenario: Fallback when the renderer fails

- **WHEN** the surface's A2UI messages fail client-side validation
- **THEN** the frame shows `fallbackText` and an error affordance, not a broken widget tree

#### Scenario: Submitted surface is read-only

- **WHEN** a surface has status `submitted`
- **THEN** its inputs and buttons are disabled
- **AND** the frame shows a "Submitted" label

#### Scenario: Waiting surface

- **WHEN** the turn is waiting on the surface's interaction
- **THEN** the frame's entry state is `awaitingInput`

### Requirement: The A2UI renderer adapter lives in the app, outside the pure package

The A2UI renderer SHALL be implemented in `app/lib/features/chat/a2ui/` on exactly pinned versions of `a2ui_flutter` and `a2ui_core`. Neither package SHALL be added to `app/packages/assistant_ui/pubspec.yaml`, and the `assistant_ui` allowlist SHALL stay unchanged. The adapter SHALL register only an assistant-catalog `CatalogImplementation` built from `BasicComponents` under the assistant catalog id, with the `Image` component replaced by an app implementation. Its component set SHALL match the shared fixture.

#### Scenario: Package boundary holds

- **WHEN** the `assistant_ui` boundary test runs
- **THEN** it passes with no `a2ui_*` dependency in the package

#### Scenario: Catalog parity

- **WHEN** the adapter's catalog test compares its component names to the shared fixture
- **THEN** the two sets are equal

### Requirement: Attachment images load through the authenticated client

The adapter's `Image` implementation SHALL load same-origin `/api/attachments/{id}` URLs through the app's authenticated HTTP client, sending the bearer token and space header, and SHALL apply the URL policy before any load.

#### Scenario: Attachment image renders with auth headers

- **WHEN** a surface Image references `/api/attachments/{id}`
- **THEN** the request carries the `Authorization` and space headers
- **AND** the image renders

### Requirement: Surfaces inherit the app theme

Rendered surfaces SHALL take colours and typography from the active app `ThemeData`, with light and dark both supported, and shapes and padding from `AssistantRadius` and `AssistantSpacing`. Surfaces SHALL NOT read any theme carried in A2UI messages. Surface content SHALL use Material widgets on every platform; this is an accepted deviation from the adaptive-widgets rule, recorded in the design.

#### Scenario: Dark mode

- **WHEN** the app is in dark mode
- **THEN** a rendered Card and Button use the dark theme's surface and primary colours, as the dark golden test shows

### Requirement: Client enforces the URL policy and regex input cap on resolved values

The client SHALL apply the same URL policy as the server to every resolved image URL and `openUrl` target, including values bound through `@path`. Images that violate the policy SHALL render a placeholder. Links that violate it SHALL be disabled. `openUrl` SHALL launch only through the app's `url_launcher` callback and only from a user gesture. The `regex` function SHALL evaluate at most 1,000 characters of input; longer input SHALL fail the check without running the pattern.

#### Scenario: Bound URL resolves to an unsafe scheme

- **WHEN** an Image `url` is bound to a data-model value that the user changed to `data:image/png;base64,...`
- **THEN** the image is not loaded and a placeholder is shown

#### Scenario: Long input to a regex check

- **WHEN** a TextField with a `regex` check receives 5,000 characters
- **THEN** the check fails immediately without evaluating the pattern

### Requirement: Actions are sent on the right route

When the user triggers an A2UI `action`, the client SHALL send it on the interaction channel if a pending `interaction_requested` of kind `surface_action` names this surface's `surface_id` and the turn is still running. The client SHALL NOT render a generic interaction card for kind `surface_action`. In every other case it SHALL send a new message with an empty `message` and `surface_action` populated, through `ApiClient.streamMessages`, queued like any message sent while a turn is running. While the request is in flight, the surface SHALL show a pending state. `streamMessages` SHALL surface the HTTP status on its `ErrorEvent`. On a 409 for a submitted surface, the client SHALL mark the surface submitted and SHALL NOT retry. On a 410 from the respond endpoint, the client SHALL drop the interaction and send the action on the post-turn route.

#### Scenario: Post-turn submit

- **WHEN** the user taps a submit Button on an `active` surface from an earlier turn
- **THEN** the client sends `POST /api/conversations/{id}/messages` with `surface_action` and an empty `message`
- **AND** the server-built user message appears in the timeline with a "Submitted via form" marker
- **AND** the surface shows `submitted` after the status `surface_update` that follows `run_started`

#### Scenario: Double submit returns 409

- **WHEN** the post-turn request fails with an `ErrorEvent` whose status is 409
- **THEN** the surface is marked submitted and no retry is made

#### Scenario: In-turn submit

- **WHEN** the turn is waiting on the surface's interaction and the user taps submit
- **THEN** the client posts `{kind: "surface_action", action, data_model?}` to the interaction respond endpoint
- **AND** it does not start a new message

#### Scenario: No generic card for surface interactions

- **WHEN** an `interaction_requested` of kind `surface_action` arrives
- **THEN** the timeline shows no interaction card
- **AND** the matching surface's entry state becomes `awaitingInput`

#### Scenario: Interaction expired before the tap

- **WHEN** the respond call returns 410
- **THEN** the client sends the same action on the post-turn route

### Requirement: Surfaces reload from conversation history

When a conversation loads, the client SHALL map `ConversationDetail.surfaces` into surface timeline entries at the position of the tool call they are anchored to: the call whose `tool_call_id` matches, or, when the provider gave no id, the call at `anchor_call_index` in the message `anchor_message_id`. The client SHALL NOT match surfaces by turn order. Successful `render-surface` tool calls SHALL NOT render as chips; failed ones SHALL render as ordinary error chips. Submitted surfaces SHALL show their submitted values, and the user message named by `action_message_id` SHALL carry the "Submitted via form" marker.

#### Scenario: History shows surface instead of chip

- **WHEN** a conversation that contains a `render-surface` call and its surface is loaded
- **THEN** the timeline shows a surface entry at that position
- **AND** it shows no `render-surface` tool chip for that call

#### Scenario: Failed call before a retry

- **WHEN** a turn has a failed `render-surface` call followed by a successful retry
- **THEN** the failed call shows as an error chip and the surface sits at the retry's position

#### Scenario: Provider without tool call ids

- **WHEN** the tool calls have no `tool_call_id`
- **THEN** each surface is placed by `anchor_message_id` and `anchor_call_index`
