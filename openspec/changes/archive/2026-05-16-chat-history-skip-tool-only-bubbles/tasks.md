# Tasks: Chat History — Skip Tool-Only Bubbles

This change captures work that has already landed on `worktree-snazzy-dazzling-nest`. Task boxes reflect implementation state at proposal time.

## 1. Extract the history → ChatMessage mapping into a pure function

- [x] 1.1 Add a file-scope `chatMessagesFromHistory(Iterable<MessageSummary>)` function in `app/lib/features/chat/chat_provider.dart` that returns the flat `List<ChatMessage>` the chat screen consumes.
- [x] 1.2 Move the tool-call chip emission (synthetic `id: 'toolcall-${tc.name}-${m.id}'`, `TimelineEntryType.toolCall`, mapped `ToolCallRecord`) into the new function unchanged.
- [x] 1.3 Move the message-bubble emission into the new function, preserving `id`, `role`, `content`, `ttsAvailable`, and attachment mapping.
- [x] 1.4 Extract the `_parseToolStatus` switch into a top-level `_parseToolStatusString` used by both the streaming path and the new helper. Remove the now-unused private wrapper.
- [x] 1.5 Replace the inline loop in `ChatNotifier.loadConversation` with a single call to `chatMessagesFromHistory(detail.messages)`.

## 2. Gate the empty bubble emission

- [x] 2.1 Inside `chatMessagesFromHistory`, compute `isAssistantToolOnly = role == 'assistant' && content.isEmpty && toolCalls.isNotEmpty && attachments empty`.
- [x] 2.2 When `isAssistantToolOnly` is true, `continue` past the bubble emission while still emitting the chips.
- [x] 2.3 Document the OpenAI-protocol rationale in the function's doc comment so the next maintainer can match the predicate to the wire format.

## 3. Test coverage

- [x] 3.1 Add `app/test/unit/chat/chat_history_mapping_test.dart` with a fixture matching the schorschvm shape (user → 3 × `assistant(content="", tool_calls=[X])` → final `assistant(content="Done.")`).
- [x] 3.2 Assert that tool-only assistant rows produce three `TimelineEntryType.toolCall` entries and no corresponding `TimelineEntryType.message` entries.
- [x] 3.3 Assert that a mixed assistant row (non-empty content + tool calls) produces both a chip and a bubble.
- [x] 3.4 Assert user rows are always preserved (including the defensive empty-user-content case).
- [x] 3.5 Assert chip `toolName`, `status`, and `result` round-trip correctly from `ToolCallSummary` → `ToolCallRecord`.

## 4. Validate

- [x] 4.1 `flutter analyze` — zero issues.
- [x] 4.2 `flutter test` — full suite green (811 tests).
- [x] 4.3 Verify against the deployed instance (`schorschvm`) that the persisted shape matches the fixture (one assistant row per ReAct iteration with `content=""` and `tool_calls=[…]`).
