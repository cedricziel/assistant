## ADDED Requirements

### Requirement: Turn dispatch requires an AuthContext (compiler-enforced)

Every turn-submission entry point SHALL require an `AuthContext` as a parameter,
so that dispatching a turn without one does not compile. This MUST hold across
all three submission surfaces — the inherent `Orchestrator::submit_turn*`
methods, the `AssistantInterface` trait, and the `OrchestrationEngine` trait —
and their implementations. There MUST be no turn-submission path that defaults
the caller identity silently.

Network-facing adapters (the `/api` handlers, and A2A in a later phase) MUST
pass an `AuthContext` resolved from the request. Trusted local/non-network
callers (the scheduler, BOOT hooks, the CLI, MCP stdio, messenger adapters,
tests) MUST pass `AuthContext::system()` explicitly at the call site.

#### Scenario: Streaming handler resolves AuthContext before dispatch

- **WHEN** the `/api` streaming message handler is invoked
- **THEN** it SHALL resolve an `AuthContext` (via the `Extension<AuthContext>`
  populated by the auth middleware) before submitting the turn

#### Scenario: A future adapter cannot skip the contract

- **WHEN** a new turn-accepting caller is added without supplying an
  `AuthContext`
- **THEN** the workspace SHALL fail to compile until one is supplied (either a
  request-resolved context or an explicit `AuthContext::system()`)

#### Scenario: Trusted local callers name a system identity explicitly

- **WHEN** a non-network caller (scheduler, CLI, MCP, BOOT hook) submits a turn
- **THEN** it SHALL pass `AuthContext::system()` rather than relying on any
  silent default

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
