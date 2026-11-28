## Why

Stop does not stop anything. The input-row `stop_button` and the bubble's Stop action call `ChatNotifier.cancelStream()` (`chat_screen.dart:541-543`, `:595-596`), which is client-only (`chat_provider.dart:274-287`). The server turn keeps calling tools, spends tokens and persists its full reply, which appears on reload. The server cancel (`turns.rs:215-244`) is reached only by the stall "Skip" button (`turn_progress_card.dart:123-136`).

A local-only stop also stalls the queue, discards the partial text, never persists a cancelled reply (so a reload shows no trace of the stop), loses cancels that land before a worker claims the turn, and cannot reach voice turns. design.md lists each defect with file references.

## What Changes

- **Runtime.** A cancel before a worker claims the turn is honoured: the worker skips the request, and the error says the turn never started.
- **Storage.** `messages` gains a nullable `finish_reason` column (new space.db migration `message_finish_reason`). Stopped replies are stored with `finish_reason = "cancelled"`, and `MessageSummary` exposes it.
- **Web-ui.** A turn cancelled before it started ends its stream at once, and its `agent_error` frame carries `user_message_persisted: false`. Cancel checks that the run belongs to the conversation in the path. A cancelled turn persists its streamed text as its assistant message, or the short placeholder "_Stopped._" when nothing streamed, and the frame carries `message_id`. A stopped first turn still gets an auto-title. Voice turns move onto the shared turn runner, which gives them a run id, `X-Run-Id`, replay and cancel.
- **Client.** One `ChatNotifier.stopTurn()` replaces `cancelStream()` (removed) and the public `requestCancelTurn()` (made private). All three Stop affordances call it. It asks the server to cancel and keeps listening until the terminal frame, which keeps the partial reply as a **Stopped** message. After a reload the same reply shows **Stopped**, read from `finish_reason`. Every wait is bounded, and failures end in a local detach with an honest message. The queue drains only after the stopped turn has reconciled.
- `AgentErrorEvent` is parsed into structured fields. New statuses: `MessageStatus.stopped` and `ToolCallStatus.cancelled`.

No new SSE event type or endpoint. One additive, nullable column and its API field.

## Capabilities

### New Capabilities

- `chat-turn-stop`: what the client's Stop does, including the stopping state, reconciliation, the partial reply, the "Stopped" state after reload and the edge cases.

### Modified Capabilities

- `turn-status-api`: cancel covers turns not yet claimed, checks that the run belongs to the conversation, and persists the partial reply (or a "_Stopped._" placeholder) with `finish_reason`, and titles a stopped first turn. Voice joins the run lifecycle.
- `chat-message-queue`: the queue drains after the stopped turn reconciles, not immediately.

## Impact

- **Crates:** `assistant-core` (`Message`, `MessageFinishReason`), `assistant-storage` (migration, both conversation stores), `assistant-runtime` (`orchestrator/worker.rs`, `orchestrator/mod.rs`, `history.rs`), `assistant-web-ui` (`api/messages.rs`, `api/turns.rs`, `api/conversations.rs`).
- **Database:** space.db migration `message_finish_reason` (`ALTER TABLE messages ADD COLUMN finish_reason TEXT`). The stack plan assigns its number. It shares the `Message` struct lane with `message-branching` PR 1, `tool-result-renderers` S3 and `message-feedback` PR 3.
- **API:** `agent_error` gains optional `message_id` and `user_message_persisted`. `MessageSummary` gains optional `finish_reason`. The voice response gains `X-Run-Id` and `run_started`. `openapi.json` is updated and the generated `assistant_api` client regenerated.
- **Flutter:** `features/chat/*` and `api/`.
- **Siblings:** builds on `message-branching` PR 1 (message lane), `message-branching` PR 5 (shared turn runner) and `chat-element-library` PR 2; edges in tasks.md.
- **UI changes:** a disabled "Stopping…" Stop button, a "Stopped" label on stopped replies, and a muted icon on cancelled tool calls, and a "_Stopped._" reply when the user stopped before any text. No captured Playwright screen shows these states, so baselines should stay unchanged (verified in tasks.md).
- **Documentation verdict:** yes. `docs/operations/turn-status-endpoint.md` (persisted partial and placeholder, `finish_reason`, new frame fields, cancel before claim, voice runs) and `docs/development/chat-stream-probe.md` (`stopTurn`, Stopping state, grace and detach) are updated.

## Non-goals

- Marking replies that ended any other way (error, length limit) in `finish_reason`. Only `"cancelled"` is written in this change.
- Auto-titling from anything but the user's own text: a turn cancelled before it started saves no message and gets no title.
- Cancelling from messengers, which already have `/stop`.
- Stopping a single tool call or subagent while the turn continues.
- Changing how `agent_error {reason: "error"}` displays.
- Moving widgets into `assistant_ui` (owned by `chat-element-library`).
