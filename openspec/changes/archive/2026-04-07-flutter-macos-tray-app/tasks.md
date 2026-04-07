## 1. Dependencies

- [x] 1.1 Add `tray_manager` to `app/pubspec.yaml` (macOS tray icon and menu)
- [x] 1.2 Add `window_manager` to `app/pubspec.yaml` (show/hide/focus window on macOS)
- [x] 1.3 Run `flutter pub get` in `app/` to resolve new dependencies

## 2. macOS Platform Configuration

- [x] 2.1 Set `LSUIElement` to `YES` in `app/macos/Runner/Info.plist` to remove the Dock icon
- [x] 2.2 Add `window_manager` required `NSPrincipalClass` override in `app/macos/Runner/Info.plist` if needed (per package docs)
- [x] 2.3 Verify/update `app/macos/Runner/DebugProfile.entitlements` for any sandbox permissions required by `tray_manager` / `window_manager`
- [x] 2.4 Verify/update `app/macos/Runner/Release.entitlements` for the same

## 3. Tray Service Implementation

- [x] 3.1 Create `app/lib/tray/tray_service.dart` — initialize `tray_manager` with an icon and context menu (Open, Quit items), implement `TrayListener` for menu callbacks
- [x] 3.2 Add a no-op stub `app/lib/tray/tray_service_stub.dart` for web/non-macOS so imports compile cleanly
- [x] 3.3 Use conditional imports (`dart:io Platform.isMacOS` guard) or a platform-check in `main.dart` to only call tray init on macOS desktop

## 4. Window Close Behavior

- [x] 4.1 Implement `WindowListener` (from `window_manager`) in a widget or service to override `onWindowClose` — call `windowManager.hide()` instead of allowing quit
- [x] 4.2 Wire the listener into the app root widget (e.g., in `AssistantApp` or a dedicated `MacosWindowWrapper`)

## 5. App Entry Point Wiring

- [x] 5.1 Update `app/lib/main.dart` to call `WidgetsFlutterBinding.ensureInitialized()` before `runApp`
- [x] 5.2 Call `windowManager.ensureInitialized()` and `trayService.init()` on macOS before `runApp`
- [x] 5.3 Ensure the main window is shown on first launch (`windowManager.show()`)

## 6. Tray Icon Asset

- [x] 6.1 Add a suitable tray icon image (e.g., a 22×22 monochrome PNG or template image) at `app/assets/tray_icon.png`
- [x] 6.2 Declare the asset in `pubspec.yaml` under `flutter.assets`

## 7. Verification

- [x] 7.1 Run `flutter build macos` — build must succeed with no errors
- [x] 7.2 Launch the macOS app and confirm: tray icon visible, Dock icon present
- [x] 7.3 Test "Open" menu item shows/focuses the main window
- [x] 7.4 Test "Quit" menu item exits the app
- [x] 7.5 Test closing the window (red X) hides it but keeps the tray icon active
- [x] 7.6 Run `flutter analyze` in `app/` — zero issues
- [x] 7.7 Run `flutter test` in `app/` — all tests pass
