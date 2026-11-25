## ADDED Requirements

### Requirement: A running turn can request an interaction from its owner

The runtime SHALL let an interactive turn put a typed interaction to the user
who owns the turn. Each interaction SHALL have a server-assigned
`interaction_id` (UUID), a `kind`, a kind-specific request, a `created_at`, and
an `expires_at`. It SHALL be persisted in the space's `interactions` store with
status `pending` before it is announced. The turn SHALL suspend until the
interaction is resolved, expires, or the turn is cancelled.

#### Scenario: Interaction is announced on the run stream

- **WHEN** an interactive web turn requests a `tool_approval` interaction
- **THEN** the run's SSE stream SHALL carry an `interaction_requested` event
  whose JSON contains `interaction_id`, `kind`, `created_at`, `expires_at` and
  the kind-specific fields
- **AND** the event SHALL be persisted to the conversation event log with the
  next sequence number of that run

#### Scenario: Non-interactive turns never request interactions

- **WHEN** a scheduler, messenger, A2A, quick-message, voice or subagent turn
  reaches a point that would need an interaction
- **THEN** no interaction SHALL be created AND the turn SHALL continue with the
  non-interactive fallback (denial for approvals)

### Requirement: Interactions work when the turn runs in another process

The interaction channel SHALL NOT assume that the turn runs in the process that
serves HTTP. A waiting turn SHALL observe a resolution written by any process
through the shared `interactions` store within 2 seconds. The turn's events,
including `interaction_requested` and `interaction_resolved`, SHALL reach the
HTTP process's run stream over the message bus (topic `turn.event`, keyed by
the run id) when no in-process sink exists. The interaction-backed
confirmation callback SHALL be wired in the process that runs turns.

#### Scenario: Split web-ui and orchestrator

- **GIVEN** the HTTP routes and the turn worker run in separate runtimes that
  share `space.db` and the bus
- **WHEN** a web turn yields `Ask` and the owner posts `allow_once` to the
  HTTP runtime
- **THEN** the SSE stream SHALL carry `interaction_requested`, then
  `interaction_resolved{resolved}`, then the tool result and `done`

#### Scenario: Waiting turn in the same process

- **WHEN** the respond route and the waiting turn share a process
- **THEN** the turn SHALL resume without waiting for the store poll

### Requirement: The run stream outlives the SSE client

The turn runner SHALL keep draining, persisting and broadcasting the run's
events after the SSE client disconnects, and SHALL only stop forwarding them to
that client. It SHALL run the end-of-run interaction cleanup on every exit
path.

#### Scenario: Answer after closing the tab

- **GIVEN** a run with a pending approval whose SSE client has disconnected
- **WHEN** the owner answers `allow_once` through the respond route
- **AND** a client then replays the run with `since=0`
- **THEN** the replay SHALL contain `interaction_resolved`, the tool result and
  `done`

### Requirement: Only streaming web chat turns are interactive

`ExecutionContext.interactive` SHALL come from an explicit submit option, not
from `Interface`. It SHALL be true only for CLI REPL turns and for turns
submitted by the shared streaming turn runner: `POST
/api/conversations/{id}/messages` and the edit and regenerate routes of
`message-branching`. `ExecutionContext` SHALL carry the turn's `run_id`.

#### Scenario: Quick message is not interactive

- **WHEN** a turn is submitted through `POST /api/quick-message`
- **THEN** its execution context SHALL have `interactive = false`

#### Scenario: Voice is not interactive

- **WHEN** a turn is submitted through the voice message route
- **THEN** its execution context SHALL have `interactive = false`

#### Scenario: Streaming chat is interactive

- **WHEN** a turn is submitted through `POST /api/conversations/{id}/messages`
- **THEN** its execution context SHALL have `interactive = true` and `run_id`
  equal to the SSE `run_started.run_id`

#### Scenario: CLI REPL stays interactive

- **WHEN** a turn is submitted from the CLI REPL
- **THEN** its execution context SHALL have `interactive = true`

#### Scenario: One-shot CLI and background orchestrator are not

- **WHEN** a turn is submitted by a one-shot CLI command, or by a process
  started with `--no-repl`
- **THEN** its execution context SHALL have `interactive = false`

### Requirement: Owners answer through one respond endpoint

The server SHALL expose
`POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`
(`operationId: respond_to_interaction`) taking a `RespondToInteractionRequest`
tagged by `kind` (`tool_approval`, `question`, or `decline`), with unknown
fields rejected. It SHALL validate the answer through the handler registered
for the interaction's kind. It SHALL return `200` with an `InteractionDetail`.
It SHALL require `Access::Write` in the selected space, the
`conversations:write` scope, ownership of the conversation, and that the
interaction's recorded `user_id`, `conversation_id` and `run_id` match the
caller and the path. Every error, including conversation lookup failures, SHALL
use the `{"error": …}` envelope.

#### Scenario: Owner answers

