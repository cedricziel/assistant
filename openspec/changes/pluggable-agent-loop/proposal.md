## Why

The current `assistant-runtime` crate tightly couples the ReAct agent loop with the storage layer, tool executor, skill registry, and interface plumbing. Extending or intercepting the loop requires forking core code. Projects like **pi** (badlogic/pi-mono) and **opencode** (sst/opencode) show that a clean hook/plugin API with lifecycle events, tool interception, and provider extensibility dramatically lowers the barrier for building new capabilities — and their clean designs came from not carrying backward-compatibility debt.

## What Changes

- **`assistant-runtime` is replaced by `assistant-agent-loop`** — a standalone, pluggable ReAct loop library with no storage, skill, or interface dependencies baked in.
- **`Orchestrator`, `OrchestratorEvent`, `ChannelRunner`, `InterfaceRunner` are deleted** — `AgentLoop` is the single entry point; `AgentBus` is the single event channel.
- **`Plugin` trait system** — async lifecycle hooks (`before_tool_call`, `after_tool_call`, `transform_context`, `before_llm_request`, `after_llm_response`, session/turn lifecycle) with default no-op implementations and a `PluginRegistry` dispatcher.
- **Storage moves to `StoragePlugin`** — conversation persistence is implemented as a `Plugin`, not baked into the loop. Opt-in.
- **Skills move to `SkillPlugin`** — skill injection and skill-defined tool registration are implemented as a `Plugin`. Opt-in.
- **`AgentBus` is the only event channel** — typed `tokio::sync::broadcast`-based PubSub; interface crates subscribe directly instead of receiving an mpsc sender. The dual-channel system (`OrchestratorEvent` mpsc + bus) is eliminated.
- **All interface crates updated** — `interface-cli`, `interface-slack`, `interface-mattermost`, `interface-matrix`, `interface-nextcloud`, `interface-signal`, `web-ui` import `assistant-agent-loop` instead of `assistant-runtime`.

## Capabilities

### New Capabilities

- `agent-loop-core`: `AgentLoop` struct, `AgentLoopConfig`, `ToolMode` (Sequential/Parallel), ReAct loop execution, tool dispatch, cancellation.
- `agent-loop-hooks`: `Plugin` trait, `PluginRegistry`, context/result structs (`ToolCallContext`, `ToolCallResult`, `LlmRequest`, `LlmResponse`), `BeforeToolCallOutcome`.
- `agent-loop-bus`: `AgentBus`, `AgentEvent` enum — unified event channel replacing `OrchestratorEvent` and the old mpsc sender pattern.
- `agent-loop-storage-plugin`: `StoragePlugin` implementing `Plugin` to persist conversations via `ConversationStore`.
- `agent-loop-skill-plugin`: `SkillPlugin` implementing `Plugin` for filesystem-based skill discovery and catalog injection via `transform_context`. Skills are instructions loaded by the model via `file-read` — NOT tools. Deletes the `load-skill` and `list-skills` builtin tools and the SQLite-backed `SkillRegistry`.

### Modified Capabilities

- None — all capabilities listed above are net-new. Existing `assistant-runtime` capabilities are superseded and removed.

## Impact

- **New crate**: `crates/agent-loop/` (`assistant-agent-loop`).
- **Deleted crate**: `crates/runtime/` (`assistant-runtime`) — removed from workspace after migration.
- **Modified crates**: all 7 interface/UI crates replace `assistant-runtime` imports with `assistant-agent-loop`; dependency declarations updated in their `Cargo.toml` files.
- **`assistant-cli`** (`crates/interface-cli`): constructs `AgentLoopConfig` directly, registers `StoragePlugin` + `SkillPlugin` + any CLI-specific plugins.
- **`assistant-tool-executor`**: `load-skill` and `list-skills` builtins deleted; `SkillRegistry` dependency removed.
- **`assistant-storage`**: `SkillRegistry` struct and SQLite skill table removed; skill state is filesystem-only.
- **Breaking**: `Orchestrator`, `OrchestratorEvent`, `TurnResult`, `ChannelRunner`, `InterfaceRunner`, `AssistantInterface`, `MetricsRecorder` (runtime versions) are gone. Interfaces use `AgentLoop`, `AgentEvent`, `AgentLoopResult` from the new crate.
- **Root `Cargo.toml`**: `crates/runtime` member removed; `crates/agent-loop` member added.
