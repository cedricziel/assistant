## Why

A2A is the half-built interface — and the cautionary tale — the
`protocol-adapter-platform` epic exists to fix. `crates/web-ui/src/a2a/handlers.rs`
accepts messages, but `send_message` / `send_message_streaming` are stubs
(`"Processing is not yet wired to the LLM backend"`) over an in-memory
`TaskStore`. `A2AState` is `{ task_store, agent_card }` — no Orchestrator, no
`AuthContext` — even though it sits behind the `require_auth` layer and the
Orchestrator is in scope where `A2AState` is built (`lib.rs:635`).

Phase 0 shipped exactly what makes wiring it correct: the shared projection
layer and the `&AuthContext` turn-dispatch seam. This change makes A2A the first
real consumer of both — converting an orphaned door into a real one.

## What Changes

- `A2AState` gains the Orchestrator handle + conversation-store access. A2A
  handlers resolve `Extension<AuthContext>` and gate posting on
  `conversations:write` (the same seam as `/api`).
- `send_message` submits a real turn via
  `Orchestrator::submit_turn(&auth, …, Interface::A2a, …)` and returns the
  agent's actual reply as the A2A `Task`.
- `send_message_streaming` registers a token sink, runs the turn, and projects
  `OrchestratorEvent`s to A2A `StreamResponse` frames via a new `A2aProjector`
  (implements the Phase 0 `StreamProjector` trait; lives in `web-ui/src/a2a`
  because it maps to `a2a-json-schema` wire types).
- New `Interface::A2a` variant.
- A2A `context_id` maps to a conversation (get-or-create by stable key) so
  multi-turn A2A threads share history.
- **A2A tasks are persisted in SQLite** so they survive restart: a new
  `A2aTaskStore` trait pair (`SqliteA2aTaskStore` + `InMemoryA2aTaskStore`, the
  ADR-0009 pattern) backed by a new `migrations/042_a2a_tasks.sql` table in the
  space db. The current in-memory `TaskStore` becomes `InMemoryA2aTaskStore`.

## Capabilities

### New Capabilities

- `a2a-messaging`: authenticated A2A `message/send` and `message/stream` produce
  real Orchestrator turns, with events projected to the A2A wire.

## Impact

- **Code touched**: `crates/web-ui/src/a2a/{handlers,mod,task_store}.rs`,
  `lib.rs` (A2AState construction), new `crates/web-ui/src/a2a/projection.rs`
  (`A2aProjector`), new `migrations/042_a2a_tasks.sql` + its registration in
  `crates/storage/src/lib.rs`, `crates/core/src/types/conversation.rs`
  (`Interface::A2a`).
- **Tests**: `A2aProjector` totality + mapping; `A2aTaskStore` round-trip
  (in-memory + SQLite parity, persistence across a fresh pool); handler tests
  for real-turn send, streaming, and the 403 gate (`ScriptedLlmProvider` +
  `StorageLayer::new_in_memory()`).
- **Behavior change**: A2A `message/send` and `message/stream` now return real
  assistant output instead of the stub string and **persist tasks across
  restart**; unauthenticated/under-scoped callers get `401`/`403`. No route or
  response-body shape change (still `SendMessageResponse` / `StreamResponse`), so
  **no `openapi.json` regeneration**.
- **Non-goals**:
  - Org/space turn re-scoping (still via `state.agent_id`, per Phase 0).
  - AG-UI / ACP adapters (other Phase 1+ changes).
- **User-facing documentation needed**: Yes — operator/developer docs in `docs/`
  for driving the assistant via A2A `message/send` + `message/stream`.
