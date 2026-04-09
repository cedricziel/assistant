## 1. Dependencies & Platform Setup

- [x] 1.1 Add `flutter_local_notifications ^18.x` to `app/pubspec.yaml` and run `flutter pub get`
- [x] 1.2 Add macOS entitlement for user notifications in `app/macos/Runner/DebugProfile.entitlements` and `Release.entitlements`
- [x] 1.3 Add `web_push` crate to `crates/web-ui/Cargo.toml` (workspace dep) for VAPID-signed Web Push dispatch
- [x] 1.4 Verify `flutter_local_notifications` initializes without error on macOS and web (smoke test in a dev build)

## 2. Rust: VAPID Key Provisioning

- [x] 2.1 Add `[notifications]` section to `config.toml` schema with `vapid_private_key` and `vapid_public_key` fields
- [x] 2.2 On `assistant webui serve` startup, check if VAPID keys are present in config; if not, generate with `web_push` crate and write to `~/.assistant/config.toml`
- [x] 2.3 Add `GET /api/push/vapid-public-key` endpoint that returns the base64url-encoded public key

## 3. Rust: Push Subscription Storage

- [x] 3.1 Add SQLite migration: `push_subscriptions(id, endpoint TEXT UNIQUE, p256dh TEXT, auth TEXT, created_at)`
- [x] 3.2 Add `PushSubscriptionStore` in `crates/storage` with `upsert(endpoint, p256dh, auth)`, `delete(endpoint)`, `list_all()` methods
- [x] 3.3 Add `POST /api/push/subscribe` endpoint — upsert subscription, return `201`
- [x] 3.4 Add `DELETE /api/push/subscribe` endpoint — delete by endpoint URL, return `204`
- [x] 3.5 Write unit tests for `PushSubscriptionStore` using in-memory SQLite

## 4. Rust: Push Dispatch

- [x] 4.1 Create `PushDispatcher` in `crates/web-ui` that holds VAPID keys and a `reqwest::Client`
- [x] 4.2 Implement `PushDispatcher::send_to_all(title, body, conversation_id)` — fetches all subscriptions, sends VAPID-signed Web Push to each, deletes any endpoint that returns `410 Gone`
- [x] 4.3 Wire push dispatch into the new assistant message SSE event path (fires after emitting the SSE event)
- [x] 4.4 Wire push dispatch into the skill run completion event path
- [x] 4.5 Wire push dispatch into the agent critical error event path
- [ ] 4.6 Write integration test: register a mock push endpoint, trigger an event, verify HTTP push call was made with correct payload

## 5. Flutter: Service Worker

- [x] 5.1 Create `app/web/sw.js` — handles `push` event: parses JSON payload, calls `self.registration.showNotification(title, { body, data })`
- [x] 5.2 Handle `notificationclick` event in `sw.js`: call `clients.openWindow(url)` with conversation deep-link URL
- [x] 5.3 Confirm `sw.js` does not conflict with Flutter's generated `flutter_service_worker.js` (different filenames, same scope — verify no caching conflicts)

## 6. Flutter: NotificationService

- [x] 6.1 Create `app/lib/features/notifications/notification_service.dart` with `NotificationService` class
- [x] 6.2 Implement lazy `initialize()`: on macOS calls `flutterLocalNotificationsPlugin.initialize()`; on web registers `sw.js`, fetches VAPID public key, calls `PushManager.subscribe()` via `dart:js_interop`, posts subscription to `POST /api/push/subscribe`
- [x] 6.3 Implement `requestPermission()` — request OS/browser permission, handle granted/denied outcomes
- [x] 6.4 Implement `show(String title, String body, {String? conversationId})` — checks permission, dispatches local notification (macOS) or Web Notifications API (web foreground)
- [x] 6.5 Implement graceful no-op for unsupported platforms and insecure web context
- [x] 6.6 Handle `onDidReceiveNotificationResponse` to navigate via go_router when `conversationId` payload present
- [x] 6.7 Expose `notificationServiceProvider` as a Riverpod `Provider<NotificationService>`
- [x] 6.8 On web: call `DELETE /api/push/subscribe` when `NotificationService.dispose()` or user disables all notifications

## 7. Flutter: Notification Preferences

- [x] 7.1 Create `app/lib/features/notifications/notification_preferences.dart` with `NotificationPreferences` wrapping `SharedPreferences`
- [x] 7.2 Add typed getters/setters: `notifyChatMessages`, `notifySkillComplete`, `notifyAgentErrors` (all default `true`)
- [x] 7.3 Expose `notificationPreferencesProvider` as a Riverpod `AsyncNotifier`

## 8. Flutter: Agent Event Wiring

- [x] 8.1 In `ChatScreen` (or its parent), add `ref.listen` on chat provider to detect new assistant messages
- [x] 8.2 Check `WidgetsBindingObserver` lifecycle — only fire local notification when not in `resumed` state
- [x] 8.3 Truncate message content to 80 characters for notification body
- [x] 8.4 Respect `notifyChatMessages` preference before firing
- [x] 8.5 Create `app/lib/features/notifications/agent_event_listener.dart` — widget listening to skill/workflow run completion events
- [x] 8.6 Wire skill run success notification: title "Skill complete", body includes skill name
- [x] 8.7 Wire skill run failure notification: title "Skill failed", body includes skill name + error summary
- [x] 8.8 Wire agent critical error notification: title "Assistant error", brief error description
- [x] 8.9 Wrap `AgentEventListener` in the widget tree above `ChatScreen`

## 9. macOS Tray Badge

- [ ] 9.1 Create `NotificationBadgeNotifier` (Riverpod `Notifier<int>`) tracking unread count
- [ ] 9.2 Increment in `NotificationService.show()` on macOS
- [x] 9.3 Add `WidgetsBindingObserver` in root widget to clear badge on `AppLifecycleState.resumed`
- [ ] 9.4 Call `tray_manager` to update badge label (investigate `setBadgeLabel` availability in 0.2.x; fall back to composite icon if needed)

## 10. Notification Settings UI

- [x] 10.1 Locate existing settings screen (`app/lib/features/settings/`)
- [x] 10.2 Add "Notifications" section with `SwitchListTile` per category: "New chat messages", "Skill completions", "Agent errors"
- [x] 10.3 Wire each toggle to read/write `notificationPreferencesProvider`

## 11. Tests

- [x] 11.1 Unit test `NotificationPreferences` — defaults, persist/read round-trip with fake `SharedPreferences`
- [x] 11.2 Unit test `NotificationBadgeNotifier` — increment, clear
- [x] 11.3 Unit test `PushSubscriptionStore` (Rust) — upsert, delete, list with in-memory SQLite
- [x] 11.4 Widget test notification settings section — toggles render and fire preference updates
- [x] 11.5 Widget test `AgentEventListener` — verify `NotificationService.show` called with correct args via a fake `NotificationService`
