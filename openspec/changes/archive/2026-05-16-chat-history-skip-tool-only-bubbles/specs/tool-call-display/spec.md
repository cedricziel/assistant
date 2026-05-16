## ADDED Requirements

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
