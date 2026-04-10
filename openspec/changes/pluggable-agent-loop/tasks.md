## 1. Preparatory: Relocate non-loop modules out of assistant-runtime

- [ ] 1.1 Move `crates/runtime/src/scheduler.rs` to `crates/interface-cli/src/scheduler.rs`; update imports in `interface-cli`
- [ ] 1.2 Move `crates/runtime/src/bootstrap.rs` to `crates/interface-cli/src/bootstrap.rs`; update imports
- [ ] 1.3 Move `crates/runtime/src/telemetry.rs` and `otel_spans.rs` to `crates/interface-cli/src/telemetry.rs`; update all interface crates that call `init_tracing`
- [ ] 1.4 Move `crates/runtime/src/webhook_dispatch.rs` to `crates/web-ui/src/webhook_dispatch.rs`; update imports in `web-ui`
- [ ] 1.5 Move `crates/runtime/src/memory_indexer/` to `crates/storage/src/memory_indexer/`; update imports; re-export from `assistant-storage` crate root
- [ ] 1.6 Move `crates/runtime/src/metrics.rs` and `MetricsRecorder` to each interface crate that uses it (or inline)
- [ ] 1.7 Run `make check` — verify workspace still compiles after each move

## 2. Crate Scaffolding: assistant-agent-loop

- [ ] 2.1 Create `crates/agent-loop/Cargo.toml` with package name `assistant-agent-loop`, edition 2021, features `storage-plugin`, `skill-plugin`, `subagent-plugin`, and `bus-worker`
- [ ] 2.2 Add `crates/agent-loop` to root `Cargo.toml` workspace members
- [ ] 2.3 Declare dependencies: `assistant-core`, `assistant-llm`, `assistant-tool-executor` (always); `assistant-storage` (feature `storage-plugin`), `assistant-skills` (feature `skill-plugin`); workspace deps `tokio`, `async-trait`, `anyhow`, `tracing`, `serde_json`, `uuid`, `futures`, `tokio-util`
- [ ] 2.4 Create `crates/agent-loop/src/lib.rs` with module declarations: `pub mod bus`, `pub mod hooks`, `pub mod config`, `pub mod loop_core`, `pub mod dispatcher`, `pub mod memory_plugin`, `#[cfg(feature="storage-plugin")] pub mod storage_plugin`, `#[cfg(feature="skill-plugin")] pub mod skill_plugin`, `#[cfg(feature="subagent-plugin")] pub mod subagent_plugin`, `#[cfg(feature="bus-worker")] pub mod turn_worker`

## 3. AgentBus (agent-loop-bus)

- [ ] 3.1 Define `ToolCallStatus` enum: `Ok`, `Error`, `Denied` — `Clone + Debug`
- [ ] 3.2 Define `AgentEvent` enum with all variants — all `Clone + Debug`:
  - `SessionStarted { session_id: Uuid }`
  - `SessionEnded { session_id: Uuid }`
  - `TurnStarted { session_id: Uuid, turn: u32 }`
  - `TurnCompleted { session_id: Uuid, turn: u32, answer: String, attachments: Vec<Attachment> }`
  - `ToolCallStarted { session_id: Uuid, tool: String, call_id: String }`
  - `ToolCallCompleted { session_id: Uuid, tool: String, call_id: String, status: ToolCallStatus }`
  - `MessageChunk { session_id: Uuid, chunk: String }`
  - `StatusUpdate { session_id: Uuid, message: String }`
  - `SkillCompleted { session_id: Uuid, skill_name: String, success: bool, summary: String }`
  - `LoopError { session_id: Uuid, error: String }`
- [ ] 3.3 Implement `AgentBus` struct: `broadcast::Sender<AgentEvent>`, capacity 1024, `new()`, `subscribe() -> Receiver<AgentEvent>`, `send()` (discards `SendError`), `Clone` impl
- [ ] 3.4 Write unit tests: subscriber receives events in order, `TurnCompleted` carries answer and attachments, dropped receiver is no-op, two cloned buses share channel, lagged receiver gets `RecvError::Lagged`

## 4. Plugin Trait System (agent-loop-hooks)

- [ ] 4.1 Define context structs: `SessionContext { session_id: Uuid }`, `TurnContext { session_id: Uuid, turn: u32 }`, `ToolCallContext { tool_name: String, call_id: String, args: serde_json::Value }`
- [ ] 4.2 Define mutation structs: `ToolCallResult { content: String, is_error: bool }`, `LlmRequest { temperature: Option<f32>, max_tokens: Option<u32>, extra: serde_json::Map<String,Value> }`, `LlmResponse { content: String }`
- [ ] 4.3 Define `BeforeToolCallOutcome` enum: `Allow`, `Block { reason: String }`
- [ ] 4.4 Define `Plugin` trait (`#[async_trait]`, `Send + Sync`): `name()` required; all lifecycle methods with default no-op bodies
- [ ] 4.5 Implement `PluginRegistry`: `Vec<Arc<dyn Plugin>>`, `register()`, dispatch methods for each hook with correct composition semantics (pipeline for transforms, first-block-wins for gate, swallow-and-warn for lifecycle)
- [ ] 4.6 Write unit tests: empty registry no-op, transform pipeline composes, first-block skips later plugins, lifecycle error swallowed, minimal plugin (name only) works

