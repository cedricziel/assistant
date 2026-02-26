# Message Bus

A durable, topic-based message bus that decouples components and enables
multi-agent, multi-user, multi-interface communication.

## Architecture

The bus is defined as a backend-agnostic trait (`MessageBus`) in
`assistant-core` with an SQLite implementation (`SqliteMessageBus`) in
`assistant-storage`. The trait can be swapped for NATS, Redis Streams,
or any other broker without changing consumers.

```
                         +-----------+
                         | MessageBus|  (trait in assistant-core)
                         +-----+-----+
                               |
                  +------------+------------+
                  |                         |
          +-------+--------+      +--------+-------+
          |SqliteMessageBus|      | NatsMessageBus |
          |(assistant-      |      | (future)       |
          | storage)        |      |                |
          +----------------+      +----------------+
```

Components interact through the bus via `Arc<dyn MessageBus>`:

```
Interface ──publish──> [bus_messages] ──claim──> Orchestrator
Orchestrator ──publish──> [bus_messages] ──claim──> Tool Executor
Tool Executor ──publish──> [bus_messages] ──claim──> Orchestrator
Orchestrator ──publish──> [bus_messages] ──claim──> Interface
```

## Routing Model

Topics represent **message types**, not destinations. Routing to specific
agents, users, or conversations is done via metadata fields on the message
and `ClaimFilter` on consumption.

This means the topic space stays flat and manageable regardless of how many
agents or users exist. Consumers filter what they care about at claim time.

## Topics

| Topic             | Producer       | Consumer          | Purpose                             |
|-------------------|----------------|-------------------|-------------------------------------|
| `turn.request`    | Interface      | Orchestrator      | User sent a message                 |
| `turn.status`     | Orchestrator   | Interface         | Status update (thinking, tools ...) |
| `turn.result`     | Orchestrator   | Interface         | Final answer ready                  |
| `tool.execute`    | Orchestrator   | Tool executor     | Run a tool                          |
| `tool.result`     | Tool executor  | Orchestrator      | Tool output                         |
| `agent.spawn`     | Agent          | Agent supervisor  | Create a sub-agent                  |
| `agent.report`    | Sub-agent      | Parent agent      | Sub-agent finished                  |
| `agent.terminate` | Supervisor     | Agent             | Shut down an agent                  |
| `schedule.trigger`| Scheduler      | Orchestrator      | Cron task fired                     |

## Typed Envelopes

Each topic has a corresponding Rust struct in `assistant_core::bus_messages`
that defines the payload shape at compile time. Topic names are constants in
`assistant_core::topic`.

| Topic               | Constant                | Envelope struct    |
|----------------------|-------------------------|--------------------|
| `turn.request`       | `topic::TURN_REQUEST`   | `TurnRequest`      |
| `turn.result`        | `topic::TURN_RESULT`    | `TurnResult`       |
| `turn.status`        | `topic::TURN_STATUS`    | `TurnStatus`       |
| `tool.execute`       | `topic::TOOL_EXECUTE`   | `ToolExecute`      |
| `tool.result`        | `topic::TOOL_RESULT`    | `ToolResult`       |
| `agent.spawn`        | `topic::AGENT_SPAWN`    | `AgentSpawn`       |
| `agent.report`       | `topic::AGENT_REPORT`   | `AgentReport`      |

### Publishing with typed envelopes

```rust
use assistant_core::{topic, PublishRequest, TurnRequest};

let envelope = TurnRequest {
    prompt: "hello".into(),
    conversation_id: conv_id,
    extension_tools: vec!["reply".into()],
};

bus.publish(
    PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&envelope)?)
        .with_user_id("U123")
        .with_agent_id("main")
        .with_conversation_id(conv_id)
        .with_interface("slack")
).await?;
```

### Consuming with typed envelopes

```rust
use assistant_core::{TurnRequest, ToolExecute};

if let Some(msg) = bus.claim(topic::TURN_REQUEST, "orchestrator").await? {
    let req: TurnRequest = serde_json::from_value(msg.payload)?;
    // req.prompt, req.conversation_id, req.extension_tools
    // are all typed fields
    bus.ack(msg.id).await?;
}
```

### Available envelopes

**`TurnRequest`** -- user or agent initiates a turn:
- `prompt: String` -- the user's message
- `conversation_id: Uuid`
- `extension_tools: Vec<String>` -- interface-provided tools (default empty)

**`TurnResult`** -- agent's final answer:
- `conversation_id: Uuid`
- `content: String`
- `turn: i64`

**`TurnStatus`** -- progress update during a turn:
- `conversation_id: Uuid`
- `phase: TurnPhase` -- `Thinking`, `CallingTools`, or `Responding`
- `detail: Option<String>` -- e.g. which tool is running

