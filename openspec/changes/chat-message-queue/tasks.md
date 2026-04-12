## 1. State Model

- [ ] 1.1 Add `MessageStatus` enum (`sending`, `ok`, `failed`) to `chat_provider.dart`
- [ ] 1.2 Add `status` field to `ChatMessage` (default `ok`; user messages start as `sending`)
- [ ] 1.3 Add `pendingQueue: List<String>` to `ChatState`
- [ ] 1.4 Update `ChatState.copyWith` to handle `pendingQueue` and message status updates

## 2. Queue and Drain Logic

- [ ] 2.1 Rename `sendMessage` to an enqueue entry point: append to `pendingQueue` in state and kick off `_drainQueue()` if not already draining
- [ ] 2.2 Add `bool _draining` guard field to `ChatNotifier`
- [ ] 2.3 Implement `_drainQueue()`: loop while `pendingQueue` is non-empty, pop front, stream response, update message status on `DoneEvent`/error, then continue
- [ ] 2.4 Wrap drain loop body in try/finally to always reset `_draining = false` on exit
- [ ] 2.5 On `DoneEvent`: mark the corresponding user message `status = ok`
- [ ] 2.6 On `ErrorEvent` / catch: mark the user message `status = failed` (keep it in list); do NOT clear it

## 3. Stop / Cancel Behaviour

- [ ] 3.1 Update `cancelStream()` to cancel only the current in-flight request; leave `pendingQueue` intact
- [ ] 3.2 After cancel, resume draining by calling `_drainQueue()` if queue is non-empty

## 4. UI — Input Row

- [ ] 4.1 Remove `enabled: !isSending` from the `TextField` in `_InputRow`
- [ ] 4.2 Keep the stop/send button toggle based on `isSending` (stop cancels current stream; send always enqueues)
- [ ] 4.3 Add a queue depth badge/label above or next to the input when `pendingQueue.length > 0`

## 5. UI — Message Bubble

- [ ] 5.1 Pass `onRetry` callback to `_MessageBubble`
- [ ] 5.2 Render a small Retry action (e.g., `TextButton` or icon+label) below the bubble when `message.status == failed`
- [ ] 5.3 Apply a visual failed indicator on the bubble (e.g., subtle red border or error icon) when `status == failed`
- [ ] 5.4 Wire Retry to call `chatProvider.notifier.retryMessage(message)` which clears failed status and re-enqueues the text

## 6. Notifier — Retry Entry Point

- [ ] 6.1 Add `retryMessage(ChatMessage msg)` to `ChatNotifier`: set message `status = sending`, call `sendMessage(msg.content)`

## 7. Tests

- [ ] 7.1 Unit test: submitting a second message while `isSending` adds it to `pendingQueue`
- [ ] 7.2 Unit test: queue drains in FIFO order after `DoneEvent`
- [ ] 7.3 Unit test: `ErrorEvent` leaves user message in list with `status == failed`
- [ ] 7.4 Unit test: `retryMessage` re-enqueues and clears failed status
- [ ] 7.5 Widget test: input field is enabled while `isSending == true`
- [ ] 7.6 Widget test: Retry button appears on a failed message bubble
