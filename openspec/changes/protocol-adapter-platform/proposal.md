## Why

We hand-roll three things that emerging open protocols now standardize: the
orchestrator's wire serialization, the Flutter client transport, and our
agent-interop surface. Investigation (see `design.md`) shows these map to
_distinct relationships_ — frontend↔agent backend (AG-UI), agent↔agent peers
(A2A), editor↔coding-agent (ACP) — plus the two we already own well:
management CRUD (OpenAPI `/api`) and tool exposure (MCP). No single protocol
covers all of them; forcing one to is what made earlier "replatform on X"
framings fail.

Two structural gaps block adopting any of them cleanly: (1) `OrchestratorEvent`
is re-serialized by hand at every boundary (`web-ui/src/api/messages.rs`, the
CLI, the REPL) with no shared projector; (2) the existing A2A surface is an
orphan — an in-memory task store with no `AuthContext`, org, or space,
disconnected from the Orchestrator (its `send_message` is a stub). Adding
protocols on this footing would multiply both gaps four times over.

## What Changes

This is an **epic** — it establishes the architecture and sequences the work;
it ships no protocol by itself. Concretely it defines:

- A **ports-and-adapters** contract: every protocol is a thin adapter over the
  _one_ Orchestrator (matching the existing `interface-implementation` rule).
- A **shared event-projection layer**: `OrchestratorEvent → {wire events}`
  with exactly one projector per protocol.
- A **shared auth spine**: every inbound adapter resolves the same
  `AuthContext` (org/space/scopes) before dispatch.
- The protocol↔concern map and a four-phase roadmap, each phase a separate
  child change.

Detailed, TDD-first implementation tasks live in the child changes, not here.

## Capabilities

### New Capabilities

- `protocol-adapters`: the architectural invariants every protocol adapter
  MUST satisfy (single Orchestrator, shared projection, shared auth, shared
  content model).

### Modified Capabilities

(none — individual phases will add or modify per-protocol capabilities.)

## Impact

- **Scope**: architecture + roadmap only. Each phase is independently
  proposed, reviewed, and shipped as a child change (stacked PRs).
- **Phases**: 0 keystone (projection layer + auth spine) → 1 finish A2A +
  AG-UI stream schema → 2 ACP-as-client (subagent delegation) → 3 ACP-as-agent
  (CLI).
- **Non-goals**:
  - Replacing `/api` CRUD or the generated Dart client (OpenAPI is the right
    tool for management; kept).
  - Replacing the internal domain model with an external protocol's types
    (`ContentBlock` is already MCP-shaped — the right amount of standard).
  - Implementing any single protocol within this change.
- **Resolved decision** (2026-05-23, scopes Phase 1): adopt AG-UI **fully** —
  community Rust + `ag_ui` Dart SDKs — to delete the hand-rolled SSE glue, with
  a spike-first / vendor-if-stalled de-risking strategy. See `design.md`.
- **User-facing documentation needed**: Not for the epic. Each protocol phase
  that exposes an external surface MUST ship operator/developer docs in
  `docs/`.
