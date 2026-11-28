## ADDED Requirements

### Requirement: AgentLoop drives the ReAct loop with no storage or skill dependencies

The `AgentLoop` struct SHALL execute a ReAct loop given an `AgentLoopConfig` with zero dependency on `assistant-storage`, `assistant-skills`, or any interface crate. It SHALL accept a `session_id: Uuid` and a `Vec<ChatHistoryMessage>`, and SHALL return an `AgentLoopResult` on completion.

#### Scenario: Single-turn completion with no tool calls

- **WHEN** `AgentLoop::run(session_id, messages)` is called and the LLM returns a plain text response
- **THEN** the loop completes after one LLM round-trip and returns `AgentLoopResult { answer, attachments }`

#### Scenario: Multi-turn with tool calls

- **WHEN** the LLM returns tool calls
- **THEN** the loop dispatches them, appends results, and calls the LLM again until the LLM returns no tool calls or `max_turns` is reached

#### Scenario: Max-turns limit enforced

- **WHEN** the turn count reaches `AgentLoopConfig::max_turns`
- **THEN** the loop emits `AgentEvent::LoopError` on the bus and returns `Err`

### Requirement: AgentLoopConfig uses public struct fields; AgentBus is required

`AgentLoopConfig` SHALL have these public fields: `provider: Arc<dyn LlmProvider>`, `tools: Arc<ToolExecutor>`, `plugins: PluginRegistry`, `bus: AgentBus`, `max_turns: u32` (default 25), `tool_mode: ToolMode`, `cancel: CancellationToken`. `bus` is NOT optional — callers that don't need events create a bus and drop the receiver.

#### Scenario: Caller ignores bus events

- **WHEN** a caller constructs `AgentLoopConfig` with a fresh `AgentBus` and immediately drops the `Receiver`
- **THEN** `AgentLoop::run()` succeeds; `broadcast::send()` returns `SendError` which is silently discarded

#### Scenario: Cancellation token fires mid-turn

- **WHEN** the `CancellationToken` is cancelled while the loop awaits an LLM response
- **THEN** the loop stops and returns `Err` indicating cancellation

### Requirement: ToolMode controls parallel vs sequential tool dispatch

`ToolMode` SHALL be an enum with `Sequential` and `Parallel` variants. `Parallel` dispatches all tool calls from a single LLM response concurrently via `futures::future::join_all`; `Sequential` dispatches in order.

#### Scenario: Parallel dispatch

- **WHEN** `tool_mode` is `Parallel` and the LLM returns three tool calls
- **THEN** all three tool handlers run concurrently; results are collected before the next LLM call

#### Scenario: Sequential dispatch preserves order

- **WHEN** `tool_mode` is `Sequential` and the LLM returns two tool calls
- **THEN** the second tool is not dispatched until the first has completed

### Requirement: AgentBus is the sole event channel — no mpsc sender parameter

`AgentLoop::run()` SHALL NOT accept an `mpsc::Sender` parameter. All events (text chunks, tool lifecycle, session lifecycle, errors) SHALL be emitted exclusively on the `AgentBus` configured in `AgentLoopConfig`.

#### Scenario: Interface crate receives streaming chunks via bus

- **WHEN** the LLM streams a text response
- **THEN** each chunk results in an `AgentEvent::MessageChunk` on the `AgentBus` in arrival order
