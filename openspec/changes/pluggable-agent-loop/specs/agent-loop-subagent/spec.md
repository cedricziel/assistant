## ADDED Requirements

### Requirement: SubagentPlugin registers a task tool that spawns child AgentLoop instances in-process

`SubagentPlugin` SHALL implement the `Plugin` trait and expose a `tools()` method returning a `task` `ToolHandler`. When the LLM calls `task`, the handler SHALL construct a new child `AgentLoop` (sharing the parent's `ToolExecutor` and `LlmProvider` but with its own `PluginRegistry`, `AgentBus`, and `CancellationToken`) and run it in-process. The child result SHALL be returned to the parent LLM as a tool result containing the final answer text and a `task_id` UUID. The `Plugin` trait SHALL gain an optional `fn tools(&self) -> Vec<Arc<dyn ToolHandler>> { vec![] }` method; the `AgentLoop` collects tools from all registered plugins and merges them into the session's tool list.

#### Scenario: Parent LLM delegates work via task tool

- **WHEN** the parent LLM calls the `task` tool with `{ "description": "analyse the auth module" }`
- **THEN** a child `AgentLoop` is created, run to completion in-process, and the parent LLM receives `{ answer: "...", task_id: "<uuid>" }` as the tool result

#### Scenario: Child AgentLoop runs with its own AgentBus

- **WHEN** the child `AgentLoop` runs
- **THEN** child `AgentEvent`s are emitted on a separate child `AgentBus`; the parent's bus receives a `ToolCallStarted` and `ToolCallCompleted` bracketing the delegation but NOT the child's internal events (no event leakage)

### Requirement: task tool returns task_id enabling resumption

The `task` tool SHALL accept an optional `task_id: Uuid` parameter. When provided and a `StoragePlugin` is configured, the child session's prior messages SHALL be loaded from the conversation store and prepended to the child `AgentLoop`'s input, continuing from where the prior session left off. When `task_id` is absent, a fresh `Uuid` is generated and returned. The `task_id` SHALL always appear in the tool result so the parent LLM can pass it back in a follow-up call.

#### Scenario: Parent resumes a stalled child session

- **WHEN** a prior `task` call returned `task_id: "abc-123"` and the parent LLM calls `task` again with `{ "task_id": "abc-123", "description": "continue the analysis" }`
- **THEN** the child `AgentLoop` is initialised with the prior session's messages prepended, effectively resuming the conversation

#### Scenario: No StoragePlugin — task_id is returned but resumption is unavailable

- **WHEN** `StoragePlugin` is not registered and `task_id` is provided
- **THEN** the tool returns an error message indicating resumption requires storage; a fresh session is NOT started silently

### Requirement: Nesting depth is enforced via AgentLoopConfig::depth

`AgentLoopConfig` SHALL include a `depth: u32` field (default 0). The `task` tool handler SHALL increment `depth` when constructing the child `AgentLoopConfig`. When `depth` reaches `MAX_AGENT_DEPTH` (value: 5, matching the existing `DEFAULT_MAX_AGENT_DEPTH` constant in `assistant-core`), the tool SHALL return an error result without spawning a child loop.

#### Scenario: Depth limit prevents infinite nesting

- **WHEN** the `task` tool is called and `AgentLoopConfig::depth == MAX_AGENT_DEPTH`
- **THEN** the tool returns `ToolOutput::error("Maximum subagent depth (5) exceeded")` without constructing a child `AgentLoop`

#### Scenario: Depth increments correctly across generations

- **WHEN** a root loop (depth 0) spawns a child (depth 1) which spawns a grandchild (depth 2)
- **THEN** the grandchild's `AgentLoopConfig::depth == 2` and it can still spawn children up to depth 5

### Requirement: SubagentPlugin is constructed with an AgentLoopFactory

`SubagentPlugin` SHALL be constructed with an `AgentLoopFactory` — a struct or closure that knows how to build a child `AgentLoopConfig` given a parent config and an optional persona/agent name. This avoids circular dependency between the plugin and the loop.

```rust
pub struct SubagentPlugin {
    factory: Arc<dyn AgentLoopFactory>,
}

pub trait AgentLoopFactory: Send + Sync {
    fn build_child(&self, parent: &AgentLoopConfig, task_id: Uuid) -> AgentLoopConfig;
}
```

#### Scenario: Factory produces correctly scoped child config

- **WHEN** `factory.build_child(parent, task_id)` is called
- **THEN** the returned config shares `provider` and `tools` with the parent, has `depth = parent.depth + 1`, has a fresh `AgentBus`, and has a fresh `CancellationToken` (child cancellation is independent of parent)

### Requirement: task tool supports parallel and sequential multi-step invocation via the LLM

The `task` tool interface (its JSON schema exposed to the LLM) SHALL accept a single `description` string. Parallel delegation and chaining are achieved by the LLM calling the `task` tool multiple times in the same turn (parallel mode) or across turns (sequential). No special `parallel` or `chain` parameter is needed — `ToolMode::Parallel` in the parent `AgentLoopConfig` already causes multiple `task` calls in one LLM response to be dispatched concurrently.

#### Scenario: LLM spawns two parallel subagents in one turn

- **WHEN** the LLM returns two `task` tool calls in a single response and parent `tool_mode` is `Parallel`
- **THEN** both child `AgentLoop`s run concurrently; both results are collected before the next parent LLM call
