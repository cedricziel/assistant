## Context

The assistant has one reasoning core — the Orchestrator (`assistant-runtime`,
the ReAct loop) — fronted by several surfaces that have grown independently:

- `/api/*` (OpenAPI REST + SSE) consumed by the Flutter app via a generated
  Dio client (`app/packages/assistant_api/`, ~47k generated LOC) plus a
  hand-rolled SSE layer (`app/lib/api/api_client.dart`) with platform-specific
  hacks (iOS chunked-transfer buffering, web `EventSource` fallback).
- Messenger interfaces (Slack, Mattermost, Matrix, Nextcloud, Signal) in
  `assistant-interfaces`, all driven through `ChannelRunner` → Orchestrator.
- An MCP server and MCP client (`assistant-mcp-server` / `-client`) — JSON-RPC
  2.0 over stdio — exposing/consuming tools.
- An A2A surface in `web-ui/src/a2a/` — types are complete, the agent card and
  registry are real, but `message/send` and `message/stream` are stubs
  (`// TODO: Wire to Orchestrator`) backed by an in-memory `TaskStore`.

Investigation (the conversation that produced this epic) evaluated three
external agent protocols against our needs. Each models a **different
relationship**, and we have all of those relationships:

| Concern                                                  | Protocol       | Direction | Today             |
| -------------------------------------------------------- | -------------- | --------- | ----------------- |
| Management / CRUD (orgs·spaces·personas·skills·traces·…) | OpenAPI `/api` | inbound   | correct, keep     |
| First-party + 3rd-party chat **stream**                  | AG-UI          | inbound   | bespoke SSE       |
| Peer agent interop (agents delegate to/from us)          | A2A            | in + out  | stub, orphaned    |
| Editors drive our assistant (use us as backend)          | ACP-as-agent   | inbound   | none (CLI fits)   |
| We drive external coding agents as subagents             | ACP-as-client  | outbound  | none (best fit)   |
| Tool / capability exposure                               | MCP            | in + out  | already have both |

Key findings that shape this epic:

1. **No single protocol covers everything.** Each covers ~one slice. `/api`
   owns ~26 management resource families that _no_ agent protocol models.
2. **The pain is the boundary, not the core.** The internal domain model
   (`ContentBlock`, `Message`, `OrchestratorEvent`) is an asset; `ContentBlock`
   is already MCP-shaped, which AG-UI/A2A/ACP all reuse.
3. **Two structural gaps block clean adoption** (confirmed by grep):
   - `OrchestratorEvent` is hand-serialized at every boundary; there is **no
     shared projector** (`fn project(&OrchestratorEvent) -> Vec<WireEvent>`
     does not exist).
   - The A2A surface has **no `AuthContext`/org/space** — `A2AState` is just
     `{ task_store, agent_card }`. It is an unauthenticated orphan.
4. **The convergence signal is strong.** We independently reinvented protocol
   primitives: `SafetyGate` ≈ ACP `session/request_permission` ≈ AG-UI
   interrupts; the in-flight `slash-commands` change ≈ ACP
   `available_commands_update`; the in-flight `web-session-resilience` change ≈
   AG-UI `STATE_SNAPSHOT`/`STATE_DELTA`; `Subagent*` events ≈ ACP-as-client.

## Goals / Non-Goals

**Goals:**

- Establish "one Orchestrator brain, N thin protocol adapters, one per
  concern" as the platform architecture.
- Define the two shared pieces that make N protocols cost ≈ `1× + ε` instead
  of `N×`: a single event-projection layer and a single auth spine.
- Map every protocol to its concern and direction, and sequence the work into
  independently shippable phases.

**Non-Goals:**

- Implementing any protocol in this change (each is a child change).
- Replacing `/api` CRUD or the generated Dart client.
- Replacing the internal domain model with external protocol types.
- Locking in the community AG-UI SDKs (schema adoption ≠ SDK adoption).

## Architecture: ports & adapters around one Orchestrator

```
            INBOUND doors                              OUTBOUND doors
   (someone drives the assistant)            (the assistant drives someone)

  OpenAPI ─┐                                            ┌─ A2A-client → peer agents
  AG-UI  ──┤                                            │
  A2A-in ──┤     ┌────────────────────────────┐        ├─ ACP-client → coding agents
  ACP-agent┼────▶│        ORCHESTRATOR        │───────▶│             (= Subagent Process)
  Slack ───┤     │   (ONE ReAct brain)         │        │
  Matrix ──┤     │  + AuthContext (org/space)  │        └─ MCP-client → external tools
  MCP-srv ─┘     └────────────────────────────┘
                             │
                 ┌───────────┴────────────┐
                 │  EVENT PROJECTION LAYER │  ← keystone (does not exist yet)
                 │  OrchestratorEvent  →   │
                 │   {AG-UI | A2A | ACP |  │
                 │    /api-SSE | CLI}      │
                 └─────────────────────────┘
```

This matches the grain of the codebase: `.claude/skills/interface-implementation`
already mandates that _every interface goes through the Orchestrator_. "Support
all protocols" is just "every protocol is an interface."

## The keystone (what makes the epic worth doing)

The value is not in the protocols — it is in two shared pieces beneath them.

**1. One event-projection layer.** `OrchestratorEvent` is the canonical
internal stream. Each protocol becomes a pure projector. Adding a protocol =
adding one projector + one conformance suite, not a new serializer copy-pasted
across handlers. This is the difference between `N×` and `1× + ε`.