- **WHEN** the turn owner posts a valid answer to a pending interaction
- **THEN** the response SHALL be `200` with `status = resolved`
- **AND** the waiting turn SHALL resume with that answer

#### Scenario: Another member cannot answer

- **WHEN** a different member of the same space posts to the endpoint
- **THEN** the response SHALL be `404` with an `{"error": …}` body AND the
  interaction SHALL stay pending

#### Scenario: Viewer is refused

- **WHEN** a caller with only viewer access posts to the endpoint
- **THEN** the response SHALL be `403`

#### Scenario: Wrong run in the path

- **WHEN** the `turn_id` in the path differs from the interaction's run
- **THEN** the response SHALL be `404`

#### Scenario: Answer does not fit the kind

- **WHEN** a `question` answer names a `question_id` or option id that the
  request did not contain, or a `tool_approval` decision is not among the
  offered `options`
- **THEN** the response SHALL be `400`

#### Scenario: Oversized answer

- **WHEN** a free-text answer is longer than 4000 characters
- **THEN** the response SHALL be `400`

#### Scenario: Oversized body

- **WHEN** the request body is larger than 64 KiB
- **THEN** the response SHALL be `413`

### Requirement: Interaction kinds are registered, not hard-coded

The runtime SHALL hold a registry of interaction kind handlers. Each handler
SHALL provide response validation, the LLM observation for each outcome, push
copy, a progress label, and whether the kind renders a card in the timeline.
`InteractionDetail` SHALL carry `renders_card`. This change SHALL register
`tool_approval` and `question`. Sibling changes SHALL add kinds by registering
a handler and adding a variant to `RespondToInteractionRequest`.

#### Scenario: Registered kind without a card

- **WHEN** a kind registered with `renders_card = false` requests an interaction
- **THEN** `interaction_requested` SHALL carry `renders_card: false`

#### Scenario: Unregistered kind

- **WHEN** the respond route receives an answer for an interaction whose kind
  has no registered handler
- **THEN** the response SHALL be `400`

### Requirement: Interaction requests are bounded in size

The `arguments` carried by a `tool_approval` request SHALL be capped at 16 KiB
of serialized JSON in the stored row, the event, the SSE frame and
`InteractionDetail`. Longer values SHALL be cut with a `…[truncated]` marker
and the request SHALL carry `arguments_truncated: true`. The tool SHALL still
run with its full arguments.

#### Scenario: Large file write

- **WHEN** a `file-write` call whose content is 2 MiB yields `Ask`
- **THEN** the `interaction_requested` payload SHALL be at most 16 KiB of
  arguments AND SHALL carry `arguments_truncated: true`

### Requirement: Responses are idempotent and first-writer-wins

Resolving an interaction SHALL be an atomic compare-and-set on `status =
pending`. Repeating the same answer SHALL return `200` with the stored
resolution. A different answer after resolution SHALL return `409`. An answer to
an expired or cancelled interaction SHALL return `410`.

#### Scenario: Double tap

- **WHEN** the owner posts the same `allow_once` answer twice
- **THEN** both responses SHALL be `200` with identical bodies AND the tool
  SHALL run once

#### Scenario: Conflicting answers from two devices

- **WHEN** one device posts `allow_once` and another then posts `reject_once`
- **THEN** the first SHALL win AND the second SHALL receive `409`

#### Scenario: Late answer

- **WHEN** the owner answers after `expires_at`
- **THEN** the response SHALL be `410`

### Requirement: A pending interaction without a live waiter is expired

A waiting turn SHALL refresh the interaction's `waiter_seen_at` at least every
15 seconds. A `pending` interaction whose `waiter_seen_at` is older than 45
seconds SHALL be treated as having no live waiter: the respond route SHALL mark
it `expired` and return `410`, and the recovery sweep SHALL mark it `expired`.

#### Scenario: Answer after the turn process restarted

- **GIVEN** the turn process restarted while an interaction was pending
- **WHEN** the owner answers it
- **THEN** the response SHALL be `410` AND the interaction SHALL be `expired`

#### Scenario: Sweep after restart

- **WHEN** the recovery sweep runs and a pending interaction's waiter has been
  silent for longer than 45 seconds
- **THEN** the interaction SHALL become `expired`, whether or not its run still
  has an active bus message

### Requirement: Every interaction ends with one resolution event

For every run whose turn runner is alive, the run's SSE stream SHALL carry
exactly one `interaction_resolved` event per interaction, with
`interaction_id`, `kind`, `outcome` (`resolved`, `cancelled` or `expired`) and
`resolved_at`. On `resolved` it SHALL also carry `response`, plus `rule_id`
when a rule was saved. Runs without a live runner SHALL carry at most one. The
event SHALL be persisted in run order through the runner that owns the run's
sequence numbers, and the stored interaction SHALL record that its resolution
was emitted.

#### Scenario: Resolution follows request on replay

- **WHEN** a client replays a run with `since=0` after the interaction was
  answered
- **THEN** it SHALL receive `interaction_requested` followed later by
  `interaction_resolved{outcome: resolved}` for the same id

