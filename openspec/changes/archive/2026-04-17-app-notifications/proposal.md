## Why

The assistant app currently runs silently in the background — users miss important events like completed skill runs, incoming messages, or errors unless they actively open the app. Native notifications would surface these events immediately, making the app feel alive and responsive.

On macOS the app already runs persistently in the tray, so local notifications are sufficient. On web, the app may be installed as a PWA and the tab may be closed entirely — local (foreground-only) notifications don't help at all in that case. Proper PWA background push requires a service worker, VAPID keys, and a backend push dispatcher.

## What Changes

- Add a notification service that bridges assistant runtime events to OS-level notifications (macOS + web foreground)
- Add a PWA Web Push pipeline: service worker, push subscription management, backend VAPID-signed push dispatch
- Emit a notification when a background skill/workflow run completes (success or failure)
- Emit a notification when a new chat message arrives while the app is not the focused window
- Emit a notification when an agent encounters a critical error requiring user attention
- Respect Do Not Disturb / notification permission settings on each platform
- Provide a user-facing toggle in settings to enable/disable each notification category

## Capabilities

### New Capabilities

- `notifications-service`: Core notification dispatch service — platform detection, permission requests, and notification delivery abstraction over macOS native (`UNUserNotificationCenter`) and web foreground (Web Notifications API)
- `pwa-push-notifications`: Full PWA background push pipeline — Flutter service worker registration + JS interop for `PushManager.subscribe()`, push subscription storage API on the Rust backend, VAPID-signed Web Push dispatch from Rust when events occur
- `notification-settings`: User preferences for which categories of notifications are enabled, persisted via `shared_preferences`
- `agent-event-notifications`: Wires assistant runtime events (skill complete, message received, agent error) to the notification service and/or the PWA push dispatcher

### Modified Capabilities

- `macos-tray`: Badge the tray icon with a count of unread notifications; clear badge when user opens the app

## Impact

- **Flutter app**: new `features/notifications/` directory; `pubspec.yaml` gains `flutter_local_notifications`; new `app/web/sw.js` service worker; JS interop for push subscription
- **Rust backend** (`crates/web-ui`): new `POST /api/push/subscribe` and `DELETE /api/push/subscribe` endpoints; new `push_subscriptions` SQLite table; VAPID key configuration; Web Push dispatch integrated into the event pipeline
- **Configuration**: VAPID public/private key pair added to `config.toml` (generated on first run)
- **Platforms**: macOS (local notifications) and web/PWA (Web Push); iOS/Android out of scope
- **Settings screen**: new notification preferences section
