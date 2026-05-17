## Why

The user-facing symptom that motivated PR #809 was "my messages went into a queue and never got out, with no progress indicator anywhere". That's a UX problem, not a cancellation problem: the server was working (or had stopped), but the client surfaced nothing. The user's only recourse was to background and foreground the app, which killed the network connection as a side effect.

The chat-streaming protocol already carries plenty of progress signal — `run_started`, `status`, `thinking`, `tool_result`, `skill_complete`, `agent_error`, `subagent_started`, `subagent_completed`, `audio_ready`, and `done` are all emitted on `POST /api/conversations/{id}/messages`. The client renders some of these (token output, tool calls in the timeline) but not in a way that lets a user see "we know the server is still working, here's what it's doing".

There are also no affordances for the queue itself. When a user types a follow-up while a stream is in-flight, the second message enters `pendingQueue` silently — no visual indication that it was received, no way to see what's ahead of it, no way to cancel it.

This change closes both gaps: a persistent in-flight progress card for the active turn (showing the most recent activity event), and visible queue affordances for pending messages.

## What Changes

- **In-flight progress indicator.** Render a compact card above the message composer (or pinned in the message list) showing the active turn's state: "Calling tool X…" / "Thinking…" / "Generating response…" / "Waiting on server…" — driven by the most recent SSE event for the turn. The card persists for the duration of the turn and disappears on `done` / `agent_error`. Updates in real time as new events arrive.
- **Stall indicator.** If no SSE event has arrived in the last N seconds (initial proposal: 30s), the card surfaces a "Server is taking longer than expected" state with the elapsed time. This is purely informational — no automatic cancellation. The card optionally exposes a "Skip" button when N is exceeded (cancels the current turn explicitly and advances the queue — gated by future health-check work, see `turn-status-endpoint` change).
- **Queue visibility.** Render queued (`pendingQueue`) messages in the conversation list above the composer as "ghosted" placeholder bubbles in send-order. Each shows the message text and a "Queued" badge. Users can long-press (mobile) / right-click (desktop) to remove a queued message before it's sent.
- **Replay/reconnect visibility.** Surface the existing replay-on-resume behaviour (`attemptReconnect`) as user-visible state: a brief "Reconnecting…" banner when the app resumes and an interrupted stream is being recovered. Already implemented internally; just needs UI.
- **Telemetry.** Log every state transition (turn started, stalled, recovered, skipped, completed) via the existing trace pipeline so we can measure how often users hit each state in practice. Informs the cancellation-policy decision in the follow-up `turn-status-endpoint` change.

## Capabilities

### New Capabilities

- `chat-stream-progress-ux`: A persistent visual indicator of the active turn's progress, sourced from SSE events, plus a stall indicator that surfaces server-side silence > N seconds. Covers in-flight card rendering, stall thresholds, queue affordances, and reconnect banners.

### Modified Capabilities

- `chat-message-queue`: Add requirement that queued (`pendingQueue`) messages SHALL be rendered as visible ghosted bubbles in send-order, and that the user SHALL be able to remove a queued message before it sends. Today the queue is invisible to the user.

## Impact

- **Code**:
  - New widget: `app/lib/features/chat/turn_progress_card.dart` (renders in-flight state).
  - New widget: `app/lib/features/chat/queued_message_bubble.dart`.
  - `chat_screen.dart` integrates both above the composer (estimated ~50 LOC).
  - `chat_provider.dart` exposes a `currentTurnStatus` derived state (last event, elapsed since last event, turn ID).
  - Reconnect banner already has a partial implementation via `update_banner.dart`-style overlay; extend or copy the pattern.
- **Tests**:
  - Widget tests for the new components, including all status states (starting, tool-running, thinking, generating, stalled, reconnecting, done, errored).
  - Provider tests for the new `currentTurnStatus` derived state.
  - Visual regression update for the chat screen (Playwright baseline).
- **Telemetry**: Hooks into the existing OpenTelemetry SQLite exporter — new event kinds for turn state transitions.
- **No backend changes.** Uses the existing SSE event protocol entirely.
- **Dependency**: Best deployed after `sse-keepalive` lands. Without keep-alive, the stall indicator fires false positives (silence on a healthy slow stream).
- **Out of scope**:
  - Automatic stream cancellation policy (the "Skip" button's destructive path) is gated by the `turn-status-endpoint` change, which adds an authoritative server-side health probe before the client decides to cancel.
  - The original PR #809 fixes (2) (`isSending` guard in recovery) and (3) (`attemptReconnect` drains queue when `!_needsReconnect`) are correct independent fixes; they should ship as a small standalone PR alongside this work.
