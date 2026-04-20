## 1. Dependency & Provider Setup

- [x] 1.1 Add `connectivity_plus` to `app/pubspec.yaml` dependencies and run `flutter pub get`
- [x] 1.2 Create `app/lib/api/connectivity_provider.dart` with a `connectivityProvider` (StreamProvider wrapping `Connectivity().onConnectivityChanged`) and a derived `isOnlineProvider` (Provider<bool> that returns `result != ConnectivityResult.none`)
- [x] 1.3 Write unit test for `connectivity_provider.dart` verifying the derived `isOnlineProvider` maps connectivity results correctly

## 2. Offline Banner

- [x] 2.1 Add an offline banner widget to `app/lib/shared/nav_shell.dart` that watches `isOnlineProvider` and displays a persistent `MaterialBanner` when offline (skip on web platform using `kIsWeb` guard)
- [x] 2.2 Write widget test for the offline banner: verify it appears when connectivity is none and disappears when restored

## 3. Auto-Reconnect on Connectivity Restoration

- [x] 3.1 In `app/lib/features/notifications/agent_event_listener.dart`, watch `connectivityProvider` for none→connected transitions and call `chatProvider.attemptReconnect()` (mirroring the existing lifecycle-based trigger)
- [x] 3.2 Write unit test verifying that `attemptReconnect()` is called on connectivity restoration when `_needsReconnect` is true, and NOT called when there is no pending reconnect

## 4. Offline Guards for Voice & Upload

- [x] 4.1 In `app/lib/features/chat/voice_recorder_button.dart`, check `isOnlineProvider` before starting recording; show a SnackBar with "Voice recording requires an internet connection" and abort if offline
- [x] 4.2 In `app/lib/features/chat/attachment_provider.dart` (guard in chat_screen.dart `_pickImages`), check `isOnlineProvider` before initiating file upload; show a SnackBar with "File upload requires an internet connection" and abort if offline
- [x] 4.3 Write widget test for voice recorder offline guard: verify recording does not start and feedback is shown when offline

## 5. Verification

- [x] 5.1 Run `flutter analyze` and `flutter test` — all pass with zero issues
- [ ] 5.2 Manual smoke test: toggle Wi-Fi off/on on macOS, verify banner appears/disappears and SSE reconnects automatically
