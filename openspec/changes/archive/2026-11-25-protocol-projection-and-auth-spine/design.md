## Context

Phase 0 of the `protocol-adapter-platform` epic. The epic's `protocol-adapters`
spec mandates two cross-cutting invariants: a single event-projection layer and
a shared auth spine. This change implements the _enforcement scaffolding_ for
both, so later per-protocol phases inherit them.

Grounding (verified against the current tree):

- `OrchestratorEvent` (`crates/runtime/src/orchestrator/stream_event.rs`) has
  **10 variants**: `Token`, `Status`, `ToolResult`, `SkillComplete`,
  `AgentError`, `Thinking`, `SubagentStarted`, `SubagentCompleted`,
  `SubagentEvent` (recursive — boxes an inner event), `AudioReady`. It derives
  `Debug, Clone` only — **no `Serialize`**; every boundary serializes by hand.
- The SSE mapping lives inline in `crates/web-ui/src/api/messages.rs`
  (~lines 200-405) and also handles thinking-batching, durable persistence
  (`event_store.append_event`), sequence numbering, and live broadcast. Those
  are transport concerns and stay in `messages.rs`; only the pure
  event→frame mapping moves.
- Event names + payload shapes today: `token` (raw text), `thinking`
  (`{content}`), `status` (`{message, tool_call_id?}`), `tool_result`
  (`{tool_name, status, tool_call_id?, ...}`), `skill_complete`
  (`{skill_name, success, summary}`), `agent_error` (raw message),
  `subagent_started` (`{agent_id, task}`), `subagent_completed`
  (`{agent_id, status, summary}`), subagent inner (`{agent_id, data}`),
  `audio_ready` (`{audio_id, auto_play}`), plus terminal `run_started` / `done`.
- Auth: `AuthExtractor(pub AuthContext)` exists in `crates/auth/src/middleware.rs`
  and is used by CRUD handlers. The streaming `send_message`
  (messages.rs:88) takes `(State, Path, Json)` only and scopes off
  `state.agent_id.read().await` (messages.rs:104). A2A handlers take neither.
  Both `/api` and A2A sit behind the `require_auth` route-layer
  (lib.rs:921-923), so they are _authenticated_ but do not _thread_
  `AuthContext` into turn dispatch.

## Goals / Non-Goals

**Goals:**

- One total, tested `OrchestratorEvent → frame` projector per existing wire.
- Byte-identical SSE output (behavior-preserving).
- An enforced contract that inbound turn dispatch requires a resolved
  `AuthContext`.

**Non-Goals:**

- Replacing `state.agent_id` turn scoping with `AuthContext`-derived org/space
  routing (deferred — multi-org turn routing is its own change).
- Touching A2A/AG-UI/ACP (later phases).
- Changing event variants, wire names, or payload shapes.

## Decisions

### 1. Projection layer lives in `assistant-runtime`, emits a neutral frame

New module `crates/runtime/src/projection/`. It owns `OrchestratorEvent` and is
already depended on by both `web-ui` and `interface-cli`, so it is the only
no-cycle home. The SSE projector emits a transport-neutral
`ProjectedFrame { event: String, data: serde_json::Value }`; `messages.rs`
adapts that to an axum `Event` and keeps persistence/seq/batching around it.

**Alternative considered:** a new `assistant-protocol` crate. Rejected for
Phase 0 — premature; revisit if projector count grows.

### 2. One projector per wire, behind a `StreamProjector` seam

```
trait StreamProjector {
    type Frame;
    fn project(&self, event: &OrchestratorEvent) -> Vec<Self::Frame>;
}
```

`SseProjector` (`Frame = ProjectedFrame`) and `CliProjector`
(`Frame = String`, the rendered line). The CLI's existing inline rendering in
`interface-cli` moves behind `CliProjector`. Returning `Vec` accommodates
`SubagentEvent`, which recurses into the inner projector and may yield a
prefixed/nested frame.

### 3. Totality is compiler-enforced; conformance test guards intent

Each projector's `match` carries **no `_` arm**, so adding an
`OrchestratorEvent` variant fails to compile until projected. A conformance
test additionally constructs one sample of every variant (including a nested
`SubagentEvent`) and asserts a non-empty projection, documenting the contract.

### 4. Wire parity proven by a golden test

A golden test feeds a representative sequence through `SseProjector` and asserts
event names + serialized payload JSON match the pre-refactor output exactly. The
refactor is rejected if any byte differs. (`token` and `agent_error` keep their
current raw-text data form; all others keep their JSON object form.)

### 5. Auth seam: full compiler enforcement across all three submission surfaces

Decision (revised during implementation, at the user's direction): rather than a
contained chokepoint, require `&AuthContext` on **every** turn-submission entry
point — illegal states made unrepresentable everywhere. `submit_turn` exists at
three layers, all updated:

- inherent `Orchestrator::submit_turn` / `_with_attachments` / `_with_request_id`
  (derive `TurnIdentity::from_auth`),
- the `AssistantInterface` trait + its `Orchestrator` impl and test mocks,
- the `OrchestrationEngine` trait + its `Orchestrator` impl and `Stub` mock.

A new `AuthContext::system()` constructor gives trusted local/non-network callers
(scheduler, BOOT hook, CLI, MCP, messengers, tests) an explicit identity to pass
at the call site — no silent default survives. Network adapters pass a
request-resolved context: the `/api` streaming handlers take
`Extension<AuthContext>` (populated by the existing `require_auth` middleware).

The resolved context is genuinely used: the `/api` handlers gate posting on the
`conversations:write` scope (`caller_can_post`, returning `403`), and identity
flows into `TurnIdentity`. Full org/space turn _re-scoping_ (replacing
`state.agent_id`) stays deferred per Non-Goals.

### 6. No `_` discovery hidden: this is not a pure refactor end-to-end

The projection extraction is behavior-preserving. The auth seam is a _new
guardrail_ and a _minor authz tightening_ (scope check on the streaming
handler). This is intended and called out so reviewers expect a new 403 path,
not just moved code.

### 7. Two streaming handlers, two drifted subagent wires — unified (discovered)

Implementation surfaced a **second** streaming handler (the voice/audio path in
`messages.rs`) with a divergent subagent wire: its `data` carried a redundant
`"event_type"` field and a different inner fallback. The Flutter client
(`app/lib/api/api_client.dart`) parses only `agent_id` and `data` — it never
reads `event_type` — so the field was dead weight. Decision: route the voice
handler through `SseProjector` too, dropping the redundant field and unifying
both handlers on one wire. The SSE _bytes_ for voice subagent events change, but
there is **no observable client behavior change**, and no `openapi.json` /
Flutter client regeneration is needed (SSE bodies are not in the OpenAPI spec).
This converts unintentional drift into a single source of truth.
