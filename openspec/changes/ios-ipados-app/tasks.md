## 1. Platform Guards — macOS-only packages

- [ ] 1.1 Wrap `tray_manager` and `window_manager` init in `app/lib/main.dart` with `if (!kIsWeb && defaultTargetPlatform == TargetPlatform.macOS)` guard
- [ ] 1.2 Wrap tray service calls in `app/lib/tray/` with the same macOS-only guard
- [ ] 1.3 Run `flutter analyze` and confirm zero errors on all platforms

## 2. Platform Guards — Embedded Server

- [ ] 2.1 Add a `static bool get isAvailable` check to `EmbeddedServerService` that returns `false` on iOS (using `defaultTargetPlatform`)
- [ ] 2.2 Update `ConnectionScreen` to suppress the mode-toggle segmented button when `EmbeddedServerService.isAvailable` is `false` (already partially in place; verify iOS path)
- [ ] 2.3 Ensure `embeddedServerProvider` is never watched when `isAvailable` is false (review `connection_screen.dart` and `embedded_server_provider.dart`)
- [ ] 2.4 Run `flutter build ios --no-codesign` locally and confirm no compile errors from embedded server imports

## 3. Adaptive Navigation Shell

- [ ] 3.1 Create `app/lib/shared/widgets/adaptive_shell.dart` — `AdaptiveShell` widget that wraps `NavigationRail` (width >= 600 dp) or `BottomNavigationBar` (width < 600 dp)
- [ ] 3.2 Wire `AdaptiveShell` into the shell route in `app/lib/router/app_router.dart`, replacing the current `NavigationRail`-only layout
- [ ] 3.3 Map all existing navigation destinations (Chat, Personas, Skills, Traces, Logs, Settings) into the `BottomNavigationBar` items
- [ ] 3.4 Verify no `RenderFlex overflowed` error appears on iPhone SE form factor (375 × 667 dp) in the iOS Simulator

## 4. Remote-Only Connection Screen on iOS

- [ ] 4.1 Confirm `ConnectionScreen` hides the `SegmentedButton` mode toggle when `EmbeddedServerService.isAvailable` is false (already gated on this; add explicit iOS Simulator smoke test)
- [ ] 4.2 Verify the URL field default text is `http://127.0.0.1:8080` (not `Uri.base.origin`) on iOS — `isWebPlatform` is already false on iOS, so this should be correct
- [ ] 4.3 Verify token is stored via `flutter_secure_storage` in iOS Keychain (use Simulator + Keychain viewer or integration test)

## 5. Xcode Project Configuration

- [ ] 5.1 Set `IPHONEOS_DEPLOYMENT_TARGET = 16.0` in `app/ios/Runner.xcodeproj/project.pbxproj` for all build configurations (Debug, Profile, Release)
- [ ] 5.2 Set bundle identifier to `com.assistant.app` (or matching macOS bundle ID pattern) in `project.pbxproj`
- [ ] 5.3 Add `keychain-access-groups` entitlement to `app/ios/Runner/Runner.entitlements` (required by `flutter_secure_storage` on iOS)
- [ ] 5.4 Verify `Podfile` sets `platform :ios, '16.0'`
- [ ] 5.5 Run `pod install` in `app/ios/` and confirm no dependency conflicts

## 6. CI Integration

- [ ] 6.1 Add a `build-ios` job to `.github/workflows/flutter.yml` that runs `flutter build ios --no-codesign` on pushes/PRs touching `app/**`
- [ ] 6.2 Set `runs-on: macos-latest` for the new job (Xcode required)
- [ ] 6.3 Confirm the job completes successfully in CI on a test PR

## 7. Smoke Testing

- [ ] 7.1 Boot iPhone 15 Simulator and launch the app — confirm connection screen shows remote-only form
- [ ] 7.2 Connect to a running assistant server from the Simulator — confirm chat screen loads
- [ ] 7.3 Boot iPad Pro 13-inch Simulator — confirm `NavigationRail` is shown instead of bottom bar
- [ ] 7.4 Boot iPhone SE (3rd gen) Simulator — confirm no overflow errors
- [ ] 7.5 Terminate and relaunch the Simulator app — confirm saved context persists and chat screen opens directly
