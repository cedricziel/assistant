## Context

Today the client decides "is this stream alive?" purely from byte-level observation of the SSE socket — bytes arrive (alive), bytes don't arrive (suspect dead). Once `sse-keepalive` lands, the false-positive case (slow tool → looks dead) is fixed. But the dual false-negative remains: a stream can keep ferrying keep-alive comment bytes (or arrive over a connection that LOOKS open at the TCP layer) while the server-side turn has died, crashed, been garbage-collected, or never existed.

For the client to make any cancellation decision — automatic or user-initiated — it needs a way to ask the server's authoritative view of the turn. Right now no such surface exists.

The runtime layer (`assistant-runtime`) tracks turn execution internally via the Orchestrator and the turn-result bus, so the data is available; it just isn't exposed over HTTP.

## Goals / Non-Goals

**Goals:**

- A read endpoint that returns the authoritative state of an in-flight or recently-completed turn: `running` / `completed` / `errored` / `unknown`.
- A write endpoint that explicitly cancels an in-flight turn, propagating the abort through the runtime so any spawned task tree (LLM call, tool invocation, subagent) is torn down.
- Atomic state transitions: a turn cannot be both "completed" and "cancellable" — the cancel endpoint returns `409 Conflict` with the terminal state in the body when racing against natural completion.
- Client integration that uses the probe (a) on suspected stall, (b) on AppLifecycleState.resumed with an interrupted turn, and (c) when the user clicks Skip. The byte-level watchdog stops being the cancellation trigger — it becomes a probe trigger.

**Non-Goals:**