#### Scenario: Expiry

- **WHEN** `expires_at` passes with no answer
- **THEN** the row SHALL become `expired`, the stream SHALL carry
  `interaction_resolved{outcome: expired}`, and the turn SHALL resume with an
  expired answer

#### Scenario: Answer wins the race with cancellation

- **GIVEN** the respond route has resolved an interaction
- **WHEN** the turn is cancelled before the waiter emits the resolution
- **THEN** the runner SHALL emit `interaction_resolved{outcome: resolved}` for
  it before the terminal frame

### Requirement: Cancelling a turn cancels its pending interactions

When a turn ends while interactions are pending, whether by cancellation,
error, or the turn deadline, those interactions SHALL become `cancelled`. The
runner SHALL emit an `interaction_resolved` for every interaction of the run
whose resolution was not yet emitted, before the terminal `done` or
`agent_error` frame. When the recovery sweep closes an orphaned run with a
synthetic terminal frame, it SHALL first append
`interaction_resolved{outcome: expired}` for each such interaction.

#### Scenario: Stop while waiting for approval

- **WHEN** the owner calls `POST …/turns/{turn_id}/cancel` while an approval is
  pending
- **THEN** the stream SHALL carry `interaction_resolved{outcome: cancelled}`
  and then `agent_error{reason: cancelled}`
- **AND** a later respond call SHALL return `410`

#### Scenario: Branching while waiting

- **WHEN** the owner edits or regenerates a message while an approval for the
  conversation's running turn is pending
- **THEN** the waiting turn SHALL be cancelled through the same path as Stop,
  the interaction SHALL end `cancelled` (with `interaction_resolved` before the
  terminal frame), the tool SHALL NOT run, AND the new branch's turn SHALL
  start

#### Scenario: Orphaned run closed by the sweep

- **WHEN** the recovery sweep appends a synthetic terminal frame to a run with
  a pending interaction
- **THEN** the event log SHALL contain `interaction_resolved{outcome: expired}`
  for it before the terminal frame

### Requirement: Interaction timeout is configurable per org

`[permissions] interaction_timeout_secs` in `org.toml` SHALL set the expiry for
every interaction kind, with a default of `600` (10 minutes). Values outside `30..=3600`
SHALL fail config loading with an error naming the key. `expires_at` SHALL NOT
be later than the turn's own deadline.

#### Scenario: Out-of-range timeout

- **WHEN** `interaction_timeout_secs = 5`
- **THEN** loading the org config SHALL fail

#### Scenario: Persona deadline is shorter

- **GIVEN** a Persona with `turn_timeout_secs = 120` and an org with
  `interaction_timeout_secs = 600`
- **WHEN** a turn requests an interaction 30 seconds after it started
- **THEN** `expires_at` SHALL be at most 90 seconds after `created_at`

### Requirement: Long waits do not get the turn redelivered

While a turn runs, the worker SHALL refresh the claim on its bus message at
least every 60 seconds (SQLite: `claimed_at`; NATS: in-progress ack), so that a
turn waiting on an interaction is not reaped and redelivered by the stale-claim
sweep.

#### Scenario: Turn waits longer than the stale timeout

- **WHEN** a turn has waited 7 minutes on a pending interaction
- **THEN** its bus message SHALL still be claimed by the same worker AND the
  turn SHALL NOT have been started a second time

### Requirement: Clients can discover pending interactions

`GET /api/conversations/{conversation_id}/interactions` (`operationId:
list_conversation_interactions`) SHALL return a bare JSON array of
`InteractionDetail` for the caller's conversation, filterable by
`?status=pending`. `ConversationDetail` SHALL include `interactions`, all of the
conversation's interactions as `InteractionDetail` in creation order.

#### Scenario: Opening a conversation on another device

- **WHEN** a client opens a conversation with a pending interaction
- **THEN** the pending list SHALL contain it with its `run_id`, so the client
  can attach to `…/runs/{run_id}/events/stream`

### Requirement: Clients ignore what they do not understand

The Flutter client SHALL parse `interaction_requested` and
`interaction_resolved` into the sealed `StreamEvent` hierarchy. It SHALL keep a
registry of interaction kinds it can render. An interaction whose `kind` is not
registered and whose `renders_card` is not false SHALL render as an inert
notice and SHALL never be answered automatically. A kind with
`renders_card = false` SHALL produce no timeline card but SHALL still mark the
turn as awaiting input. Unknown SSE event names SHALL continue to be ignored.

#### Scenario: Unknown kind

- **WHEN** the client receives `interaction_requested` with `kind = "future_thing"`
- **THEN** it SHALL show a notice that the request needs a newer app AND SHALL
  NOT call the respond endpoint

#### Scenario: Kind rendered elsewhere

- **WHEN** the client receives `interaction_requested` with
  `renders_card: false`
- **THEN** it SHALL add no timeline card AND the progress card SHALL read
  "Waiting for your input" unless the kind registers another label
