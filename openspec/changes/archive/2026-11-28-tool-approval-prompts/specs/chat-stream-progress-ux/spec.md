## MODIFIED Requirements

### Requirement: Stall indicator on prolonged silence

When the in-flight turn has had no SSE event for longer than the stall threshold (default: 30 seconds), the progress card SHALL surface the silence with an elapsed-time indicator. No automatic cancellation SHALL occur. While the turn has a pending interaction (an `interaction_requested` without its `interaction_resolved`), the silence is expected and the stall indicator SHALL NOT be shown.

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

#### Scenario: No stall while awaiting input

- **WHEN** the active turn has a pending interaction and no event has arrived for 5 minutes
- **THEN** the card SHALL keep its awaiting-input label and SHALL NOT show the stall label

## ADDED Requirements

### Requirement: Progress card shows awaiting input

The progress card SHALL show the pending interaction kind's progress label instead of its activity label: "Waiting for your approval" for `tool_approval`, "Waiting for your answer" for `question`, and "Waiting for your input" for a kind with no label of its own. The label SHALL be passed to the pure `TurnProgressCard` as `awaitingLabel`; there SHALL be no separate awaiting-input `TurnEventKind`. When the interaction resolves, the card SHALL return to the label of the next event. A turn status of `awaiting_input` SHALL be treated like `running` by the stalled-stream probe.

#### Scenario: Approval pending

- **WHEN** the latest event of the active turn is `interaction_requested{kind: tool_approval}`
- **THEN** the card label SHALL be "Waiting for your approval"

#### Scenario: Status probe while waiting

- **WHEN** the stalled-stream probe receives `state = awaiting_input`
- **THEN** the client SHALL keep the stream open AND SHALL NOT fall back to recovery

#### Scenario: Resolved

- **WHEN** `interaction_resolved` arrives for that interaction
- **THEN** the card SHALL no longer show "Waiting for your approval"
