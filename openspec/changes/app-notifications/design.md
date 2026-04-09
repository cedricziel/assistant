## Context

The Flutter app communicates with the Rust backend over HTTP + SSE streaming. Chat messages and agent events already arrive in real-time via `ChatNotifier` and related Riverpod providers. The `macos-tray` spec established that the app runs persistently in the background as a tray app on macOS.

On **macOS**, the app is always running — local notifications via `UNUserNotificationCenter` are sufficient.

On **web/PWA**, the browser tab may be closed entirely. The Web Notifications API only works while the tab is open. True background delivery requires:

1. A registered **service worker** that can receive push events from a server
2. The browser subscribing to a **Web Push endpoint** (via `PushManager.subscribe()`)
3. The **Rust backend** sending a VAPID-signed HTTP request to the push endpoint when events occur

State management is Riverpod 2/3. Settings are persisted via `shared_preferences`. The backend uses SQLite (sqlx) and axum.

## Goals / Non-Goals

**Goals:**

- Deliver OS-level notifications for: skill/workflow run completion, new chat message while window unfocused, agent critical errors
- macOS: local notifications via `flutter_local_notifications`
- Web/PWA (tab open): foreground notification via Web Notifications API
- Web/PWA (tab closed): background push notification via Web Push + service worker
- Request notification permission on first relevant event (not at startup)
- Per-category opt-out toggles in the settings screen
- Badge the macOS tray icon with unread count; clear on focus
- VAPID key pair auto-generated on first Rust server start, stored in config

**Non-Goals:**

- Firebase Cloud Messaging or any third-party push gateway
- Rich notification actions (reply from notification, etc.) in v1
- Notification history / inbox within the app
- Android/iOS support
- Multi-device push fan-out (notifications go to the most recently subscribed endpoint per user)

## Decisions

### 1. Split notification delivery by platform

**Decision**: Use `flutter_local_notifications` for macOS. Use direct Web Push (VAPID) for PWA background. For web foreground, the existing Web Notifications API path in `flutter_local_notifications` covers the tab-open case.

**Rationale**: `flutter_local_notifications` does not support background Web Push — it can only fire notifications in the active tab. PWA background delivery fundamentally requires a server-side push and a service worker. Keeping the two paths separate is cleaner than forcing one package to do both.

### 2. VAPID keys generated and stored in config.toml

**Decision**: On first `assistant webui serve` startup, if no VAPID keys exist in `config.toml`, generate a new VAPID key pair using the `web-push` Rust crate and persist them under `[notifications]` in `~/.assistant/config.toml`.

**Rationale**: VAPID keys must be stable — if they change, all existing push subscriptions become invalid. Storing them in the user's config file (outside the binary) satisfies this. The public key is served at `GET /api/push/vapid-public-key` so the Flutter app can subscribe.

**Alternative considered**: Environment variables — rejected because the deployment target (`schorschvm`) uses `config.toml` for all config; mixing env vars would be inconsistent.

### 3. Push subscription stored in SQLite per browser endpoint

**Decision**: A new `push_subscriptions` table stores `(id, endpoint, p256dh, auth, created_at)`. `POST /api/push/subscribe` upserts by endpoint URL; `DELETE /api/push/subscribe` removes by endpoint. No per-user scoping in v1 (single-user assistant).

**Rationale**: Simple and consistent with the existing storage pattern. The table is small (one row per browser install).

### 4. Service worker registered from Flutter web build output

**Decision**: Add `app/web/sw.js` — a hand-written service worker that handles `push` events and calls `self.registration.showNotification()`. Flutter's `flutter_service_worker.js` (caching SW) is separate; this SW is registered alongside it using `navigator.serviceWorker.register('/sw.js')` via a `dart:js_interop` call in `NotificationService`.

**Rationale**: Flutter's generated SW handles offline caching only and cannot be extended. A parallel SW registration for push is the standard pattern for Flutter PWAs.

