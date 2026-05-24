# Epic Roadmap — Protocol Adapter Platform

> This is an **epic**. The tasks below are tracking milestones, not 2-hour
> implementation chunks. Each phase ships as its own OpenSpec change with its
> own TDD-first `tasks.md`. This epic is "done" when all child changes are
> archived and the `protocol-adapters` invariants are enforced.

## Phase 0 — Keystone (`protocol-projection-and-auth-spine`)

- [x] Create the child change `protocol-projection-and-auth-spine`
- [ ] Extract the shared event-projection layer from
      `crates/web-ui/src/api/messages.rs` (one projector per existing wire:
      `/api`-SSE and CLI), behind a conformance test over all
      `OrchestratorEvent` variants
- [ ] Define the `AuthContext`-resolution contract every inbound adapter must
      satisfy; add an architecture conformance test (cf.
      `tests/workspace_lint_policy.rs` precedent)
- [ ] No new protocol, no external dependency — pure refactor, green build
- [ ] Archive child change; confirm `protocol-adapters` projection + auth
      invariants hold

## Phase 1 — Finish half-built + highest-pain

> Decision resolved (2026-05-23): adopt AG-UI **fully**, incl. community SDKs.

- [x] Create child change `a2a-orchestrator-wiring`
  - [ ] Replace `web-ui/src/a2a/handlers.rs` stub with a real adapter over the
        Orchestrator via the Phase 0 projector
  - [ ] Add `AuthContext`/org/space to `A2AState`; reject unauthenticated calls
  - [ ] Persist tasks via the storage layer (retire the in-memory `TaskStore`
        or back it with SQLite)
  - [ ] Ship A2A operator/developer docs in `docs/`
- [ ] Create child change `ag-ui-stream-schema` (full SDK adoption)
  - [ ] **Spike (throwaway, go/no-go gate)**: drive one real turn end-to-end
        through the community Rust + `ag_ui` Dart SDKs; assert every
        `OrchestratorEvent` variant round-trips (text/reasoning/tool/lifecycle,
        subagent + audio via `Custom`/`Raw`, error, cancel). Fallback on
        failure: own-the-plumbing variant against the same AG-UI schema
  - [ ] Add the AG-UI projector in the Phase 0 projection layer (boundary stays
        ours even with SDKs, so an SDK swap never touches the Orchestrator)
  - [ ] Server: integrate the AG-UI Rust SDK; pin exact version
  - [ ] Client: integrate the `ag_ui` Dart SDK; pin exact version; delete
        `app/lib/api/api_client.dart` SSE layer + iOS/web platform hacks
  - [ ] Vendor/fork plan documented in case either SDK stalls upstream

## Phase 2 — New capability, best architectural fit

- [ ] Create child change `acp-client-subagents`
  - [ ] Orchestrator acts as an ACP **client** driving external coding agents
        as `Subagent Process`es
  - [ ] Surface external agent `session/update`s through existing `Subagent*`
        `OrchestratorEvent`s
  - [ ] Lend the server-side workspace/terminal as the ACP "client" environment
  - [ ] Ship docs in `docs/`

## Phase 3 — Nice-to-have

- [ ] Create child change `acp-agent-cli`
  - [ ] Expose the assistant as an ACP **agent** for editors, scoped to the CLI
        (the only first-party client whose env model fits ACP)
  - [ ] Map ACP `session/request_permission` to `SafetyGate`; ship docs

## Epic exit criteria

- [ ] All phase child changes implemented and archived
- [ ] `protocol-adapters` spec promoted to `openspec/specs/` on archive
- [ ] Each externally-exposed protocol has docs in `docs/`
- [ ] No inbound adapter dispatches without a resolved `AuthContext`
- [ ] No handler hand-serializes `OrchestratorEvent` (single projection layer)
