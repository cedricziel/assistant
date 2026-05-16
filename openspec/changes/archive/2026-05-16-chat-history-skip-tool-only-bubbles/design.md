## Context

The Flutter chat timeline blends two render paths over the same `ChatMessage` model:

1. **Streaming path** (`ChatNotifier._streamMessage` and friends). A single `assistant-streaming` placeholder bubble is appended on send; `StatusEvent`s push tool-call timeline entries; `TokenEvent`s flow into the placeholder; `DoneEvent.content` becomes the final answer. Empty bubbles never arise because the placeholder is collapsed once `DoneEvent` arrives — if there is no final-answer text, it stays empty for the duration of the stream and is harmlessly hidden by the streaming dots indicator.

2. **History-replay path** (`ChatNotifier.loadConversation`). Reads `GET /api/conversations/{id}` and walks `detail.messages`. For each persisted row the loader:
   - emits one `TimelineEntryType.toolCall` chip per entry in `m.tool_calls`,
   - then emits a `TimelineEntryType.message` bubble using `m.content`.

Persisted assistant rows for ReAct tool-only steps have `content == ""` by protocol: each tool invocation is its own assistant message so the next `role == 'tool'` row can reference it via `tool_call_id`. Data sample from `schorschvm` confirms this is the dominant shape: in one inspected conversation, turns 1–6 are all `assistant(content="", tool_calls=[X])` followed by `tool(content="<result>")`, with turn 7 carrying the only natural-language reply.

In the existing rendering rules, every `TimelineEntryType.message` row paints the rounded `surfaceContainerHighest` container unconditionally — the inner markdown is gated on `content.isNotEmpty`, but the bubble shell is not. The result is one redundant grey pill per tool-only ReAct step.

## Goals / Non-Goals

**Goals**

- Stop emitting empty `TimelineEntryType.message` rows from the history-replay path when the underlying persisted row is an assistant tool-only step.
- Keep the mapping pure (no state, no I/O) so the contract is unit-testable without mocking the API client.
- Preserve every other case verbatim: chips, user rows, tool result rows (`role == 'tool'`), and assistant rows that carry text or attachments.

**Non-Goals**

- Changing what the server persists or how the orchestrator constructs the assistant messages. The wire format remains canonical OpenAI tool-calling shape.
- Reworking the streaming path. It already collapses correctly on `DoneEvent`.
- Filtering or restyling `role == 'tool'` rows — those carry user-meaningful content (the tool result) and have their own rendering concerns.
- Introducing a new spec capability. This is an additive requirement on the existing `tool-call-display` capability.

## Decisions

### 1. Extract the mapping into a top-level pure function

`loadConversation` mixed Riverpod state mutation, API I/O, and message reshaping. The reshaping is the only piece that needs new behaviour and the only piece that benefits from a unit test. Extracting it to a top-level `List<ChatMessage> chatMessagesFromHistory(Iterable<MessageSummary>)` keeps the patch surface minimal and the test fast.

**Alternative considered:** `@visibleForTesting static` method on `ChatNotifier`. Rejected because callers would need to instantiate the notifier (which requires a `Ref`) — the function has no notifier state to access, so a top-level function is more honest about its inputs.

### 2. Gate the bubble emission on `role + content + tool_calls + attachments`

The skip predicate is intentionally narrow:

```
role == 'assistant'
  && content.isEmpty
  && toolCalls.isNotEmpty
  && (attachments == null || attachments.isEmpty)
```

Each clause defends a real shape:

- `role == 'assistant'` — never silently drop a user or tool row; user rows in particular are user-authored and must always be visible.
- `content.isEmpty` — if the model produced any reply text we render it.
- `toolCalls.isNotEmpty` — if there are no chips to stand in for the row, dropping it would erase the turn entirely. (An empty-content assistant row with no tool calls shouldn't occur in practice, but if it does we keep it visible as a debugging signal.)
- `attachments` empty — assistant rows can carry image attachments produced by the model. Their bubble must remain to host the thumbnails.

**Alternative considered:** Skip purely on `content.isEmpty && role == 'assistant'`. Rejected because it would suppress empty-but-attachment-bearing rows.

### 3. Decide at mapping time, not render time

The bubble shell could equally be suppressed in `_MessageBubble.build()` by returning `SizedBox.shrink()` for an empty assistant message with no attachments. The mapping-time fix is preferred because:

- It keeps the local `messages` list honest — what is in the list is what gets rendered. Adjacent-message logic in `chat_screen.dart` (the `prevMsg.role == msg.role` grouping check) does not need to learn to skip invisible siblings.
- The bubble widget remains simple; a future maintainer reading `_MessageBubble` doesn't need to know that some `ChatMessage`s render as nothing.
- Render-time skipping leaves a `ChatMessage` in the list whose only purpose is to be invisible — a footgun for any future iteration that builds on the list (counting, scrolling, retry actions).

## Risks / Trade-offs

- **Risk:** A future server change starts persisting empty-content assistant rows with semantically meaningful state (e.g. just attachments, no text, no tool calls). → **Mitigation:** the predicate requires `toolCalls.isNotEmpty`, so attachments-only and pure-empty assistant rows are still rendered. The skip is opt-in to a specific protocol shape.
- **Risk:** The pure function diverges from the streaming render path. → **Mitigation:** the streaming path already converges on the same `ChatMessage` shape (it constructs `TimelineEntryType.toolCall` entries via `_onStatusEvent`); the new function preserves the same `id` convention (`'toolcall-${tc.name}-${m.id}'`) and `ToolCallRecord` mapping it had before extraction, so the only behavioural delta is the skip.
- **Trade-off:** A consumer that wanted to count "assistant turns" by walking the timeline list will now under-count tool-only iterations. There are no such consumers today, and counting ReAct iterations by `TimelineEntryType.toolCall` is more meaningful anyway.

## Migration Plan

No data migration. The change is a pure client-side rendering rule. Old persisted conversations and new conversations render identically under the new rule. Rollback is a code revert with no follow-up state changes.
