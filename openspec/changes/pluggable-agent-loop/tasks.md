## 1. Preparatory: Relocate non-loop modules out of assistant-runtime

- [ ] 1.1 Move `crates/runtime/src/scheduler.rs` to `crates/interface-cli/src/scheduler.rs`; update imports in `interface-cli`
- [ ] 1.2 Move `crates/runtime/src/bootstrap.rs` to `crates/interface-cli/src/bootstrap.rs`; update imports
- [ ] 1.3 Move `crates/runtime/src/telemetry.rs` and `otel_spans.rs` to `crates/interface-cli/src/telemetry.rs`; update all interface crates that call `init_tracing`
- [ ] 1.4 Move `crates/runtime/src/webhook_dispatch.rs` to `crates/web-ui/src/webhook_dispatch.rs`; update imports in `web-ui`
- [ ] 1.5 Move `crates/runtime/src/memory_indexer/` to `crates/storage/src/memory_indexer/`; update imports
- [ ] 1.6 Move `crates/runtime/src/metrics.rs` and `MetricsRecorder` to each interface crate that uses it (or inline)
- [ ] 1.7 Run `make check` — verify workspace still compiles after each move

## 2. Crate Scaffolding: assistant-agent-loop

- [ ] 2.1 Create `crates/agent-loop/Cargo.toml` with package name `assistant-agent-loop`, edition 2021, features `storage-plugin` and `skill-plugin`
- [ ] 2.2 Add `crates/agent-loop` to root `Cargo.toml` workspace members
- [ ] 2.3 Declare dependencies: `assistant-core`, `assistant-llm`, `assistant-tool-executor` (always); `assistant-storage` (feature `storage-plugin`), `assistant-skills` (feature `skill-plugin`); workspace deps `tokio`, `async-trait`, `anyhow`, `tracing`, `serde_json`, `uuid`, `futures`, `tokio-util`
- [ ] 2.4 Create `crates/agent-loop/src/lib.rs` with module declarations: `pub mod bus`, `pub mod hooks`, `pub mod config`, `pub mod loop_core`, `#[cfg(feature="storage-plugin")] pub mod storage_plugin`, `#[cfg(feature="skill-plugin")] pub mod skill_plugin`

## 3. AgentBus (agent-loop-bus)

- [ ] 3.1 Define `AgentEvent` enum with all variants (`SessionStarted`, `SessionEnded`, `TurnStarted`, `TurnEnded`, `ToolCallStarted`, `ToolCallCompleted`, `MessageChunk`, `LoopError`) — all `Clone + Debug`
- [ ] 3.2 Implement `AgentBus` struct: `broadcast::Sender<AgentEvent>`, capacity 1024, `new()`, `subscribe() -> Receiver<AgentEvent>`, `send()` (discards `SendError`), `Clone` impl
- [ ] 3.3 Write unit tests: subscriber receives events in order, dropped receiver is no-op, two cloned buses share channel, lagged receiver gets `RecvError::Lagged`

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

## 6. AgentLoop Core Implementation

- [ ] 6.1 Implement `AgentLoop::new(config: AgentLoopConfig) -> Self`
- [ ] 6.2 Implement inner turn: `plugins.transform_context` → `plugins.before_llm_request` → LLM call → `plugins.after_llm_response` → parse tool calls
- [ ] 6.3 Implement tool dispatch: for each call, emit `ToolCallStarted`, call `plugins.before_tool_call` (gate), execute or skip, call `plugins.after_tool_call`, emit `ToolCallCompleted`
- [ ] 6.4 Implement `Parallel` dispatch with `futures::future::join_all`
- [ ] 6.5 Implement outer ReAct loop: repeat while tool calls returned; stop on no-tools, max-turns, or cancellation
- [ ] 6.6 Emit `SessionStarted`/`SessionEnded`/`TurnStarted`/`TurnEnded`/`LoopError` on bus at correct lifecycle points; emit `MessageChunk` for each streamed text token
- [ ] 6.7 `AgentLoop::run()` takes no mpsc sender — bus is the only channel
- [ ] 6.8 Write unit tests with mock `LlmProvider`: single-turn, multi-turn, max-turns error, cancellation, parallel dispatch, bus events in order, before_tool_call block prevents execution

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

## 9. Migrate Interface Crates

- [ ] 9.1 Migrate `crates/interface-cli`: remove `assistant-runtime` dep, add `assistant-agent-loop` with `storage-plugin,skill-plugin` features; construct `AgentLoopConfig` + `StoragePlugin` + `SkillPlugin`; subscribe to `AgentBus` for streaming output; remove all `Orchestrator`/`OrchestratorEvent`/`ChannelRunner` references
- [ ] 9.2 Migrate `crates/interface-slack`: replace `assistant-runtime` with `assistant-agent-loop`; subscribe to `AgentBus`; match on `AgentEvent`
- [ ] 9.3 Migrate `crates/interface-mattermost`: same pattern as Slack
- [ ] 9.4 Migrate `crates/interface-matrix`: same pattern
- [ ] 9.5 Migrate `crates/interface-nextcloud`: same pattern
- [ ] 9.6 Migrate `crates/interface-signal`: same pattern
- [ ] 9.7 Migrate `crates/web-ui`: replace `assistant-runtime` with `assistant-agent-loop`; adapt SSE streaming to `AgentBus` events
- [ ] 9.8 Run `make check` after each crate migration; fix compile errors before proceeding to next crate

## 10. Delete assistant-runtime

- [ ] 10.1 Verify no crate in the workspace still depends on `assistant-runtime` (`grep -r "assistant-runtime" crates/ --include="*.toml"`)
- [ ] 10.2 Remove `crates/runtime` from root `Cargo.toml` workspace members
- [ ] 10.3 Delete `crates/runtime/` directory
- [ ] 10.4 Run `make build` — full workspace build must succeed

## 11. Final Validation

- [ ] 11.1 Run `make test` — all tests pass
- [ ] 11.2 Run `make lint` and `make format` — zero warnings, clean formatting
- [ ] 11.3 Run `make test-integration` — smoke tests pass
- [ ] 11.4 Add `//!` crate-level doc to `crates/agent-loop/src/lib.rs` with usage example showing `AgentLoopConfig` construction and `AgentBus` subscription
