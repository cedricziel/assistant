## Why

This is **Phase 0 of the `protocol-adapter-platform` epic** — the keystone that
makes adding protocols cheap and safe. Two structural gaps block every later
phase:

1. **No shared projection.** `OrchestratorEvent` is mapped to wire events by
   hand inside `crates/web-ui/src/api/messages.rs` (~250 lines of inline
   `match`), and again in the CLI. There is no single, tested place that turns
   one orchestrator event into one protocol's frames, so every new protocol
   would copy-paste a serializer and silently drift.
2. **No enforced auth seam.** The CRUD handlers resolve `AuthContext` via
   `AuthExtractor`, but the streaming turn handler scopes off a global
   `state.agent_id` (messages.rs:104) and the A2A handlers ignore identity
   entirely. Nothing _prevents_ a new inbound adapter from dispatching a turn
   with no resolved `AuthContext` — the exact mistake the A2A stub already made.

Fixing both now means Phase 1+ adapters add one projector and inherit the auth
seam, instead of multiplying these gaps.

## What Changes

- Extract a **shared event-projection layer** (new module in
  `assistant-runtime`): the canonical `OrchestratorEvent → ProjectedFrame`
  mapping, with one projector per existing wire (`/api`-SSE and the CLI).
  `messages.rs` and the CLI consume the projector instead of inline matches.
- Add a **totality conformance test**: a sample of every `OrchestratorEvent`
  variant is projected, and the match carries no `_` arm, so a new variant
  forces an explicit projector decision.
- Add a **wire-parity golden test**: the SSE projector emits byte-identical
  event names and payload JSON to today (behavior-preserving refactor).
- Establish the **inbound auth contract**: route inbound turn dispatch through
  a seam that requires a resolved `AuthContext`, enforced by a conformance test
  (compiler seam preferred, source-scan fallback à la
  `tests/workspace_lint_policy.rs`).

## Capabilities

### New Capabilities

- `event-projection`: a single, total, tested mapping from `OrchestratorEvent`
  to each protocol's wire frames.
- `inbound-auth-spine`: every inbound turn-accepting adapter resolves an
  `AuthContext` before dispatch, enforced rather than conventional.

## Impact

- **Code touched**: new `crates/runtime/src/projection/` module;
  `crates/web-ui/src/api/messages.rs` (consume projector + auth seam);
  `crates/interface-cli/src/{main,repl_helpers}.rs` (consume projector);
  workspace conformance test(s).
- **Tests**: totality test, SSE wire-parity golden test, auth-seam conformance
  test.
- **Behavior change**: none intended. SSE wire stays byte-identical; no route
  shape change, so **no `openapi.json` or Flutter client regeneration**.
- **Non-goals**:
  - Re-scoping the turn from `AuthContext` (replacing `state.agent_id` with
    org/space-derived routing) — deferred; it entails multi-org turn routing.
  - Wiring A2A to the Orchestrator (Phase 1 `a2a-orchestrator-wiring`).
  - Any AG-UI/ACP projector (later phases).
  - Changing `OrchestratorEvent` variants or the SSE wire vocabulary.
- **User-facing documentation needed**: No. Internal refactor + test
  guardrails; no user-visible behavior change.