**2. One auth/identity spine.** Every inbound adapter MUST resolve the same
`AuthContext` (org · space · roles · scopes) before touching the Orchestrator.
`/api` already does this (API keys + OAuth). A2A does not — fixing that is part
of Phase 1, and the invariant prevents every future door from becoming a fresh
unauthenticated attack surface against multi-tenant data.

**Shared substrate we already have right:** the domain content model.
`ContentBlock` (`crates/core/src/llm/types.rs`) is MCP-shaped; AG-UI, A2A, and
ACP all reuse MCP content representations. It stays the single serialization
source — we do **not** externalize the domain model.

## The cautionary tale (already in the repo)

The current A2A surface is what "support all protocols" looks like done wrong:
a parallel `task_store` (its own half-brain), no Orchestrator, no auth,
hand-rolled in isolation. Every new door MUST be **subtractive in concept** —
it adds a serializer and a route, never a second brain or a second data model.
Phase 1 explicitly converts A2A from this anti-pattern into a proper adapter.

## Phased roadmap

Each phase is a separate OpenSpec change (stacked PRs). The epic tracks them;
it does not contain their implementation tasks.

- **Phase 0 — Keystone** (`protocol-projection-and-auth-spine`)
  Extract the event-projection layer from `api/messages.rs`; make `AuthContext`
  mandatory on every inbound adapter. Pure refactor, no new protocol, no
  external commitment. Unlocks everything else.

- **Phase 1 — Finish the half-built + highest-pain**
  - `a2a-orchestrator-wiring`: replace the A2A stub with a real adapter over
    the Orchestrator via the projector + auth spine; persist tasks.
  - `ag-ui-stream-schema`: re-shape the first-party stream to AG-UI's event
    vocabulary ("speak the standard, own the plumbing"). Gated on the open
    decision below.

- **Phase 2 — New capability, best architectural fit**
  - `acp-client-subagents`: the Orchestrator drives external coding agents
    (Claude Code, Gemini CLI, …) as `Subagent Process`es over ACP. The
    env-ownership inversion that breaks ACP-as-agent **resolves** here because
    the server already owns the workspace/terminal; reuses `Subagent*` events.

- **Phase 3 — Nice-to-have**
  - `acp-agent-cli`: expose the assistant as an ACP agent for editors. Only the
    CLI's environment model fits ACP's "client owns the filesystem" assumption;
    web/mobile never will.

## Resolved decision — adopt AG-UI including the SDKs (2026-05-23)

AG-UI is the only external protocol _built for_ the frontend↔agent-backend
axis, maps cleanly onto `OrchestratorEvent`, and is the only one with a Dart
client on pub.dev. **Decision:** Phase 1's `ag-ui-stream-schema` adopts AG-UI
**fully** — the community Rust SDK on the server and the community `ag_ui` Dart
SDK on the client — to delete the most hand-rolled glue (`api_client.dart`'s
SSE layer and its iOS/web platform hacks). The driver is ecosystem interop plus
the largest reduction in client transport code we own.

**Accepted risk:** both SDKs are community-maintained and early (Dart `ag_ui`
v0.1.0, ~8 months stale, ~969 downloads; Rust SDK is a community crate). We
therefore commit to a de-risking strategy rather than a blind dependency:

1. **Spike first.** Phase 1 opens with a throwaway spike that drives a real
   turn end-to-end through both SDKs, asserting every `OrchestratorEvent`
   variant survives the round trip (text, reasoning, tool calls, subagents via
   `Custom`/`Raw`, audio, errors, cancel). Go/no-go gate before committing.
2. **Vendor + pin.** Pin exact versions; be prepared to vendor/fork either SDK
   into the workspace (Rust) or `app/packages/` (Dart) if upstream stalls. The
   AG-UI _schema_ is the durable asset; the SDKs are replaceable plumbing.
3. **Keep the projector boundary.** Even with SDKs, the
   `OrchestratorEvent → AG-UI` mapping lives in our Phase 0 projection layer
   behind conformance tests, so swapping or forking an SDK never touches the
   Orchestrator.

This sets the scope of `ag-ui-stream-schema`. The fallback if the spike fails
is the lower-risk "speak the schema, own the plumbing" variant (our own thin
emitter/consumer against the same AG-UI wire schema) — no Orchestrator rework
required, because the projector boundary is identical.

## Decisions

### 1. Model protocols as adapters over one Orchestrator, not as parallel surfaces

Every protocol is an interface in the existing sense. **Alternative
considered:** independent per-protocol stacks (the current A2A shape).
Rejected — it duplicates the data model, auth, and the reasoning loop, and is
the exact debt this epic exists to prevent.

### 2. Standardize the boundary, not the core

Align wire serialization with open schemas; keep the internal domain model
hand-rolled. **Alternative considered:** adopt an external protocol's types as
the core model. Rejected — couples the core to externally-versioned wire specs
for no internal benefit.

### 3. CRUD stays on OpenAPI; only the stream is a protocol question

`/api` + generated client is the correct tool for management resources.
**Alternative considered:** push everything through one agent protocol.
Rejected — no agent protocol has vocabulary for orgs/spaces/personas/traces;
we would smuggle them into `metadata` blobs and lose the standard's value.

### 4. Sequence by ROI and architectural fit, ship phases independently

Keystone first (de-risks all doors), then finish/fix, then new capability,
then nice-to-have. **Alternative considered:** build all protocols at once.
Rejected — multiplies the cautionary-tale risk and blocks review; stacked
single-phase PRs match the team's working style.
