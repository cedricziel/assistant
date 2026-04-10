## Context

`assistant-runtime` is the monolithic home of the ReAct loop, conversation persistence, skill injection, interface adapters (`ChannelRunner`, `InterfaceRunner`), metrics, scheduling, and memory indexing. This coupling makes the loop non-reusable and non-extensible.

The no-BC design deletes `assistant-runtime` entirely and replaces it with `assistant-agent-loop` — a loop library with zero storage/skill/interface dependencies. Storage and skill concerns become opt-in plugins. All interface crates are updated to use the new API directly.

Reference: pi (badlogic/pi-mono) and opencode (sst/opencode) — both achieve rich extensibility by keeping the core loop pure and offloading cross-cutting concerns to a plugin/hook layer.

## Goals / Non-Goals

**Goals:**

- `assistant-agent-loop` compiles without `assistant-storage`, `assistant-skills`, or any interface crate.
- `AgentLoop` is the single loop entry point; `AgentBus` is the single event channel.
- `Plugin` trait covers all meaningful interception points — tool gate, result mutation, context transform, LLM request/response, session/turn lifecycle.
- Storage and skill injection are `Plugin` implementations, not loop internals.
- All 7 interface/UI crates compile against the new crate with no `assistant-runtime` dependency.
- The loop is fully testable with a mock `LlmProvider` and in-process `ToolExecutor` — no SQLite, no filesystem.

**Non-Goals:**

- Backward compatibility with `Orchestrator`, `OrchestratorEvent`, `ChannelRunner`, or `InterfaceRunner`.
- Dynamic plugin loading from `.so`/`.dylib`.
- A JavaScript plugin host.
- Parallel hook dispatch (sequential for determinism).

## Decisions

### D1: Delete `assistant-runtime`; new crate is the replacement

**Decision:** `crates/runtime/` is deleted after migration. `crates/agent-loop/` (`assistant-agent-loop`) is the replacement. The workspace `members` array is updated accordingly.

**Rationale:** A thin wrapper that re-exports the old API defeats the purpose. Keeping the old crate alive adds maintenance surface and tempts future code to shortcut through it. A clean break forces every interface crate to adopt the new API, which is the goal.

**Alternatives considered:**

- _Keep `assistant-runtime` as a compatibility shim_: Rejected — the shim becomes permanent technical debt and negates the architectural improvement.

---

### D2: `AgentBus` is required in `AgentLoopConfig`; mpsc sender is removed

**Decision:** `AgentBus` is a required field (not `Option<AgentBus>`). The old `mpsc::Sender<OrchestratorEvent>` parameter on `run()` is gone. Interface crates call `bus.subscribe()` to receive events.

**Rationale:** Making the bus required ensures every caller gets a uniform event stream. The dual-channel system (mpsc for streaming + broadcast for observation) existed only to preserve the old `Orchestrator` API; without BC, one channel suffices. Callers that don't care about events can create a bus and drop the receiver.

```rust
pub struct AgentLoopConfig {
    pub provider:  Arc<dyn LlmProvider>,
    pub tools:     Arc<ToolExecutor>,
    pub plugins:   PluginRegistry,
    pub bus:       AgentBus,
    pub max_turns: u32,
    pub tool_mode: ToolMode,
    pub cancel:    CancellationToken,
}
```

---

### D3: `AgentEvent` unifies `OrchestratorEvent` and the old streaming channel

**Decision:** A single `AgentEvent` enum covers all events previously split across `OrchestratorEvent` variants and the mpsc streaming channel:

```rust
pub enum AgentEvent {
    SessionStarted   { session_id: Uuid },
    SessionEnded     { session_id: Uuid },
    TurnStarted      { session_id: Uuid, turn: u32 },
    TurnEnded        { session_id: Uuid, turn: u32 },
    ToolCallStarted  { session_id: Uuid, tool: String, call_id: String },
    ToolCallCompleted{ session_id: Uuid, tool: String, call_id: String, success: bool },
    MessageChunk     { session_id: Uuid, chunk: String },
    LoopError        { session_id: Uuid, error: String },
}
```

All variants are `Clone + Debug`. Interface crates match on `AgentEvent` instead of `OrchestratorEvent`.

