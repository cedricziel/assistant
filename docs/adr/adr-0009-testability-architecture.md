# ADR 0009: Workspace Testability Architecture

**Status**: Accepted
**Date**: 2026-05-18

## Context

Before this ADR, large swaths of the workspace were difficult to test in
isolation. The most painful patterns:

- Production code constructed `chrono::Utc::now()`, `std::time::SystemTime::now()`,
  and `reqwest::Client::new()` inline, leaving no seam for tests to control
  time or HTTP behaviour.
- Persistence types were monolithic SQLite-backed structs (`ConversationStore`,
  `TraceStore`, `AttachmentStore`, etc.). Tests had to boot a `StorageLayer`
  for every scenario, even ones that only needed a single store.
- The `Orchestrator`, `ToolExecutor`, and `SkillRegistry` were concrete
  dependencies of `mcp-server`, `web-ui`, and the messenger adapters.
  Unit-testing dispatch logic in those crates required wiring an entire
  runtime.

The result: mcp-server had 0% test coverage. Runtime tests took 5+ seconds
to compile their fixtures. Wiremock was the only seam for LLM behaviour.
Every new feature paid a tax for testability that the architecture didn't
collect.

This ADR documents the testability patterns established by the
`workspace-test-coverage-floor` OpenSpec change.

## Decision

### D1: Inject `Clock` everywhere a current-time read happens

A workspace-wide `Clock` trait lives in `assistant-core::clock` with two
production-ready implementations:

```rust
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn now_instant(&self) -> Instant;
}

pub struct SystemClock;             // production
pub struct FakeClock { ... }        // tests
```

**Rule**: production code never calls `chrono::Utc::now()` or
`std::time::SystemTime::now()` directly. Either it owns an
`Arc<dyn Clock>` (preferred — most stores do) or it uses
`assistant_core::clock::SystemClock.now()` at the point of use.

**Enforcement**: `tests/workspace_clock_lint.rs` fails the build when
direct calls appear in non-test code outside the canonical `SystemClock`
wrapper. Exempt paths are documented in the lint file.

### D2: Inject `reqwest::Client` at constructor seams

Production code never builds a `reqwest::Client` inside a method body.
Types that issue outbound HTTP own a `reqwest::Client` field set at
construction. A `with_client(client)` builder accepts a wiremock-pointed
client in tests.

**Rule**: `reqwest::Client::new()` / `reqwest::Client::builder()` appear
only inside `Self`-returning constructors, `main.rs` files, or documented
utility helpers listed in the lint's `EXEMPT_PATHS` backlog.

**Enforcement**: `tests/workspace_http_client_lint.rs` catches new
violations. A constructor-detection heuristic walks back to the enclosing
`fn` and inspects its return type.

### D3: Persistence trait symmetry

Every persistence type in `assistant-storage` follows this shape:

```rust
#[async_trait]
pub trait FooStore: Send + Sync { ... }

pub struct SqliteFooStore { ... }     // production
pub struct InMemoryFooStore { ... }   // tests

#[async_trait]
impl FooStore for SqliteFooStore { ... }
#[async_trait]
impl FooStore for InMemoryFooStore { ... }
```

Both implementations are plain `pub` — neither is `#[cfg(test)]`-gated.
The `InMemory` variants are real implementations (HashMap-backed,
Mutex-protected, honouring scoping and cascade semantics), not stubs.
Where SQLite-only features can't be replicated (e.g. FTS5 BM25 ranking),
the in-memory variant degrades to a documented best-effort (substring
scan) and consumers that need real ranking use the SQLite impl.

Cross-impl contract tests at `crates/storage/tests/contract/<name>.rs`
run the same scenarios against both impls. Drift between them fails CI.

**Consumer shape**: external API boundaries take `&dyn FooStore` or
`Arc<dyn FooStore>`. Internal helpers within a struct's own impl can
use the concrete `SqliteFooStore` type when convenient — the trait
abstraction is what crosses crate / function-API boundaries.

### D4: Trait facades for orchestration

The runtime exposes three trait facades so consumers (`mcp-server`,
`web-ui`, messenger adapters) can depend on traits rather than the
concrete `Orchestrator` / `ToolExecutor` / `SkillRegistry` types:

```rust
// in assistant_runtime::orchestration
pub trait OrchestrationEngine: Send + Sync {
    async fn submit_turn(...) -> Result<TurnResult>;
    async fn cancel_turn(...) -> CancelOutcome;
}

// in assistant_core::tool
pub trait ToolDispatcher: Send + Sync {
    fn to_specs(&self) -> Vec<ToolSpec>;
    async fn execute(...) -> Result<ToolOutput>;
    fn is_mutating(&self, name: &str) -> bool;
}

// in assistant_storage::registry
pub trait SkillCatalog: Send + Sync {
    async fn list(&self) -> Vec<SkillDef>;
    async fn get(&self, name: &str) -> Option<SkillDef>;
}
```

