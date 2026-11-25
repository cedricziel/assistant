## ADDED Requirements

### Requirement: BranchPicker element

`app/packages/assistant_ui` SHALL export a stateless `BranchPicker` taking `index` (zero-based), `count`, `onPrevious`, `onNext`, and an optional `busy` flag. It SHALL render "‹ {index+1}/{count} ›", disable the previous control at the first version and the next control at the last, disable both while `busy`, and expose the semantics label "Version {index+1} of {count}". It SHALL have a widgetbook use-case and golden tests, and its `ELEMENTS.md` row "Message branches" SHALL name `BranchPicker` and `MessageEditor`.

#### Scenario: Middle version

- **WHEN** `BranchPicker(index: 1, count: 3)` is pumped
- **THEN** it shows "2/3" and both controls are enabled

#### Scenario: First version

- **WHEN** `BranchPicker(index: 0, count: 3)` is pumped
- **THEN** the previous control is disabled and tapping it does not invoke `onPrevious`

#### Scenario: Screen reader label

- **WHEN** semantics are inspected for `BranchPicker(index: 1, count: 3)`
- **THEN** a node labelled "Version 2 of 3" exists

### Requirement: MessageEditor element

`assistant_ui` SHALL export a `MessageEditor` taking `initialText`, `onSubmit(String)`, `onCancel` and an optional `attachments` list of widgets, rendering an inline multi-line field with Cancel and Send controls. The attachments SHALL be shown read-only beneath the field: the editor changes text only, and the message's attachments carry forward. Send SHALL be disabled when the trimmed text is empty or unchanged from `initialText`. Escape SHALL cancel. It SHALL have a widgetbook use-case and golden tests.

#### Scenario: Unchanged text cannot be sent

- **WHEN** the editor opens with "hello" and the user does not change it
- **THEN** Send is disabled

#### Scenario: Attachments shown read-only

- **WHEN** the editor opens with two attachment widgets
- **THEN** both are rendered beneath the field and neither offers a remove control

#### Scenario: Escape cancels

- **WHEN** the user presses Escape in the editor
- **THEN** `onCancel` is invoked and `onSubmit` is not

### Requirement: RegenerateMenuButton element

`assistant_ui` SHALL export a stateless `RegenerateMenuButton` taking `models` (a list of names with a default flag) and `onSelected(String)`. It SHALL render nothing when `models` has fewer than two entries, and otherwise a menu listing every model with the default marked. It SHALL have a widgetbook use-case and golden tests.

#### Scenario: Single model hides the menu

- **WHEN** `RegenerateMenuButton(models: [model-a])` is pumped
- **THEN** no menu control is rendered

#### Scenario: Selecting a model

- **WHEN** models are `[model-a (default), model-b]` and the user opens the menu and picks model-b
- **THEN** `onSelected("model-b")` is invoked

### Requirement: Branch actions are composed app-side

`MessageActions` SHALL stay free of branching knowledge. The app's `buildMessageActions` SHALL supply Edit and Regenerate as `MessageActionButton` entries, the `RegenerateMenuButton`, and the `BranchPicker` in `MessageActions.leading`. Edit SHALL be supplied only for persisted user messages, with or without attachments. Regenerate SHALL be supplied only for persisted assistant messages. Both, and the picker controls, SHALL be enabled while no turn is streaming and while the streaming turn is waiting for input (`awaitingInput`), and disabled while a turn is working.

#### Scenario: Edit entry for a plain user message

- **WHEN** `buildMessageActions` runs for a persisted user message and no turn is streaming
- **THEN** the result contains an Edit action

#### Scenario: Edit offered for attachment messages

- **WHEN** a persisted user message has attachments
- **THEN** Edit is offered

#### Scenario: Actions enabled while a turn awaits input

- **WHEN** the streaming turn is waiting for an approval or a question
- **THEN** Edit, Regenerate and the picker controls are enabled

#### Scenario: Actions disabled while a turn works

- **WHEN** the streaming turn is running with no pending interaction
- **THEN** Edit and Regenerate are not offered and the picker controls are disabled

#### Scenario: Picker in the leading slot

- **WHEN** a message carries `branch`
- **THEN** `MessageActions.leading` is a `BranchPicker` for it

### Requirement: Branch picker on messages with siblings

The chat timeline SHALL render a `BranchPicker` for every message whose history entry carries `branch`, and under a user message that carries `reply_branch`. Tapping a control SHALL call `ChatNotifier.switchBranch(siblingIds[newIndex])`, which calls `POST /api/conversations/{id}/switch-branch` and replaces the timeline with the returned detail. While the request is pending the picker SHALL be `busy`; on failure the timeline SHALL be unchanged and an error SHALL be surfaced.

#### Scenario: Switching to another reply

- **WHEN** the user taps next on a reply showing "1/2"
- **THEN** the timeline shows the second reply and the picker shows "2/2"

#### Scenario: Switch fails

- **WHEN** switch-branch returns 409
- **THEN** the timeline is unchanged and an error banner is shown

#### Scenario: Pickers disabled while a turn works

- **WHEN** a turn is streaming and not waiting for input
- **THEN** pickers are disabled

#### Scenario: Switching while an approval is pending

- **WHEN** the streaming turn is waiting for an approval and the user taps a picker control
- **THEN** the switch request is sent, the old stream's cancellation is not shown as an error, and the timeline shows the selected branch

#### Scenario: Reply without visible text

- **WHEN** the active reply to U1 consists only of a tool call and U1 carries `reply_branch` with count 2
- **THEN** a picker showing "2/2" is rendered under U1

