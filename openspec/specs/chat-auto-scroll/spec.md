## ADDED Requirements

### Requirement: Scroll to bottom on conversation open

The chat view SHALL automatically scroll to the latest (bottom) message whenever a conversation is opened or switched to, using a smooth animation.

#### Scenario: Opening a conversation with existing messages

- **WHEN** the user navigates to a conversation that has messages
- **THEN** the message list SHALL animate to the bottom within one rendered frame after the list is laid out

#### Scenario: Switching between conversations

- **WHEN** the user selects a different conversation from the sidebar
- **THEN** the message list SHALL animate to the bottom of the newly loaded conversation

#### Scenario: Opening an empty conversation

- **WHEN** the user opens a conversation with no messages
- **THEN** no scroll action is taken and the empty state widget is shown

### Requirement: Auto-scroll on new messages when at bottom

The chat view SHALL automatically scroll to the bottom when new messages arrive, but only when the user is already at or near the bottom of the message list (within 80 logical pixels).

#### Scenario: New message arrives while at bottom

- **WHEN** a new message is added and the user's scroll position is within 80 dp of the bottom
- **THEN** the view SHALL animate to the new bottom

#### Scenario: New message arrives while scrolled up

- **WHEN** a new message is added and the user has scrolled up more than 80 dp from the bottom
- **THEN** the scroll position SHALL remain unchanged

### Requirement: Scroll-to-bottom button when scrolled up

The chat view SHALL display a "scroll to bottom" button overlaid on the message list when the user has scrolled more than 80 logical pixels above the bottom of the list.

#### Scenario: Button appears after scrolling up

- **WHEN** the user scrolls upward past 80 dp from the bottom
- **THEN** a scroll-to-bottom button SHALL become visible in the bottom-right area of the message list

#### Scenario: Button disappears at bottom

- **WHEN** the user scrolls back to within 80 dp of the bottom (manually or via the button)
- **THEN** the scroll-to-bottom button SHALL no longer be visible

#### Scenario: Tapping the button scrolls to bottom

- **WHEN** the user taps the scroll-to-bottom button
- **THEN** the message list SHALL animate smoothly to the latest message