Each trait has:

- A production impl on the concrete type (delegates to existing methods).
- A `Stub*` / `InMemory*` test variant that records calls and replays
  queued responses.

**Slim by design**: only the methods that external consumers actually
call cross the trait boundary. Per-conversation streaming, registry
reload, executor-internal queue management all stay on the concrete
types.

### D5: `assistant-test-support` as composition layer

A workspace crate `assistant-test-support` re-exports every fake from
its owning crate and exposes `FixtureBuilder` for composed setup:

```rust
use assistant_test_support::prelude::*;

let fx = FixtureBuilder::new()
    .with_canned_llm_responses(vec![...])
    .with_fake_clock_at(1_700_000_000)
    .build()
    .await;

fx.conversation_store.create_conversation(...).await?;
```

`Fixture` exposes 20+ `Arc<dyn TraitName>` fields so test code can
freely clone individual stores into the code path under test.

**Boundary rule**: `assistant-test-support` must NOT appear in any
crate's `[dependencies]` — only `[dev-dependencies]`. Enforced by
`tests/workspace_test_support_lint.rs`.

A related lint, `tests/workspace_test_impls_in_prod.rs`, fails the
build when types matching `InMemory*`, `Scripted*`, or `Stub*` are
constructed in non-test production code paths. Documented exemptions
exist for legitimate production fallbacks (e.g.
`InMemoryConversationBroadcaster` for live SSE).

## Consequences

**Positive**:

- mcp-server went from 0% → 14 dispatch tests covering every JSON-RPC
  method by composing the three trait facades + stubs.
- Storage contract tests catch behavioural drift between SQLite and
  in-memory impls (118+ tests run against both per-trait).
- Tests no longer need to boot SQLite to exercise persistence-shaped
  code; `FixtureBuilder::new().build().await` returns a fully-wired
  in-memory environment.
- Clock-dependent code (auth JWT expiry, scheduler windows, storage
  timestamps) is unit-testable with deterministic time.
- HTTP-dependent code (workflow actions, Web Push, OIDC discovery) is
  unit-testable with `wiremock` instead of integration testing against
  real services.
- The four workspace lints (`Clock`, `http_client`, `test_support_dep`,
  `test_impls_in_prod`) catch regressions at PR review time rather than
  in production.

**Negative**:

- Workspace compile graph is denser — test-support pulls in core,
  storage, llm-provider, tool-executor as `[dependencies]`. Acceptable
  because test-support is itself only a `[dev-dependencies]` of
  consumer crates.
- `InMemory*Store` variants for query-heavy stores (TraceStore's FTS,
  MetricsStore's analytics) are best-effort — they don't replicate
  SQL aggregation behaviour exactly. Tests that need real query
  semantics fall back to SQLite. The trait contract test makes any
  divergence visible.
- Some methods retain the concrete type as a return value in internal
  helpers (e.g. `Orchestrator::prepare_history` returns
  `SqliteConversationStore` rather than `Box<dyn ConversationStore>`).
  This is a deliberate ergonomics call — internal helpers within a
  single struct's impl don't cross a consumer boundary.

## Alternatives Considered

- **`#[cfg(test)]`-gated fakes**: rejected. Fakes that compile only
  under test prevent reuse in demo/offline modes and complicate
  cross-crate fixture composition. The lint approach gives us the
  same "don't ship fakes to production" guarantee without `cfg`-gating.

- **One mega-crate trait module**: rejected. Each crate owns the
  traits relevant to its own surface. `assistant-core` hosts traits
  whose method signatures only mention core types (Clock, ToolDispatcher);
  `assistant-storage` hosts traits that involve storage records
  (ConversationStore, TraceStore, …); `assistant-runtime` hosts
  OrchestrationEngine because TurnResult lives in runtime. This
  keeps the dependency graph honest.

- **Compose fixtures via macro**: rejected for now. `FixtureBuilder` is
  hand-written. A macro could in principle generate the field list +
  `with_*` builders, but the explicit struct is easier to read and IDE
  navigation is straightforward. Revisit if the field count grows past
  ~30.

## References

- OpenSpec change: `openspec/changes/workspace-test-coverage-floor/`
- Lints: `tests/workspace_clock_lint.rs`,
  `tests/workspace_http_client_lint.rs`,
  `tests/workspace_test_impls_in_prod.rs`,
  `tests/workspace_test_support_lint.rs`
- Test-support crate: `crates/test-support/`
- Trait facades: `crates/runtime/src/orchestration.rs`,
  `crates/core/src/tool.rs` (ToolDispatcher),
  `crates/storage/src/registry.rs` (SkillCatalog)
