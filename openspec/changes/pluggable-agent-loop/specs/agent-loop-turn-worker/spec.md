## ADDED Requirements

### Requirement: TurnDispatcher abstracts how turns reach the loop

`TurnDispatcher` SHALL be a trait in `assistant-agent-loop` (no heavy dependencies, no feature flag) that decouples _submitting_ a turn from _executing_ it:

```rust
#[async_trait]
pub trait TurnDispatcher: Send + Sync {
    /// Submit a turn for execution. Returns immediately after enqueuing.
    async fn dispatch(&self, request: DispatchRequest) -> Result<()>;
}

pub struct DispatchRequest {
    pub session_id: Uuid,
    pub conversation_id: Uuid,
    pub messages: Vec<ChatHistoryMessage>,
    /// AgentBus to subscribe for this turn's events. The caller subscribes
    /// before calling dispatch(); the dispatcher sends events to this bus.
    pub bus: AgentBus,
    /// Optional cancellation token.
    pub cancel: CancellationToken,
}
```

Two implementations are provided:

- **`LocalTurnDispatcher`** (base crate, always available): calls `AgentLoop::run()` inline in the current task. This is the default used by `ChannelRunner` and the CLI. No external dependencies.
- **`BusTurnDispatcher`** (feature `bus-worker`, depends on `assistant-core` `MessageBus` trait): publishes a `TurnRequest` envelope to the bus and returns. A `TurnWorker` running elsewhere picks it up and executes the loop. The `AgentBus` events are forwarded via a separate response channel (see below).

`ChannelRunner` holds an `Arc<dyn TurnDispatcher>`. Interface crates configure which implementation to use at startup.

#### Scenario: Local dispatcher executes inline

- **WHEN** `LocalTurnDispatcher::dispatch()` is called
- **THEN** `AgentLoop::run()` runs in the same tokio task; `TurnCompleted` is emitted on the provided `AgentBus` before `dispatch()` returns to the caller (the caller awaits completion)

#### Scenario: Bus dispatcher publishes and returns

- **WHEN** `BusTurnDispatcher::dispatch()` is called
- **THEN** a `TurnRequest` is published to the `MessageBus` and `dispatch()` returns immediately; a `TurnWorker` elsewhere claims the message and drives the loop; events from the remote loop are forwarded to the caller's `AgentBus` via `TurnStatus` messages on `topic::TURN_STATUS`

---

### Requirement: TurnWorker claims from MessageBus and drives AgentLoop

`TurnWorker` (feature `bus-worker`) SHALL implement the worker side of durable execution. It wraps an `Arc<AgentLoopConfigTemplate>` and an `Arc<dyn MessageBus>`, claims `TurnRequest` messages, runs `AgentLoop::run()`, and publishes a `TurnResult` when done.

```rust
pub struct TurnWorker {
    template: Arc<AgentLoopConfigTemplate>,
    bus: Arc<dyn MessageBus>,
    claim_filter: ClaimFilter,
}

impl TurnWorker {
    pub fn new(
        template: Arc<AgentLoopConfigTemplate>,
        bus: Arc<dyn MessageBus>,
        claim_filter: ClaimFilter,
    ) -> Self;

    /// Spawn a background task that loops: claim → run → ack/nack.
    pub fn spawn(self) -> tokio::task::JoinHandle<()>;
}
```

The claim loop:

1. Call `bus.claim(ClaimFilter { topic: TURN_REQUEST, .. }, ack_timeout)` to fetch the next available `TurnRequest`.
2. Deserialise the payload into `TurnRequest`.
3. Load prior messages from `StoragePlugin`'s `ConversationStore` using `conversation_id`.
4. Construct `AgentLoopConfig` from the template; attach a fresh `AgentBus`.
5. Publish `TurnStatus { status: "running" }` on `topic::TURN_STATUS` for the conversation.
6. Run `AgentLoop::run()`. Forward each `AgentEvent` to `topic::TURN_STATUS` for distributed observers.
7. On `TurnCompleted`: publish `TurnResult { answer, attachments }` on `topic::TURN_RESULT`; call `bus.ack(message_id)`.
8. On error: publish `TurnResult { error }` on `topic::TURN_RESULT`; call `bus.nack(message_id)`.

Claim errors and deserialization failures are logged at `error!` level; the worker loop continues without crashing.

#### Scenario: Worker claims and executes a TurnRequest

- **WHEN** a `TurnRequest` is published on `topic::TURN_REQUEST` and a `TurnWorker` is running
- **THEN** the worker claims it, runs `AgentLoop::run()`, publishes `TurnResult` on `topic::TURN_RESULT`, and acks the message

#### Scenario: Worker failure re-queues for retry

- **WHEN** the worker process crashes mid-turn (before ack)
- **THEN** the `MessageBus` backend (SQLite reap_stale or NATS `ack_wait`) re-makes the message available; another worker claims and retries — **at-least-once** delivery

#### Scenario: Multiple workers run in parallel

- **WHEN** two `TurnWorker` instances share the same `MessageBus`
- **THEN** each message is claimed by exactly one worker; concurrent turns for different conversations run in parallel without coordination

---

### Requirement: AgentEvent is forwarded to the MessageBus for distributed observers

When running under `TurnWorker`, each `AgentEvent` emitted on the internal `AgentBus` SHALL also be published as a `TurnStatus` message on the `MessageBus` under topic `turn.status`. This allows distributed processes (e.g. a web-ui SSE endpoint on a different host) to observe streaming events from a remote worker.

The forwarding happens inside `TurnWorker::spawn()` via a `tokio::spawn`-ed subscriber on the internal `AgentBus`. The `TurnStatus` payload includes `session_id`, `event_type`, and the serialised `AgentEvent`.

Forwarding is best-effort: `SendError` from the broadcast receiver (lagged) is logged at `debug!` and skipped. `MessageBus::publish` errors are logged at `warn!` and skipped — the loop is never interrupted by forwarding failures.

#### Scenario: Web-ui observes remote worker events via NATS

- **WHEN** `AgentLoop` runs on worker host A and the web-ui SSE endpoint is on host B
- **THEN** host B subscribes to `turn.status` on NATS; the `TurnWorker` on host A publishes each `AgentEvent` there; host B maps them to SSE events for the browser

#### Scenario: Local dispatcher does not forward to bus

- **WHEN** `LocalTurnDispatcher` is used (no bus)
- **THEN** no `TurnStatus` messages are published; events are only available via the in-process `AgentBus`

---

### Requirement: TurnDispatcher replaces the direct Orchestrator call in ChannelRunner

`ChannelRunner` SHALL hold an `Arc<dyn TurnDispatcher>` instead of `Arc<AgentLoopConfigTemplate>` directly. Messenger interfaces configure the dispatcher at startup. For single-host deployments, `LocalTurnDispatcher` wrapping an `AgentLoopConfigTemplate` is used. For distributed deployments, `BusTurnDispatcher` is used.

#### Scenario: ChannelRunner with LocalTurnDispatcher behaves identically to today

- **WHEN** `ChannelRunner` is constructed with a `LocalTurnDispatcher`
- **THEN** turns execute inline, the `AgentBus` is turn-scoped, and adapter hooks fire from bus events — identical behaviour to the direct `Orchestrator::run_turn_with_tools()` call today

#### Scenario: ChannelRunner with BusTurnDispatcher delegates to worker fleet

- **WHEN** `ChannelRunner` is constructed with a `BusTurnDispatcher`
- **THEN** the Slack message triggers a publish to the `MessageBus`; a remote `TurnWorker` runs the loop; the reply arrives via `TurnResult` on the bus; the adapter sends the reply to the user
