## ADDED Requirements

### Requirement: A2A message handlers dispatch real Orchestrator turns

The A2A `message/send` and `message/stream` handlers SHALL produce the agent's
real response by submitting a turn to the shared Orchestrator (tagged
`Interface::A2a`). They MUST NOT return a stubbed/placeholder reply, and MUST NOT
implement a parallel reasoning or tool-dispatch path.

#### Scenario: message/send returns the agent's real reply

- **WHEN** an authenticated A2A `message/send` request arrives with user text
- **THEN** the handler SHALL submit a turn to the Orchestrator AND return an A2A
  `Task` whose final message is the agent's actual answer (not the placeholder
  string)

#### Scenario: message/stream streams real turn events

- **WHEN** an authenticated A2A `message/stream` request arrives
- **THEN** the handler SHALL register a token sink, run the turn, and emit A2A
  `StreamResponse` frames derived from the Orchestrator's events, ending in a
  `Completed` task

### Requirement: A2A message handlers require an AuthContext and gate posting

The A2A `message/send` and `message/stream` handlers SHALL resolve an
`AuthContext` and SHALL reject callers lacking `conversations:write` with `403`,
reusing the same authorization rule as the `/api` streaming handlers. A turn
MUST NOT be dispatched for an under-scoped or unauthenticated caller.

#### Scenario: under-scoped caller is rejected

- **WHEN** an authenticated A2A caller without `conversations:write` invokes
  `message/send` or `message/stream`
- **THEN** the handler SHALL respond `403 Forbidden` AND SHALL NOT submit a turn

#### Scenario: authorized caller proceeds

- **WHEN** an authenticated A2A caller holding `conversations:write` invokes the
  endpoint
- **THEN** the turn SHALL be dispatched and the A2A response produced

### Requirement: OrchestratorEvents are projected to the A2A wire by one projector

Conversion from `OrchestratorEvent` to A2A `StreamResponse` SHALL be performed by
a single `A2aProjector` implementing the shared `StreamProjector` trait. The A2A
handler MUST NOT inline-`match` over `OrchestratorEvent`, and the projector's
`match` MUST NOT use a catch-all `_` arm, so a new variant fails to compile until
projected. A totality test SHALL cover every variant.

#### Scenario: every event variant is projected

- **WHEN** any `OrchestratorEvent` variant (including a nested `SubagentEvent`)
  is projected by `A2aProjector`
- **THEN** it SHALL yield at least one `StreamResponse` frame

#### Scenario: adding a variant fails to compile until handled

- **WHEN** a new `OrchestratorEvent` variant is added
- **THEN** `A2aProjector` SHALL fail to compile until the variant is projected

### Requirement: A2A context_id maps to a conversation for multi-turn continuity

A2A messages sharing a `context_id` SHALL be dispatched against the same
conversation, so multi-turn A2A threads accumulate history. When `context_id` is
absent, the handler SHALL create a new conversation and surface its identifier as
the task's `context_id`.

#### Scenario: same context_id continues one conversation

- **WHEN** two A2A messages carry the same `context_id`
- **THEN** both turns SHALL run against the same conversation

#### Scenario: missing context_id starts a new conversation

- **WHEN** an A2A message arrives with no `context_id`
- **THEN** a new conversation SHALL be created AND its identifier returned as the
  task's `context_id`

### Requirement: A2A tasks are durably persisted

A2A tasks SHALL be persisted in the space database so they survive process
restart and remain retrievable via `GET /tasks` and `GET /tasks/{id}`. Task
storage SHALL use an `A2aTaskStore` trait with a SQLite-backed implementation in
production and an in-memory implementation for tests (the ADR-0009 trait pair).
Live SSE subscriptions MAY remain in-memory; only durable task state MUST be
persisted.

#### Scenario: a task survives a fresh store over the same database

- **WHEN** a task is created and completed through `SqliteA2aTaskStore`, then a
  new store is opened over the same pool
- **THEN** `get_task` SHALL return the task with its final state and history

#### Scenario: in-memory and SQLite stores behave identically

- **WHEN** the same create/update/get/list sequence runs against
  `InMemoryA2aTaskStore` and `SqliteA2aTaskStore`
- **THEN** both SHALL return equivalent task snapshots
