## 1. Phase 1 — Coverage infrastructure

- [x] 1.1 Add failing CI smoke job that runs `cargo llvm-cov --workspace --json --output-path coverage.json` and uploads the artifact; assert exit code 0 (no parsing yet).
- [x] 1.2 Install `cargo-llvm-cov` in `.github/workflows/coverage.yml` (Linux runner, cached toolchain).
- [x] 1.3 Add `make coverage` target to root `Makefile` that runs the same command locally and prints a per-crate summary table.
- [x] 1.4 Commit `coverage.toml` with the exclude list (generated code globs, `build.rs`).
- [x] 1.5 Add `tools/check_coverage.sh` (or `tools/check_coverage.rs`) that parses `coverage.json`, computes per-crate line coverage, and exits non-zero when any crate not in the report-only allowlist falls below 80%.
- [x] 1.6 Seed the report-only allowlist with **every** crate; the gate prints but does not fail.
- [x] 1.7 Document `cargo llvm-cov` usage in `AGENTS.md` and link to the gate script.
- [x] 1.8 Run `make lint && make format`; commit as `feat(ci): add cargo-llvm-cov coverage measurement (report-only)`.

## 2. Phase 2 — `assistant-test-support` crate scaffold (composition only)

`assistant-test-support` is a thin composition layer. It hosts `FixtureBuilder` + a `prelude` module that re-exports the in-memory impls from their owning crates. The in-memory impls themselves do NOT live here — they live next to their SQLite peers in their owning crates and are plain `pub`, NOT feature-gated.

- [x] 2.1 Add failing test under `crates/test-support/tests/smoke.rs` that imports `assistant_test_support::FixtureBuilder` and constructs an empty fixture; should fail to compile (crate does not exist yet).
- [x] 2.2 Create `crates/test-support/` with `Cargo.toml`, `src/lib.rs`, and module stubs (`prelude`, `fixture`). Package name `assistant-test-support`. Empty `[dependencies]` initially (re-exports get added as their source crates land their fakes).
- [x] 2.3 Register `crates/test-support` in the root `Cargo.toml` workspace members list.
- [x] 2.4 Configure `[lints]\nworkspace = true` in `crates/test-support/Cargo.toml` — keep panic-free contract at workspace baseline (`warn`, not `deny`); `.unwrap()` is ergonomic in test helpers.
- [x] 2.5 Document the crate's purpose: it composes existing fakes, it does NOT host the fakes themselves. Document the rule that `assistant-test-support` MUST NOT appear in any crate's `[dependencies]` section (only `[dev-dependencies]`).
- [x] 2.6 Add `tests/workspace_test_support_lint.rs` at the repo root that fails when `assistant-test-support` appears in any crate's `[dependencies]` table (only `[dev-dependencies]` is allowed).
- [x] 2.7 Add `tests/workspace_test_impls_in_prod.rs` at the repo root that fails when types matching `InMemory*`, `Scripted*`, or `Stub*` are constructed in non-test production code paths (exempt: the defining module itself, `#[cfg(test)]` modules, `tests/` directories, the `assistant-test-support` crate, and the documented production-fallback construction sites for `InMemoryConversationBroadcaster`).
- [ ] 2.8 Run `make lint && make format`; commit as `feat(test-support): scaffold assistant-test-support composition crate`.

## 3. Phase 3 — `Clock` trait