## 5. AgentLoopConfig and ToolMode

- [ ] 5.1 Define `ToolMode` enum: `Sequential`, `Parallel`
- [ ] 5.2 Define `AgentLoopConfig` struct with public fields: `provider`, `tools`, `plugins`, `bus` (required), `max_turns` (default 25), `tool_mode` (default `Sequential`), `cancel`
- [ ] 5.3 Implement `AgentLoopConfig::new(provider, tools) -> Self` constructor with defaults for remaining fields

## 6. AgentLoopConfigTemplate and AgentLoop Core Implementation

- [ ] 6.1 Define `AgentLoopConfigTemplate` struct holding `Arc`-shared fields: `provider`, `tools`, `plugins`; add `fn config(&self) -> AgentLoopConfig` that clones into a fresh `AgentLoopConfig` with new `AgentBus` + `CancellationToken` and `depth: 0`
- [ ] 6.2 Implement `AgentLoop::new(config: AgentLoopConfig) -> Self`
- [ ] 6.3 Implement inner turn: emit `StatusUpdate` before each tool dispatch; `plugins.transform_context` → `plugins.before_llm_request` → LLM call → `plugins.after_llm_response` → parse tool calls
- [ ] 6.4 Implement tool dispatch: emit `ToolCallStarted`, call `plugins.before_tool_call` (gate — emit `ToolCallCompleted { status: Denied }` on block), execute, call `plugins.after_tool_call`, emit `ToolCallCompleted { status: Ok|Error }`
- [ ] 6.5 Implement `Parallel` dispatch with `futures::future::join_all`
- [ ] 6.6 Implement outer ReAct loop: repeat while tool calls returned; stop on no-tools, max-turns, or cancellation
- [ ] 6.7 Emit `SessionStarted`, `TurnStarted`, `TurnCompleted { answer, attachments }`, `SessionEnded`, `LoopError` at correct lifecycle points; emit `MessageChunk` per streamed text token
- [ ] 6.8 `AgentLoop::run()` takes no mpsc sender — bus is the only channel
- [ ] 6.9 Write unit tests: single-turn (check `TurnCompleted.answer`), multi-turn, max-turns error, cancellation, parallel dispatch, `StatusUpdate` before each tool, `ToolCallCompleted::Denied` on block

## 7. StoragePlugin (feature: storage-plugin)

- [ ] 7.1 Implement `StoragePlugin::new(store: Arc<ConversationStore>) -> Self`
- [ ] 7.2 Implement `Plugin` for `StoragePlugin`: `on_turn_end` persists answer + tool results to store
- [ ] 7.3 Write unit tests: turn end writes to in-memory store, two loops sharing one store don't race

## 8. SkillPlugin (feature: skill-plugin) and skill cleanup

- [ ] 8.1 Delete `crates/tool-executor/src/builtins/load_skill.rs` and `list_skills.rs`; remove from `builtins/mod.rs` and `ToolExecutor::register_builtins()`
- [ ] 8.2 Remove `SkillRegistry` dependency from `crates/tool-executor/src/executor.rs`; verify `ToolExecutor` has no skill imports
- [ ] 8.3 Delete `SkillRegistry` struct and SQLite skill table migration from `crates/storage/`; remove all `SkillRegistry` re-exports
- [ ] 8.4 Implement `SkillPlugin::new(scan_dirs: Vec<PathBuf>, persona_filter: Option<PersonaFilter>) -> Self`
- [ ] 8.5 Implement `on_session_start`: scan configured dirs for `SKILL.md` files following agentskills.io path conventions (`<project>/.agents/skills/`, `<project>/.<client>/skills/`, `~/.agents/skills/`, `~/.<client>/skills/`); build `HashMap<String, SkillDef>`; apply project-over-user precedence; log `warn!` on name collision
- [ ] 8.6 Implement `transform_context`: if catalog non-empty, prepend a `System`-role message with `<available_skills>` XML (name, description, location per skill) and a one-line instruction telling the model to use `file-read` on `<location>` to activate a skill; return messages unmodified if catalog is empty
- [ ] 8.7 Implement allowlisting: register all discovered skill base directories with the loop's permission layer so `file-read` calls into skill dirs require no user confirmation
- [ ] 8.8 Apply `persona_filter` (allowlist/blocklist) during `transform_context` to restrict which skills appear in the catalog
- [ ] 8.9 Write unit tests: catalog injected with correct XML, project skill shadows user skill with warn log, empty scan → no catalog message, persona blocklist hides skill, works without StoragePlugin, tool list lacks `load-skill` and `list-skills`

