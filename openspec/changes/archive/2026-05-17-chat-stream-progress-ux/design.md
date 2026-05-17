## Context

The chat-streaming protocol (`POST /api/conversations/{id}/messages`) emits a rich set of SSE events during a turn: `run_started`, `token`, `status`, `thinking`, `tool_result`, `skill_complete`, `agent_error`, `subagent_started`, `subagent_completed`, `audio_ready`, `done`. The Flutter client today renders some of these inline (tokens flow into the streaming message, tool calls appear as timeline chips) but offers no compact, persistent indicator of "what is the active turn doing right now?". When a tool call takes 30 seconds, the only visual feedback is the absence of new tokens.

The `pendingQueue` mechanism (added by `chat-message-queue`) accepts user messages typed while a stream is in-flight but renders nothing for them. A user who types two follow-up messages in quick succession sees them disappear into a void until the in-flight turn completes — at which point they reappear in send order. This is the symptom that motivated PR #809.

## Goals / Non-Goals

**Goals:**

- Persistent in-flight indicator above the composer that summarises the active turn's most recent state in plain language ("Calling fetch_url…" / "Thinking…" / "Generating response…").
- Visible queued-message bubbles for `pendingQueue` entries, in send order, with a way to remove them before they fire.
- A stall affordance — when no SSE event has arrived for N seconds (initial proposal: 30s), the in-flight card surfaces the silence with a timer and an optional "Skip" button. **The Skip button does not cancel here**; it is wired to `turn-status-endpoint`'s explicit cancel surface, which lands in a follow-up change. Until that lands, Skip is hidden.
- A "Reconnecting…" banner when `attemptReconnect` is in-flight following `AppLifecycleState.resumed`.
- All states are exercised by widget tests; visual regression baselines updated once.

**Non-Goals:**

- Server-side changes — the protocol already provides everything needed.
- Automatic cancellation of in-flight streams (gated to `turn-status-endpoint`).
- Re-architecting the message timeline. The progress card lives alongside the existing message rendering, not in place of it.
- Localisation of the status strings (covered by general project i18n later).

## Decisions

### Decision 1: Progress state derived from a `currentTurnStatus` Riverpod selector

Add a derived state on `ChatNotifier` (or a sibling selector) that exposes:

- `turnId` — null when no turn is in-flight.
- `lastEventKind` — `run_started` / `token` / `status` / `thinking` / `tool_result` / `subagent_*` / etc.
- `lastEventAt` — timestamp of the most recent SSE event for the active turn.
- `secondsSinceLastEvent` — derived in the widget via a 1-second `Ticker` so the card reflects elapsed silence.
- `currentToolName` — populated when the most recent `tool_result` / `subagent_started` event includes a name; cleared on next non-tool event.

**Why:** The in-flight card needs a tight, observable source of truth that's independent of the conversation history. Adding it as a derived state on the existing chat provider keeps the data-flow simple — the card consumes a Riverpod selector and re-renders on relevant changes.

**Alternatives considered:**

- Putting the elapsed-time logic in the card itself with an internal `Ticker`. Rejected: makes the card harder to test deterministically; better to derive elapsed-seconds in a single place.
- A separate `turnProgressProvider`. Possible refactor later; for now the data lives in `ChatNotifier`'s state machine alongside the events it processes.

### Decision 2: Status string is computed from `lastEventKind`, not pre-rendered server-side

A pure-Dart `turnStatusLabel(TurnStatusSnapshot)` returns the user-facing string ("Thinking…", "Calling fetch_url…"). The server emits structured events; the client formats them.

**Why:** Keeps localisation a client concern. The label can evolve without protocol changes. The server's event vocabulary stays stable.

**Alternatives considered:**

- Server-side rendered status strings via a new `status_label` SSE event. Rejected as over-protocol for a UI concern.

### Decision 3: Stall threshold is 30 seconds, configurable, no automatic action

The stall threshold defaults to 30 seconds (longer than the average human attention span, shorter than most enterprise web tool calls). The card transitions from "Generating response…" to "Server is taking longer than expected — 0:34" when silence crosses the threshold.

**No automatic cancellation.** The Skip button only appears once `turn-status-endpoint` lands and provides an explicit cancel surface. Until then, the card communicates the situation but does not act.

**Why:** The PR #809 thread showed that "kill the stream after N seconds" is the wrong default. Stall indication is informational; cancellation is a deliberate user choice that requires a trustworthy server-side health probe.

**Alternatives considered:**

- Show Skip from day one without a health probe — rejected (re-creates the false-positive cancellation risk from PR #809).
- Auto-cancel after stall — rejected (same reason).

### Decision 4: Queued messages render as ghosted bubbles, not as a banner

Each `pendingQueue` entry renders as a "ghosted" version of the message bubble in the conversation list, positioned after the most recent committed message but before the streaming placeholder. Visual treatment: muted text colour, a "Queued" badge in the bottom-right, no avatar.

Long-press (mobile) / right-click (desktop) opens an `AdaptiveActionSheet` with a "Remove from queue" action.

**Why:** Matches the conversation's existing message-list spatial metaphor. The user sees their input in the same conceptual place as their committed messages, just visually distinguished. A separate banner would be cheaper to build but disconnects the queued text from the conversation flow.

**Alternatives considered:**

- A "Queue: 2 messages" pill on the composer with a tap-to-expand list. Rejected: doesn't show the message text inline, requires an extra interaction.
- Inline ghost bubbles with no removal affordance. Rejected: removing typos / mistakes from the queue is a clear user need that comes up the moment queues exist.

## Risks / Trade-offs

- **More UI surface, more chance of inconsistency between web and macOS.** → Mitigation: widget tests cover every status state on both platforms; Playwright golden re-baseline catches accidental regressions.
- **The 30s stall threshold may be wrong.** → Mitigation: surface telemetry on how often the stall state is hit and how long users wait at it; tune from real data.
- **Queued-message ghost bubbles may be confused with failed messages.** → Mitigation: distinct visual treatment (the "Queued" badge differs from the existing "Retry" affordance for failed messages); manual visual review before merge.
- **Ticker overhead.** A 1-second `Ticker` updating the elapsed-seconds label costs a frame per second while a turn is in-flight. → Mitigation: dispose the ticker on `done`/`agent_error`; on a typical turn (<10 s) the overhead is negligible.

## Migration Plan

1. **Phase A** — derived state: add `currentTurnStatus` to `ChatNotifier` + unit tests. No UI changes. Zero user-visible impact.
2. **Phase B** — in-flight card: build `TurnProgressCard` widget + widget tests for every state. Integrate above the composer in `chat_screen.dart`. Re-baseline Playwright.
3. **Phase C** — queued bubbles: build `QueuedMessageBubble` widget + widget tests. Integrate. Re-baseline Playwright.
4. **Phase D** — reconnect banner: surface `attemptReconnect` activity via a brief banner. Re-baseline Playwright.

Each phase is its own PR. Phases A and B are the meat; C and D can ship in parallel with each other once A is in.

## Open Questions

- Should the elapsed-time label show seconds (0:34) or be more relaxed ("about 30 seconds")? Probably seconds for power users, relaxed for everyone else — but doing both is over-engineered. Pick seconds, revisit.
- Does the stall threshold need to be different on mobile vs desktop? Probably not — the human waiting is the same human regardless of device.
- Are there events we should treat as "still progressing" but currently render nothing for (e.g. `audio_ready` halfway through a long voice synthesis)? Worth a small audit during Phase A.
