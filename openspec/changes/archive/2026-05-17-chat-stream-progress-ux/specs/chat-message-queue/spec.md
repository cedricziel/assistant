## ADDED Requirements

### Requirement: Queued messages render as visible ghost bubbles

Messages sitting in `ChatState.pendingQueue` SHALL be rendered in the conversation list, in send order, as visually-distinguished "ghost" bubbles. The user SHALL be able to see exactly what is queued without being able to confuse it with a committed message.

#### Scenario: Two queued messages render as ghost bubbles

- **WHEN** the user types and sends a message while a previous turn is streaming, and then types and sends a second message before the first turn completes
- **THEN** both queued messages SHALL appear as ghost bubbles in the conversation list, in the order they were sent
- **THEN** each ghost bubble SHALL display the message text and a "Queued" badge
- **THEN** the ghost bubbles SHALL visually differ from committed user bubbles (muted text colour, no avatar, "Queued" badge)

#### Scenario: Ghost bubble promotes to a normal bubble when its turn starts

- **WHEN** the in-flight turn completes and the next queued message starts streaming
- **THEN** that queued message's ghost bubble SHALL be removed and a normal user bubble SHALL appear in its place at the same position in the list
- **THEN** the visual transition SHALL NOT cause the conversation to jump or reorder

### Requirement: Queued messages are removable before they send

The user SHALL be able to remove a queued message from `pendingQueue` before it begins streaming, without affecting the in-flight turn.

#### Scenario: Long-press to remove on mobile

- **WHEN** the user long-presses a ghost bubble for a queued message
- **THEN** an action sheet SHALL appear with a "Remove from queue" action (and a Cancel)
- **WHEN** the user selects "Remove from queue"
- **THEN** the entry SHALL be removed from `pendingQueue` and the ghost bubble SHALL disappear within one frame

#### Scenario: Right-click to remove on desktop

- **WHEN** the user right-clicks a ghost bubble for a queued message on web or macOS
- **THEN** a context menu SHALL appear with a "Remove from queue" action
- **WHEN** the user selects it
- **THEN** the entry SHALL be removed from `pendingQueue` and the ghost bubble SHALL disappear

#### Scenario: Cannot remove the actively-streaming turn

- **WHEN** the user attempts to remove the message that is currently streaming (its bubble is no longer a ghost — it has been promoted to a normal user bubble)
- **THEN** the remove affordance SHALL NOT appear on that bubble; the in-flight turn can only be ended via the future `turn-status-api` cancel surface or by waiting for it to complete
