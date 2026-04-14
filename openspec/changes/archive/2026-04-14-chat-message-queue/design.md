## Context

The Flutter chat UI (`app/lib/features/chat/`) uses a single `ChatNotifier` backed by a Riverpod `AsyncNotifier`. `sendMessage` is an `async` method that streams SSE events and awaits completion before returning. The input field is gated by `enabled: !isSending`, so the user cannot type while a response is in-flight.

There is no concept of message status — a user message is added to the list optimistically but carries no indication of failure. When an error occurs the streaming placeholder is removed and a banner error is shown, but the failed text is gone.

## Goals / Non-Goals

**Goals:**

- Users can type and submit a new message at any point, even while the assistant is responding.
- Queued messages drain automatically, one at a time, after each response finishes.
- Failed user messages persist in the message list and show a Retry button.
- Retry re-enqueues the original text through the same send path.

**Non-Goals:**

- Parallel/concurrent requests (queue is strictly sequential, one in-flight at a time).
- Persistent queue across app restarts (in-memory only).
- Server-side changes — no API modifications.
- Edit of an in-flight or failed message (type a corrected version and send fresh).

## Decisions

### D1 — Queue lives in `ChatState`, not a separate provider

Alternatives considered:

- **Separate `QueueNotifier`**: cleaner separation but requires cross-provider coordination; ordering bugs become possible when two providers update state independently.
- **Queue inside `ChatNotifier` as a plain `List`** (non-observable): simpler, but the UI can't react to queue depth without polling.

**Decision**: Add `pendingQueue: List<String>` directly to `ChatState`. This keeps all observable chat state co-located, the UI can derive queue depth trivially, and `ChatNotifier` controls enqueue/drain entirely.

### D2 — Per-message failure state via a status enum on `ChatMessage`

Alternatives considered:

- **Separate `failedMessages: Set<String>`** in `ChatState`: introduces two sources of truth (message list + failed set).
- **Keep error only in banner**: current behavior — text is lost after failure, no retry.

**Decision**: Add `MessageStatus { sending, ok, failed }` to `ChatMessage`. User messages start as `sending` while in-flight and transition to `ok` on `DoneEvent` or `failed` on error. The `_MessageBubble` renders a Retry chip only when `status == failed`.

### D3 — `sendMessage` becomes fire-and-forget enqueue; a private `_drainQueue` loop owns streaming

Current flow: `_InputRow` calls `await chatProvider.notifier.sendMessage(text)`, which blocks until the stream completes.

New flow:

1. `sendMessage(text)` — appends `text` to `pendingQueue` in state, then calls `_drainQueue()` if not already draining.
2. `_drainQueue()` — loops while `pendingQueue` is non-empty: pops the first item, streams it, then continues. A `bool _draining` guard (non-observable, internal to notifier) prevents re-entrant draining.

This means the UI send button is always active; the input is never disabled by `isSending`.

### D4 — Input field always enabled; send button always active

Remove `enabled: !isSending` from `TextField` and the conditional `stop` button replaces `send` only when streaming. Users can submit while streaming; the message queues silently.

The stop button (`Icons.stop_rounded`) still cancels the current stream. On cancel the streaming placeholder is removed; queued messages are **not** discarded (user explicitly queued them).

## Risks / Trade-offs

- **Rapid queue buildup** → the user sends many messages before any response arrives; each gets its own API round-trip when drained. Mitigation: show queue depth badge; future enhancement could collapse duplicates.
- **Retry of a stale message** → user retries a message from a conversation that has since been switched. Mitigation: retry uses the `conversationId` stored at the time the message was originally sent (capture at enqueue time, not drain time).
- **`_draining` flag desync** → if an unhandled exception escapes `_drainQueue`, the flag stays `true` and no future messages drain. Mitigation: wrap the drain loop body in try/finally to always reset the flag.

## Migration Plan

Purely additive Flutter-side change. No migration needed — state is in-memory and resets on hot restart.

## Open Questions

- Should cancelling a stream also clear the pending queue, or only the current in-flight message? (Current proposal: queue is preserved.)
- Should there be a maximum queue depth (e.g. 5 messages) with a UI warning? (Deferred to a follow-up.)
