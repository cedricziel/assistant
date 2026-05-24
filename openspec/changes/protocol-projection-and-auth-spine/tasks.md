# Tasks — protocol-projection-and-auth-spine (Phase 0)

TDD throughout: each implementation task is preceded by a failing test that is
confirmed RED before any production code is written. Chunks are ≤ ~2h.

## 1. Projection layer — scaffolding + totality (red first)

- [ ] 1.1 RED: add `crates/runtime/src/projection/mod.rs` with the
      `StreamProjector` trait and a `ProjectedFrame { event, data }` type; write
      a conformance test that constructs every `OrchestratorEvent` variant
      (incl. nested `SubagentEvent`) and asserts a non-empty projection.
      Confirm it fails to compile / fails.
- [ ] 1.2 GREEN: implement `SseProjector` with an exhaustive `match` (no `_`
      arm) covering all 10 variants. Make the conformance test pass.
- [ ] 1.3 Verify totality guard: temporarily add a throwaway variant locally and
      confirm the projector fails to compile; revert.

## 2. SSE wire parity (red first)

- [ ] 2.1 RED: record a golden of the current inline SSE mapping (event names +
      payload JSON) for a representative sequence (`Token`, `Thinking`,
      `Status`, `ToolResult`, `SkillComplete`, `SubagentStarted`,
      `SubagentEvent`, `AudioReady`, `AgentError`); assert `SseProjector` output
      equals it. Confirm RED (projector not yet wired to match exact shapes).
- [ ] 2.2 GREEN: align `SseProjector` payloads to byte-match — `token`/
      `agent_error` raw text, all others JSON objects per `design.md`. Pass.

## 3. Consume the projector in `messages.rs` (behavior-preserving)

- [ ] 3.1 Replace the inline `OrchestratorEvent` `match` in
      `crates/web-ui/src/api/messages.rs` with calls to `SseProjector`, keeping
      thinking-batching, `event_store.append_event` persistence, sequence
      numbering, and live broadcast in `messages.rs`.
- [ ] 3.2 Run existing web-ui streaming tests; confirm SSE responses unchanged.

## 4. CLI projector (red first)

- [ ] 4.1 RED: add a `CliProjector` (`Frame = String`) test asserting rendered
      lines for representative variants. Confirm RED.
- [ ] 4.2 GREEN: implement `CliProjector` (exhaustive, no `_` arm); move the
      inline rendering in `crates/interface-cli/src/{main,repl_helpers}.rs`
      behind it. Pass; confirm REPL output unchanged manually.

## 5. Auth seam — spike compiler enforcement (red first)

- [ ] 5.1 SPIKE: attempt a dispatch seam requiring `&AuthContext` for inbound
      turn submission. Timebox; if it stays contained (does not cascade through
      CLI/messengers/A2A signatures), proceed; else record the decision and use
      the source-scan fallback in 5.3.
- [ ] 5.2 RED: thread `AuthExtractor` into the `/api` streaming `send_message`
      (and stream/quick-message) handlers; add a test asserting a caller lacking
      the message-posting scope gets `403` and no turn is dispatched. Confirm
      RED.
- [ ] 5.3 GREEN: implement the scope check using the resolved `AuthContext`
      (still scoping the store via `state.agent_id` — re-scoping is out of
      scope). Add the enforcement guard: compiler seam (preferred) OR a
      source-scanning conformance test à la `tests/workspace_lint_policy.rs`
      asserting inbound turn-accepting handlers resolve `AuthContext`. Pass.

## 6. Finalize

- [ ] 6.1 Confirm `openapi.json` is unchanged (no route shape change); no
      `make dump-openapi` / `make generate-flutter-client` needed.
- [ ] 6.2 Run `make lint && make format && make test`. (No `app/` changes →
      Flutter checks not required.)
- [ ] 6.3 `openspec validate protocol-projection-and-auth-spine`; tick epic
      task 1 in `protocol-adapter-platform/tasks.md`.
