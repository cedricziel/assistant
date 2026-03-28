# Implementation Plan: Matrix Interface

**Branch**: `002-matrix-interface` | **Date**: 2026-03-28 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-matrix-interface/spec.md`

## Summary

Add a new `assistant-interface-matrix` crate that connects the assistant to a Matrix homeserver via the `matrix-sdk` Rust crate. The interface follows the same thin-adapter pattern as Slack, Mattermost, and Nextcloud: it receives room events, resolves the conversation context (by room/DM), routes each turn through the shared `Orchestrator`, and sends the reply back to the originating room. Configuration (`MatrixConfig`) is added to `assistant-core` and the CLI gains a `matrix` feature flag and a dedicated `Matrix` subcommand.

## Technical Context

**Language/Version**: Rust 2021 edition (workspace edition)
**Primary Dependencies**: `matrix-sdk` (Matrix client SDK for Rust), `tokio` (async runtime), `anyhow` (error handling), `tracing` (structured logging), `lru` (bounded conversation cache), `uuid` (conversation IDs), `serde` + `toml` (configuration)
**Storage**: N/A — conversation context is managed by the existing `Orchestrator`; the `matrix-sdk` crate maintains its own local session store for Matrix sync state
**Testing**: `cargo test` with `#[tokio::test]`; `StorageLayer::new_in_memory()` for DB-backed tests; unit tests for config, allowlist logic, and session-key derivation
**Target Platform**: Linux/macOS server process (same targets as existing interfaces)
**Project Type**: Library crate (`assistant-interface-matrix`) consumed by the unified CLI binary (`assistant-cli`)
**Performance Goals**: Respond within 30 seconds under normal load (same as other interfaces; bound by LLM latency, not message dispatch)
**Constraints**: Must work in both single-binary mode (all interfaces in one process) and distributed mode (Matrix worker separate from orchestrator); reconnect within 60 s on homeserver outage
**Scale/Scope**: Single bot account across multiple rooms; LRU-capped conversation map (10 000 entries) for unbounded room support

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

| Principle                 | Status  | Notes                                                                                                                                 |
| ------------------------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| I. Crate-First Modularity | ✅ PASS | New `crates/interface-matrix/` crate with single responsibility; independently compilable                                             |
| II. Trait-Based DI        | ✅ PASS | `Arc<Orchestrator>` injected; config type in `assistant-core`; no concrete types crossed                                              |
| III. Test Discipline      | ✅ PASS | `#[tokio::test]`, in-file unit tests, no live network in unit tests                                                                   |
| IV. Observability         | ✅ PASS | `tracing` macros throughout; no `println!`                                                                                            |
| V. Simplicity / YAGNI     | ✅ PASS | Thin adapter only; no novel abstractions; follows existing patterns exactly                                                           |
| VI. Interface Parity      | ✅ PASS | All turns route through `Orchestrator::submit_turn`; no interface-specific business logic                                             |
| VII. Code Quality Gate    | ✅ PASS | `fmt` + `clippy -D warnings` + `machete` enforced by pre-commit hooks                                                                 |
| VIII. Dual-Mode Parity    | ✅ PASS | Filtered worker pattern (`run_worker_filtered("matrix-worker", Some("Matrix"))`) matches Slack/Mattermost; distributed mode preserved |

No violations. Complexity Tracking table not required.

## Project Structure

### Documentation (this feature)

```text
specs/002-matrix-interface/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── matrix-config.md
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
crates/
├── core/src/types.rs                   # ADD: MatrixConfig struct, Matrix variant to Interface enum,
│                                       #      matrix field to AssistantConfig
├── interface-matrix/                   # NEW crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                      # pub mod config, runner, tools; pub use
│       ├── config.rs                   # MatrixConfigExt trait (resolved_homeserver_url, resolved_access_token)
│       ├── runner.rs                   # MatrixInterface struct, event handler, reconnect loop
│       └── tools.rs                   # build_matrix_tools() — matrix-reply extension tool
└── interface-cli/
    ├── Cargo.toml                      # ADD feature: matrix = ["dep:assistant-interface-matrix"]
    │                                   # ADD optional dep: assistant-interface-matrix
    └── src/main.rs                     # ADD: Matrix subcommand, matrix-only mode, background startup

Makefile                                # ADD: run-matrix target
docs/adr/                               # ADD: ADR for Matrix interface decision
```

**Structure Decision**: New crate follows the established `crates/interface-<name>/` pattern with `config.rs` / `runner.rs` / `tools.rs`. No new abstractions are introduced — the existing `Orchestrator`, `Interface` enum, and config pattern are extended minimally.