**`ToolExecute`** -- run a single tool:
- `tool_name: String` -- kebab-case tool name
- `call_id: String` -- LLM's tool call ID for correlation
- `params: HashMap<String, Value>`
- `conversation_id: Uuid`
- `turn: i64`

**`ToolResult`** -- output from a tool execution:
- `tool_name: String`
- `call_id: String` -- matches `ToolExecute::call_id`
- `content: String`
- `success: bool`
- `data: Option<Value>`

**`AgentSpawn`** -- create a sub-agent:
- `agent_id: String`
- `task: String`
- `system_prompt: Option<String>`
- `model: Option<String>`
- `allowed_tools: Vec<String>` -- empty = all tools

**`AgentReport`** -- sub-agent reports back:
- `status: AgentReportStatus` -- `Completed`, `Failed`, or `Cancelled`
- `content: String`
- `data: Option<Value>`

## Message Envelope

Every message carries three categories of metadata alongside the payload:

### Identity -- who is involved

| Field      | Type            | Purpose                                  |
|------------|-----------------|------------------------------------------|
| `user_id`  | `Option<String>`| User who initiated the chain of work     |
| `agent_id` | `Option<String>`| Agent that produced or should consume it |

### Routing -- where it goes

| Field             | Type            | Purpose                                      |
|-------------------|-----------------|----------------------------------------------|
| `conversation_id` | `Option<Uuid>`  | Conversation thread                          |
| `interface`       | `Option<String>`| Originating interface (cli, slack, ...)       |
| `reply_to`        | `Option<String>`| Topic for the consumer to send responses to  |

### Correlation -- how messages relate

| Field            | Type           | Purpose                                         |
|------------------|----------------|-------------------------------------------------|
| `correlation_id` | `Option<Uuid>` | Traces the entire request chain end-to-end      |
| `causation_id`   | `Option<Uuid>` | Links to the specific message that caused this  |
| `batch_id`       | `Option<Uuid>` | Groups parallel fan-out (e.g. N tool calls)     |

## Publishing

Use `PublishRequest` with builder-style `with_*` methods. Prefer typed
envelopes (see above) for the payload:

```rust
use assistant_core::{topic, PublishRequest, TurnRequest};

let envelope = TurnRequest {
    prompt: "hello".into(),
    conversation_id: conv_id,
    extension_tools: vec![],
};

let id = bus.publish(
    PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&envelope)?)
        .with_user_id("U123")
        .with_agent_id("main")
        .with_conversation_id(conv_id)
        .with_interface("slack")
        .with_reply_to(topic::TURN_RESULT)
        .with_correlation_id(correlation_id)
).await?;
```

All metadata fields are optional. A minimal publish only needs a topic and
payload:

```rust
bus.publish(PublishRequest::new(topic::TURN_REQUEST, json!({"prompt": "hi"}))).await?;
```

## Claiming

### Unfiltered

Claim the next pending message on a topic, oldest first:

```rust
if let Some(msg) = bus.claim("turn.request", "worker-1").await? {
    // process msg
    bus.ack(msg.id).await?;
}
```

### Filtered

Claim selectively using `ClaimFilter`:

```rust
use assistant_core::ClaimFilter;

// Only claim messages for a specific agent
let filter = ClaimFilter::new().with_agent_id("research-agent");
if let Some(msg) = bus.claim_filtered("turn.request", "worker-1", &filter).await? {
    // ...
}

// Only claim tool results from a specific batch
let filter = ClaimFilter::new().with_batch_id(batch_id);
if let Some(msg) = bus.claim_filtered("tool.result", "orchestrator", &filter).await? {
    // ...
}

// Combine filters (AND semantics)
let filter = ClaimFilter::new()
    .with_agent_id("main")
    .with_conversation_id(conv_id);
```

## Message Lifecycle

```
  publish        claim          ack
 ---------> [Pending] -----> [Claimed] -----> [Done]
                 ^               |
                 |     nack      |     fail
                 +---------------+----------> [Failed]
```

- **Pending** -- waiting to be claimed by a worker.
- **Claimed** -- a worker owns it; must `ack`, `nack`, or `fail`.
- **Done** -- successfully processed. Subject to `purge`.
- **Failed** -- permanently failed. Will not be retried.

### Error handling

- `ack(id)` -- mark as done after successful processing.
- `nack(id)` -- release back to pending for retry (e.g. transient error).
- `fail(id)` -- mark as permanently failed (e.g. bad payload, unrecoverable).

## Housekeeping

### Stale claim reaping

If a worker crashes after claiming but before acking, the message is stuck.
`reap_stale` reclaims messages that have been claimed longer than a timeout:

