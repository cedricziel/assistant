# Tasks — protocol-projection-and-auth-spine (Phase 0)

TDD throughout: each implementation task is preceded by a failing test that is
confirmed RED before any production code is written. Chunks are ≤ ~2h.

## 1. Projection layer — scaffolding + totality (red first)

- [x] 1.1 RED: add `crates/runtime/src/projection/mod.rs` with the
      `StreamProjector` trait and a `ProjectedFrame { event, data }` type; write
      a conformance test that constructs every `OrchestratorEvent` variant
      (incl. nested `SubagentEvent`) and asserts a non-empty projection.
      Confirm it fails to compile / fails.
- [x] 1.2 GREEN: implement `SseProjector` with an exhaustive `match` (no `_`
      arm) covering all 10 variants. Make the conformance test pass.
- [x] 1.3 Verify totality guard: outer `match` has no `_` arm, so a new variant
      is a compile error (E0004) — verified by inspection.

## 2. SSE wire parity (red first)

- [x] 2.1 RED: record a golden of the current inline SSE mapping (event names +
      payload JSON) for a representative sequence (`Token`, `Thinking`,
      `Status`, `ToolResult`, `SkillComplete`, `SubagentStarted`,
      `SubagentEvent`, `AudioReady`, `AgentError`); assert `SseProjector` output
      equals it. Confirm RED (projector not yet wired to match exact shapes).
- [x] 2.2 GREEN: align `SseProjector` payloads to byte-match — `token`/
      `agent_error` raw text, all others JSON objects per `design.md`. Pass.

## 3. Consume the projector in `messages.rs` (behavior-preserving)

- [x] 3.1 Replace the inline `OrchestratorEvent` `match` in
      `crates/web-ui/src/api/messages.rs` with calls to `SseProjector`, keeping
      thinking-batching, `event_store.append_event` persistence, sequence
      numbering, and live broadcast in `messages.rs`. Also unified the voice
      handler onto the projector (drift fix — see design Decision 7).
- [x] 3.2 Run existing web-ui streaming tests; confirm SSE responses unchanged
      (369 web-ui lib tests pass).

## 4. CLI projector (red first)

- [ ] 4.1 RED: add a `CliProjector` (`Frame = String`) test asserting rendered
      lines for representative variants. Confirm RED.
- [ ] 4.2 GREEN: implement `CliProjector` (exhaustive, no `_` arm); move the
      inline rendering in `crates/interface-cli/src/{main,repl_helpers}.rs`
      behind it. Pass; confirm REPL output unchanged manually.

## 5. Auth seam — full compiler enforcement (all 3 submission surfaces)

> Decision (user-directed): do the full seam, not a contained chokepoint.

- [x] 5.1 Add `AuthContext::system()` for trusted local/non-network callers.
- [x] 5.2 Require `&AuthContext` on every submission entry point: inherent
      `Orchestrator::submit_turn*` (derive `TurnIdentity::from_auth`), the
      `AssistantInterface` trait + impls/mocks, and the `OrchestrationEngine`
      trait + impl/stub. Update all call sites (web → real `Extension<AuthContext>`;
      scheduler/MCP/CLI/BOOT/tests → `AuthContext::system()`).
- [x] 5.3 Gate `/api` posting on `conversations:write` (`caller_can_post` → 403);
      add a web test asserting a caller lacking the scope gets `403` and no turn
      is dispatched (store scoping via `state.agent_id` stays — re-scoping
      deferred). 25 web-ui messages tests green incl. the new 403 test.

## 6. Finalize

- [x] 6.1 Auth gate added `403` to 3 endpoints → ran `make dump-openapi`
      (adds the `403` responses) and `make generate-flutter-client` (README
      only; error responses don't change generated models).
- [x] 6.2 `cargo fmt --all` clean; `cargo clippy --workspace -- -D warnings`
      clean (added `#[allow(clippy::too_many_arguments)]` to the 8-arg
      `submit_turn_with_request_id` variants; collapsed push-notification `if`
      into let-chains). All member crates + root crate (isolated) tests pass.
      Note: `workspace_clock_lint` is flaky under concurrent `--workspace`
      (passes isolated; a pre-existing test-isolation race, no banned patterns
      introduced).
- [x] 6.3 `openspec validate protocol-projection-and-auth-spine`; epic task 1
      already ticked.
