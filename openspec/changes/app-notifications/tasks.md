## 1. Dependencies & Platform Setup

- [ ] 1.1 Add `flutter_local_notifications ^18.x` to `app/pubspec.yaml` and run `flutter pub get`
- [ ] 1.2 Add macOS entitlement for user notifications in `app/macos/Runner/DebugProfile.entitlements` and `Release.entitlements`
- [ ] 1.3 Add `web_push` crate to `crates/web-ui/Cargo.toml` (workspace dep) for VAPID-signed Web Push dispatch
- [ ] 1.4 Verify `flutter_local_notifications` initializes without error on macOS and web (smoke test in a dev build)

## 2. Rust: VAPID Key Provisioning

- [ ] 2.1 Add `[notifications]` section to `config.toml` schema with `vapid_private_key` and `vapid_public_key` fields
- [ ] 2.2 On `assistant webui serve` startup, check if VAPID keys are present in config; if not, generate with `web_push` crate and write to `~/.assistant/config.toml`
- [ ] 2.3 Add `GET /api/push/vapid-public-key` endpoint that returns the base64url-encoded public key

## 3. Rust: Push Subscription Storage

- [ ] 3.1 Add SQLite migration: `push_subscriptions(id, endpoint TEXT UNIQUE, p256dh TEXT, auth TEXT, created_at)`
- [ ] 3.2 Add `PushSubscriptionStore` in `crates/storage` with `upsert(endpoint, p256dh, auth)`, `delete(endpoint)`, `list_all()` methods
- [ ] 3.3 Add `POST /api/push/subscribe` endpoint — upsert subscription, return `201`
- [ ] 3.4 Add `DELETE /api/push/subscribe` endpoint — delete by endpoint URL, return `204`
- [ ] 3.5 Write unit tests for `PushSubscriptionStore` using in-memory SQLite

## 4. Rust: Push Dispatch

- [ ] 4.1 Create `PushDispatcher` in `crates/web-ui` that holds VAPID keys and a `reqwest::Client`
- [ ] 4.2 Implement `PushDispatcher::send_to_all(title, body, conversation_id)` — fetches all subscriptions, sends VAPID-signed Web Push to each, deletes any endpoint that returns `410 Gone`
- [ ] 4.3 Wire push dispatch into the new assistant message SSE event path (fires after emitting the SSE event)
- [ ] 4.4 Wire push dispatch into the skill run completion event path
- [ ] 4.5 Wire push dispatch into the agent critical error event path
- [ ] 4.6 Write integration test: register a mock push endpoint, trigger an event, verify HTTP push call was made with correct payload

## 5. Flutter: Service Worker

- [ ] 5.1 Create `app/web/sw.js` — handles `push` event: parses JSON payload, calls `self.registration.showNotification(title, { body, data })`
- [ ] 5.2 Handle `notificationclick` event in `sw.js`: call `clients.openWindow(url)` with conversation deep-link URL
- [ ] 5.3 Confirm `sw.js` does not conflict with Flutter's generated `flutter_service_worker.js` (different filenames, same scope — verify no caching conflicts)

## 6. Flutter: NotificationService

- [ ] 6.1 Create `app/lib/features/notifications/notification_service.dart` with `NotificationService` class
- [ ] 6.2 Implement lazy `initialize()`: on macOS calls `flutterLocalNotificationsPlugin.initialize()`; on web registers `sw.js`, fetches VAPID public key, calls `PushManager.subscribe()` via `dart:js_interop`, posts subscription to `POST /api/push/subscribe`
- [ ] 6.3 Implement `requestPermission()` — request OS/browser permission, handle granted/denied outcomes
- [ ] 6.4 Implement `show(String title, String body, {String? conversationId})` — checks permission, dispatches local notification (macOS) or Web Notifications API (web foreground)
- [ ] 6.5 Implement graceful no-op for unsupported platforms and insecure web context
- [ ] 6.6 Handle `onDidReceiveNotificationResponse` to navigate via go_router when `conversationId` payload present
- [ ] 6.7 Expose `notificationServiceProvider` as a Riverpod `Provider<NotificationService>`
- [ ] 6.8 On web: call `DELETE /api/push/subscribe` when `NotificationService.dispose()` or user disables all notifications

## 7. Flutter: Notification Preferences

- [ ] 7.1 Create `app/lib/features/notifications/notification_preferences.dart` with `NotificationPreferences` wrapping `SharedPreferences`
- [ ] 7.2 Add typed getters/setters: `notifyChatMessages`, `notifySkillComplete`, `notifyAgentErrors` (all default `true`)
- [ ] 7.3 Expose `notificationPreferencesProvider` as a Riverpod `AsyncNotifier`

## 8. Flutter: Agent Event Wiring

- [ ] 8.1 In `ChatScreen` (or its parent), add `ref.listen` on chat provider to detect new assistant messages
- [ ] 8.2 Check `WidgetsBindingObserver` lifecycle — only fire local notification when not in `resumed` state
- [ ] 8.3 Truncate message content to 80 characters for notification body
- [ ] 8.4 Respect `notifyChatMessages` preference before firing
- [ ] 8.5 Create `app/lib/features/notifications/agent_event_listener.dart` — widget listening to skill/workflow run completion events
- [ ] 8.6 Wire skill run success notification: title "Skill complete", body includes skill name
- [ ] 8.7 Wire skill run failure notification: title "Skill failed", body includes skill name + error summary
- [ ] 8.8 Wire agent critical error notification: title "Assistant error", brief error description
- [ ] 8.9 Wrap `AgentEventListener` in the widget tree above `ChatScreen`

## 9. macOS Tray Badge

- [ ] 9.1 Create `NotificationBadgeNotifier` (Riverpod `Notifier<int>`) tracking unread count
- [ ] 9.2 Increment in `NotificationService.show()` on macOS
- [ ] 9.3 Add `WidgetsBindingObserver` in root widget to clear badge on `AppLifecycleState.resumed`
- [ ] 9.4 Call `tray_manager` to update badge label (investigate `setBadgeLabel` availability in 0.2.x; fall back to composite icon if needed)

## 10. Notification Settings UI

- [ ] 10.1 Locate existing settings screen (`app/lib/features/settings/`)
- [ ] 10.2 Add "Notifications" section with `SwitchListTile` per category: "New chat messages", "Skill completions", "Agent errors"
- [ ] 10.3 Wire each toggle to read/write `notificationPreferencesProvider`

## 11. Tests

- [ ] 11.1 Unit test `NotificationPreferences` — defaults, persist/read round-trip with fake `SharedPreferences`
- [ ] 11.2 Unit test `NotificationBadgeNotifier` — increment, clear
- [ ] 11.3 Unit test `PushSubscriptionStore` (Rust) — upsert, delete, list with in-memory SQLite
- [ ] 11.4 Widget test notification settings section — toggles render and fire preference updates
- [ ] 11.5 Widget test `AgentEventListener` — verify `NotificationService.show` called with correct args via a fake `NotificationService`