### Requirement: Edit a sent user message

Choosing Edit SHALL replace the bubble with a `MessageEditor`. Submitting SHALL drop every message after the edited one from the visible timeline, show the new text as a sending user message (with the original attachments), stream the response from `POST .../messages/{id}/edit`, and reload the conversation detail after the terminal frame. The editor SHALL be openable while the streaming turn is waiting for input, and submitting then SHALL cancel that wait on the server without showing the cancelled run as a failure.

#### Scenario: Edit and resend

- **WHEN** the user edits their second message to "new text" and taps Send
- **THEN** messages after it disappear, "new text" shows as sending, the reply streams in, and afterwards the edited message shows a picker "2/2"

#### Scenario: Edit keeps attachments

- **WHEN** the user edits a message that has an attachment and taps Send
- **THEN** the sending bubble and the reloaded edited message both show the attachment

#### Scenario: Edit while an approval is pending

- **WHEN** the streaming turn shows an approval card and the user edits an earlier message and taps Send
- **THEN** the approval card resolves as cancelled, no error banner is shown for the old run, and the new reply streams on the new branch

#### Scenario: Connection drops during an edit

- **WHEN** the SSE connection drops while an edit is streaming and the client reconnects
- **THEN** after reconnection the timeline shows the edited message active with a picker "2/2" and no message from the old branch after it

### Requirement: Regenerate an assistant reply

Regenerating SHALL drop the reply and everything after it from the visible timeline, stream from `POST .../messages/{id}/regenerate`, and reload the detail after the terminal frame. When `GET /api/models` returns more than one model, the `RegenerateMenuButton` SHALL list them; the models provider SHALL expose loading, error, and empty states, and the menu SHALL be hidden when only the default is available.

#### Scenario: Regenerate

- **WHEN** the user taps Regenerate on the last reply
- **THEN** a new reply streams in its place and afterwards shows a picker "2/2"

#### Scenario: Regenerate cancelled before a worker claims it

- **WHEN** a regenerate ends with `agent_error` (cancelled) before any row was persisted
- **THEN** the client reloads the detail and the old reply is shown again

#### Scenario: Regenerate with another model

- **WHEN** models are `[model-a (default), model-b]` and the user picks model-b
- **THEN** the regenerate request body is `{"model": "model-b"}`

#### Scenario: Only the default model

- **WHEN** `GET /api/models` returns a single entry
- **THEN** the "Regenerate with…" menu is not shown

### Requirement: Client accepts run_started user_message_id

The client `RunStartedEvent` SHALL parse the optional `user_message_id` from the `run_started` frame, and the stream session SHALL replace the optimistic user message id with it. That id is the id the run will persist, not proof that it has been persisted. Unknown `run_started` fields SHALL be ignored. Every exhaustive switch over `StreamEvent` SHALL keep compiling.

#### Scenario: Optimistic id is reconciled

- **WHEN** a `run_started` frame with `user_message_id: "U"` arrives for an optimistic user bubble
- **THEN** that bubble's id becomes "U"

### Requirement: Retry of a failed message with a server id regenerates it

When Retry is tapped on a failed user message that carries a server-assigned id from `run_started.user_message_id`, and replay of its run does not complete with `done` (it ends in `agent_error`, or returns 404 or 410), the client SHALL call `POST /api/conversations/{id}/messages/{message_id}/regenerate` instead of re-sending the text. If regenerate returns 404, because the run never persisted the message, the client SHALL fall back to `sendMessage`. Without a server id, Retry SHALL keep re-sending through `sendMessage`, as `chat-message-retry` specifies today. A failed edit SHALL be retried by regenerating the edited message's server id. If that regenerate returns 404, the client SHALL re-issue the edit of the original message with the same text, and SHALL NOT call `sendMessage`.

#### Scenario: Retry after an agent_error regenerates

- **WHEN** the user taps Retry on a failed message whose server id is "U" and replay ends with `agent_error`
- **THEN** the client calls regenerate for "U"
- **AND** no new user message is sent

#### Scenario: Retry after replay 404 regenerates

- **WHEN** the user taps Retry on a failed message whose server id is "U" and replay returns 404
- **THEN** the client calls regenerate for "U"

#### Scenario: Regenerate 404 falls back to sending

- **WHEN** Retry calls regenerate for "U" and the server returns 404
- **THEN** the message text is sent via `sendMessage`

#### Scenario: Retry without a server id re-sends

- **WHEN** the user taps Retry on a failed message with no server id
- **THEN** the message text is sent via `sendMessage`

#### Scenario: Retry a failed edit

- **WHEN** an edit to "new text" fails after `run_started` reported `user_message_id: "U2'"` and the user taps Retry
- **THEN** the client calls regenerate for "U2'" and does not send "new text" again

#### Scenario: Retry an edit that never persisted

- **WHEN** an edit of "U2" to "new text" fails because it was cancelled before a worker claimed it, and the user taps Retry and regenerate for the edited id returns 404
- **THEN** the client calls edit on "U2" with "new text" again and does not call `sendMessage`

### Requirement: A rejected send keeps the draft

When `POST .../messages` returns 409 because an edit or regenerate is running, the client SHALL keep the composer draft, mark the message as not sent, and show the error. It SHALL NOT drop the text.

#### Scenario: Send during a running edit

- **WHEN** the user sends a message while an edit turn is running and the server returns 409
- **THEN** the message is shown as failed with Retry and the text is retained
