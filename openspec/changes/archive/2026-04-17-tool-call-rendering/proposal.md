## Why

Tool calls are invisible in the chat UI: status messages disappear the moment tokens start flowing, and completed tool results leave no trace in the message history. Users have no way to understand what the assistant actually did to produce a response.

## What Changes

- Add a `ToolCallChip` widget that renders inline inside the assistant message bubble, showing tool name and status (in-progress, success, error, denied)
- Extend `ChatMessage` to carry a list of tool call records (name + status) so they persist after streaming ends
- Update `ChatState` to accumulate tool results per-message during streaming rather than overwriting a single `lastToolResult`
- Show the in-progress chip immediately on `StatusEvent` (even after tokens have started flowing), replacing it with a result chip on `ToolResultEvent`
- Render a visual divider between the tool call chips and the assistant's reply text inside the bubble

## Capabilities

### New Capabilities

- `tool-call-display`: Inline rendering of tool call chips (in-progress, success, error, denied) inside assistant message bubbles, persisted after streaming

### Modified Capabilities

- `chat-message-retry`: `ChatMessage` data model gains a `toolCalls` field — retry logic must preserve this field on copy

## Impact

- `app/lib/features/chat/chat_provider.dart` — `ChatMessage`, `ChatState`, `_streamMessage`, `_streamVoiceMessage`
- `app/lib/features/chat/chat_screen.dart` — `_MessageBubble.build`
- New widget: `app/lib/features/chat/tool_call_chip.dart`
- No API or backend changes required (events already emitted)
- No breaking changes to existing stored messages (field defaults to empty list)
