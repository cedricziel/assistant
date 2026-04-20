## Why

The Flutter app has no proactive network-state awareness. When connectivity drops (laptop lid close, Wi-Fi switch, mobile data loss), the SSE chat stream dies silently and the user sees stale UI until they manually retry. On macOS desktop — where the app stays in the foreground for long sessions — the lifecycle-based reconnect (`AppLifecycleState.resumed`) never fires because the app isn't backgrounded. Users deserve immediate feedback when offline and automatic recovery when connectivity returns.

## What Changes

- Add `connectivity_plus` dependency to the Flutter app
- Introduce a Riverpod `connectivityProvider` (StreamProvider) exposing real-time network state
- Derive an `isOnlineProvider` boolean for simple consumption
- Show a persistent offline banner in the navigation shell when connectivity is lost
- Trigger `ChatProvider.attemptReconnect()` on connectivity restoration (none → connected transition) in addition to the existing app-resume path
- Gate voice recording and file uploads behind connectivity check to prevent wasted work

## Non-goals

- Server-reachability probing (captive portals, VPN issues) — out of scope, existing DioException path handles this
- Offline queue / message drafts — messages still fail when offline, but fail fast with clear UX
- Web platform offline detection — browsers handle this natively via `navigator.onLine`

## Capabilities

### New Capabilities

- `network-awareness`: Real-time connectivity monitoring, offline banner, and automatic SSE reconnection on network restoration

### Modified Capabilities

(none — no existing spec-level behavior changes)

## Impact

- **Dependencies**: adds `connectivity_plus` to `app/pubspec.yaml`
- **Code**: new provider file, modifications to `nav_shell.dart`, `chat_provider.dart`, `voice_recorder_button.dart`, `attachment_provider.dart`
- **Platforms**: iOS/macOS need no entitlement changes (connectivity_plus uses reachability APIs already permitted). Web uses `navigator.onLine` under the hood — no additional permissions.
- **APIs**: no backend changes
