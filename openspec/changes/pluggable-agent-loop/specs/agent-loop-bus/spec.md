## ADDED Requirements

### Requirement: AgentBus is the sole event channel replacing OrchestratorEvent and mpsc streaming

`AgentBus` SHALL replace both the old `mpsc::Sender<OrchestratorEvent>` streaming channel and any secondary broadcast bus. There is one event mechanism: `tokio::sync::broadcast` with capacity 1024. `AgentBus::subscribe()` returns a `broadcast::Receiver<AgentEvent>`. Interface crates subscribe before calling `AgentLoop::run()` and receive all events — including streaming text chunks — from that single receiver.

#### Scenario: Interface crate receives all event types from one subscriber

- **WHEN** an interface crate calls `bus.subscribe()` before `AgentLoop::run()`
- **THEN** the receiver delivers `SessionStarted`, `TurnStarted`, `MessageChunk`, `ToolCallStarted`, `ToolCallCompleted`, `TurnCompleted`, and `SessionEnded` events in order for a single turn with one tool call

#### Scenario: Multiple independent subscribers each receive all events

- **WHEN** two callers each call `bus.subscribe()` before the loop runs
- **THEN** both receivers independently receive every event; each is unaffected by the other's consumption rate up to the lagged-drop threshold

#### Scenario: No subscriber — loop runs without error

- **WHEN** a caller creates an `AgentBus` and immediately drops the `Receiver` returned by `subscribe()`
- **THEN** `AgentLoop::run()` completes successfully; `SendError` from `broadcast::send()` is discarded

### Requirement: AgentBus is turn-scoped for messenger interfaces; session-scoped for web/CLI

For messenger interfaces (Slack, Mattermost, Matrix, etc.) `AgentLoop` and `AgentBus` SHALL be constructed once per inbound turn, not shared across concurrent conversations. This ensures each turn's subscriber receives only that turn's events with no cross-conversation noise. For web-ui SSE streams and CLI, `AgentLoop` MAY be longer-lived; in that case all events carry `session_id` and subscribers filter by it.

#### Scenario: Slack turn produces isolated events

- **WHEN** two Slack messages arrive concurrently, each dispatched in their own `AgentLoop::run()` call with separate `AgentBus` instances
- **THEN** the subscriber for conversation A receives no events from conversation B

#### Scenario: Web-ui SSE subscriber filters by session_id

- **WHEN** a long-lived `AgentLoop` handles two concurrent web sessions and a subscriber only wants session `abc`
- **THEN** the subscriber discards events where `session_id != "abc"` and processes only the matching ones

### Requirement: AgentEvent enum covers all events consumed by current interfaces

The `AgentEvent` enum SHALL define:

- `SessionStarted   { session_id: Uuid }`
- `SessionEnded     { session_id: Uuid }`
- `TurnStarted      { session_id: Uuid, turn: u32 }`
- `TurnCompleted    { session_id: Uuid, turn: u32, answer: String, attachments: Vec<Attachment> }`
- `ToolCallStarted  { session_id: Uuid, tool: String, call_id: String }`
- `ToolCallCompleted{ session_id: Uuid, tool: String, call_id: String, status: ToolCallStatus }`
- `MessageChunk     { session_id: Uuid, chunk: String }`
- `StatusUpdate     { session_id: Uuid, message: String }`
- `SkillCompleted   { session_id: Uuid, skill_name: String, success: bool, summary: String }`
- `LoopError        { session_id: Uuid, error: String }`

`ToolCallStatus` SHALL be an enum: `Ok`, `Error`, `Denied`.

All variants SHALL derive `Clone` and `Debug`. `session_id` SHALL be a `uuid::Uuid`. `Attachment` is re-exported from `assistant-core`.

`TurnCompleted` (not `TurnEnded`) carries the final answer and attachments so messenger interfaces can send the reply without inspecting the full message history.

`StatusUpdate` maps to the existing `OrchestratorEvent::Status` — human-readable progress strings like `"Calling tool: web-search"`.

`SkillCompleted` maps to `OrchestratorEvent::SkillComplete` — consumed by the push-notification hook in `web-ui`.

#### Scenario: Messenger interface drives reply from TurnCompleted

- **WHEN** a Slack adapter subscribes to the bus and the turn finishes
- **THEN** the subscriber receives `TurnCompleted { answer, .. }` and calls `adapter.send(user, answer)` — no separate answer extraction needed

#### Scenario: Tool denial surfaces in ToolCallCompleted

- **WHEN** a tool call is blocked by a `before_tool_call` plugin returning `Block`
- **THEN** `ToolCallCompleted { status: ToolCallStatus::Denied, .. }` is emitted

#### Scenario: SkillCompleted fires after skill body is loaded and acted upon

- **WHEN** the model reads a `SKILL.md` via `file-read` and the skill's instructions lead to a completed sub-task
- **THEN** `SkillCompleted` is emitted by `SkillPlugin` via `after_tool_call` when it detects a file-read of a skill path

#### Scenario: StatusUpdate emitted before each tool dispatch

- **WHEN** the loop is about to call tool `bash`
- **THEN** `StatusUpdate { message: "Calling tool: bash" }` is emitted on the bus before `ToolCallStarted`

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
