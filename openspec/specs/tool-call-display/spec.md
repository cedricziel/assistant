## Purpose

Define how tool invocations performed by the assistant are surfaced in the chat UI: when chips appear, what status states they carry, how they relate to reply text, and how they survive across streaming and history reconstruction.

## Requirements

### Requirement: Tool calls render as inline chips inside the assistant bubble

The assistant message bubble SHALL display a chip for each tool call associated with the message. Chips appear above the reply text, separated by a visual divider when reply text is also present.

#### Scenario: In-progress chip appears on StatusEvent

- **WHEN** a `StatusEvent` is received during streaming
- **THEN** a chip with a spinner and the tool name appears inside the assistant bubble immediately
- **AND** the chip is visible even if tokens have already started streaming

#### Scenario: In-progress chip is replaced by result chip on ToolResultEvent

- **WHEN** a `ToolResultEvent` is received for a tool that has a pending chip
- **THEN** the spinner is replaced by a status icon matching the result status (✓ for ok, ✗ for error, ⊘ for denied)
- **AND** the chip background/colour reflects the status

#### Scenario: Divider appears between chips and reply text

- **WHEN** a message has both tool call chips and non-empty reply text
- **THEN** a visual divider is rendered between the chips and the text
- **AND** no divider is shown when reply text is empty

#### Scenario: Chips have no divider when reply text is absent

- **WHEN** an assistant message has tool call chips but no reply text content
- **THEN** no divider is rendered below the chips

### Requirement: Tool calls persist on the message after streaming ends

Tool call records SHALL be stored on `ChatMessage` and remain visible after the stream completes, including when the conversation is scrolled or the screen is rebuilt.

#### Scenario: Chips visible after DoneEvent

- **WHEN** the stream completes with a `DoneEvent`
- **THEN** all tool call chips from that stream remain rendered on the final message bubble

#### Scenario: Multiple sequential tool calls all shown

- **WHEN** a single assistant response invokes multiple tools in sequence
- **THEN** a chip for each tool call is shown, in invocation order

### Requirement: Tool call status icons are distinct and accessible

Each status SHALL use a distinct icon and colour so users can differentiate outcomes without relying on colour alone.

#### Scenario: Pending status renders spinner

- **WHEN** a tool call chip has status `pending`
- **THEN** a circular progress indicator (spinner) is shown as the icon

#### Scenario: Ok status renders checkmark

- **WHEN** a tool call chip has status `ok`
- **THEN** a checkmark icon (✓) with a success colour (green) is shown

#### Scenario: Error status renders cross

- **WHEN** a tool call chip has status `error`
- **THEN** an error icon (✗) with an error colour (red) is shown

#### Scenario: Denied status renders prohibition symbol

- **WHEN** a tool call chip has status `denied`
- **THEN** a prohibition icon (⊘) with a warning colour (amber) is shown

### Requirement: Tool-call-only assistant rows render chips without an empty message bubble

When reconstructing a conversation from history, the chat timeline SHALL NOT render a message bubble shell for an assistant row whose `content` is empty, whose `tool_calls` list is non-empty, and which carries no attachments. The tool-call chip(s) derived from that row already represent the ReAct step fully; an additional empty bubble is redundant and visually fragments the timeline.

#### Scenario: Persisted assistant tool-only row produces only chips

- **WHEN** conversation history contains an `assistant` message with `content == ""`, one or more entries in `tool_calls`, and no attachments
- **THEN** the chat timeline contains one tool-call chip entry per `tool_calls` entry
- **AND** the chat timeline contains no `TimelineEntryType.message` entry corresponding to that persisted row

#### Scenario: Mixed turn with content and tool calls keeps the bubble

- **WHEN** conversation history contains an `assistant` message with non-empty `content` and one or more entries in `tool_calls`
- **THEN** the chat timeline contains one tool-call chip per `tool_calls` entry
- **AND** the chat timeline contains a `TimelineEntryType.message` entry rendering the reply text

#### Scenario: Assistant row with attachments but no content keeps the bubble

- **WHEN** conversation history contains an `assistant` message with `content == ""`, one or more entries in `tool_calls`, and at least one attachment
- **THEN** the chat timeline contains a `TimelineEntryType.message` entry so the attachment thumbnails are rendered

#### Scenario: User rows are always preserved

- **WHEN** conversation history contains a `user` message, regardless of `content` being empty
- **THEN** the chat timeline contains a `TimelineEntryType.message` entry for that row
- **AND** the bubble shell is rendered (the user-authored turn must remain visible)

#### Scenario: Final-answer assistant row after tool-only iterations renders normally

- **WHEN** conversation history contains a sequence of `assistant(content="", tool_calls=[X])` rows followed by an `assistant` row with non-empty `content`
- **THEN** only the chips appear for the tool-only iterations
- **AND** the final assistant row renders as a normal message bubble with its reply text
