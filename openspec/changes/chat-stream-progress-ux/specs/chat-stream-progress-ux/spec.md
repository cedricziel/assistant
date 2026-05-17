## ADDED Requirements

### Requirement: In-flight turn progress card

While a chat turn is in-flight (between `run_started` and `done` / `agent_error`), the chat screen SHALL render a compact, persistent progress card that summarises the active turn's most recent state. The card is sourced from the most recent SSE event for the active turn.

#### Scenario: Card appears on run_started and disappears on done

- **WHEN** the client receives a `run_started` SSE event on an active stream
- **THEN** the progress card SHALL render above the message composer (or in an equivalent persistent slot) within one frame
- **WHEN** the same stream emits a `done` event
- **THEN** the card SHALL disappear within one frame

#### Scenario: Card disappears on agent_error

- **WHEN** the stream emits an `agent_error` event
- **THEN** the progress card SHALL disappear and any existing error-surfacing UI (e.g. the failed message bubble) SHALL be the user's source of truth for the failure

#### Scenario: Card summarises tool calls in plain language

- **WHEN** the most recent SSE event for the active turn is a `tool_result` (or `subagent_started`) carrying a tool name (e.g. `fetch_url`)
- **THEN** the card's label SHALL read in the form `"Calling fetch_url…"` (the actual tool name interpolated)

#### Scenario: Card summarises thinking and generating phases

- **WHEN** the most recent event for the active turn is `thinking`
- **THEN** the card's label SHALL read `"Thinking…"`
- **WHEN** the most recent event for the active turn is `token`
- **THEN** the card's label SHALL read `"Generating response…"`

### Requirement: Stall indicator on prolonged silence

When the in-flight turn has had no SSE event for longer than the stall threshold (default: 30 seconds), the progress card SHALL surface the silence with an elapsed-time indicator. No automatic cancellation SHALL occur.

#### Scenario: Stall transition at 30 seconds

- **WHEN** the most recent SSE event for the active turn was 30 or more seconds ago
- **THEN** the card SHALL transition from its activity label (e.g. `"Generating response…"`) to a stall label such as `"Server is taking longer than expected — 0:32"`
- **THEN** the elapsed time SHALL update at least once per second

#### Scenario: Stall recovery on next event

- **WHEN** a new SSE event arrives after the stall threshold has been crossed
- **THEN** the card SHALL return to its activity label and the elapsed-time indicator SHALL be hidden

#### Scenario: No automatic cancellation

- **WHEN** the stall threshold is crossed and remains crossed for any duration
- **THEN** the client SHALL NOT cancel the in-flight stream automatically
- **THEN** any "Skip" affordance is only enabled when the `turn-status-api` capability is available (a separate change); until then no cancellation surface is visible

### Requirement: Reconnect banner during attemptReconnect

When `ChatNotifier.attemptReconnect()` is in-flight (triggered by `AppLifecycleState.resumed` after an interrupted stream), the chat screen SHALL render a brief banner communicating the reconnect state to the user.

#### Scenario: Banner appears on resume with interrupted stream

- **WHEN** `attemptReconnect()` begins running
- **THEN** a banner SHALL render reading `"Reconnecting…"` (or equivalent)
- **WHEN** `attemptReconnect()` resolves successfully
- **THEN** the banner SHALL disappear

#### Scenario: Banner does not appear on routine resume

- **WHEN** the app resumes from background and there was no interrupted stream
- **THEN** the banner SHALL NOT appear (no spurious "Reconnecting…" on every foreground transition)

### Requirement: currentTurnStatus is observable

The `ChatNotifier` (or a sibling provider) SHALL expose a `currentTurnStatus` derived state with at least: `turnId`, `lastEventKind`, `lastEventAt`, `currentToolName`. This is the data the progress card consumes. Other UI surfaces MAY observe it.

#### Scenario: Selector updates on each SSE event

- **WHEN** any SSE event for the active turn is received
- **THEN** `currentTurnStatus` SHALL be updated with at least the event's kind and timestamp before the next widget rebuild