- Background cancellation of orphaned turns server-side (e.g. via TTL). Out of scope; covered by general operational maintenance.
- Multi-turn cancellation (cancel an entire conversation's outstanding turns). The endpoint is per-turn.
- A "pause" or "resume" mechanism. Cancel is one-way.
- Generic stop-task plumbing for the assistant runtime beyond what's needed to support cancel. Future work may generalise.

## Decisions

### Decision 1: Endpoints under `/api/conversations/{id}/turns/{turnId}/…`

Place the new endpoints under the existing conversation namespace rather than at a global `/api/turns/{id}` location. This matches the project's URL conventions (`URL patterns: Nested resources: /api/{resource}/{id}/{sub-resource}` per `AGENTS.md`) and makes authorisation trivial — the conversation owner is the only legitimate caller, and the existing conversation-scoped auth middleware extends naturally.

**Endpoints:**

- `GET /api/conversations/{conversationId}/turns/{turnId}/status` — read.
- `POST /api/conversations/{conversationId}/turns/{turnId}/cancel` — write.

**Why:** Convention-following; existing auth surfaces apply; client knows the conversationId already.

**Alternatives considered:**

- Flat `/api/turns/{turnId}/…`. Rejected: breaks the existing nesting pattern, requires separate auth lookup.

### Decision 2: Four-state terminal taxonomy

The status response uses a closed enum:

- `running` — the turn is being processed (LLM, tool, or subagent in flight).
- `completed` — the turn finished cleanly; final response is durable in the conversation.
- `errored` — the turn ended with an `agent_error`; recovery may be possible via retry.
- `unknown` — the turn ID was never recorded, has been garbage-collected, or belongs to a different conversation.

Anything not in this set must round-trip to one of these four when serialised — the client's switch is exhaustive.

**Why:** Closed enums survive backwards-incompatibly only if exhausted. Keeping the set tight forces deliberate evolution.

**Alternatives considered:**

- Add `cancelling` / `cancelled` as separate states. Rejected for v1: cancel resolves into one of `completed` (partial output saved) or `errored` (no partial output) quickly enough that the intermediate state isn't observable, and exposing it would require defining its semantics carefully (Is `cancelled` retriable? Does it count as `errored`?). Revisit if user research shows the intermediate state matters.

### Decision 3: Cancel propagates via the runtime's existing abort surface

The runtime layer already has internal abort hooks (the Orchestrator can drop an in-flight turn, e.g. on shutdown). The new endpoint wires a public surface to that existing internal mechanism, with the safety property that partial output already streamed to the client SHALL be saved as a `failed` (partial) assistant message in the conversation — never silently dropped.

**Why:** Reusing the existing abort surface avoids adding a parallel mechanism. Saving partial output matches the existing best-effort behaviour during natural error termination.

**Alternatives considered:**

- Hard-drop partial output on cancel. Rejected: users who skip a slow tool call may still want to see "what the assistant got to so far" in the conversation history. Saving partial output preserves the trail.

### Decision 4: Cancel emits a final `agent_error` SSE event on any open stream

When `POST /…/cancel` succeeds, the server emits a final `agent_error` event with a cancellation marker on any still-connected SSE stream for that turn. This lets clients connected via SSE (including those that didn't initiate the cancel) see a consistent terminal state — they don't need to poll status to find out the turn ended.

**Why:** The SSE event stream is the canonical UI feed. Driving cancellation through it keeps all client views consistent.

**Alternatives considered:**

- Cancel sends a custom `cancelled` event. Rejected: forces every client (Flutter, MCP, A2A, third-party) to handle a new event kind. `agent_error` with a marker fits the existing vocabulary.

### Decision 5: Client integration replaces byte-watchdog cancellation entirely

After this change lands, the client's `_byteHeartbeatTimeout` no longer closes streams on its own. Instead, when the heartbeat fires, the client probes `GET …/status`. If `running`, the client extends its watchdog and waits. If terminal, the client reconciles to the terminal state. If `unknown`, the client refetches the conversation.

User-initiated Skip flows through `POST …/cancel` exclusively.

**Why:** Eliminates the entire class of "client thinks server is dead but isn't" bugs.

**Alternatives considered:**

- Keep the byte-watchdog as a backstop in addition to the probe. Considered worth it for one or two releases as a safety belt, then can be removed once telemetry confirms the probe is reliable.

## Risks / Trade-offs

- **Cancel must reliably tear down spawned tasks.** A leaked LLM call or tool invocation costs money and may produce surprising side effects. → Mitigation: integration test covers the full task tree — spawn a long tool, fire cancel, assert the tool was actually killed and no further events arrive. Telemetry on cancel duration.
- **Race between status probe and natural completion.** Probe says `running`; by the time the client decides to cancel, the turn finished cleanly. → Mitigation: cancel returns `409 Conflict` with the actual terminal state in the body; client reconciles without showing a misleading error.
- **Partial-output durability semantics.** Saving partial output as `failed` means the conversation can contain truncated mid-sentence text. → Mitigation: matches existing error-termination behaviour; the new SSE `agent_error` event includes enough context for the client to label the bubble distinctively (e.g. "Cancelled — partial response").
- **Authorisation.** Anyone with conversation access can cancel any turn in that conversation. → Mitigation: existing conversation-scoped auth middleware applies; no new permission model needed.

## Migration Plan

1. **Backend phase 1**: implement runtime hooks to expose turn state lookup + abort. Unit tests at the runtime layer.
2. **Backend phase 2**: implement the two HTTP handlers, OpenAPI docs, integration tests.
3. **Client phase 1**: regenerate the Dart client via `make dump-openapi && make generate-flutter-client`.
4. **Client phase 2**: integrate the probe into `ChatNotifier` — replace byte-watchdog stream-close with a probe call; replace stall-cancellation in `chat-stream-progress-ux`'s Skip surface with a cancel call. Unit + widget tests.
5. **Client phase 3**: remove the residual `_byteHeartbeatTimeout` stream-close path once the probe-based flow has shipped for a release and telemetry confirms reliability.

Each phase is its own PR. Backend phases must land first; client phases follow.

## Open Questions

- Should the cancel endpoint allow a reason string in the request body for audit/telemetry purposes? Probably yes — small addition. Default to empty.
- Should `status` return the partial assistant text accumulated so far? Useful for the client to show "0.3 seconds of response so far…" in the stall card. Possibly out of scope for v1; the SSE event stream already carries this.
- How long do `unknown` results persist? If a turn ID is garbage-collected after 24 hours, a client checking on an old turn gets `unknown` rather than `completed`. Acceptable? Probably — the conversation history is the source of truth for old turns, not the status endpoint.
- Should we ship the Skip button in `chat-stream-progress-ux` immediately when `turn-status-api` lands, or gate it behind a feature flag for one release while we measure cancel reliability? Likely flag, then default-on after a release.
