# message-feedback-ui Specification

## Purpose

The feedback elements and their wiring into the chat thread.

## Requirements

### Requirement: FeedbackButtons element

`app/packages/assistant_ui` SHALL provide `FeedbackButtons`, a pure widget that takes the current `MessageRating?` (`positive`, `negative` or none), an `onChanged(MessageRating?)` callback and an `enabled` flag. It SHALL render two `MessageActionButton`s with the keys `feedback_positive_action` and `feedback_negative_action` and the labels "Good response" and "Bad response", used as tooltip and semantics label. The button matching the current rating SHALL be `selected`. It SHALL NOT use Riverpod, `assistant_api` or I/O. It SHALL have a widgetbook use-case and golden tests (none, positive, negative, disabled; light and dark).

#### Scenario: Selecting a rating

- **GIVEN** `FeedbackButtons` with no rating
- **WHEN** the positive button is tapped
- **THEN** `onChanged` SHALL fire with `positive`

#### Scenario: Tapping the selected rating clears it

- **GIVEN** `FeedbackButtons` with rating `negative`
- **WHEN** the negative button is tapped
- **THEN** `onChanged` SHALL fire with `null`

#### Scenario: Selection is exposed to assistive technology

- **GIVEN** `FeedbackButtons` with rating `positive`
- **WHEN** it renders
- **THEN** the "Good response" button's semantics SHALL report selected and the "Bad response" button's SHALL NOT

#### Scenario: Disabled buttons ignore taps

- **GIVEN** `FeedbackButtons` with `enabled: false`
- **WHEN** either button is tapped
- **THEN** `onChanged` SHALL NOT fire

### Requirement: FeedbackCommentField element

`app/packages/assistant_ui` SHALL provide `FeedbackCommentField`, a pure inline text field with Send and Dismiss affordances, taking `initialText`, `onSubmit(String)`, `onDismiss`, `maxLength` (default 2000) and `sending` (default false). The length limit SHALL count Unicode scalar values, matching the server. Pressing Enter SHALL submit and Escape SHALL dismiss. Send SHALL be disabled while the trimmed text is empty or `sending` is true. It SHALL have a widgetbook use-case and golden tests.

#### Scenario: Submit a comment

- **GIVEN** the field with text " missed the deadline "
- **WHEN** Send is activated
- **THEN** `onSubmit` SHALL fire with "missed the deadline"

#### Scenario: Escape dismisses

- **WHEN** Escape is pressed while the field has focus
- **THEN** `onDismiss` SHALL fire and `onSubmit` SHALL NOT

#### Scenario: Limit counts scalar values

- **GIVEN** `maxLength` 2000
- **WHEN** the user enters 2000 Unicode scalar values made of multi-scalar emoji
- **THEN** the field SHALL accept them and SHALL reject further input

### Requirement: Feedback controls on assistant messages

The chat thread SHALL show `FeedbackButtons` in the `MessageActions.trailing` list of every assistant message entry that has non-empty content, is not streaming, is not failed, and has a server-assigned id. The trailing list SHALL be composed by a pure app-side builder in a fixed order, with any timing indicator before the feedback buttons. It SHALL NOT show them on user, tool-call, thinking, subagent or command entries. The buttons SHALL reflect the caller's existing vote from conversation history.

#### Scenario: History shows the existing vote

- **GIVEN** conversation history where an assistant message has `feedback.rating` `positive`
- **WHEN** the conversation loads
- **THEN** that message's positive button SHALL be selected

#### Scenario: Hidden while streaming and for unsaved replies

- **WHEN** an assistant message is streaming, or its id is the client fallback `assistant-<timestamp>`
- **THEN** no feedback buttons SHALL be shown for it

#### Scenario: Reply that just finished can be rated

- **GIVEN** a turn that ended with a `done` frame carrying `message_id`
- **WHEN** the reply renders
- **THEN** feedback buttons SHALL be shown, and rating SHALL target that `message_id`

### Requirement: Feedback updates are optimistic and reconcile on failure

Changing a rating SHALL update the visible selection immediately, then call `PUT .../feedback` (for a rating) or `DELETE .../feedback` (for none). Calls for the same message SHALL be applied in the order the user made them. The client SHALL keep, per message, the last server-confirmed value. On failure, it SHALL drop that message's remaining queued calls, restore the last server-confirmed rating and comment, and show an error snackbar. While a message has queued or in-flight calls, its local value SHALL take precedence over any message list loaded from history. Success and failure handling SHALL NOT modify visible state or show a snackbar when the message is no longer in the current conversation's state.

#### Scenario: Server rejects the vote

- **GIVEN** a message with no rating
- **WHEN** the user taps positive and the server responds with an error
- **THEN** the positive button SHALL end unselected and an error SHALL be shown

#### Scenario: Rapid toggles resolve in order

- **WHEN** the user taps positive, then positive again, before the first call completes
- **THEN** the final server state SHALL be no feedback and the buttons SHALL show no selection

#### Scenario: Rollback restores the confirmed value

- **GIVEN** a message whose server-confirmed rating is `positive`
- **WHEN** the user taps negative, then clears it, and the negative call fails
- **THEN** the queued clear SHALL NOT be sent and the positive button SHALL end selected

#### Scenario: Pending vote survives a history reload

- **GIVEN** a PUT for `negative` is in flight on a message whose history value is `positive`
- **WHEN** the conversation detail is reloaded before the PUT completes
- **THEN** the message SHALL still show `negative` selected

#### Scenario: Stale callback after switching conversation

- **GIVEN** a PUT is in flight for a message in conversation A
- **WHEN** the user opens conversation B and the PUT then fails
- **THEN** conversation B's messages SHALL be unchanged and no snackbar SHALL be shown

#### Scenario: Pending state is exposed

- **WHEN** a feedback call for a message is in flight
- **THEN** that message's comment Send action SHALL be disabled until the call settles

### Requirement: Optional comment after a vote

Choosing either rating, positive or negative, SHALL open `FeedbackCommentField` below the message's actions. Submitting SHALL send `PUT .../feedback` with the current rating and the comment. Dismissing SHALL keep the rating without a comment. Switching to the other rating SHALL send the new rating without a comment and reopen the field empty. Clearing the rating SHALL close the field. A stored comment SHALL prefill the field when it is reopened by choosing the same rating again.

#### Scenario: Add a comment to a negative vote

- **GIVEN** the user chose negative on a message
- **WHEN** they type "wrong file" and submit
- **THEN** the client SHALL send `{"rating": "negative", "comment": "wrong file"}` and close the field

#### Scenario: Add a comment to a positive vote

- **GIVEN** the user chose positive on a message
- **WHEN** they type "clear and short" and submit
- **THEN** the client SHALL send `{"rating": "positive", "comment": "clear and short"}` and close the field

#### Scenario: Both ratings open the comment field

- **WHEN** the user chooses positive on one message and negative on another
- **THEN** a comment field SHALL be shown for each, and neither SHALL require text to keep the rating

#### Scenario: Dismissing keeps the rating

- **GIVEN** the comment field is open after a vote
- **WHEN** the user dismisses it
- **THEN** the rating SHALL stay selected and no further request SHALL be sent

#### Scenario: Switching rating drops the comment

- **GIVEN** a message rated `negative` with comment "wrong file"
- **WHEN** the user chooses positive
- **THEN** the client SHALL send `{"rating": "positive"}` without a comment and the field SHALL reopen empty

#### Scenario: Clearing closes the field

- **GIVEN** the comment field is open
- **WHEN** the user taps the selected rating to clear it
- **THEN** the field SHALL close
