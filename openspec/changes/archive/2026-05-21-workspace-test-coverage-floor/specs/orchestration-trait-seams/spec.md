## ADDED Requirements

### Requirement: trait facades for load-bearing types live in `assistant-core`

The `assistant-core` crate SHALL define trait facades for the workspace's
load-bearing concrete types:

- `OrchestrationEngine` — the subset of `assistant_runtime::Orchestrator`
  consumed by external callers (`assistant-mcp-server`, `assistant-web-ui`,
  `assistant-interfaces`).
- `ToolDispatcher` — the subset of `assistant_tool_executor::ToolExecutor`
  consumed by external callers.
- `SkillCatalog` — the subset of `assistant_storage::SkillRegistry`
  consumed by external callers.

The trait surface SHALL contain only the methods external callers
actually use. New trait methods MUST be added when a new consumer
requires them; the concrete struct surface is not the source of truth.

#### Scenario: trait facade method surface

- **WHEN** an external caller (`mcp-server`, `web-ui`, `interfaces`)
  needs a method on `Orchestrator`/`ToolExecutor`/`SkillRegistry`
- **THEN** the method is exposed on the corresponding trait in
  `assistant-core`, and the consumer takes `Arc<dyn Trait>` rather
  than `Arc<Concrete>`

#### Scenario: trait facade lives in core

- **WHEN** a developer searches for the `OrchestrationEngine` definition
- **THEN** it is found in `crates/core/src/orchestration.rs` (or
  similar), not in `crates/runtime/`

### Requirement: consumers depend on the trait, not the concrete type

Consumers of orchestration, tool dispatch, or skill catalog functionality SHALL declare dependencies in terms of the `assistant-core` traits, not the concrete types. The exception is `assistant-runtime` itself (which defines and owns the concrete `Orchestrator`).

#### Scenario: mcp-server takes the trait

- **WHEN** `assistant-mcp-server` exposes `handle_request`
- **THEN** its signature accepts `Arc<dyn OrchestrationEngine>`,
  `Arc<dyn ToolDispatcher>`, `Arc<dyn SkillCatalog>` — never the
  concrete types

#### Scenario: web-ui takes the trait

- **WHEN** `assistant-web-ui::ApiState` is constructed
- **THEN** its orchestrator field is typed `Arc<dyn OrchestrationEngine>`

#### Scenario: messenger adapter takes the trait

- **WHEN** an adapter in `assistant-interfaces` is constructed
- **THEN** its orchestrator handle is typed `Arc<dyn OrchestrationEngine>`

### Requirement: concrete types still implement the trait

Concrete types `Orchestrator`, `ToolExecutor`, and `SkillRegistry` SHALL implement their respective trait facades. Adoption is additive — the concrete struct fields and method surface are not changed by this spec.

#### Scenario: Orchestrator implements OrchestrationEngine

- **WHEN** `crates/runtime/src/orchestrator/mod.rs` is inspected
- **THEN** an `impl OrchestrationEngine for Orchestrator { ... }` block
  exists alongside the existing `impl Orchestrator { ... }` blocks

### Requirement: tests use hand-rolled fakes against the trait

Unit tests for code that consumes the trait facades SHALL construct
hand-rolled fakes (test-only structs implementing the trait) rather
than booting a real `Orchestrator`/`ToolExecutor`/`SkillRegistry`.
This rule unblocks coverage for `mcp-server`, messenger runner
dispatch, and `web-ui` handlers without dragging the full runtime
into each test.

#### Scenario: mcp-server unit test

- **WHEN** `crates/mcp-server/tests/dispatch.rs` runs `handle_request`
- **THEN** the test constructs `FakeOrchestrationEngine`,
  `FakeToolDispatcher`, `FakeSkillCatalog` instances; it MUST NOT
  construct a real `Orchestrator`
