## ADDED Requirements

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
