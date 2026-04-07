## Context

The Flutter app targets both web and macOS. The macOS build currently runs as a standard windowed application with a Dock icon. Users want a persistent, lightweight menu bar presence so the assistant is always one click away without consuming Dock space or requiring window management.

Flutter has no built-in tray API; the `tray_manager` package provides a cross-platform Dart API backed by platform channel implementations. On macOS it uses `NSStatusBar` and `NSStatusItem`.

## Goals / Non-Goals

**Goals:**

- Show a menu bar icon when the app launches on macOS
- Context menu with: **Open** (show/focus window) and **Quit**
- Keep the app in the macOS Dock (standard windowed app + tray icon)
- Main window opens on first launch; can be re-opened via tray
- Platform-guarded: no effect on web builds

**Non-Goals:**

- Windows or Linux tray support (can be added later; same package supports it)
- Rich tray popover or embedded mini-chat in the menu bar
- Persistent background service / running without the Flutter engine

## Decisions

### D1: Package — `tray_manager` over `system_tray`

`tray_manager` (leanflutter) is actively maintained, null-safe, and has a clean `TrayListener` mixin pattern. `system_tray` is less maintained. `tray_manager` is the de-facto standard for Flutter tray apps.

### D2: LSUIElement = 1 in Info.plist

Setting `LSUIElement` to `YES` removes the app from the Dock and hides it from the App Switcher. The tray icon becomes the only persistent UI anchor. The main window is still a regular `NSWindow` and can be focused normally.

**Trade-off**: Without a Dock icon, users can't easily re-open the app via Cmd+Tab. Mitigated by the tray "Open" menu item.

### D3: Window management via `window_manager`

`tray_manager` handles the icon/menu; showing and hiding the actual `NSWindow` requires `window_manager` (same ecosystem, leanflutter). This gives us `windowManager.show()`, `hide()`, `focus()` with proper macOS semantics.

### D4: Platform guard in Dart

Use `dart:io`'s `Platform.isMacOS` (or `defaultTargetPlatform == TargetPlatform.macOS`) to guard tray initialization. Flutter web does not expose `dart:io`, so use a conditional import with a no-op stub for web.

## Risks / Trade-offs

- **Sandbox entitlements**: `tray_manager` and `window_manager` may require adjusting `com.apple.security.app-sandbox` or adding specific entitlement keys. The macOS Runner is sandboxed by default. → Mitigate by testing both Debug and Release profiles; loosen only the specific entitlement needed.
- **Window state on first launch**: If `LSUIElement = 1` is set without calling `windowManager.show()` on startup, the app launches invisibly. → Always call `show()` once at init before hiding becomes user-triggered.
- **macOS version compatibility**: `NSStatusItem` behavior changed slightly in macOS 14. `tray_manager` abstracts this but may need a version bump. → Pin to latest stable release.

## Migration Plan

1. Add `tray_manager` and `window_manager` to `pubspec.yaml`
2. Update macOS `Info.plist` (`LSUIElement`)
3. Update macOS entitlements files if sandbox needs relaxing
4. Create `lib/tray/tray_manager_service.dart` with init/teardown logic
5. Call tray init from `main.dart` behind macOS platform guard
6. Manual test: launch macOS build, verify tray icon, open/quit, no Dock icon
7. CI: `flutter build macos` must continue to pass

## Open Questions

- Should the tray icon use the app icon or a dedicated smaller icon? (Assume a simple SF Symbol–style monochrome icon for now)
- Should the window hide when the user closes it (red X), or quit? (Propose: hide, keep tray alive)
