## REMOVED Requirements

### Requirement: Tool calls render as inline chips inside the assistant bubble

**Reason**: Out of date. `ToolCallChip` (`app/lib/features/chat/tool_call_chip.dart`) is no longer referenced from `app/lib/` (its tests are its only importer). Tool calls render as standalone timeline entries through `StreamingTimelineEntry` (`streaming_timeline_entry.dart:231`), and this change moves them to `TimelineEntryShell` + `ToolCallBody` in `assistant_ui`. The chip widget and its tests are deleted.

**Migration**: Tool-call rendering is specified by the `chat-element-library` requirement "Tool call body renders through a resolver scope with a generic fallback". Status-icon semantics stay under "Tool call status icons are distinct and accessible" in this capability, reworded below to timeline entries. Two scenario titles keep the word "chips" ("Chips visible after DoneEvent", "Persisted assistant tool-only row produces only chips") because archive matches scenarios by title; their bodies describe timeline entries.

## RENAMED Requirements

- FROM: `### Requirement: Tool-call-only assistant rows render chips without an empty message bubble`
- TO: `### Requirement: Tool-call-only assistant rows render tool call entries without an empty message bubble`

## MODIFIED Requirements

### Requirement: Tool calls persist on the message after streaming ends

Each tool call SHALL be a timeline entry of type `TimelineEntryType.toolCall`, carrying its `ToolCallRecord` on a `ChatMessage`, rendered by `TimelineEntryShell` with a `ToolCallBody`. Tool call entries SHALL remain visible after the stream completes, including when the conversation is scrolled or the screen is rebuilt.

#### Scenario: Chips visible after DoneEvent

- **WHEN** the stream completes with a `DoneEvent`
- **THEN** every tool call timeline entry from that stream SHALL remain in the timeline, in the `complete` entry state, with its final status icon

#### Scenario: Multiple sequential tool calls all shown

- **WHEN** a single assistant response invokes multiple tools in sequence
- **THEN** one tool call timeline entry per tool call SHALL be shown, in invocation order

### Requirement: Tool call status icons are distinct and accessible

Each status SHALL use a distinct icon and colour in the header of the tool call timeline entry, so users can differentiate outcomes without relying on colour alone.

#### Scenario: Pending status renders spinner

- **WHEN** a tool call timeline entry has status `pending`
- **THEN** a circular progress indicator (spinner) SHALL be shown as the status icon

#### Scenario: Ok status renders checkmark

- **WHEN** a tool call timeline entry has status `ok`
- **THEN** a checkmark icon with a success colour (green) SHALL be shown

#### Scenario: Error status renders cross

- **WHEN** a tool call timeline entry has status `error`
- **THEN** an error icon with an error colour (red) SHALL be shown

#### Scenario: Denied status renders prohibition symbol

- **WHEN** a tool call timeline entry has status `denied`
- **THEN** a prohibition icon with a warning colour (amber) SHALL be shown

### Requirement: Tool-call-only assistant rows render tool call entries without an empty message bubble

When reconstructing a conversation from history, the chat timeline SHALL NOT render a message bubble shell for an assistant row whose `content` is empty, whose `tool_calls` list is non-empty, and which carries no attachments. The tool call timeline entries derived from that row already represent the ReAct step fully; an additional empty bubble is redundant and visually fragments the timeline.

#### Scenario: Persisted assistant tool-only row produces only chips

- **WHEN** conversation history contains an `assistant` message with `content == ""`, one or more entries in `tool_calls`, and no attachments
- **THEN** the chat timeline SHALL contain one tool call timeline entry per `tool_calls` entry
- **AND** the chat timeline SHALL contain no `TimelineEntryType.message` entry corresponding to that persisted row

#### Scenario: Mixed turn with content and tool calls keeps the bubble

- **WHEN** conversation history contains an `assistant` message with non-empty `content` and one or more entries in `tool_calls`
- **THEN** the chat timeline SHALL contain one tool call timeline entry per `tool_calls` entry
- **AND** the chat timeline SHALL contain a `TimelineEntryType.message` entry rendering the reply text

#### Scenario: Assistant row with attachments but no content keeps the bubble

- **WHEN** conversation history contains an `assistant` message with `content == ""`, one or more entries in `tool_calls`, and at least one attachment
- **THEN** the chat timeline SHALL contain a `TimelineEntryType.message` entry so the attachment thumbnails are rendered

#### Scenario: User rows are always preserved

- **WHEN** conversation history contains a `user` message, regardless of `content` being empty
- **THEN** the chat timeline SHALL contain a `TimelineEntryType.message` entry for that row
- **AND** the bubble shell SHALL be rendered (the user-authored turn must remain visible)

#### Scenario: Final-answer assistant row after tool-only iterations renders normally

- **WHEN** conversation history contains a sequence of `assistant(content="", tool_calls=[X])` rows followed by an `assistant` row with non-empty `content`
- **THEN** only tool call timeline entries SHALL appear for the tool-only iterations
- **AND** the final assistant row SHALL render as a normal message bubble with its reply text
