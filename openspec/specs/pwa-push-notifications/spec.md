# pwa-push-notifications Specification

## Purpose
TBD - created by archiving change pwa-push-notifications. Update Purpose after archive.

## Requirements

### Requirement: VAPID key provisioning

The Rust backend SHALL generate a VAPID key pair on first startup if none exists and persist it in `config.toml`. The public key SHALL be served to the Flutter app.

#### Scenario: Keys generated on first run

- **WHEN** the server starts and no VAPID keys are present in config
- **THEN** a new VAPID key pair is generated and written to `~/.assistant/config.toml` under `[notifications]`
- **AND** the server continues starting normally

#### Scenario: VAPID public key endpoint

- **WHEN** the Flutter app calls `GET /api/push/vapid-public-key`
- **THEN** the response contains the base64url-encoded VAPID public key

### Requirement: Push subscription registration

The system SHALL allow the Flutter web app to register a browser push subscription with the backend so the backend can send push notifications to that browser.

#### Scenario: Subscribe

- **WHEN** the Flutter app calls `POST /api/push/subscribe` with `{endpoint, p256dh, auth}`
- **THEN** the subscription is upserted into the `push_subscriptions` table (keyed by endpoint URL)
- **AND** the server responds with `201 Created`

#### Scenario: Unsubscribe

- **WHEN** the Flutter app calls `DELETE /api/push/subscribe` with `{endpoint}`
- **THEN** the subscription row is deleted
- **AND** the server responds with `204 No Content`

#### Scenario: Stale subscription cleaned up

- **WHEN** the backend sends a push to an endpoint and receives `410 Gone`
- **THEN** the subscription is deleted from the database

### Requirement: Service worker registration

The Flutter web app SHALL register a push-capable service worker (`/sw.js`) that can receive push events and display notifications when the browser tab is closed.

#### Scenario: Service worker registered on web init

- **WHEN** the Flutter web app initializes on a supported browser with HTTPS
- **THEN** `/sw.js` is registered via `navigator.serviceWorker.register('/sw.js')`
- **AND** the app subscribes to push notifications using the VAPID public key from the backend
- **AND** the resulting push subscription is posted to `POST /api/push/subscribe`

#### Scenario: Push received with tab closed

- **WHEN** the Rust backend sends a Web Push message to a registered endpoint
- **AND** the browser tab is closed (PWA installed or background tab)
- **THEN** the service worker receives the `push` event
- **AND** calls `self.registration.showNotification(title, { body, data: { conversationId } })`

#### Scenario: Notification tap deep-links into app

- **WHEN** the user taps a push notification from the service worker
- **THEN** the PWA opens (or is focused) and navigates to the conversation indicated by `conversationId` in the notification payload

### Requirement: Backend push dispatch

The Rust backend SHALL send a VAPID-signed Web Push message to all registered subscriptions when a notifiable event occurs.

#### Scenario: Push on new assistant message

- **WHEN** the assistant sends a new chat message
- **AND** at least one push subscription is registered
- **THEN** the backend sends a Web Push message with title "New message" and the first 80 chars of the message body to each registered endpoint

#### Scenario: Push on skill completion

- **WHEN** a skill run completes (success or failure)
- **AND** at least one push subscription is registered
- **THEN** the backend sends a Web Push message with the skill name and status

#### Scenario: Push on agent critical error

- **WHEN** the agent emits a critical error
- **AND** at least one push subscription is registered
- **THEN** the backend sends a Web Push message with title "Assistant error" and a brief description

#### Scenario: No subscriptions — no push attempt

- **WHEN** a notifiable event occurs
- **AND** no push subscriptions are registered
- **THEN** no push HTTP request is made

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
