## ADDED Requirements

### Requirement: Every inbound protocol adapter dispatches through the Orchestrator

Every inbound protocol adapter that accepts a user or agent message SHALL
execute it by submitting a turn to the shared Orchestrator. This includes
OpenAPI `/api`, AG-UI, A2A inbound, ACP-as-agent, and the messenger adapters.
An adapter MUST NOT implement a parallel reasoning loop, a parallel
tool-dispatch path, or a parallel message/turn data model.

#### Scenario: A2A message is processed by the Orchestrator

- **WHEN** an authenticated A2A `message/send` request arrives
- **THEN** the adapter SHALL submit a turn to the Orchestrator AND the response
  SHALL be produced from the Orchestrator's output — never from a stubbed or
  adapter-local reply

#### Scenario: No adapter owns a second brain

- **WHEN** any inbound adapter is added or modified
- **THEN** it SHALL NOT introduce its own ReAct loop or tool registry; tool
  execution and reasoning SHALL remain in `assistant-runtime`

### Requirement: Every inbound protocol adapter resolves a shared AuthContext before dispatch

Before submitting a turn, an inbound adapter SHALL resolve the request to the
same `AuthContext` used by `/api` — carrying user identity, org, space roles,
and scopes — via either an OAuth token or an API key. An adapter MUST NOT
dispatch a turn without a resolved `AuthContext`, and the resolved org/space
SHALL scope all storage and tool access for that turn.

#### Scenario: Unauthenticated request is rejected

- **WHEN** a request reaches a protected protocol endpoint with no valid token
  or API key
- **THEN** the adapter SHALL reject it (401) AND SHALL NOT submit a turn

#### Scenario: Turn is scoped to the caller's org and space

- **WHEN** an authenticated request is dispatched through any adapter
- **THEN** the Orchestrator turn SHALL run within the caller's org and space
  AND SHALL NOT read or write data outside that scope

### Requirement: The Orchestrator stream is projected to wire via a single projection layer

Conversion from `OrchestratorEvent` to a protocol's wire events SHALL be
performed by a single, shared projection layer with exactly one projector per
protocol. Protocol handlers MUST NOT hand-serialize `OrchestratorEvent`
inline. Adding a new streaming protocol SHALL require adding one projector plus
its conformance tests, with no change to the Orchestrator or to other
protocols' projectors.

#### Scenario: One projector per protocol

- **WHEN** a new streaming protocol is added
- **THEN** exactly one new projector function/type SHALL be introduced AND no
  existing handler SHALL gain a bespoke `OrchestratorEvent` serialization block

#### Scenario: Projection is covered by conformance tests

- **WHEN** a projector maps `OrchestratorEvent` variants to wire events
- **THEN** a conformance test SHALL assert the mapping for each supported
  variant, so a new `OrchestratorEvent` variant forces an explicit projector
  decision rather than silently dropping

### Requirement: The domain content model is the single serialization source

All protocol adapters SHALL serialize from the shared domain content model
(`ContentBlock` and related core types). The internal domain model MUST NOT be
replaced by, or coupled to, any external protocol's wire types; external
protocol types SHALL exist only at the adapter boundary.

#### Scenario: Adapter serializes from the domain model

- **WHEN** any adapter renders message content onto its wire format
- **THEN** it SHALL derive that content from `ContentBlock` (or the shared core
  types) AND core crates SHALL NOT depend on a protocol crate's wire types

#### Scenario: CRUD management surface stays on OpenAPI

- **WHEN** a management resource (orgs, spaces, personas, skills, traces, API
  keys, etc.) is exposed
- **THEN** it SHALL be served by the OpenAPI `/api` surface AND SHALL NOT be
  re-modeled inside an agent protocol's message/task payloads
