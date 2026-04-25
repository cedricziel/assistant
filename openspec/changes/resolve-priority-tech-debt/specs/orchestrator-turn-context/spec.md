## ADDED Requirements

### Requirement: TurnContext bundles per-turn state

`assistant-runtime` SHALL define a `TurnContext` struct that aggregates the parameters currently passed individually through the ReAct loop entry points in `crates/runtime/src/orchestrator/{mod,worker,dispatch,turn_control}.rs`. Functions in those modules SHALL accept `TurnContext` (or `&TurnContext`/`&mut TurnContext`) instead of long argument lists.

#### Scenario: no too_many_arguments allows in orchestrator

- **WHEN** `cargo clippy -p assistant-runtime -- -D warnings` is run
- **THEN** there are zero `#[allow(clippy::too_many_arguments)]` annotations remaining in `crates/runtime/src/orchestrator/{mod,worker,dispatch,turn_control}.rs`

#### Scenario: function signatures use the context

- **WHEN** an orchestrator turn entry point is invoked
- **THEN** it receives a `TurnContext` value that owns or borrows the per-turn dependencies (LLM provider handle, tool registry, persona, conversation handle, span/trace handles, error sink, etc.)

### Requirement: turn-level error tracking

`TurnContext` SHALL provide an error-tracking sink (e.g., `turn_had_errors: bool` updated through a `record_tool_error(...)` method or an internal counter) that is set whenever tool dispatch returns a non-fatal error. The orchestrator main loop SHALL use this signal in place of the hard-coded `let turn_had_errors = false` at `crates/runtime/src/orchestrator/mod.rs:962`.

#### Scenario: tool error is recorded

- **WHEN** a tool dispatch returns `ToolOutput::error(...)` or a recoverable `Err(...)` during a turn
- **THEN** the `TurnContext` records the error so that `turn_had_errors` is `true` for the remainder of that turn

#### Scenario: turn-end uses recorded signal

- **WHEN** a turn completes
- **THEN** the orchestrator reads `turn_had_errors` from `TurnContext` (not a hard-coded `false`) and uses it to drive subsequent retry, telemetry, and post-turn logic

#### Scenario: clean turn reports no errors

- **WHEN** a turn completes with no tool errors
- **THEN** `turn_had_errors` is `false` and downstream code is unchanged from current behavior

### Requirement: tests updated to construct TurnContext

The test suite in `crates/runtime/src/orchestrator/tests.rs` SHALL be updated so that all tests construct a `TurnContext` (via a `TurnContext::for_test(...)` helper or the production builder) instead of passing positional arguments.

#### Scenario: orchestrator test suite passes

- **WHEN** `cargo test -p assistant-runtime` is run
- **THEN** all orchestrator tests pass and no test relies on a positional-argument entry point that has been removed