## 9. SubagentPlugin and task tool (feature: subagent-plugin)

- [ ] 9.1 Add optional `fn tools(&self) -> Vec<Arc<dyn ToolHandler>> { vec![] }` method to the `Plugin` trait
- [ ] 9.2 Update `AgentLoop` startup: collect `tools()` from all registered plugins and merge into the session tool list (after builtin tools, plugin tools may shadow by name)
- [ ] 9.3 Add `depth: u32` field to `AgentLoopConfig` (default 0); add `MAX_AGENT_DEPTH` constant (reuse `DEFAULT_MAX_AGENT_DEPTH = 5` from `assistant-core`)
- [ ] 9.4 Define `AgentLoopFactory` trait: `fn build_child(&self, parent: &AgentLoopConfig, task_id: Uuid) -> AgentLoopConfig` — child shares `provider` + `tools`, gets fresh `AgentBus` + `CancellationToken`, `depth = parent.depth + 1`
- [ ] 9.5 Implement `DefaultAgentLoopFactory` that implements `AgentLoopFactory` with the above semantics
- [ ] 9.6 Implement `SubagentPlugin::new(factory: Arc<dyn AgentLoopFactory>) -> Self`; return `TaskToolHandler` from `Plugin::tools()`
- [ ] 9.7 Implement `TaskToolHandler`: schema `{ description: string, task_id?: string }`; on call: check depth guard → error if at limit; generate or reuse `task_id`; load prior messages from `StoragePlugin` if `task_id` provided (error if no storage); run child `AgentLoop`; return `{ answer, task_id }` text
- [ ] 9.8 Wire child `AgentBus` isolation: child events stay on child bus; parent bus receives only `ToolCallStarted`/`ToolCallCompleted` for the `task` call
- [ ] 9.9 Delete `crates/runtime/src/orchestrator/subagent.rs` and `SubagentRunner` trait from `assistant-core` (replaced by `SubagentPlugin` + `AgentLoopFactory`)
- [ ] 9.10 Write unit tests: depth guard returns error at limit, depth increments across generations, parallel task calls run child loops concurrently, resume with task_id loads prior messages, resume without storage returns error

## 10. MemoryPlugin (base crate, no feature flag)

- [ ] 10.1 Add `MemoryPlugin` struct to `crates/agent-loop/src/memory_plugin.rs` with `pub fn new(loader: MemoryLoader) -> Self`; `MemoryLoader` is imported from `assistant-core`
- [ ] 10.2 Implement `Plugin` for `MemoryPlugin`: `name()` returns `"memory"`
- [ ] 10.3 Implement `transform_context`: call `loader.load_system_prompt()`; if non-empty, prepend a `ChatHistoryMessage` with `role: System` and the assembled content; return messages unmodified if result is empty
- [ ] 10.4 Implement `on_session_start`: (a) if BOOTSTRAP.md exists at `loader.bootstrap_path()`, delete it after `MemoryLoader` has included it in the assembled content (deletion happens after the first `load_system_prompt` call in this session); (b) read BOOT.md from `loader.boot_path()`; strip HTML comments; if non-empty, submit as a system turn via the loop handle — log `warn!` on any error and continue
- [ ] 10.5 Declare `pub mod memory_plugin` in `crates/agent-loop/src/lib.rs`; re-export `MemoryPlugin`
- [ ] 10.6 Write unit tests:
  - memory content prepended as System message when files present
  - empty `load_system_prompt()` → no message prepended
  - BOOT.md executed at session start when present
  - missing BOOT.md → `on_session_start` completes silently
  - BOOT.md failure → `warn!` logged, session continues (no panic/error propagation)
  - BOOTSTRAP.md present → included in first session's content, deleted afterward
  - BOOTSTRAP.md absent → no deletion attempt

## 11. TurnDispatcher and TurnWorker (feature: bus-worker)

