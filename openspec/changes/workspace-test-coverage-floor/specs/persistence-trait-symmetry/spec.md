## ADDED Requirements

### Requirement: persistence components are defined as traits

Persistence components SHALL be exposed as traits owned by `assistant-core` (or, when domain-specific, by the domain crate). Concrete backend implementations SHALL live in the implementing crate. Consumers SHALL depend on the trait, never on the concrete struct.

In-scope persistence components include the following stores currently exposed as concrete structs:

- `ConversationStore`
- `ConversationEventStore`
- `CommandEventStore`
- `TraceStore`
- `LogStore`
- `MetricsStore`
- `AttachmentStore`
- `AudioStore`
- `AgentStore`
- `MemoryChunkStore`
- `PersonaStore`
- `PersonaSkillAccess`
- `PushSubscriptionStore`
- `RefinementStore`
- `ScheduledTasksStore`
- `SlackThreadStore`
- `WebhookStore`
- `WorkflowStore`
- `SkillRegistry` (read-side; `SkillCatalog` trait already proposed in `orchestration-trait-seams`)

#### Scenario: trait extraction for ConversationStore

- **WHEN** `assistant-core` is inspected
- **THEN** a `pub trait ConversationStore: Send + Sync` exists defining the methods consumers use (`create`, `get`, `list`, `update_title`, `mark_locked`, etc.)
- **AND** `crates/storage/src/conversations.rs` provides `pub struct SqliteConversationStore` implementing the trait
- **AND** consumers in `assistant-runtime`, `assistant-web-ui`, and `assistant-interfaces` accept `Arc<dyn ConversationStore>`, not the concrete struct

#### Scenario: consumer signature uses the trait

- **WHEN** a public function or struct field in any crate above `assistant-storage` holds a reference to a persistence component
- **THEN** the type is `Arc<dyn StoreTrait>` (or `&dyn StoreTrait`), never `Arc<SqliteConcreteStore>`

### Requirement: every persistence trait has an in-memory implementation

Each persistence trait listed above SHALL have an in-memory implementation living in the same crate as the SQLite implementation, typically as a sibling module or in the same file. The in-memory implementations SHALL be plain `pub` types, NOT gated behind `#[cfg(test)]` or a Cargo feature. They are alternative implementations of the trait — the same status as `SqliteConversationStore` — and follow the existing pattern set by `InMemoryConversationBroadcaster`, `OllamaProvider`, `AnthropicProvider`, etc.

The in-memory implementation SHALL:

- honor the trait contract for all non-exempt scenarios (see "Documented exemptions" below)
- enforce obvious referential cascades in code (e.g., deleting a conversation deletes its messages)
- preserve atomicity guarantees the trait requires (via internal `Mutex` / `RwLock`)
- avoid disk I/O entirely (no `tempfile`, no `std::fs`)
- live next to the SQLite implementation: `crates/storage/src/conversations.rs` holds both `SqliteConversationStore` and `InMemoryConversationStore`

#### Scenario: InMemoryConversationStore satisfies the contract

- **WHEN** a test constructs `assistant_storage::InMemoryConversationStore::new()`
- **THEN** the value implements `assistant_core::ConversationStore`
- **AND** all `ConversationStore` contract test scenarios pass against it

#### Scenario: in-memory impl ships ungated

- **WHEN** `crates/storage/Cargo.toml` is inspected
- **THEN** there is no `[features] test-support = []` entry covering the in-memory impls
- **AND** consumers MAY import `InMemoryConversationStore` from `assistant_storage` directly without enabling any feature

#### Scenario: upstream consumer test uses the in-memory impl

- **WHEN** a `crates/runtime` or `crates/web-ui` unit test needs persistence
- **THEN** it MAY use `Arc::new(InMemoryConversationStore::new())` instead of booting `StorageLayer::new_in_memory()`
- **AND** the test does not depend on SQLite migrations being current

### Requirement: workspace lint forbids test-only impls in production code

A `tests/workspace_test_impls_in_prod.rs` integration test SHALL fail compilation when test-only implementations (types named `InMemory*Store`, `InMemoryMessageBus`, `InMemorySkillCatalog`, `ScriptedLlmProvider`, `StubOrchestrationEngine`, `StubToolDispatcher`) are constructed in non-test production code paths.

Exempt paths:

