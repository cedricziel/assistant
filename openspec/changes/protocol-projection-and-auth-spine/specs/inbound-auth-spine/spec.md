## ADDED Requirements

### Requirement: Inbound turn dispatch requires a resolved AuthContext

Every inbound adapter that dispatches a turn to the Orchestrator SHALL first
resolve an `AuthContext`. The requirement MUST be enforced, not merely
conventional: either by a dispatch seam that takes `&AuthContext` (so dispatch
without one does not compile), or by a conformance test that fails when an
inbound turn-accepting handler does not resolve `AuthContext`. An inbound
turn-accepting handler MUST NOT submit a turn when no `AuthContext` is
available.

#### Scenario: Streaming handler resolves AuthContext before dispatch

- **WHEN** the `/api` streaming message handler is invoked
- **THEN** it SHALL resolve an `AuthContext` (via `AuthExtractor`) before
  submitting the turn

#### Scenario: A future adapter cannot skip the contract

- **WHEN** a new inbound turn-accepting handler is added without resolving
  `AuthContext`
- **THEN** the enforcement (compiler seam or conformance test) SHALL reject it
  (build or test failure)

### Requirement: The resolved AuthContext gates message posting

The `/api` streaming message handler SHALL use the resolved `AuthContext` to
authorize the request, rejecting callers that lack the scope required to post
messages. The resolved context MUST be used (not resolved-and-ignored).

#### Scenario: Caller lacking the posting scope is rejected

- **WHEN** an authenticated caller without the message-posting scope invokes the
  streaming message handler
- **THEN** the handler SHALL reject the request with `403 Forbidden` AND SHALL
  NOT submit a turn

#### Scenario: Authorized caller proceeds unchanged

- **WHEN** an authenticated caller holding the message-posting scope invokes the
  streaming message handler
- **THEN** the turn SHALL be dispatched as before, with no change to the SSE
  response shape
