## ADDED Requirements

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
