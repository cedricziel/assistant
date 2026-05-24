## Context

Phase 1 of the `protocol-adapter-platform` epic; the first change to consume the
Phase 0 keystone (the `StreamProjector` layer and the `&AuthContext` dispatch
seam).

Grounding (verified on current main):

- `A2AState` (`crates/web-ui/src/a2a/handlers.rs:31`) = `{ task_store: TaskStore,
agent_card: AgentCard }`. Built at `crates/web-ui/src/lib.rs:635`, where the
  Orchestrator and the SQLite pool are already in scope (they feed `ApiState`).
- `send_message` (handlers.rs:118) and `send_message_streaming` (handlers.rs:208)
  are stubs returning a fixed string; both carry `// TODO: Wire to Orchestrator`.
- A2A routes sit behind the `require_auth` route-layer (lib.rs), so
  `Extension<AuthContext>` is available to A2A handlers (same as `/api`).
- `StreamResponse` (`a2a-json-schema`) has `from_task`, `from_message`,
  `from_status_update`, `from_artifact_update` — direct projector targets. SSE
  sends one `StreamResponse` JSON per chunk.
- `TaskStore` is in-memory: `create_task`, `update_status`, `append_history`,
  `subscribe`. The `/api` turn pattern is
  `SqliteConversationStore::for_agent(pool, agent_id).create_conversation()` →
  `register_token_sink(conv_id, tx)` → `submit_turn_with_request_id(&auth, …)`.
- `Interface` (`crates/core/src/types/conversation.rs:151`) has no `A2a` variant.

## Goals / Non-Goals

**Goals:**

- A2A `message/send` and `message/stream` run real Orchestrator turns,
  authenticated, with events projected to the A2A wire by one projector.
- A2A satisfies the `protocol-adapters` invariants it currently violates.

**Non-Goals:**

- Org/space turn re-scoping (still `state.agent_id`, per Phase 0).
- AG-UI / ACP.

(SQLite task persistence **is** in scope — see Decision 6.)

## Decisions

### 1. Extend `A2AState` with the turn-dispatch dependencies

Add `orchestrator: Arc<dyn AssistantInterface>`, `pool: SqlitePool`, and
`agent_id: Arc<RwLock<String>>` (mirroring `ApiState`'s resolution). Built in
`lib.rs` from the same handles that feed `ApiState`.

**Alternative considered:** have A2A reuse `ApiState`. Rejected — `A2AState` is
the axum router state for a distinct router; keeping it a small explicit struct
is clearer than overloading `ApiState`.

### 2. New `Interface::A2a` variant

Turns originating from A2A are tagged `Interface::A2a` for telemetry/routing
parity with the other interfaces. A small `core` enum addition.

### 3. `A2aProjector` lives in `web-ui/src/a2a`, implementing the shared trait

The Phase 0 `StreamProjector` trait lives in `assistant-runtime`; the neutral
`SseProjector`/`CliProjector` impls live there too. A2A's projector maps to
`a2a-json-schema` wire types (`StreamResponse`), so it lives next to the A2A
adapter in `web-ui` and implements `StreamProjector<Frame = StreamResponse>`.
This keeps `assistant-runtime` free of protocol wire crates while preserving the
invariant: one projector per wire, exhaustive `match` over `OrchestratorEvent`
(no `_` arm), covered by a totality test. (Clarifies the `event-projection`
spec: protocol-specific projectors may live with their adapter as long as they
implement the shared trait.)

Mapping sketch: `Token` → `StreamResponse::from_message` (agent text chunk);
`Status`/`ToolResult`/`SkillComplete` → `from_status_update`
(`TaskStatusUpdateEvent`); `Thinking` → status update or text per A2A norms;
`Subagent*`/`AudioReady` → status update with metadata; `AgentError` → failed
status. Terminal → `Completed` task snapshot. Exact arms pinned by the
projector's tests.

### 4. `context_id` → conversation: get-or-create by stable key

A2A `context_id` is an opaque client-supplied thread key. Map it to a
conversation so multi-turn A2A threads share history: derive a deterministic
conversation id from `context_id` (UUIDv5 over a fixed namespace) and
get-or-create that conversation in the agent's store. When `context_id` is
absent, create a fresh conversation and return its id as the task's
`context_id`. Requires a get-or-create path in the conversation store (add if
missing).

**Alternative considered:** a fresh conversation per task. Rejected — it breaks
multi-turn continuity, which A2A `contextId` is specifically for.

### 5. Auth gate reuses the `conversations:write` check

A2A handlers resolve `Extension<AuthContext>` and reject callers lacking
`conversations:write` with `403` (the `caller_can_post` rule from Phase 0,
lifted to a shared helper). Unauthenticated calls already 401 at the layer.

### 6. Task lifecycle, persisted in SQLite via a trait pair

Lifecycle: `create_task` → `Working` → drive the turn, projecting events (and
updating the task) → `Completed` with the final agent message.

Tasks are **durably persisted** so they survive restart (and are visible to
`GET /tasks`, `/tasks/{id}` after one). Following ADR-0009, introduce an
`A2aTaskStore` trait with two impls:

- `InMemoryA2aTaskStore` — the current `TaskStore` logic (subscriptions stay
  in-memory; only durable task state is persisted), used in unit tests.
- `SqliteA2aTaskStore` — backed by the space db pool (the same pool the
  conversation store uses), reading/writing a new `a2a_tasks` table.

The store lives in `crates/web-ui/src/a2a` (next to the adapter) because it
serializes `a2a-json-schema` `Task` values — keeping those wire types out of
`assistant-storage`. The **migration** itself lives in `migrations/042_a2a_tasks.sql`
and is registered in `crates/storage/src/lib.rs` so the space db migrator creates
the table everywhere.

Schema (pragmatic — full fidelity without a relational message/artifact model):

```sql
CREATE TABLE a2a_tasks (
    id         TEXT PRIMARY KEY,         -- task id
    agent_id   TEXT NOT NULL,            -- owning agent/space scope
    context_id TEXT NOT NULL,            -- A2A thread key
    state      TEXT NOT NULL,            -- TaskState (for cheap filtering)
    task_json  TEXT NOT NULL,            -- full serialized Task (history, artifacts, status)
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_a2a_tasks_context ON a2a_tasks(agent_id, context_id);
```

`get_task` deserializes `task_json`; `list_tasks` filters by `agent_id`
(+ optional `context_id`/state) ordered by `updated_at`. Live SSE subscribers
remain an in-memory concern layered over the persisted state.

**Streaming-subscriber nuance:** `subscribe`/`cleanup_subscribers` are process
-local broadcast plumbing, not durable state, so they stay in-memory on both
impls; only task snapshots are persisted.