**Rationale:** One channel, one enum, one subscription pattern for all consumers. `broadcast` handles multiple independent subscribers (UI rendering, telemetry, test assertions) without coordination.

---

### D4: `StoragePlugin` and `SkillPlugin` live in `assistant-agent-loop` as optional feature-gated modules

**Decision:** `StoragePlugin` (wraps `ConversationStore`) and `SkillPlugin` (wraps `SkillRegistry`) are implemented in sub-modules of `assistant-agent-loop` behind Cargo feature flags `storage-plugin` and `skill-plugin`. The base crate has no storage/skill deps; enabling the features adds them.

```toml
[features]
default = []
storage-plugin = ["dep:assistant-storage"]
skill-plugin   = ["dep:assistant-skills"]
```

**Rationale:** Keeps the base crate dependency-light. Callers that want persistence just enable the feature. Feature gates are preferable to separate crates for functionality this tightly coupled to the loop API.

**Alternatives considered:**

- _Separate crates `assistant-storage-plugin` and `assistant-skills-plugin`_: More granular but adds two more crate names to the workspace; overkill for functionality that is always used together.

---

### D5: `Plugin` trait gains optional `tools()` method; skills are instructions loaded via file-read — `load-skill`/`list-skills` tools and `SkillRegistry` are deleted

**Decision:** `SkillPlugin` owns skill discovery (filesystem scan, no SQLite) and catalog injection via `transform_context`. The model activates skills by calling its existing `file-read` tool on the `<location>` path in the catalog — no dedicated activation tool is needed or registered. `LoadSkillHandler` and `ListSkillsHandler` are deleted from `assistant-tool-executor`. `SkillRegistry` (SQLite-backed) is deleted from `assistant-storage`.

The agentskills.io spec defines three tiers: (1) catalog in context, (2) model reads `SKILL.md` via file-read, (3) scripts/references loaded on demand the same way. Skills are context/instructions — not tool invocations.

`SkillPlugin::new(scan_dirs: Vec<PathBuf>, persona_filter: Option<PersonaFilter>) -> Self`. `StoragePlugin::new(store: Arc<ConversationStore>)` is unchanged.

**Rationale:** Registering `load-skill` as a builtin tool conflates instructions (skills) with capabilities (tools). The `ToolExecutor` should have zero knowledge of skills. `list-skills` is redundant with the catalog. SQLite-backed `SkillRegistry` makes skills opaque to the filesystem and breaks cross-client interoperability (`~/.agents/skills/` convention). Filesystem-first discovery with in-memory caching is the correct model.

**Alternatives considered:**

- _Keep `load-skill` as an `activate_skill`-style dedicated tool_: Valid per spec but unnecessary when the model has `file-read` and the catalog includes `<location>`. Adds a tool call round-trip for no benefit.
- _Keep `SkillRegistry` in SQLite as a cache_: The cache becomes stale, adds migration burden, and the filesystem is always authoritative anyway. Scan at session start instead.

---

### D6: Subagents are in-process child `AgentLoop` instances spawned via a `task` tool registered by `SubagentPlugin`

**Decision:** `SubagentPlugin` holds an `Arc<dyn AgentLoopFactory>` and contributes a `task` `ToolHandler` via a new optional `Plugin::tools()` method. The `AgentLoop` merges plugin-contributed tools into the session tool list at startup. When the LLM calls `task`, the handler calls `factory.build_child(parent_config, task_id)`, runs the child `AgentLoop` in-process (no subprocess), and returns the final answer + `task_id` as the tool result.

`AgentLoopConfig` gains a `depth: u32` field (default 0). The factory increments it. When `depth >= MAX_AGENT_DEPTH` (5, reusing the existing `DEFAULT_MAX_AGENT_DEPTH` constant from `assistant-core`) the tool returns an error without spawning.

`task_id` (a `Uuid`) is always returned in the tool result. When provided on a subsequent call, `StoragePlugin`'s conversation store is queried to load prior messages — resuming the child session. Without `StoragePlugin`, resumption returns an error rather than silently starting fresh.

Child events are emitted on a **separate child `AgentBus`** — not the parent's. The parent bus sees only `ToolCallStarted`/`ToolCallCompleted` for the `task` tool call, not the child's internal turn/chunk events.

