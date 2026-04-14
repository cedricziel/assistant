## Why

The chat input is disabled while the assistant is responding, forcing users to wait before they can type or queue their next thought. Additionally, when a message fails to send, there is no way to retry it — users must retype the message manually.

## What Changes

- The message input field and send button remain enabled while a response is streaming.
- New user messages submitted while a response is in-progress are added to a **local pending queue** and sent automatically once the current response completes.
- A queued-message count indicator is shown when there are pending messages.
- Failed user messages (network error, server error) display an inline **Retry** action on the message bubble.
- Retrying a failed message re-sends it through the same send/queue path.

## Capabilities

### New Capabilities

- `chat-message-queue`: Client-side pending queue that holds user messages submitted while a response is in-flight, draining them sequentially after each response completes.
- `chat-message-retry`: Per-message failure state and inline retry affordance for user messages that could not be delivered.

### Modified Capabilities

<!-- No existing spec-level behavior changes; this is a pure additive feature. -->

## Impact

- `app/lib/features/chat/chat_provider.dart` — `ChatState` gains a `pendingQueue` list and per-message `status` field; `ChatNotifier.sendMessage` gains queue-drain logic.
- `app/lib/features/chat/chat_screen.dart` — `_InputRow` removes the `enabled: !isSending` guard; `_MessageBubble` renders a retry button for failed user messages.
- No backend or API changes required.
