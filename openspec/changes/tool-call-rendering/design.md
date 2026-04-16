## Context

The SSE stream already emits `StatusEvent` ("Calling tool: X") and `ToolResultEvent` (tool name + ok/error/denied). The provider discards both after use — status is cleared on the next token, and tool results only trigger local notifications. `ChatMessage` has no field to carry tool call history. The `_MessageBubble` widget has no code path for rendering tool metadata.

Current condition for the status indicator:

```dart
if (chatState.isSending && chatState.streamingContent.isEmpty)
```

This means any tool call that fires after the first token is fully invisible.

## Goals / Non-Goals

**Goals:**

- Show in-progress chip inside the assistant bubble as soon as `StatusEvent` fires, regardless of whether tokens have already started
- Replace that chip with a result chip (✓ / ✗ / ⊘) when `ToolResultEvent` arrives
- Persist the full list of tool calls on `ChatMessage` so they survive after streaming ends
- Render a divider between chips and the reply text when both are present

**Non-Goals:**

- Showing tool input/output payloads (only name + status)
- Collapsible/expandable chip groups
- Separate message rows for tool calls
- Any backend or OpenAPI changes

## Decisions

### D1: Tool call records live on `ChatMessage`, not `ChatState`

**Choice:** Add `List<ToolCallRecord>` to `ChatMessage`.

**Why:** Tool calls belong to the message they contributed to. Keeping them on `ChatState` (as `lastToolResult` does today) means they're lost when the state transitions. Attaching them to the message makes history work for free — loaded messages from the server can carry the list, and retry preserves it via `copyWith`.

**Alternative considered:** Keep tool calls in `ChatState` and render them as a separate overlay. Rejected — makes history display impossible without another API round-trip.

### D2: Accumulate per-message during streaming via mutation, not immutable copy

**Choice:** During streaming, mutate the `ChatMessage` object in place for the tool calls list (same pattern used today for `content` and `isStreaming`).

**Why:** `ChatMessage.content` is already a mutable `String` field updated directly during streaming. Extending the same pattern to `toolCalls` keeps the streaming loop simple and avoids rebuilding the full message list on every `ToolResultEvent`.

### D3: Single `ToolCallChip` widget with a status enum

**Choice:** One widget, `ToolCallChip({required String toolName, required ToolCallStatus status})` where `ToolCallStatus` is `{pending, ok, error, denied}`.

**Why:** Centralises icon, colour, and label logic. Easy to test in isolation. The `_MessageBubble` just maps `message.toolCalls` to a `Wrap` of chips.

**Alternative considered:** Separate widgets per status. Rejected — duplicates layout logic.

### D4: Divider between chips and reply text

**Choice:** Render a thin `Divider` inside the assistant `Column` when both `toolCalls` is non-empty and `content` is non-empty.

**Why:** Clear visual boundary between "what the assistant did" and "what it said". Matches the ASCII art design reference.

### D5: In-progress chip uses a pending `ToolCallRecord` inserted on `StatusEvent`

**Choice:** On `StatusEvent`, push a `ToolCallRecord(toolName: event.message, status: pending)` onto the streaming message's `toolCalls` list. On `ToolResultEvent`, find the matching record by tool name and update its status.

**Why:** Avoids a separate state field for the "current in-progress tool". The chip list is the single source of truth throughout the stream.

**Risk:** `StatusEvent` carries a human-readable string ("Calling tool: web-search"), not a structured tool name. Matching against `ToolResultEvent.toolName` requires stripping the prefix. Keep a regex/split to extract the tool name; fall back to showing the full string if parsing fails.

## Risks / Trade-offs

- **Stored message history**: Messages persisted in SQLite before this change have no `toolCalls` field. When loaded they'll default to an empty list — correct behaviour, chips simply won't appear for old messages.
- **StatusEvent parsing fragility**: If the server changes the "Calling tool: X" format, the name-extraction regex breaks and the pending chip shows the full string. Mitigation: defensive fallback; a server-side structured event would be cleaner but is out of scope.
- **Voice streaming path**: `_streamVoiceMessage` duplicates event handling. `ToolResultEvent` is not handled there today — must add the same accumulation logic to keep both paths consistent.

## Migration Plan

No migration needed. The `toolCalls` field defaults to `[]`. No DB schema changes. Deploy is a standard app release.

## Open Questions

- Should chips also appear on user messages that triggered tool calls? (Current answer: no — chips live only on assistant messages.)
- Should the server eventually send a structured `tool_call_start` event instead of a text `StatusEvent`? Out of scope for this change, tracked separately.