**Rationale:** In-process is simpler and faster than spawning OS subprocesses (pi's approach). The `task_id` resumption pattern (from opencode) is the key improvement over the existing `SubagentRunner` implementation in `assistant-runtime`, which creates a fresh conversation every time. `AgentLoopFactory` as an injected trait avoids a circular dependency between `SubagentPlugin` and `AgentLoop`. The `Plugin::tools()` extension method keeps the pattern consistent — plugins are the only extension point; no separate tool-registration API is needed.

**Alternatives considered:**

- _Subprocess spawning (pi-style)_: Process isolation is appealing but adds spawn latency, IPC complexity, and breaks shared in-process state (e.g. OpenTelemetry context propagation). In-process with depth guard is sufficient.
- _`canTask` flag on agent config (opencode-style)_: Replaced by the numeric `depth` guard, which is already present in `assistant-core` and is more precise than a boolean.
- _Registering `task` as a global builtin in `ToolExecutor`_: Would require `ToolExecutor` to know about `AgentLoop`, creating a circular dep. Plugin-contributed tools via `Plugin::tools()` avoids this cleanly.

---

### D7: Interface crates own their `AgentLoopConfig` construction

**Decision:** Each interface crate constructs its own `AgentLoopConfig`, registers the plugins it needs, and subscribes to the bus. There is no shared "runtime factory." `assistant-cli` registers `StoragePlugin + SkillPlugin + MetricsPlugin`; `interface-slack` registers what it needs.

**Rationale:** Eliminates the implicit shared-state problem in `assistant-runtime` where all interfaces shared one `Orchestrator`. Each interface is now fully self-contained.

---

### D7: Modules from `assistant-runtime` that are not loop concerns move to appropriate crates

| Old module                       | Destination                                                       |
| -------------------------------- | ----------------------------------------------------------------- |
| `scheduler.rs`                   | `assistant-cli` (CLI-specific) or new `assistant-scheduler` crate |
| `memory_indexer/`                | `assistant-storage` or a plugin                                   |
| `telemetry.rs` / `otel_spans.rs` | `assistant-cli` or shared init util in `assistant-core`           |
| `webhook_dispatch.rs`            | `assistant-web-ui`                                                |
| `metrics.rs`                     | Each interface crate registers its own metrics                    |
| `bootstrap.rs`                   | `assistant-cli`                                                   |

## Risks / Trade-offs

- **[Risk] Large migration surface — 7 interface crates must update** → Mitigation: migrate mechanically crate-by-crate; keep old `crates/runtime/` on a feature branch until all crates compile against the new API.
- **[Risk] `broadcast` channel overflow for slow subscribers** → Mitigation: capacity 1024; document that `AgentBus` is best-effort for observers; no correctness dependency on delivery.
- **[Risk] Feature-gated `StoragePlugin` complicates CI** → Mitigation: `make test` runs `--all-features`; the CI matrix already handles feature combinations.
- **[Risk] `scheduler`, `memory_indexer`, `telemetry` displacement creates temporary churn** → Mitigation: move these modules in a preparatory commit before deleting `assistant-runtime`, so the deletion PR is clean.

## Migration Plan

1. **Prep**: move `scheduler`, `memory_indexer`, `telemetry`, `webhook_dispatch`, `bootstrap` to their destination crates. Keep `assistant-runtime` compiling throughout.
2. **Create** `crates/agent-loop/` with `AgentLoop`, `Plugin`, `PluginRegistry`, `AgentBus`, `AgentLoopConfig`, `StoragePlugin` (feature-gated), `SkillPlugin` (feature-gated). Full unit test coverage.
3. **Migrate interfaces** one at a time: update each crate's `Cargo.toml` and replace `assistant-runtime` imports with `assistant-agent-loop`. Start with `interface-cli` (most complete example), then the messenger crates, then `web-ui`.
4. **Delete** `crates/runtime/` and remove from workspace once all crates compile.
5. **Run** `make test` and `make test-integration` to validate.

## Open Questions

- Should `MetricsPlugin` be a first-class plugin shipped in `assistant-agent-loop`? (Likely yes — instrument all interfaces uniformly.)
- Does `memory_indexer` move to `assistant-storage` or become a plugin? (Leaning plugin — it reacts to turn completion.)
- Should `AgentBus` expose a `drain()` helper for tests that want to collect all events synchronously?