- the defining module itself (the file containing the type's `impl` block)
- `#[cfg(test)]` modules anywhere in the workspace
- files under any `tests/` directory
- the `assistant-test-support` crate's `FixtureBuilder`
- top-level production fallback constructions documented in module-level docs (e.g., `InMemoryConversationBroadcaster` used when no external broadcaster is configured)

#### Scenario: lint catches accidental production use

- **WHEN** a contributor adds `let store = InMemoryConversationStore::new();` to a production `fn main()` or to a non-test method on a production struct
- **THEN** `cargo test -p assistant tests::workspace_test_impls_in_prod` fails with a message naming the file and line

#### Scenario: lint allows fixture builder

- **WHEN** `assistant-test-support::FixtureBuilder` constructs `InMemoryConversationStore`
- **THEN** the lint passes — the test-support crate is on the exempt list

### Requirement: `assistant-test-support` crate hosts composition helpers

A workspace crate `assistant-test-support` SHALL host cross-cutting test helpers that compose pieces from multiple crates:

- `FixtureBuilder` — wires `Arc<dyn OrchestrationEngine>` (from `assistant-runtime`'s stub), `Arc<dyn LlmProvider>` (from `assistant-llm-provider`'s `ScriptedLlmProvider`), in-memory stores (from `assistant-storage`), `Arc<dyn Clock>` (from `assistant-core::FakeClock`), and default config.
- `prelude` module — single-import re-exports of the most-used fakes and in-memory impls so test files have one `use` line.
- helper macros for common assertion patterns (`assert_recorded_llm_call!`, `assert_persisted_message!`).

The crate SHALL be added as a `[dev-dependencies]` of every crate that uses `FixtureBuilder` in tests; it SHALL NOT appear in any crate's `[dependencies]` section. The in-memory impls and fakes themselves do NOT live in this crate — they live next to their trait implementations in the owning crates.

The crate's `Cargo.toml` SHALL declare panic-free lint policy at the workspace baseline (`warn`), not `deny` — `.unwrap()` is ergonomic in test helpers.

#### Scenario: production binary excludes test-support

- **WHEN** `cargo build --release -p assistant-cli` is run
- **THEN** `assistant-test-support` does not appear in the resulting binary's dependency closure

#### Scenario: test depends on test-support as dev-dep

- **WHEN** `crates/web-ui/Cargo.toml` is inspected
- **THEN** `assistant-test-support` appears under `[dev-dependencies]`, not `[dependencies]`

#### Scenario: prelude consolidates imports

- **WHEN** a test file imports `use assistant_test_support::prelude::*;`
- **THEN** `InMemoryConversationStore`, `InMemoryMessageBus`, `ScriptedLlmProvider`, `StubOrchestrationEngine`, `FakeClock`, `FixtureBuilder` are all in scope

### Requirement: contract tests run every implementation of a trait

For every persistence trait covered by this spec, a contract test SHALL exist under `crates/storage/tests/contract/<trait_name>.rs` that runs the same test script against every implementation of the trait (SQLite-backed and in-memory). The contract test SHALL be parameterized so that adding a new implementation requires only registering it in the matrix.

#### Scenario: contract test matrix runs both impls

- **WHEN** `cargo test -p assistant-storage --test contract` is run
- **THEN** each contract scenario executes against `SqliteConversationStore` and `InMemoryConversationStore`
- **AND** any divergence in observable behavior (return values, error variants, ordering) fails the test

#### Scenario: new impl joins the matrix

- **WHEN** a contributor adds a third implementation of `ConversationStore`
- **THEN** registering it in the contract test matrix is the only edit needed; no contract scenario is duplicated

### Requirement: documented exemptions

Some persistence concerns SHALL be exempt from the in-memory-variant requirement because they are SQLite-specific by nature:

- `crates/storage/src/migration.rs` — schema migration logic.
- `crates/storage/src/pool_factory.rs` — SQLite connection pool factory.
- `OrgStorageLayer` SQLite-coupled bootstrap methods (e.g., schema initialization).
- FTS-specific query paths on `MemoryChunkStore` — the trait's `search_fts` method's _ranking_ behavior is SQLite-specific. The in-memory impl SHALL provide a naive substring scan that satisfies the trait contract but is not expected to match SQLite's BM25 ranking.

Each exemption SHALL be documented in module-level docs of the exempt code and noted in the contract test as a `// EXEMPT: ...` annotation.

#### Scenario: contract test annotates an exemption

- **WHEN** a contract test scenario does not apply to the in-memory impl
- **THEN** the scenario is annotated `// EXEMPT: <reason>` and the in-memory impl is skipped for that scenario only — not the entire trait

### Requirement: naming conventions distinguish impl roles

Implementation names SHALL follow these conventions:

- `Sqlite<TraitName>` — SQLite-backed production impl.
- `InMemory<TraitName>` — in-memory impl. Applies whether the impl is test-only or also serves as a production fallback. Disambiguation comes from module docs, not naming.
- `Scripted<TraitName>` — for non-persistence traits where the in-memory variant is driven by a pre-loaded queue (e.g., `ScriptedLlmProvider`).
- `Stub<TraitName>` — for trait facades where there is no plausible production semantic, only a test-only stand-in (e.g., `StubOrchestrationEngine`, `StubToolDispatcher`).
- `Fake<TraitName>` — reserved for cases where neither "InMemory", "Scripted", nor "Stub" fits, used sparingly. `FakeClock` falls in this category by convention.

#### Scenario: ScriptedLlmProvider naming

- **WHEN** `crates/llm-provider/src/scripted.rs` is inspected
- **THEN** it defines `pub struct ScriptedLlmProvider` with a queue-driven `chat` and `chat_stream` impl
- **AND** the type is not named `FakeLlmProvider` — it is a legitimate `LlmProvider` impl with real use cases (demo mode, offline mode, contract tests)

#### Scenario: StubOrchestrationEngine naming

- **WHEN** `crates/runtime/src/orchestrator/stub.rs` is inspected
- **THEN** it defines `pub struct StubOrchestrationEngine` whose `submit_turn` returns canned `TurnResult` values and records inputs
- **AND** the naming signals "test-only stand-in", distinct from a real orchestrator

### Requirement: `FixtureBuilder` composes the common test scaffold

`assistant-test-support` SHALL expose a `FixtureBuilder` that wires the common test scaffold — in-memory stores, `ScriptedLlmProvider`, `InMemoryMessageBus`, `FakeClock`, default config — with sensible defaults and per-field overrides. Tests SHALL be able to construct a complete test environment in a single chained-builder expression.

#### Scenario: minimal fixture construction

- **WHEN** a test calls `assistant_test_support::FixtureBuilder::new().build().await`
- **THEN** the returned `Fixture` carries an `Arc<dyn OrchestrationEngine>` wired to a `ScriptedLlmProvider`, an `InMemoryMessageBus`, in-memory store implementations, and a `FakeClock` seeded to a deterministic timestamp

#### Scenario: per-field override

- **WHEN** a test calls `FixtureBuilder::new().with_clock(custom_fake_clock).with_canned_llm_responses(vec![...]).build().await`
- **THEN** the returned fixture uses the supplied clock and LLM response queue, with all other fields defaulted
