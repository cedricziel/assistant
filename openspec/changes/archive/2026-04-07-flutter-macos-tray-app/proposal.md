## Why

The Flutter app currently only runs as a full-window macOS application. Users want quick access to the assistant without switching away from their current work — a menu bar tray icon provides always-available, low-friction access from any context.

## What Changes

- Add `tray_manager` (or `system_tray`) Flutter package as a dependency for macOS menu bar icon support
- Add macOS-specific entitlements/Info.plist changes to support running as a tray/menu bar app (LSUIElement)
- Implement a tray icon with a context menu (Open / Quit)
- Make the main window show/hide on tray icon click, rather than appearing in the macOS Dock by default
- Wire the tray lifecycle into the Flutter app's `main.dart` behind a platform guard

## Capabilities

### New Capabilities

- `macos-tray`: Menu bar tray icon for macOS — show/hide the main window, quit the app, with correct LSUIElement behavior (no Dock icon)

### Modified Capabilities

- (none)

## Impact

- `app/pubspec.yaml`: add tray manager package dependency
- `app/lib/main.dart`: initialize tray manager on macOS at startup
- `app/macos/Runner/Info.plist`: set `LSUIElement = 1` to hide Dock icon
- `app/macos/Runner/DebugProfile.entitlements` / `Release.entitlements`: may need `com.apple.security.app-sandbox` adjustments
- No Rust backend changes required
- Web target unaffected (platform guard)