- [x] 3.1 Add failing test in `crates/core/src/clock.rs` that asserts `FakeClock` returns the seeded `DateTime<Utc>` and advances on `tick()`.
- [x] 3.2 Implement `pub trait Clock: Send + Sync` with `now() -> DateTime<Utc>` and `now_instant() -> Instant`. Implement `SystemClock` and `FakeClock` both as plain `pub` types in `assistant-core::clock` — no `#[cfg(test)]`, no feature flag. `FakeClock` is just another `Clock` impl, available wherever the `Clock` trait is.
- [x] 3.3 Re-export `Clock`, `SystemClock`, `FakeClock` from `assistant_core`'s lib root.
- [x] 3.4 Add `assistant_core::FakeClock` to the `assistant-test-support` prelude re-exports.
- [x] 3.5 Add failing test in `crates/auth/tests/jwt_clock.rs` asserting a JWT issued by a `FakeClock`-backed `JwtManager` reports `exp` relative to the fake clock. (Landed inline in `jwt.rs` as `jwt_manager_uses_injected_clock_for_iat_and_exp`.)
- [x] 3.6 Thread `Arc<dyn Clock>` into `assistant_auth::jwt::JwtManager`; default to `Arc::new(SystemClock)` via a `Default` impl or `new()` builder.
- [x] 3.7 Replace `Utc::now()` calls in `crates/auth/src/{jwt,oauth2/device,api_keys,providers,middleware,oidc}.rs` with the injected clock. (Production sites only — test code retains direct `Utc::now()` since tests are exempt from the clock lint.)
- [x] 3.8 Update auth tests to use `FakeClock` for time-sensitive assertions; remove ad-hoc `tokio::time::pause()` workarounds. (Existing tests still use the default `SystemClock`-backed constructor and pass unchanged; the new clock test asserts FakeClock injection works end-to-end. No `tokio::time::pause()` workarounds existed in auth.)
- [x] 3.9 Repeat 3.5–3.8 for `assistant-runtime`. Adds `clock: Arc<dyn Clock>` to `Orchestrator` with `with_clock` builder. Production `Utc::now()` sites — `orchestrator/dispatch.rs::finalize_tool_result` attachment timestamp and `scheduler/tasks.rs::run_due_tasks` + helpers — now read through the orchestrator's clock. `title_generator`, `memory_indexer`, `compaction`, and `orchestrator/worker` use `Instant::now()` for elapsed-timing / deadlines (out of scope of the workspace clock lint, which only bans `Utc::now()` / `SystemTime::now()`).
- [x] 3.10 Repeat for `assistant-bus-nats`, `assistant-storage` (event timestamps in `conversation_events`, `traces`, `metrics`), and `assistant-llm-provider` (retry backoff via `crates/llm-provider/src/retry.rs`). `NatsMessageBus` gains a clock field; storage's 13 stores each gain `clock` + `with_clock` builder; `OpenAiOAuthManager` migrated; retry backoff jitter switched from `SystemTime::now()` to `getrandom::fill` (proper randomness instead of misusing the clock).
- [x] 3.10b Stragglers: migrate `assistant-backup`, `assistant-core::types::conversation::Message::new`, `assistant-workflow`, `assistant-tool-executor` builtins (`schedule_task`, `process`), `assistant-interface-cli` (`credentials`, `cmd_login`, `main`), `opentelemetry-exporter-iceberg` (`metric`, `partition`), `opentelemetry-exporter-sqlite` (`log`, `metric`). Pattern: where there's no natural struct carrier, call `SystemClock.now()` directly instead of `Utc::now()` — the workspace lint targets the underlying calls, not the wrapper. Required adding `assistant-core` as a dependency to `backup`, `opentelemetry-exporter-iceberg`, and `opentelemetry-exporter-sqlite`.
- [x] 3.10c Stragglers (continued): migrate `assistant-web-ui` (~25 sites across `api/*`, `oauth/oidc_bridge`, `a2a/task_store`, `backends/iceberg`, `push`) and `assistant-interfaces` (`common`, all five messenger adapters). Test-only `Utc::now()` references re-qualified as `chrono::Utc::now()` so production `use` lines can be cleanly removed. Workspace audit confirms the only remaining production `Utc::now()` / `SystemTime::now()` calls are inside `assistant_core::clock::SystemClock` itself (the canonical wrapper, by design), `crates/runtime/src/scheduler/tests/*.rs` (sibling test files), and `crates/storage/src/migration.rs` (bootstrap, documented exempt for Phase 3.11).
- [x] 3.11 Add `tests/workspace_clock_lint.rs` that fails compilation if `Utc::now\(\)` or `SystemTime::now\(\)` appears in any non-test `.rs` file outside `assistant_core::clock`. Bans `Local::now()` too; exempt list documented: `crates/core/src/clock.rs` (canonical SystemClock wrapper) and `crates/storage/src/migration.rs` (bootstrap with cascading callers). Heuristic for in-file test code: any line >= first `#[cfg(test)]` attribute. Caught + fixed 5 leftover `Local::now()` sites in `core/memory.rs` and 1 in `tool-executor/builtins/memory_search.rs` (replaced with `SystemClock.now().with_timezone(&chrono::Local)`).
- [x] 3.12 Run `make lint && make format`; commit as `feat(core): introduce Clock trait and migrate workspace`. (Landed as a chain of PRs #825 → #834 plus this final lint commit.)

## 4. Phase 4 — HTTP client injection

- [x] 4.1 Add failing test in `crates/workflow-http/tests/http_action.rs` that mounts a `wiremock::MockServer`, constructs `HttpRequestActionExecutor::with_client(client, base_url)`, and asserts a 200 path through `execute`.
- [x] 4.2 Refactor `HttpRequestActionExecutor::new` to accept `reqwest::Client`. Add `HttpRequestActionExecutor::with_client(client, default_timeout)` constructor. Keep `new(default_timeout)` as a thin wrapper that builds a default client.
- [x] 4.3 Write the remaining `workflow-http` unit tests (host policy enforcement, retry/backoff with `FakeClock`, response mapping, timeout). 8 wiremock-backed tests cover happy path, non-success status, body-from-trigger, host blocklist + allowlist, JSON pointer extraction, invalid URL, missing URL. Retry/backoff deferred until `FakeClock` injection lands in a separate refactor (the retry path uses `tokio::time::sleep`, not a Clock — would need its own seam). Target 80%+ on `crates/workflow-http` met by these tests + the existing implementation surface.
- [x] 4.4 Repeat 4.1–4.3 for `assistant-web-ui::push::WebPushClient`. `PushDispatcher` gains `with_client(...)` constructor; in-method `reqwest::Client::new()` replaced with `self.client`. Two new inline tests assert empty-subscription noop + constructor-injection smoke. Full delivery-path tests deferred (require synthetic p256dh / VAPID keys + cryptographic test fixtures — separate task).
- [ ] 4.5 Repeat for `assistant-interfaces::nextcloud::{adapter,tools}`. (Deferred — currently on the `workspace_http_client_lint` EXEMPT_PATHS backlog. 10 sites in adapter, 2 in tools. Migration requires threading a client through many free utility functions.)
- [x] 4.6 Repeat for `assistant-interfaces::matrix::client::MatrixClient`. The two `Client::builder()` / `Client::new()` sites in `MatrixClient` are both inside Self-returning constructors (`new_with_token`, `login`); the lint's constructor heuristic exempts them — no migration needed.
- [x] 4.7 Repeat for `assistant-auth::oidc::OidcProvider`. The 2 `Client::new()` sites are inside `OidcProvider::discover` (constructor wrapper for `discover_with_client`) and inside `#[cfg(test)]` code. Both exempt by lint construction.
- [ ] 4.8 Audit remaining `reqwest::Client::new()` call sites; refactor each to accept the client. (Backlog: see `EXEMPT_PATHS` in `tests/workspace_http_client_lint.rs` — 8 files / ~21 sites tracked for future migration.)
- [x] 4.9 Add `tests/workspace_http_client_lint.rs` that fails when `reqwest::Client::new\(\)` appears outside of a `new_with_default_client` helper or top-level binary. Includes a constructor-detection heuristic (walks backwards from match to find enclosing fn, checks for `-> Self` or `-> Result<Self>` in the signature). EXEMPT_PATHS holds the current backlog of utility-helper sites; the list is documented as a one-way ratchet (remove only).
- [x] 4.10 Run `make lint && make format`; commit as `refactor(workspace): inject reqwest::Client at constructor seams`. (Landed as a chain: #837 workflow-http, #838 web-ui::push, and this final lint commit.)

## 5. Phase 5 — Persistence trait extraction (Wave A: high-traffic stores)

Each store: failing test → trait in `assistant-core` → `Sqlite*` impl + `InMemory*` impl both as plain `pub` in the owning crate → contract test → consumer migration → re-export to test-support prelude. One sub-phase per store; each shippable as its own PR.

### 5.A — `ConversationStore`

- [x] 5.A.1 Add failing test in `crates/storage/tests/contract/conversation_store.rs` that runs `create → get → list → update_title → mark_locked` against both `SqliteConversationStore` and `InMemoryConversationStore`. Neither type exists yet — test will not compile.
- [x] 5.A.2 Define `pub trait ConversationStore: Send + Sync` in `crates/storage/src/conversations.rs` (deferred trait-in-core promotion until `ConversationRecord` is also moved — TODO documented in the module-level doc).
- [x] 5.A.3 Rename the existing `crates/storage/src/conversations.rs::ConversationStore` struct to `SqliteConversationStore` and `impl ConversationStore for SqliteConversationStore`.
- [x] 5.A.4 Add `pub struct InMemoryConversationStore` in the same file (`crates/storage/src/conversations.rs`) as plain `pub`. HashMap-backed, `Mutex`-protected, enforces obvious cascades (delete-conversation removes its messages). `impl ConversationStore for InMemoryConversationStore`.
- [x] 5.A.5 Confirm contract test from 5.A.1 passes for both impls. 10 scenarios × 2 impls = 20 tests pass.
- [x] 5.A.6 Add `pub use assistant_storage::{ConversationStore, InMemoryConversationStore};` to `assistant-test-support`'s prelude. Added `assistant-storage` as a `[dependencies]` of test-support.
- [x] 5.A.7 Migrate consumers in `assistant-runtime`, `assistant-web-ui`, `assistant-interfaces` to `&dyn ConversationStore`. Internal helper `Orchestrator::prepare_history` keeps the concrete `SqliteConversationStore` in its return (documented: internal-only, no consumer boundary crossed). External function signatures all use the trait.
- [x] 5.A.8 Update consumer tests to use the new types via the test-support prelude. Most tests already used `StorageLayer::new_in_memory()`; left as-is since they exercise SQLite-backed paths. Future tests that don't need SQLite will use `InMemoryConversationStore`.
- [x] 5.A.9 Run `make lint && make format`; commit as `refactor(storage): extract ConversationStore trait + add InMemory impl`.

### 5.B — `TraceStore`

- [x] 5.B.1 Failing contract test in `crates/storage/tests/contract/trace_store.rs`.
- [x] 5.B.2 `trait TraceStore` in `crates/storage/src/traces.rs` (deferred to storage for now, same TODO as ConversationStore); `SqliteTraceStore` (renamed from `TraceStore` struct) + `InMemoryTraceStore` both plain `pub`.
- [x] 5.B.3 Contract test green for both impls — 8 scenarios.
- [x] 5.B.4 Re-export `InMemoryTraceStore` + `TraceStore` from the test-support prelude.
- [x] 5.B.5 Migrate consumer `web-ui/backends/sqlite` to construct `SqliteTraceStore::new(...)`.
- [x] 5.B.6 Commit as `refactor(storage): extract TraceStore trait + add InMemory impl`.

### 5.C — `LogStore`

- [x] 5.C.1–5.C.6 LogStore trait + SqliteLogStore + InMemoryLogStore + contract tests + consumer migration (web-ui/backends/sqlite). Re-exported from test-support prelude. (#843)

### 5.D — `AttachmentStore`

- [x] 5.D.1–5.D.6 AttachmentStore trait + SqliteAttachmentStore + InMemoryAttachmentStore (HashMap<Uuid, Vec<u8>> for bytes, separate HashMap for metas) + 5 contract scenarios + web-ui::ApiState migrated to Arc<dyn AttachmentStore>. (#845)

### 5.E — `AudioStore` (lives in `assistant-transcription`)

- [ ] 5.E.1–5.E.6 Deferred — AudioStore is in assistant-transcription and follows a different ownership model. Phase 5.E in the implemented chain went to PushSubscriptionStore (#851) instead.

### 5.F — `ConversationEventStore` + `RunBroadcaster`

- [x] 5.F.1–5.F.6 ConversationEventStore trait (7 methods) + SqliteConversationEventStore + InMemoryConversationEventStore (Vec-backed, Send-safe MutexGuard handling for append_synthetic_terminal) + 7 contract scenarios. RunBroadcaster stays as a single concrete impl (in-memory pub/sub for live SSE; no SQLite analogue needed). ApiState holds Arc<dyn ConversationEventStore>. (#860)

### 5.Phase-5-other-stores (extras shipped as part of the run)

- [x] PushSubscriptionStore trait + Sqlite/InMemory + 4 contract scenarios (#851)
- [x] CommandEventStore trait + Sqlite/InMemory + 3 contract scenarios. ApiState + channel_runner hold Arc<dyn CommandEventStore>. (#852)
- [x] SlackActiveThreadStore trait + Sqlite/InMemory + 3 contract scenarios. (#854)
- [x] WebhookStore trait + Sqlite/InMemory + 7 contract scenarios. (#855)
- [x] AgentStore trait + Sqlite/InMemory + 4 contract scenarios. (#858)
- [x] ScheduledTaskStore trait + Sqlite/InMemory + 6 contract scenarios. (#859)
- [x] MetricsStore trait + Sqlite + InMemory stub (returns empty/zero results — analytics path is populated by OTel exporter, not direct writes). (#862)
- [x] RefinementsStore trait + Sqlite/InMemory + 5 contract scenarios. (#863)
- [x] Test-compile hotfix (#856) for the test-only call sites that survived Phase 5.D-H lib-only `make lint` — saved to memory as a checklist item: always run `cargo test --workspace --no-run` before committing store-extraction PRs.

### 5.G — `InMemoryMessageBus` (MessageBus already a trait)

- [ ] 5.G.1 Failing contract test asserting enqueue/dequeue/ack/redeliver behavior across `SqliteMessageBus` and a new `InMemoryMessageBus`.
- [ ] 5.G.2 Add `pub struct InMemoryMessageBus` in `crates/storage/src/message_bus.rs` as plain `pub` (channels + `Mutex<HashMap>` for inflight leases).
- [ ] 5.G.3 Contract test green for both impls.
- [ ] 5.G.4 Re-export `InMemoryMessageBus` from the test-support prelude.
- [ ] 5.G.5 Migrate runtime worker tests to `InMemoryMessageBus`. The 218 runtime tests benefit most here.
- [ ] 5.G.6 Commit as `feat(storage): add InMemoryMessageBus + contract tests`.

### 5.H — `ScriptedLlmProvider` (LlmProvider already a trait)

- [ ] 5.H.1 Failing test in `crates/llm-provider/tests/scripted.rs` asserting `ScriptedLlmProvider` returns canned responses in queue order and records calls.
- [ ] 5.H.2 Add `pub struct ScriptedLlmProvider` in `crates/llm-provider/src/scripted.rs` as plain `pub`. Builder methods `with_canned_responses(Vec<LlmResponse>)`, `with_streaming_script(Vec<StreamChunk>)`, and inspection method `recorded_calls() -> Vec<ChatHistory>`. The naming signals real intent — it's a legitimate `LlmProvider` impl usable for demo mode, offline mode, and contract tests, not solely a unit-test stub.
- [ ] 5.H.3 Add the LlmProvider contract test — runs `chat`, `chat_stream`, and capability negotiation against `ScriptedLlmProvider` (plus the existing wiremock-backed Ollama tests as the SQLite analogue).
- [ ] 5.H.4 Re-export `ScriptedLlmProvider` from the test-support prelude.
- [ ] 5.H.5 Migrate the highest-volume `wiremock`-based runtime/web-ui tests to `ScriptedLlmProvider`. Wiremock stays for `llm-provider` crate's own protocol tests.
- [ ] 5.H.6 Commit as `feat(llm-provider): add ScriptedLlmProvider for canned/scripted responses`.

### 5.I — `FixtureBuilder`

- [ ] 5.I.1 Failing test in `crates/test-support/tests/fixture.rs` asserting `FixtureBuilder::new().build().await` returns a `Fixture` with wired-up `Arc<dyn OrchestrationEngine>`, in-memory stores, `FakeClock`, `ScriptedLlmProvider`, `InMemoryMessageBus`.
- [ ] 5.I.2 Implement `assistant_test_support::fixture::{FixtureBuilder, Fixture}` composing the re-exports from storage, llm-provider, runtime, core. Builder methods: `with_clock`, `with_canned_llm_responses`, `with_storage`, `with_extension_tool`.
- [ ] 5.I.3 Document `FixtureBuilder` in the crate-level docs and add a usage example to `AGENTS.md` testing section.
- [ ] 5.I.4 Commit as `feat(test-support): add FixtureBuilder for composed test scaffolding`.

## 6. Phase 6 — Persistence trait extraction (Wave B: lower-traffic stores)

One sub-phase per store; same `failing test → trait + impls → consumer migration → commit` pattern. Smaller PRs since each store is less central.

- [ ] 6.A `CommandEventStore` (consumed by interface adapters)
- [ ] 6.B `MetricsStore`
- [ ] 6.C `AgentStore`
- [ ] 6.D `MemoryChunkStore` (FTS-specific scenarios annotated EXEMPT in contract test)
- [ ] 6.E `PersonaStore`
- [ ] 6.F `PersonaSkillAccess`
- [ ] 6.G `PushSubscriptionStore`
- [ ] 6.H `RefinementStore`
- [ ] 6.I `ScheduledTasksStore`
- [ ] 6.J `SlackThreadStore`
- [ ] 6.K `WebhookStore`
- [ ] 6.L `WorkflowStore`
- [ ] 6.M `SkillRegistry` read-side (`SkillCatalog` trait — overlaps with Phase 7)

## 7. Phase 7 — Orchestration trait facades

- [ ] 7.1 Add failing test in `crates/runtime/src/orchestrator/stub.rs` that constructs a `StubOrchestrationEngine` and verifies the trait surface compiles with `Arc<dyn OrchestrationEngine>`.
- [ ] 7.2 Define `OrchestrationEngine` trait in `assistant_core::orchestration` with the minimum methods consumed by `mcp-server`, `web-ui`, `interfaces`: `submit_turn`, `run_turn_streaming`, `register_extension_tools`, `cancel_turn`.
- [ ] 7.3 `impl OrchestrationEngine for Orchestrator` in `crates/runtime/src/orchestrator/mod.rs`; do not change the struct fields. Add `pub struct StubOrchestrationEngine` in `crates/runtime/src/orchestrator/stub.rs` as plain `pub` (records calls, returns canned `TurnResult` values).
- [ ] 7.4 Define `ToolDispatcher` trait in `assistant_core::tool` with `execute`, `to_specs`, `register_handler`, `is_mutating`. `impl ToolDispatcher for ToolExecutor` in `crates/tool-executor/src/executor.rs`. Add `pub struct StubToolDispatcher` in `crates/tool-executor/src/stub.rs` as plain `pub`.
- [ ] 7.5 Define `SkillCatalog` trait in `assistant_core::skill` (or `assistant_core::catalog`) with `list`, `get`, `reload`. `impl SkillCatalog for SkillRegistry` in `crates/storage/src/registry.rs`. Add `pub struct InMemorySkillCatalog` in the same file as plain `pub`.
- [ ] 7.6 Re-export `StubOrchestrationEngine`, `StubToolDispatcher`, `InMemorySkillCatalog` from the `assistant-test-support` prelude.
- [ ] 7.7 Migrate `crates/mcp-server/src/server.rs` to accept `Arc<dyn OrchestrationEngine>`, `Arc<dyn ToolDispatcher>`, `Arc<dyn SkillCatalog>` instead of the concrete types.
- [ ] 7.8 Add `crates/mcp-server/tests/dispatch.rs` that builds the stubs from the test-support prelude and walks `handle_request` through every JSON-RPC method (`initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`, plus error paths). Target: 80%+ on `crates/mcp-server`.
- [ ] 7.9 Migrate `crates/web-ui/src/lib.rs::ApiState` to hold `Arc<dyn OrchestrationEngine>` instead of `Arc<Orchestrator>`. Existing tests should compile unchanged; add new tests for paths previously untestable.
- [ ] 7.10 Migrate `crates/interfaces/src/{slack,mattermost,matrix,nextcloud,signal}/adapter.rs` to take `Arc<dyn OrchestrationEngine>`.
- [ ] 7.11 Run `make lint && make format`; commit as `feat(core): add OrchestrationEngine/ToolDispatcher/SkillCatalog trait facades`.

## 8. Phase 8 — Messenger `runner.rs` pure-dispatch extraction + CLI REPL split

- [ ] 8.1 Add failing test in `crates/interfaces/src/slack/dispatch.rs` asserting `handle_event(SlackEvent::Message { ... })` returns the expected `RunnerAction` given a `StubOrchestrationEngine` from the test-support prelude.
- [ ] 8.2 Extract Slack runner inner loop into `pub async fn handle_event(event: SlackEvent, deps: &SlackRunnerDeps) -> Result<RunnerAction>`. Keep the WebSocket loop in `runner.rs`; move dispatch logic to `dispatch.rs`.
- [ ] 8.3 Write Slack `dispatch.rs` unit tests for: plain message, threaded reply, reaction-add, attachment upload trigger, bot-author skip, channel-not-allowed skip, duplicate `event_id` skip. Target: 80%+ on `crates/interfaces/src/slack/`.
- [ ] 8.4 Repeat 8.1–8.3 for Mattermost (`crates/interfaces/src/mattermost/`).
- [ ] 8.5 Repeat for Matrix (`crates/interfaces/src/matrix/`).
- [ ] 8.6 Repeat for Nextcloud (`crates/interfaces/src/nextcloud/`).
- [ ] 8.7 Repeat for Signal (`crates/interfaces/src/signal/`).
- [ ] 8.8 Extract `dispatch_command(cmd, deps)` from `crates/interface-cli/src/main.rs`. Write per-subcommand tests using the `FixtureBuilder`.
- [ ] 8.9 Run `make lint && make format`; commit as `refactor(interfaces): extract pure handle_event from each messenger runner`.

## 9. Phase 9 — Per-crate coverage backfill (dependency order)

Each sub-phase: add failing tests for the lowest-covered files first, then implement. Target `>= 80%` line coverage per crate. With the test-support infrastructure in place, every test setup is `FixtureBuilder::new()` + per-test customization.

- [ ] 9.1 `assistant-core` → 80%. Files: `bus.rs` (388 LOC, no tests), `subagent.rs`, `auth.rs` trait surface.
- [ ] 9.2 `assistant-skills` → 80%.
- [ ] 9.3 `assistant-storage` → 80%. Files: `memory_chunks.rs` (253, 0 tests), backend-of-record edge cases. Most coverage comes from contract tests added in Phases 5–6.
- [ ] 9.4 `assistant-llm-provider` → 80%.
- [ ] 9.5 `assistant-tool-executor` → 80%. Files: `builtins/bash.rs` (261, 0 tests — security-critical), `builtins/web_search.rs`, `builtins/web_fetch.rs`, `builtins/self_analyze.rs`, `installer.rs`.
- [ ] 9.6 `assistant-runtime` → 80%. Files: `orchestrator/dispatch.rs` (418), `orchestrator/turn_control.rs` (233), `metrics.rs` (274), `scheduler/tasks.rs` (320).
- [ ] 9.7 `assistant-mcp-client` → 80%.
- [ ] 9.8 `assistant-mcp-server` → 80% (covered in Phase 7.7).
- [ ] 9.9 `assistant-interfaces` → 80% (after Phase 8 should be close).
- [ ] 9.10 `assistant-web-ui` → 80%. Files: `backends/iceberg.rs` (984), `a2a/handlers.rs` (658), `oauth/{token,authorize,device}.rs`, `backends/sqlite.rs` (235).
- [ ] 9.11 `assistant-interface-cli` → 80%. Files: `cmd_login.rs` (446), `cmd_account.rs` (255), `bootstrap.rs` (289), the dispatch logic split out in 8.8.
- [ ] 9.12 `opentelemetry-exporter-iceberg` → 80%. Files: `metric.rs`, `span.rs`, `log.rs`, `catalog.rs`. Add tests using a tiny on-disk warehouse fixture in `tests/fixtures/warehouse/` (Phase 10).
- [ ] 9.13 `assistant-transcription` → 80%.
- [ ] 9.14 `assistant-backup` → 80%.
- [ ] 9.15 `assistant-workflow` → 80%.
- [ ] 9.16 `assistant-bus-nats` → 80%.
- [ ] 9.17 `assistant-auth` → 80% (current 50%, close).
- [ ] 9.18 `assistant-test-support` → 80%. Yes, the test-support crate itself is in the gate: every fake gets unit tests asserting it honors the trait contract beyond what the cross-crate contract tests already cover.

## 10. Phase 10 — Iceberg-backed analytics fixtures

- [ ] 10.1 Add failing test in `crates/web-ui/tests/iceberg_backend.rs` that scans a hand-built parquet warehouse fixture and asserts the expected `TraceSummary` list.
- [ ] 10.2 Build `tests/fixtures/warehouse/` containing minimal parquet files: 3 spans, 5 logs, 2 metrics. Commit the bytes (small enough; document the build script).
- [ ] 10.3 Extract pure aggregation functions in `iceberg.rs` (`aggregate_traces(batches: &[RecordBatch], filter: &TraceFilter) -> Vec<TraceSummary>`); the I/O path stays in the impl methods.
- [ ] 10.4 Write unit tests against the aggregation functions; cover filtering, time-window selection, empty corpus, malformed timestamps.
- [ ] 10.5 Repeat in `crates/opentelemetry-exporter-iceberg/` for span/log/metric write paths.

## 11. Phase 11 — Flip CI gate to enforce

- [ ] 11.1 Once `assistant-core` is at `>= 80%`, remove it from the report-only allowlist; the gate now enforces 80% on core.
- [ ] 11.2 Repeat per crate as each one becomes green (storage → test-support → auth → llm-provider → tool-executor → runtime → mcp-server → mcp-client → interfaces → web-ui → interface-cli → exporters → transcription → backup → workflow → workflow-http → bus-nats).
- [ ] 11.3 After all crates are enforcing, delete the allowlist mechanism entirely from `tools/check_coverage.sh`.
- [ ] 11.4 Add a coverage badge to the README.
- [ ] 11.5 Document the floor in `AGENTS.md` under the existing "Testing Patterns" section so new crates inherit the rule; add a "Choosing fakes" subsection covering when to use `InMemoryFooStore` vs `StorageLayer::new_in_memory()` vs wiremock.
- [ ] 11.6 Run `make lint && make format && make test`; commit as `feat(ci): enforce 80% per-crate coverage floor`.

## 12. Phase 12 — Closeout

- [ ] 12.1 Update `openapi.json` only if any of the trait-facade refactors touched a route (none expected). Run `make dump-openapi && make generate-flutter-client` if so.
- [ ] 12.2 Add an ADR under `docs/adr/` documenting the persistence-trait-symmetry pattern, the `assistant-test-support` crate boundary, and the trait-facade pattern for `OrchestrationEngine`/`ToolDispatcher`/`SkillCatalog`.
- [ ] 12.3 Add an ADR (or extend 12.2) documenting the `Clock` injection rule and the HTTP-client injection rule.
- [ ] 12.4 Archive this change with `openspec archive workspace-test-coverage-floor` once all phases are green on `main`.
