## 1. Data Model

- [x] 1.1 Add `ToolCallStatus` enum (`pending`, `ok`, `error`, `denied`) to `chat_provider.dart`
- [x] 1.2 Add `ToolCallRecord` class (`toolName: String`, `status: ToolCallStatus`) to `chat_provider.dart`
- [x] 1.3 Add `toolCalls: List<ToolCallRecord>` field to `ChatMessage` (defaults to `const []`)
- [x] 1.4 Update `ChatMessage.copyWith` to include `toolCalls`

## 2. Streaming Accumulation

- [x] 2.1 On `StatusEvent` in `_streamMessage`: parse the tool name from the status string and push a `ToolCallRecord(status: pending)` onto the streaming message's `toolCalls` list
- [x] 2.2 On `ToolResultEvent` in `_streamMessage`: find the matching pending record by tool name, update its status to `ok`/`error`/`denied`; append a new record if no match found
- [x] 2.3 Mirror 2.1 and 2.2 in `_streamVoiceMessage` (currently has no `ToolResultEvent` handling)
- [x] 2.4 On `DoneEvent`: ensure `toolCalls` from the streaming placeholder are copied onto the final `ChatMessage`

## 3. ToolCallChip Widget

- [x] 3.1 Create `app/lib/features/chat/tool_call_chip.dart` with a `ToolCallChip` stateless widget
- [x] 3.2 Render a spinner for `pending`, checkmark icon (green) for `ok`, error icon (red) for `error`, block icon (amber) for `denied`
- [x] 3.3 Show the tool name as a label next to the icon
- [x] 3.4 Write widget tests covering all four status variants

## 4. Message Bubble Integration

- [x] 4.1 In `_MessageBubble.build`, render a `Wrap` of `ToolCallChip` widgets from `message.toolCalls` above the `MarkdownBody`
- [x] 4.2 Add a `Divider` between the chips `Wrap` and the `MarkdownBody` when both are non-empty
- [x] 4.3 Remove the now-superseded centered status indicator (`chatState.isSending && chatState.streamingContent.isEmpty` block) from the message list area — the chip handles in-progress state inline

## 5. Cleanup & Tests

- [x] 5.1 `lastToolResult` retained in `ChatState` for notifications (mutation-in-place prevents listener comparison; `lastToolResult` is the reliable notification signal)
- [x] 5.2 Add unit tests for `StatusEvent` → pending chip and `ToolResultEvent` → resolved chip accumulation logic in the provider
- [x] 5.3 Run `flutter analyze` with zero issues
- [x] 5.4 Run `flutter test` with all tests green
