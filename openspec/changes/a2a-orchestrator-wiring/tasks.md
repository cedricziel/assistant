# Tasks — a2a-orchestrator-wiring (Phase 1)

TDD throughout: each implementation task starts with a failing test confirmed
RED before production code. Chunks ≤ ~2h.

## 1. `Interface::A2a` + auth helper

- [x] 1.1 Add `Interface::A2a` to `crates/core/src/types/conversation.rs`
      (+ any `Display`/match arms); confirm `make check`.
- [x] 1.2 Lift `caller_can_post(&AuthContext)` to a shared helper reusable by
      both `/api` and A2A (e.g. `assistant-auth` or a web-ui `auth` util).

## 2. `A2aProjector` (red first)

- [x] 2.1 RED: add `crates/web-ui/src/a2a/projection.rs` with `A2aProjector`
      implementing `assistant_runtime::projection::StreamProjector`
      (`Frame = a2a_json_schema::StreamResponse`); totality test over every
      `OrchestratorEvent` variant (incl. nested `SubagentEvent`). Confirm RED.
- [x] 2.2 GREEN: implement the exhaustive `match` (no `_` arm) per design §3
      (`Token`→message chunk; `Status`/`ToolResult`/`SkillComplete`→status
      update; `AgentError`→failed status; `Subagent*`/`AudioReady`→status w/
      metadata). Add per-variant mapping assertions.

## 3. Task persistence — `A2aTaskStore` trait pair (red first)

- [x] 3.1 Add `migrations/042_a2a_tasks.sql` (table per design §6) and register
      it in `crates/storage/src/lib.rs`; confirm it applies on
      `StorageLayer::new_in_memory()`.
- [x] 3.2 RED: define the `A2aTaskStore` trait in `web-ui/src/a2a/task_store.rs`;
      rename the current in-memory store to `InMemoryA2aTaskStore` impl; write a
      store round-trip test (create→update→get→list). Confirm RED on the new
      `SqliteA2aTaskStore` (not yet implemented).
- [x] 3.3 GREEN: implement `SqliteA2aTaskStore` over the space pool (JSON blob +
      indexed columns). Parity test: same sequence on both impls; persistence
      test: reopen a store over the same pool and `get_task` returns it.

## 4. `A2AState` wiring

- [x] 4.1 Extend `A2AState` with `orchestrator`, `pool`, `agent_id` and a
      `task_store: Arc<dyn A2aTaskStore>`; update the `lib.rs` construction site
      (SQLite store from the same handles as `ApiState`).
- [x] 4.2 Add a get-or-create-conversation path keyed by a UUIDv5 over
      `context_id` (store helper if missing); unit-test get-or-create.

## 5. `message/send` (red first)

- [x] 5.1 RED: handler test — authenticated send returns a `Task` whose final
      message is the scripted agent answer (ScriptedLlmProvider + in-memory
      storage). Confirm RED against the stub.
- [x] 5.2 GREEN: resolve `Extension<AuthContext>` + 403 gate; map context→conv;
      `submit_turn(&auth, …, Interface::A2a, …)`; build + persist the `Task` from
      the `TurnResult`. Pass.
- [x] 5.3 RED→GREEN: test that an under-scoped caller gets `403` and no turn.

## 6. `message/stream` (red first)

- [x] 6.1 RED: handler test — streaming emits `StreamResponse` frames derived
      from scripted events and a terminal `Completed` task. Confirm RED.
- [x] 6.2 GREEN: register token sink; run turn; project events via `A2aProjector`
      into SSE; persist task state; terminal `Completed` + `[DONE]`. Pass.

## 7. Finalize

- [x] 7.1 Confirm no inline `OrchestratorEvent` match remains in A2A handlers;
      confirm `openapi.json` unchanged (no route/response-shape change).
- [x] 7.2 Write A2A developer/operator docs in `docs/` (driving the assistant
      via `message/send` + `message/stream`; note tasks now persist across
      restart).
- [x] 7.3 `cargo fmt --all` + `cargo clippy --workspace -- -D warnings` +
      affected-crate tests green. `openspec validate a2a-orchestrator-wiring`;
      tick the epic's Phase 1 `a2a-orchestrator-wiring` task.
