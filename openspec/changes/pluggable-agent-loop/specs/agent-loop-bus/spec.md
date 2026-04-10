## ADDED Requirements

### Requirement: AgentBus is the sole event channel replacing OrchestratorEvent and mpsc streaming

`AgentBus` SHALL replace both the old `mpsc::Sender<OrchestratorEvent>` streaming channel and any secondary broadcast bus. There is one event mechanism: `tokio::sync::broadcast` with capacity 1024. `AgentBus::subscribe()` returns a `broadcast::Receiver<AgentEvent>`. Interface crates subscribe before calling `AgentLoop::run()` and receive all events — including streaming text chunks — from that single receiver.

#### Scenario: Interface crate receives all event types from one subscriber

- **WHEN** an interface crate calls `bus.subscribe()` before `AgentLoop::run()`
- **THEN** the receiver delivers `SessionStarted`, `TurnStarted`, `MessageChunk`, `ToolCallStarted`, `ToolCallCompleted`, `TurnEnded`, and `SessionEnded` events in order for a single turn with one tool call

#### Scenario: Multiple independent subscribers each receive all events

- **WHEN** two callers each call `bus.subscribe()` before the loop runs
- **THEN** both receivers independently receive every event; each is unaffected by the other's consumption rate up to the lagged-drop threshold

#### Scenario: No subscriber — loop runs without error

- **WHEN** a caller creates an `AgentBus` and immediately drops the `Receiver` returned by `subscribe()`
- **THEN** `AgentLoop::run()` completes successfully; `SendError` from `broadcast::send()` is discarded

### Requirement: AgentEvent enum unifies all loop lifecycle events

The `AgentEvent` enum SHALL define:

- `SessionStarted   { session_id: Uuid }`
- `SessionEnded     { session_id: Uuid }`
- `TurnStarted      { session_id: Uuid, turn: u32 }`
- `TurnEnded        { session_id: Uuid, turn: u32 }`
- `ToolCallStarted  { session_id: Uuid, tool: String, call_id: String }`
- `ToolCallCompleted{ session_id: Uuid, tool: String, call_id: String, success: bool }`
- `MessageChunk     { session_id: Uuid, chunk: String }`
- `LoopError        { session_id: Uuid, error: String }`

All variants SHALL derive `Clone` and `Debug`.

#### Scenario: Tool call events bracket execution

- **WHEN** the LLM calls tool `bash` and it completes successfully
- **THEN** `ToolCallStarted { tool: "bash", .. }` is emitted before execution and `ToolCallCompleted { tool: "bash", success: true, .. }` after

#### Scenario: LoopError on max-turns

- **WHEN** the loop exceeds `max_turns`
- **THEN** `LoopError` is emitted with a descriptive message before the loop returns `Err`

### Requirement: AgentBus is cheaply clonable; the loop holds an internal clone

`AgentBus` SHALL implement `Clone` via the underlying `broadcast::Sender` reference count. `AgentLoopConfig` holds one `AgentBus`; the loop clones it internally as needed (e.g. for parallel tool dispatch tasks).

#### Scenario: Clone shares the same channel

- **WHEN** an `AgentBus` is cloned and both the original and clone call `send()`
- **THEN** all subscribed receivers receive events from both senders

### Requirement: Slow subscriber drops events rather than blocking the loop

`AgentBus` uses `broadcast::Sender::send()` (non-blocking). A slow receiver that falls behind the capacity limit receives `broadcast::error::RecvError::Lagged` and skips missed events. The loop is never stalled by a slow subscriber.

#### Scenario: Lagged receiver skips without blocking

- **WHEN** a subscriber stops draining and more than 1024 events are sent
- **THEN** the loop continues running and the lagged receiver eventually receives `RecvError::Lagged(n)` indicating how many events were skipped
