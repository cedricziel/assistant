## ADDED Requirements

### Requirement: Push subscriptions record their user

`POST /api/push/subscribe` SHALL store the caller's user id with the subscription. Existing subscriptions without a user id SHALL keep receiving the existing broadcast notifications, but SHALL NOT receive user-targeted notifications. There is no backfill.

#### Scenario: Subscribe records the user

- **WHEN** an authenticated user subscribes a browser
- **THEN** the stored subscription SHALL carry that user's id

### Requirement: Pending interactions push to the turn owner only

When an interaction is still pending 30 seconds after it was requested, the backend SHALL send one Web Push message to the subscriptions of the turn's owner in that space only. The title and body SHALL come from the interaction kind's registered push copy: "Approval needed" for `tool_approval` and "Your assistant has a question" for `question`. The body SHALL name the tool or the first question, truncated to 80 characters. The payload SHALL carry `conversationId` and `tag = interaction-<id>`. An interaction resolved within the delay SHALL produce no push.

#### Scenario: Owner away from the screen

- **WHEN** an approval stays pending for 30 seconds
- **THEN** exactly one push SHALL be sent to each of the owner's subscriptions and none to other members' subscriptions

#### Scenario: Answered quickly

- **WHEN** the owner answers within 30 seconds
- **THEN** no push SHALL be sent

#### Scenario: Repeat replaces instead of stacking

- **WHEN** the service worker receives a push whose payload carries `tag`
- **THEN** it SHALL pass that `tag` to `showNotification`, so a later push with the same tag replaces the earlier notification

#### Scenario: Tap opens the conversation

- **WHEN** the user taps the push notification
- **THEN** the app SHALL open the conversation, find the pending interaction through the pending-interactions list, and show its card
