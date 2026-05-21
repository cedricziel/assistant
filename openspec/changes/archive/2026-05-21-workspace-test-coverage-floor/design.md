## Context

Workspace audit (2026-05-17, branch `worktree-buzzing-enchanting-flame`) computed
test-LOC-to-src-LOC ratio per crate as a rough coverage proxy. The distribution
is heavy-tailed:

| Crate                          | Src LOC | Tests LOC | Ratio | Inline tests |
| ------------------------------ | ------: | --------: | ----: | -----------: |
| opentelemetry-exporter-sqlite  |  1 ,918 |    1 ,407 |   73% |           15 |
| auth                           |  5 ,093 |    2 ,529 |   50% |          106 |
| storage                        | 14 ,420 |    6 ,306 |   44% |          235 |
| bus-nats                       |  1 ,105 |       447 |   40% |           22 |
| web-ui                         | 26 ,160 |    9 ,057 |   35% |          245 |
| llm-provider                   |  6 ,598 |    2 ,126 |   32% |          103 |
| core                           |  8 ,584 |    2 ,667 |   31% |          183 |
| runtime                        | 16 ,983 |    4 ,857 |   29% |          218 |
| tool-executor                  |  6 ,004 |    1 ,718 |   29% |           73 |
| workflow                       |     927 |       271 |   29% |            6 |
| backup                         |  1 ,434 |       408 |   28% |           21 |
| interfaces                     |  9 ,984 |    2 ,540 |   25% |          128 |
| transcription                  |  2 ,202 |       529 |   24% |           47 |
| skills                         |     590 |       132 |   22% |            8 |
| interface-cli                  |  4 ,987 |       893 |   18% |           59 |
| mcp-client                     |     995 |       151 |   15% |           11 |
| a2a-json-schema                |  1 ,378 |       189 |   14% |           10 |
| opentelemetry-exporter-iceberg |  1 ,752 |       152 |    9% |            7 |
| workflow-http                  |     232 |         0 |    0% |            0 |
| mcp-server                     |     530 |         0 |    0% |            0 |

LOC ratio is a noisy proxy. The actual gate uses `cargo llvm-cov --json` line
coverage, which measures execution rather than file size. The 80% target is
defined in terms of llvm-cov, not the audit ratio.

The audit also identified seventeen production files >200 LOC with zero inline
tests, all of which sit behind one of four structural blockers:

1. **Concrete-type DI**: `Arc<Orchestrator>`, `Arc<ToolExecutor>`,
   `Arc<SkillRegistry>` are taken by callers that cannot construct fakes.
   `mcp-server::handle_request` and the messenger adapters all suffer from this.
2. **Constructor-internal `reqwest::Client::new()`**: `workflow-http`,
   `web-ui/push`, `nextcloud`, `matrix`, `oidc`, `cmd_login` each open their own
   client. Without a `with_base_url` seam, `wiremock` cannot redirect traffic.
3. **Direct clock calls**: 92 production sites call `Utc::now()` /
   `SystemTime::now()`. JWT expiry, device-code TTL, scheduler firing, retry
   backoff, conversation timestamps — all impossible to assert deterministically.
4. **Monolithic mixed-concern files**: `interface-cli/main.rs` (1002),
   `web-ui/backends/iceberg.rs` (984), messenger `runner.rs` files (~116 each).
   The pure logic is embedded in the I/O glue.

## Goals / Non-Goals

**Goals:**

- Every crate reaches `>= 80%` line coverage as reported by
  `cargo llvm-cov --json` per-crate.
- CI enforces the floor with a fail-fast gate; a regression in any one crate
  fails the build.
- Architectural seams are in place to make the floor sustainable:
  trait facades for `Orchestrator`/`ToolExecutor`/`SkillRegistry`,
  injected `Clock`, injected `reqwest::Client`, pure dispatch functions
  in messenger runners.

**Non-Goals:**

- Reaching 100% coverage; cosmetic / unreachable code stays uncovered.
- Mutation testing.
- Branch coverage (line coverage only).
- Removing `StorageLayer::new_in_memory()` — it remains the canonical fixture for `assistant-storage`'s own tests and for contract tests.
- Splitting `Orchestrator` into smaller types (only its trait facade is added).
- Splitting the `interfaces` crate.
- Property-based testing adoption.
- Reaching 80% on `assistant-integration-tests` (its `src/lib.rs` is a stub).
- In-memory variants for `migration` and `pool_factory` — SQLite-specific by nature; documented exemption in the spec.