- [ ] 11.1 Define `DispatchRequest` struct: `session_id: Uuid`, `conversation_id: Uuid`, `messages: Vec<ChatHistoryMessage>`, `bus: AgentBus`, `cancel: CancellationToken`
- [ ] 11.2 Define `TurnDispatcher` trait (`#[async_trait]`, `Send + Sync`): `async fn dispatch(&self, request: DispatchRequest) -> Result<()>`; declare in `crates/agent-loop/src/dispatcher.rs`
- [ ] 11.3 Implement `LocalTurnDispatcher`: constructs `AgentLoopConfig` from a stored `Arc<AgentLoopConfigTemplate>`, calls `AgentLoop::run()` in the current task; live in base crate (no feature flag)
- [ ] 11.4 Add `Cargo.toml` feature `bus-worker = ["dep:assistant-core/bus"]`; add `crates/agent-loop/src/turn_worker.rs` behind `#[cfg(feature="bus-worker")]`
- [ ] 11.5 Implement `BusTurnDispatcher`: holds `Arc<dyn MessageBus>` + routing metadata; `dispatch()` serialises `TurnRequest` and calls `bus.publish(PublishRequest { topic: TURN_REQUEST, .. })`
- [ ] 11.6 Implement `TurnWorker::new(template, bus, claim_filter) -> Self` and `TurnWorker::spawn(self) -> JoinHandle<()>`
- [ ] 11.7 Implement the claim loop in `TurnWorker::spawn`: `bus.claim()` → deserialise `TurnRequest` → load prior messages from `StoragePlugin` → `AgentLoopConfig::from_template()` → `AgentLoop::run()` → publish `TurnResult` → `bus.ack/nack()`
- [ ] 11.8 Spawn an `AgentBus` subscriber inside `TurnWorker::spawn` that forwards each `AgentEvent` to `topic::TURN_STATUS` as a `TurnStatus` envelope; log `warn!` on publish failure, never panic
- [ ] 11.9 Update `ChannelRunner` to hold `Arc<dyn TurnDispatcher>` (replacing `Arc<AgentLoopConfigTemplate>`); update `dispatch()` to call `dispatcher.dispatch(DispatchRequest { .. })`
- [ ] 11.10 Write unit tests:
  - `LocalTurnDispatcher` runs loop inline and `TurnCompleted` arrives on bus
  - `BusTurnDispatcher` publishes to mock bus without running loop
  - `TurnWorker` claims from in-memory bus, runs mock loop, acks message
  - Worker crash-before-ack: message becomes available again (test via `nack`)
  - Two workers claim different messages concurrently without racing

## 12. Update ChannelRunner and Migrate Interface Crates

- [ ] 12.1 Update `ChannelRunner` to hold `Arc<dyn TurnDispatcher>` (wired in task 11.9); drive `ChannelAdapter` hooks from bus events (`TurnCompleted` → `on_turn_success`, `LoopError` → `on_turn_error`)
- [ ] 12.2 Update `SkillPlugin::after_tool_call` to detect file-read of a skill path and emit `SkillCompleted` on the bus
- [ ] 12.3 Migrate `crates/interface-cli`: remove `assistant-runtime` dep, add `assistant-agent-loop` with `storage-plugin,skill-plugin,subagent-plugin` features; construct `LocalTurnDispatcher` wrapping `AgentLoopConfigTemplate`; subscribe to `AgentBus` for streaming output; match `MessageChunk` for token printing, `StatusUpdate` for status line, `TurnCompleted` for final answer
- [ ] 12.4 Migrate `crates/interface-slack`: construct `LocalTurnDispatcher` (or `BusTurnDispatcher` if NATS configured); pass to `ChannelRunner`; remove `OrchestratorEvent` imports
- [ ] 12.5 Migrate `crates/interface-mattermost`: same pattern as Slack
- [ ] 12.6 Migrate `crates/interface-matrix`: same pattern
- [ ] 12.7 Migrate `crates/interface-nextcloud`: same pattern
- [ ] 12.8 Migrate `crates/interface-signal`: same pattern
- [ ] 12.9 Migrate `crates/web-ui`: replace `Orchestrator` + `register_token_sink` with `AgentLoopConfigTemplate`; adapt SSE endpoint to subscribe to `AgentBus` (local) OR `topic::TURN_STATUS` on `MessageBus` (remote worker) and map `AgentEvent` variants to SSE event names (`MessageChunk` → `token`, `StatusUpdate` → `status`, `ToolCallCompleted` → `tool_result`, `SkillCompleted` → `skill_complete`, `LoopError` → `error`)
- [ ] 12.10 Run `make check` after each crate change; fix compile errors before proceeding

## 13. Delete assistant-runtime

- [ ] 13.1 Verify no crate in the workspace still depends on `assistant-runtime` (`grep -r "assistant-runtime" crates/ --include="*.toml"`)
- [ ] 13.2 Remove `crates/runtime` from root `Cargo.toml` workspace members
- [ ] 13.3 Delete `crates/runtime/` directory
- [ ] 13.4 Run `make build` — full workspace build must succeed

## 14. Final Validation

- [ ] 14.1 Run `make test` — all tests pass
- [ ] 14.2 Run `make lint` and `make format` — zero warnings, clean formatting
- [ ] 14.3 Run `make test-integration` — smoke tests pass
- [ ] 14.4 Add `//!` crate-level doc to `crates/agent-loop/src/lib.rs` with usage example showing `AgentLoopConfig` construction and `AgentBus` subscription