**Alternative considered**: `firebase_messaging` Flutter package — rejected; pulls in the entire Firebase SDK, requires a Firebase project, and is heavyweight for a self-hosted assistant.

### 5. Rust push dispatch integrated into the SSE event pipeline

**Decision**: When the Rust backend emits an event that would trigger a notification (skill complete, new assistant message, agent error), it also queries `push_subscriptions` and fires a VAPID-signed Web Push request to each stored endpoint using the `web-push` crate.

**Rationale**: The SSE pipeline already processes all events. Adding push dispatch inline (as a side-effect in the same async task) avoids a separate background queue in v1.

**Alternative considered**: A dedicated push worker reading from the DB on a timer — rejected as unnecessary complexity for v1.

### 6. NotificationService as a singleton Riverpod provider (lazy init)

**Decision**: `NotificationService` is a `Provider<NotificationService>`. Initialization is lazy — `initialize()` is called on first use. On web, it registers the service worker and subscribes to push, posting the subscription to the backend.

**Rationale**: Permission dialogs must not appear at cold start. Lazy initialization ensures we only prompt when the user triggers an action that produces a notification.

### 7. Notification preferences in SharedPreferences

**Decision**: `NotificationPreferences` wraps `SharedPreferences` with typed getters/setters per category (`notifyChatMessages`, `notifySkillComplete`, `notifyAgentErrors`). Default: all enabled.

**Rationale**: Lightweight, already a dependency, survives app restarts.

### 8. macOS tray badge via tray_manager

**Decision**: `NotificationBadgeNotifier` (Riverpod `Notifier<int>`) tracks unread count. Incremented on each local notification; cleared when app gains focus via `WidgetsBindingObserver`.

## Risks / Trade-offs

- **VAPID key rotation invalidates all subscriptions** → mitigation: never auto-rotate; document that key changes require users to re-open the PWA to re-subscribe
- **Browser push endpoint expiry** → mitigation: on `410 Gone` response from the push endpoint, delete the subscription row from the DB
- **Web Notifications API requires HTTPS** → mitigation: graceful no-op + debug warning in HTTP contexts (localhost dev)
- **Service worker scope conflicts with Flutter's caching SW** → mitigation: register at `/sw.js` (root scope); Flutter's SW at `/flutter_service_worker.js`; no scope overlap
- **macOS Notification Center permission denied** → mitigation: one-time in-app banner; never re-prompt aggressively
- **`flutter_local_notifications` web support is experimental** → mitigation: wrap in try/catch; feature degrades gracefully
- **Badge count drift** → mitigation: clear badge on every `AppLifecycleState.resumed`

## Migration Plan

1. Add `flutter_local_notifications` to `app/pubspec.yaml`
2. Add macOS notification entitlements
3. Implement `NotificationService` + `NotificationPreferences` (Flutter)
4. Add `app/web/sw.js` service worker
5. Add JS interop for `PushManager.subscribe()` in `NotificationService`
6. Add Rust: `push_subscriptions` migration, `POST /api/push/subscribe`, `DELETE /api/push/subscribe`, `GET /api/push/vapid-public-key`
7. Add Rust: VAPID key generation in startup, push dispatch in event pipeline
8. Wire event listeners in Flutter (`AgentEventListener`, `ChatScreen`)
9. Add notification settings section to settings screen
10. Implement tray badge

No breaking changes. Fully additive. Rollback: remove the wiring, the new files, and the DB migration (table drop).

## Open Questions

- Does `tray_manager` 0.2.x expose `setBadgeLabel` on macOS, or is a custom icon overlay needed?
- Should the push notification payload include a `conversationId` so the service worker can deep-link into the correct conversation on tap? (Recommended — include in v1.)
- Which `web-push` Rust crate to use: `web_push` (0.10.x) or `vapid` + manual HTTP? (`web_push` crate is more mature.)
