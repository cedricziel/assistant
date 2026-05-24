## ADDED Requirements

### Requirement: A single projection layer maps OrchestratorEvent to wire frames

The workspace SHALL provide one shared projection layer that converts an
`OrchestratorEvent` into a protocol's wire frames, with exactly one projector
per wire. Inbound/streaming handlers MUST consume a projector and MUST NOT
re-implement an inline `OrchestratorEvent` serialization. The projection layer
SHALL live in `assistant-runtime` (alongside `OrchestratorEvent`) and emit a
transport-neutral frame, so transport concerns (persistence, sequencing,
batching, framing) remain in the adapter.

#### Scenario: SSE handler projects instead of inline-matching

- **WHEN** the `/api` streaming handler serializes an `OrchestratorEvent`
- **THEN** it SHALL call the shared SSE projector AND SHALL NOT contain an
  inline `match` over `OrchestratorEvent` variants

#### Scenario: Adding a wire adds exactly one projector

- **WHEN** a new streaming wire is introduced
- **THEN** exactly one new projector SHALL be added AND no existing adapter
  SHALL gain a bespoke `OrchestratorEvent` serialization block

### Requirement: Projection is total over all OrchestratorEvent variants

Every projector SHALL handle every `OrchestratorEvent` variant. Projector match
expressions MUST NOT use a catch-all `_` arm, so that adding a new variant fails
to compile until the variant is explicitly projected. A conformance test SHALL
construct a sample of every variant — including a nested `SubagentEvent` — and
assert each yields a non-empty projection.

#### Scenario: New variant fails to compile until projected

- **WHEN** a new variant is added to `OrchestratorEvent`
- **THEN** the projector SHALL fail to compile until the variant is handled
  (no `_` arm absorbs it)

#### Scenario: Nested subagent event is projected recursively

- **WHEN** a `SubagentEvent` wrapping an inner `Token` is projected
- **THEN** the projector SHALL produce at least one frame derived from the inner
  event, scoped to the subagent's `agent_id`

### Requirement: SSE wire output is byte-identical after extraction

The extracted SSE projector SHALL produce the same event names and the same
serialized payload JSON as the pre-refactor inline mapping. A golden test SHALL
assert parity for a representative event sequence; any difference in event name
or payload bytes SHALL fail the test.

#### Scenario: Representative sequence matches the golden output

- **WHEN** a sequence containing `Token`, `Thinking`, `Status`, `ToolResult`,
  `SkillComplete`, `SubagentStarted`, `SubagentEvent`, `AudioReady`, and
  `AgentError` is projected to SSE
- **THEN** the emitted event names and payload JSON SHALL match the recorded
  golden output exactly

#### Scenario: token and agent_error keep raw-text data form

- **WHEN** a `Token` or `AgentError` event is projected to SSE
- **THEN** the frame data SHALL be the raw text (not a JSON object), matching
  current behavior
