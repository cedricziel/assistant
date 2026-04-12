## Why

The assistant app currently targets web and macOS only. iOS and iPadOS users have no native client, and the Rust-embedded server cannot run on iOS — so the mobile app must be remote-only from day one. Adding an iOS/iPadOS target unlocks a large user base and lets people chat with their assistant server from anywhere.

## What Changes

- Add `ios` as a Flutter build target (already scaffolded in `app/ios/` but never shipped).
- Introduce an iOS/iPadOS-specific app entry that skips all embedded-server logic and goes directly to remote-connection setup.
- Replace `tray_manager` and `window_manager` (macOS-only) with platform-aware guards so they don't break the iOS build.
- Add `flutter_secure_storage` keychain entitlement for iOS (token persistence).
- Add platform-specific UI adaptations: bottom navigation bar for iPhone, sidebar/NavigationRail for iPad, Cupertino-flavoured widgets where appropriate.
- Configure Xcode project settings: bundle ID, signing, minimum deployment target (iOS 16+), capability entitlements.
- Add CI job for `flutter build ios --no-codesign` on every PR touching `app/**`.

## Capabilities

### New Capabilities

- `ios-remote-connection`: iOS/iPadOS onboarding flow — URL + token entry, persisted via flutter_secure_storage, remote-only (no embedded server option exposed).
- `ios-navigation`: Adaptive navigation shell for iPhone (BottomNavigationBar) and iPad (NavigationRail/sidebar), mirroring the feature set of the existing macOS NavigationRail.
- `ios-platform-guards`: Compile-time and runtime guards that exclude macOS-only packages (`tray_manager`, `window_manager`, embedded server) from the iOS build.

### Modified Capabilities

- `macos-tray`: No requirement changes — but implementation must add `defaultTargetPlatform` guards so tray code is only imported/executed on macOS. The spec itself does not change.

## Impact

- `app/lib/features/connection/` — connection screen gains an iOS-only code path (no mode toggle).
- `app/lib/features/embedded_server/` — service and provider wrapped in macOS-only guards.
- `app/lib/tray/` — tray initialisation wrapped in macOS-only guards.
- `app/lib/router/app_router.dart` — redirect logic unchanged; connection screen already handles remote-only.
- `app/pubspec.yaml` — `tray_manager` and `window_manager` remain but are guarded; no new dependencies needed (packages already present for other platforms).
- `app/ios/` — Xcode project configured for bundle ID, signing team placeholder, minimum iOS 16.
- `.github/workflows/flutter.yml` — new CI step added.
