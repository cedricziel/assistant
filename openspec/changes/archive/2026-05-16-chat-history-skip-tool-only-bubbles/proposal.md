## Why

When the assistant invokes tools across multiple ReAct iterations, each iteration is persisted as a separate `assistant` message row with `content = ""` and a single entry in `tool_calls` — this is mandated by the OpenAI-style wire format so the following `tool` role row can reference it via `tool_call_id`. On the deployed instance (`schorschvm`), 18 of the 20 most recent assistant messages in a real conversation match this shape.

Today the Flutter history loader splits each persisted row into a tool-call timeline chip **and** an underlying message bubble. For tool-call-only turns, the bubble is rendered with empty content, producing a stack of small grey rounded pills sitting beside their chips. The chips already fully represent each ReAct step, so the bubble is redundant noise that visually fragments the timeline.

## What Changes

- The conversation history → chat timeline mapper SHALL omit the message bubble for an assistant row whose `content` is empty, whose `tool_calls` list is non-empty, and which carries no attachments. The tool-call chip(s) already represent the turn.
- Assistant rows that carry user-visible content (text or attachments) continue to render as a bubble even when they also have tool calls (final-answer turns that interleave a chip with a reply).
- User rows are unconditionally rendered as bubbles, including empty user rows (defensive — should not occur in practice).
- The mapping logic is extracted into a pure, file-scope function (`chatMessagesFromHistory`) so its contract can be exercised by unit tests without mocking the API client.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `tool-call-display`: add a requirement covering history reconstruction — tool-call-only assistant rows must surface as chips only, never as redundant empty bubbles.

## Impact

- **Code**: `app/lib/features/chat/chat_provider.dart` (extract & gate the bubble emission), `app/test/unit/chat/chat_history_mapping_test.dart` (new tests).
- **APIs / data**: none. Wire format and persistence are unchanged; this is purely a client-side rendering rule.
- **Compatibility**: no breaking change. Existing tool-call chip rendering is preserved verbatim.

## Non-goals

- Changing how the orchestrator persists assistant messages or how it threads `tool_call_id` linkage.
- Reworking the live-streaming render path. The streaming side already routes the final `DoneEvent.content` into a single placeholder bubble; only the history-replay path produces the empty pills.
- Surfacing `role == 'tool'` result rows differently — they are out of scope here.
- Reordering chips or merging adjacent tool calls into a single grouped chip.

## User-facing documentation

Not required. The change is a defect fix in the chat timeline; no user-visible feature is added or removed, and no operator setting is introduced.