## Decisions

### D1: Use `cargo llvm-cov` as the canonical coverage tool

llvm-cov is the modern standard, ships in stable Rust toolchains via
`rustup component add llvm-tools-preview`, integrates with `cargo`, and emits
JSON suitable for per-crate parsing. Tarpaulin is older and macOS-flaky;
`grcov` is harder to wire to per-crate gates.

The CI gate runs `cargo llvm-cov --workspace --json --output-path coverage.json`,
then a small `tools/check_coverage.sh` script parses the per-package coverage
section and asserts `lines.percent >= 80.0` for every crate not in the exclude
list. The exclude list contains `assistant-integration-tests` (no library code)
and `assistant-a2a-json-schema` for now (generated types — see D6).

### D2: Trait facades in `assistant-core`, concrete types stay in their crates

Mirror the existing `MessageBus` and `LlmProvider` patterns. For each
load-bearing concrete type, define a trait in `assistant-core` exposing only
the methods consumers actually call. The concrete type implements the trait
without changing its own constructor or fields.

```text
                    Today                          After
                    ─────                          ─────
mcp-server   ─►  Arc<Orchestrator>          mcp-server   ─►  Arc<dyn OrchestrationEngine>
             ─►  Arc<ToolExecutor>                       ─►  Arc<dyn ToolDispatcher>
             ─►  Arc<SkillRegistry>                      ─►  Arc<dyn SkillCatalog>

web-ui       ─►  Arc<Orchestrator>          web-ui       ─►  Arc<dyn OrchestrationEngine>
interfaces   ─►  Arc<Orchestrator>          interfaces   ─►  Arc<dyn OrchestrationEngine>
```

Method surfaces (initial cut, refined during implementation):

- `OrchestrationEngine`: `submit_turn`, `run_turn_streaming`,
  `register_extension_tools`, `cancel_turn`.
- `ToolDispatcher`: `execute`, `to_specs`, `register_handler`, `is_mutating`.
- `SkillCatalog`: `list`, `get`, `reload`.

