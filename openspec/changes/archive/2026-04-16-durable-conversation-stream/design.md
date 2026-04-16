## Context

Each call to `POST /api/conversations/{id}/messages` spawns an orchestrator task and returns a `text/event-stream` SSE response. The SSE channel is backed by an in-memory `tokio::sync::mpsc` pair: the orchestrator holds the sender, the HTTP handler holds the receiver. When the HTTP connection closes, the receiver is dropped and the sender returns errors, but the orchestrator continues running until the LLM finishes — intentional behaviour for background processing.

The problem is that there is no log of in-flight events. If a client disconnects and reconnects while the orchestrator is still running, the new connection has no way to obtain the events that were emitted while it was absent. The final message is persisted to the `messages` table only on `DoneEvent`, so partial progress is invisible after a reconnect.

## Goals / Non-Goals

**Goals:**

- Every orchestrator event (token, status, tool call, tool result, done, error) is written to a durable log keyed by `(run_id, sequence)`
- A client can re-attach to an in-progress or completed run and replay all events from a given sequence
- A single SSE endpoint covers both replay-from-cursor and live tailing, making reconnection a single call
- Events are pruned after a configurable TTL (default 24 h); the log is not permanent storage
- Existing `POST /messages` SSE behaviour is unchanged; this is additive

**Non-Goals:**

- Cross-device push of in-progress tokens (Web Push already handles completion)
- Infinite retention or event sourcing as the system of record
- Multi-subscriber fan-out to many simultaneous clients on the same run (one active subscriber is sufficient)

## Decisions

### Decision 1: Run ID as the primary key for a streaming session

Each `POST /messages` invocation generates a UUID `run_id`. Events are scoped to `(run_id, sequence)`. The run_id is surfaced to the client as the first SSE event (`event: run_started`, `data: {"run_id": "..."}`) before any tokens are emitted.

**Why not use conversation_id + timestamp?** A conversation can have multiple sequential runs. The client needs to know exactly which run to replay, not just "the latest one". A dedicated run_id makes this unambiguous and supports future concurrent-agent scenarios.

**Alternative considered:** expose `run_id` as a response header. Rejected because SSE headers arrive before the connection is confirmed live; first-event delivery integrates naturally with the existing SSE parser.

### Decision 2: One endpoint for replay and live tailing

`GET /api/conversations/{id}/runs/{run_id}/events/stream?since={seq}` serves both purposes:

- If the run is still active: replays stored events from `seq`, then switches to live tailing
- If the run is complete: replays all events from `seq` and closes the stream

The `since` parameter defaults to `0` (replay from the beginning). This avoids a separate "fetch missed events" REST call followed by a separate SSE subscription, which would create a gap window.

**Alternative considered:** separate `GET /events` (JSON array) and `GET /events/stream` (SSE). Rejected because a gap between the two calls could cause missed or duplicate events.

### Decision 3: Store event payloads as JSON text in SQLite

```sql
CREATE TABLE conversation_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id      TEXT    NOT NULL,
    conversation_id TEXT NOT NULL,
    sequence    INTEGER NOT NULL,
    event_type  TEXT    NOT NULL,  -- "token" | "status" | "tool_call" | "tool_result" | "done" | "error" | "run_started"
    payload     TEXT    NOT NULL,  -- JSON
    created_at  TEXT    NOT NULL,
    expires_at  TEXT    NOT NULL
);
CREATE UNIQUE INDEX idx_conv_events_seq ON conversation_events(run_id, sequence);
CREATE INDEX idx_conv_events_expires   ON conversation_events(expires_at);
```

**Why JSON blobs?** Event payloads are already `serde_json::Value` in the runtime. Storing them as typed columns would require a schema migration for every new event field. TTL-based pruning makes the table ephemeral; schema flexibility matters more than query performance here.

### Decision 4: Live tailing via a broadcast channel per run

The runtime registers a `broadcast::Sender<ConversationEvent>` per `run_id` in a `RwLock<HashMap>` on the `StorageLayer` (or a new `EventBroadcaster` struct). The orchestrator writes to both the event store and the broadcaster. The SSE tail handler subscribes to the broadcast channel after replaying stored events.

**Why broadcast and not the existing mpsc sink?** The existing `register_token_sink` uses `mpsc` which has a single receiver — it cannot serve a reconnecting client without dropping the original. `broadcast` allows multiple subscribers and drops late subscribers cleanly.

### Decision 5: TTL pruning via existing scheduler

A new scheduled task (`prune_conversation_events`) runs every hour and deletes rows where `expires_at < now()`. This reuses the existing `Scheduler` in `crates/runtime` rather than introducing a new background thread.

## Risks / Trade-offs

- **SQLite write amplification** — token events arrive at ~10-50 Hz during a fast LLM stream. Each token is one INSERT. With WAL mode this is acceptable for a single-user deployment, but could be a bottleneck under concurrent conversations. Mitigation: batch token writes in the orchestrator (flush every 50 ms or 20 tokens).
- **Broadcast channel lag** — if the reconnecting client is slow, the broadcast channel's ring buffer can overflow and drop events. Mitigation: the SSE handler always replays from the DB first, so a slow subscriber catches up from storage, not the channel.
- **run_id not surfaced until first SSE event** — a client that crashes before receiving `run_started` cannot replay. Mitigation: the `POST /messages` response could also include `run_id` in a `X-Run-Id` response header as a fallback.
- **Event log grows unboundedly without pruning** — if the scheduler is disabled or crashes, old events accumulate. Mitigation: prune on startup as well as on schedule.

## Migration Plan

1. Add `conversation_events` table via a new SQLite migration in `crates/storage` — additive, no existing data affected.
2. Deploy new server binary — new table exists, existing clients see no change.
3. Ship Flutter update — clients begin storing `run_id` and `last_seq`; reconnect logic activates.
4. No rollback complexity: removing the table is a single DROP if needed.

## Open Questions

- Should `run_started` also be stored as the first event in the log (sequence 0), making the log self-contained without a separate `runs` table?
- What is the right TTL for long-running agentic tasks that may take hours? Should TTL be configurable per-run based on estimated task duration?
- Should the Flutter client fall back gracefully to the existing non-reconnect flow if the server does not return a `run_started` event (for backwards compatibility during rollout)?