```rust
// Reset messages claimed more than 5 minutes ago
let count = bus.reap_stale(Duration::from_secs(300)).await?;
```

Run this periodically (e.g. from the scheduler or a background task).

### Purging old messages

Completed messages accumulate in the table. `purge` deletes `Done` messages
older than a threshold:

```rust
let cutoff = Utc::now() - chrono::Duration::days(7);
let count = bus.purge(cutoff).await?;
```

## Execution Model

Tool calls within a conversation execute **sequentially**, preserving the
order the LLM emitted them. This is intentional — LLMs can and do rely on
tool call ordering (e.g. write a file then read it back in the same
response).

The bus still adds value through decoupling, crash recovery, observability,
and the ability to run multiple conversations in parallel across agents.

```rust
use assistant_core::{topic, PublishRequest, ToolExecute};

// Publish tool calls in order
for tool_call in tool_calls {
    let envelope = ToolExecute {
        tool_name: tool_call.name.clone(),
        call_id: tool_call.id.clone(),
        params: tool_call.params.clone(),
        conversation_id: conv_id,
        turn: current_turn,
    };
    bus.publish(
        PublishRequest::new(topic::TOOL_EXECUTE, serde_json::to_value(&envelope)?)
            .with_agent_id("main")
            .with_conversation_id(conv_id)
    ).await?;
}
```

## Multi-Agent Delegation

Agents can delegate work to sub-agents through the bus:

```
User -> turn.request (agent_id=main, correlation_id=C1)
  Main agent claims it, decides to delegate
  Main -> agent.spawn (agent_id=research, correlation_id=C1, causation_id=<turn_req_id>)
    Research agent claims spawn, does work
    Research -> agent.report (correlation_id=C1, causation_id=<spawn_id>)
  Main claims report, synthesizes answer
  Main -> turn.result (correlation_id=C1)
```

The `correlation_id` traces the entire chain. The `causation_id` links each
hop. You can reconstruct the full DAG from the bus table:

```sql
SELECT * FROM bus_messages
WHERE correlation_id = ?
ORDER BY created_at ASC;
```

## SQLite Implementation Details

The `SqliteMessageBus` uses:

- **WAL mode** for concurrent readers.
- **Atomic claim** via `UPDATE ... WHERE id = (SELECT ... LIMIT 1) RETURNING *`
  inside a transaction.
- **Partial indexes** tuned for the claim path (`WHERE status = 'pending'`).
- **Dynamic WHERE-clause construction** for `claim_filtered` -- only set
  filter fields become SQL predicates.

### Schema

```sql
CREATE TABLE bus_messages (
    id              TEXT PRIMARY KEY,
    topic           TEXT NOT NULL,
    payload         TEXT NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending','claimed','done','failed')),
    user_id         TEXT,
    agent_id        TEXT,
    conversation_id TEXT,
    interface       TEXT,
    reply_to        TEXT,
    correlation_id  TEXT,
    causation_id    TEXT,
    batch_id        TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    claimed_at      DATETIME,
    claimed_by      TEXT
);
```

### Performance characteristics

| Operation        | Complexity | Notes                                     |
|------------------|------------|-------------------------------------------|
| `publish`        | O(1)       | Single INSERT                             |
| `claim`          | O(1)       | Indexed subquery + UPDATE                 |
| `claim_filtered` | O(1)       | Partial index on (topic, agent, status)   |
| `ack/nack/fail`  | O(1)       | UPDATE by primary key                     |
| `reap_stale`     | O(stale)   | Partial index on (status, claimed_at)     |
| `purge`          | O(purged)  | Scan on status + created_at               |

SQLite serialises writes. This is fine for tens of concurrent conversations.
For hundreds+, swap to NATS or Postgres.

## What Stays Off the Bus

Not everything benefits from indirection:

| Component         | Stays on          | Reason                                    |
|-------------------|-------------------|-------------------------------------------|
| Token streaming   | `tokio::sync::mpsc` | Ephemeral, high-throughput, backpressure |
| Storage reads     | `Arc<StorageLayer>` | Stateless queries, no routing needed     |
| Skill registry    | `Arc<SkillRegistry>`| In-memory cache, read-only hot path      |
| Shutdown signals  | `tokio::sync::watch` | Process-local, single value             |

## Testing

All bus operations are tested via `StorageLayer::new_in_memory()`:

```rust
let storage = StorageLayer::new_in_memory().await?;
let bus = storage.message_bus();

bus.publish(PublishRequest::new("test", json!({}))).await?;
let msg = bus.claim("test", "w1").await?.unwrap();
bus.ack(msg.id).await?;
```

Run the bus tests:

```sh
cargo test -p assistant-storage message_bus
```