Callers depend on the trait; tests inject a hand-rolled stub. The trait is
deliberately small — anything callers need that isn't on the trait stays on
the concrete type (so `Orchestrator`'s own integration tests keep their reach).

### D2.5: Persistence trait symmetry — every persistence component is a trait

The same trait-facade pattern applies to every persistence component, not
just the orchestration triad. Today, ~19 stores are still exposed as
concrete structs (`ConversationStore`, `TraceStore`, `AttachmentStore`,
`WebhookStore`, etc.). They become trait + SQLite impl + in-memory impl,
following the pattern already established for `ApiKeyStore`, `OrgStore`,
`MessageBus`, and the 14 other already-trait-shaped stores.

```text
                BEFORE                            AFTER
                ──────                            ─────
core           ApiKeyStore (trait) ✓           ConversationStore (trait)
               MessageBus (trait) ✓            TraceStore (trait)
                                                AttachmentStore (trait)
                                                ... ~19 more

storage        ConversationStore (struct)      SqliteConversationStore
               TraceStore (struct)             SqliteTraceStore
               AttachmentStore (struct)        SqliteAttachmentStore
                                                InMemoryConversationBroadcaster (prod)

test-support   (does not exist)                 InMemoryConversationStore
                                                InMemoryTraceStore
                                                InMemoryAttachmentStore
                                                ... matching every trait
```

The trait surface contains the methods consumers actually call. Some methods
specific to SQLite (raw pool access, FTS-ranked queries) stay on the concrete
`Sqlite<Name>` type, not on the trait. The contract test for each trait
asserts the trait surface is honored consistently across impls.

#### Why this is worth doing now

Three reinforcing reasons:

1. **Layer purity.** Today every test in `runtime`, `web-ui`, `interfaces`,
   and `interface-cli` transitively depends on `assistant-storage`'s
   migrations being current. With trait-based access, only `storage`'s
   own tests do. Refactoring a SQL query no longer cascades into 200 test
   files across the workspace.

2. **Coverage gate is reachable.** The 80% floor on `runtime`/`web-ui`
   today demands either booting `StorageLayer::new_in_memory()` (real
   SQLite, real migrations) per test or skipping coverage on
   persistence-touching paths. With in-memory impls, the test setup is
   3 lines and microseconds.

3. **Refactor safety.** Contract tests assert that any new persistence
   backend (e.g., a Postgres variant later, or a per-org sharded
   storage) honors the same contract as SQLite. Today there is no such
   gate; the only "contract" is "tests still pass against whichever
   impl is wired in."

#### Edge cases the contract test must handle

- **FTS ranking** (`MemoryChunkStore::search_fts`): InMemory impl does a
  naive substring scan that satisfies the trait but does not match
  SQLite's BM25 ranking. The contract test gates ranking-specific
  scenarios with a `// EXEMPT: FTS ranking is SQLite-specific` annotation.
- **Cascading FKs**: InMemory impls SHALL enforce the obvious cascades
  (delete conversation → delete messages, delete persona → null persona
  references) in code. Contract test covers them.
- **JSON column queries**: InMemory impls use `serde_json` to evaluate
  `json_extract`-equivalent paths. Contract test covers null/missing
  divergence.
- **Atomicity**: InMemory impls wrap state in `Mutex<HashMap<...>>`;
  multi-step operations (enqueue + ack on `MessageBus`) acquire the
  mutex for the whole sequence.
- **Migrations / `pool_factory` / SQLite-only OrgStorageLayer methods**:
  exempt; contract test does not run against an in-memory variant
  because none exists.

### D2.6: In-memory impls co-locate with SQLite impls; no feature gates

In-memory impls live in the same crate as their SQLite peers, as plain
`pub` types. No `#[cfg(test)]`. No `[features] test-support` flag.
They are alternative implementations of the trait — the same status as
`SqliteConversationStore` or `OllamaProvider` — and follow the existing
pattern set by `InMemoryConversationBroadcaster`, which has lived as
plain `pub` in `assistant-storage::conversation_broadcaster` since the
crate was created.

```text
crates/storage/src/conversations.rs
   pub trait ConversationStore         // moved to assistant-core
   pub struct SqliteConversationStore
   pub struct InMemoryConversationStore  ← plain pub, no gate

crates/storage/src/message_bus.rs
   pub struct SqliteMessageBus
   pub struct InMemoryMessageBus       ← plain pub

crates/llm-provider/src/scripted.rs
   pub struct ScriptedLlmProvider      ← plain pub, queue-driven

crates/runtime/src/orchestrator/stub.rs
   pub struct StubOrchestrationEngine  ← plain pub
   pub struct StubToolDispatcher

crates/core/src/clock.rs
   pub struct SystemClock
   pub struct FakeClock                 ← plain pub, alongside SystemClock

crates/test-support/src/
   prelude.rs    — re-exports of every InMemory/Scripted/Stub/Fake type
   fixture.rs    — FixtureBuilder + Fixture (~200 LOC)
```

Why no gates?

- **Workspace-internal, not published.** None of these crates ship to
  crates.io. The "API surface hiding" argument feature gates exist for
  doesn't apply.
- **Linker dead-code-elimination.** Production binaries don't construct
  these types; LLVM's DCE strips them. The binary-size argument is
  near-zero.
- **Cargo complexity disappears.** No `features = ["test-support"]`
  ceremony in dev-deps. No `#[cfg(any(test, feature = "test-support"))]`
  noise. Crates that publish in-memory impls don't add `[features]`
  sections.
- **Discoverability wins.** Reading `crates/storage/src/conversations.rs`
  shows trait + Sqlite impl + InMemory impl in one file. Adding a method
  to the trait forces the compiler to fail until both impls update.
- **Pattern already established.** `InMemoryConversationBroadcaster`,
  the four real `LlmProvider` impls, all already work this way. The
  proposal is making the implicit pattern explicit.

The risk — production code accidentally constructing a test-only impl —
is real but small. It is addressed by a workspace lint
(`tests/workspace_test_impls_in_prod.rs`) that fails the build when
`InMemory*`, `Scripted*`, or `Stub*` types appear in non-test production
code. Same enforcement pattern as the `workspace_clock_lint` and
`workspace_http_client_lint` tests proposed elsewhere in this change.

#### Naming conventions

| Convention            | Use case                                                                   | Example                                                                              |
| --------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `Sqlite<TraitName>`   | SQLite-backed production impl                                              | `SqliteConversationStore`                                                            |
| `InMemory<TraitName>` | In-memory impl; test-only OR production fallback (docs disambiguate)       | `InMemoryConversationStore`, `InMemoryConversationBroadcaster` (production fallback) |
| `Scripted<TraitName>` | Queue-driven impl; real `LlmProvider` for canned responses                 | `ScriptedLlmProvider`                                                                |
| `Stub<TraitName>`     | Test-only stand-in for trait facades with no plausible production semantic | `StubOrchestrationEngine`, `StubToolDispatcher`                                      |
| `Fake<TraitName>`     | Reserved; used sparingly when nothing above fits                           | `FakeClock` (by convention)                                                          |

The convention is intentional about the rename: today's draft used
`FakeLlmProvider` and `FakeOrchestrationEngine`. The renames
(`ScriptedLlmProvider`, `StubOrchestrationEngine`) clarify intent:
`ScriptedLlmProvider` is a legitimate `LlmProvider` impl useful for
demo mode and offline mode, not just tests; `StubOrchestrationEngine`
is honestly a test-only stand-in. `FakeClock` stays because the
"FakeClock" naming is widely understood in the testing community.

### D2.7: `assistant-test-support` is a thin composition layer

With in-memory impls living next to their trait owners, the
`assistant-test-support` crate becomes much smaller — essentially:

```text
crates/test-support/
   src/
     lib.rs       — re-exports the prelude
     prelude.rs   — `pub use assistant_storage::InMemoryConversationStore;`
                     `pub use assistant_runtime::StubOrchestrationEngine;`
                     `pub use assistant_llm_provider::ScriptedLlmProvider;`
                     `pub use assistant_core::clock::FakeClock;`
                     ... (one line per fake)
     fixture.rs   — FixtureBuilder + Fixture
   Cargo.toml     — depends on assistant-core, assistant-storage,
                    assistant-runtime, assistant-llm-provider,
                    assistant-tool-executor (for StubToolDispatcher)
                    — these are normal [dependencies], NOT features.
```

The crate has **only `[dev-dependencies]` edges from consumers**
(`assistant-web-ui`, `assistant-runtime` tests, `assistant-interfaces`
tests, etc.). Production binaries never link `assistant-test-support`
because no production code path imports it.

Total size: roughly 200–300 LOC of `FixtureBuilder` + a `prelude.rs`
that's almost entirely re-exports.

### D2.8: Contract tests run every implementation of a trait

Each persistence trait has a contract test under
`crates/storage/tests/contract/<trait>.rs`. The test is parameterized
over implementations using `test_case` or a hand-rolled
`#[tokio::test]` matrix:

```rust
async fn create_and_load<S: ConversationStore + 'static>(store: S) {
    let conv = store.create("Hi").await.unwrap();
    let loaded = store.get(conv.id).await.unwrap().unwrap();
    assert_eq!(loaded.title, Some("Hi".into()));
}

#[tokio::test]
async fn sqlite_create_and_load() {
    let storage = StorageLayer::new_in_memory().await.unwrap();
    create_and_load(SqliteConversationStore::new(storage.pool)).await;
}

#[tokio::test]
async fn memory_create_and_load() {
    create_and_load(InMemoryConversationStore::new()).await;
}
```

Adding a new impl requires only registering it in the matrix; no
contract scenario is duplicated. Both impls live in `assistant-storage`,
so the contract test does not need to import `assistant-test-support`.

### D3: `Clock` trait with `SystemClock` and `FakeClock`

```rust
// assistant-core::clock
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn now_instant(&self) -> Instant;
}

pub struct SystemClock;       // wraps Utc::now() / Instant::now()
pub struct FakeClock { /* RwLock<DateTime<Utc>> */ }  // test-only seam
```

Adoption is gradual. Production code threads `Arc<dyn Clock>` through
constructors. The `FakeClock` lives behind `#[cfg(test)]` re-exports so
non-test code cannot accidentally use it.

The 92 call sites are rolled forward in order of safety impact:
JWT expiry → device-code TTL → scheduler firing → retry backoff → everything
else. Each rollout PR converts one crate at a time so reviews stay small.

### D4: HTTP client injection contract

Any type that issues outbound HTTP MUST:

- accept a `reqwest::Client` (or a thin abstraction over it) at construction;
- expose a `with_base_url(...)` builder so tests can point it at a
  `wiremock::MockServer`;
- never call `reqwest::Client::new()` inside method bodies.

The constructor-injection rule applies to `workflow-http::HttpRequestActionExecutor`,
`assistant-web-ui::push::WebPushClient`, `assistant-interfaces::nextcloud::{adapter,tools}`,
`assistant-interfaces::matrix::client::MatrixClient`, `assistant-auth::oidc::OidcProvider`,
`assistant-interface-cli::cmd_login` flow types.

### D5: Pure dispatch functions in messenger `runner.rs`

Each messenger runner currently looks like:

```rust
loop {
    let msg = ws.next().await?;
    match decode(msg) {
        Event::Message { channel, text, .. } => {
            // 50 lines of orchestrator + storage + reaction dispatch
        }
        Event::Reaction { .. } => { /* ... */ }
    }
}
```

The refactor extracts the inner match arms into:

```rust
async fn handle_event(
    event: SlackEvent,
    deps: &SlackRunnerDeps,  // bundles orchestrator, storage, api client, clock
) -> Result<RunnerAction>;
```

`handle_event` is pure given its dependencies — it takes serde-deserialized
events, returns either a side-effect description (`RunnerAction::ReplyInThread`,
`AddReaction`, `NoOp`) or executes through the injected trait facades. Unit
tests construct fixture events with `serde_json::json!` and assert the
returned action; the WebSocket loop itself stays untested by design.

### D6: `a2a-json-schema` excluded from the floor

The crate is largely `serde`-derived structs mirroring the A2A JSON schema.
Coverage targets on generated/typed code are meaningless. Exclude it from
the gate; if hand-written logic grows there, re-evaluate.

### D7: Phased rollout, ratchet by crate dependency order

Coverage debt is paid in dependency order so each crate's tests cover the
trait facades the next crate up depends on:

```
core            ┐
auth            ├── infrastructure
storage         │
clock+http      ┘
───────────────────────────────────
llm-provider    ┐
tool-executor   ├── platform
skills          │
runtime         ┘
───────────────────────────────────
mcp-server      ┐
mcp-client      │
interfaces      ├── edge
web-ui          │
interface-cli   ┘
```

The gate is added in **report-only mode** first (CI prints coverage but does
not fail). It flips to **enforce** only after each crate is independently
green. This lets crates be promoted one at a time without a flag-day.

### D8: Coverage measurement excludes generated code

`build.rs`-generated files, `#[derive(...)]`-expanded code, and `*.pb.rs`
artifacts are excluded via `coverage.toml`. Hand-written code remains in scope.

## Risks / Trade-offs

- **R1: Trait facades add indirection.** Mitigated by keeping facades small
  and `pub(crate)` consumption paths intact. Concrete types keep their
  full surface.
- **R2: Clock injection threads a parameter through many constructors.**
  Mitigated by phased rollout (one crate at a time) and by defaulting
  `Clock` to `SystemClock` via `Default` on builder types when possible.
- **R3: 80% is a hard line; some crates may need /\* coverage: off \*/ pragmas
  for genuinely unreachable code.** llvm-cov supports `#[coverage(off)]`
  attributes; allowed sparingly and reviewed in PR.
- **R4: CI time grows.** Coverage runs `cargo test` under instrumentation
  which is ~2× slower. Run only on PRs to `main` and nightly, not on every
  push. Cache `~/.cargo` aggressively.
- **R5: Per-crate gate may pin floor at 80% and discourage further gains.**
  Document that the gate is a floor, not a target. Track median and 90th
  percentile in the coverage badge.

## Migration Plan

See `tasks.md` for the full task breakdown. The high-level order:

1. Land coverage infra (`make coverage`, CI in report-only mode).
2. Land `Clock` trait + `SystemClock` + `FakeClock`. Roll into `assistant-auth`
   first (highest correctness leverage on JWT/device-code TTL).
3. Land trait facades in `assistant-core`. Adopt in `mcp-server` first
   (smallest crate, biggest coverage uplift).
4. Land HTTP client injection. Adopt in `workflow-http` first (smallest crate,
   trivially testable once injected).
5. Per-crate coverage-debt sprints, dependency order from D7.
6. Flip CI gate to enforce, one crate at a time. Each crate becomes a
   tripwire — a regression below 80% fails the build for that crate.

## Open Questions

- Should `assistant-runtime`'s integration tests at
  `crates/runtime/src/orchestrator/tests/*.rs` count toward `runtime`'s
  coverage, or move to `tests/` directory? llvm-cov treats both as
  `runtime`'s tests, so the answer is mostly cosmetic. Default: leave as-is.
- Do we want a Flutter coverage gate too? Out of scope here; can be a
  follow-up with `lcov` from `flutter test --coverage`.
- Should the gate be 80% on day one, or step up from each crate's current
  number? Stepping up is more humane but adds long-running state to CI.
  Default: 80% hard floor, but in **enforce** mode only after the crate
  is green once.
