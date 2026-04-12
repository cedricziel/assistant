## 1. Platform Guards — macOS-only packages

- [x] 1.1 Wrap `tray_manager` and `window_manager` init in `app/lib/main.dart` with `if (!kIsWeb && Platform.isMacOS)` guard
- [x] 1.2 Wrap tray service calls in `app/lib/tray/window_close_handler.dart` with `Platform.isMacOS` guard
- [x] 1.3 Run `flutter analyze` and confirm zero errors on all platforms

## 2. Platform Guards — Embedded Server

- [x] 2.1 Add a `static bool get isAvailable` check to `EmbeddedServerService` that returns `false` on iOS (using `defaultTargetPlatform`) — already implemented
- [x] 2.2 Update `ConnectionScreen` to suppress the mode-toggle segmented button when `EmbeddedServerService.isAvailable` is `false` — already implemented
- [x] 2.3 Ensure `embeddedServerProvider` is never watched when `isAvailable` is false — guarded in `main.dart` with `!kIsWeb && Platform.isMacOS`
- [x] 2.4 Run `flutter build ios --no-codesign` locally and confirm no compile errors from embedded server imports

## 3. Adaptive Navigation Shell

- [x] 3.1 `NavShell` already implements adaptive navigation (bottom `NavigationBar` < 768 dp, `NavigationRail` >= 768 dp) — no new widget needed
- [x] 3.2 `NavShell` already wired into shell route in `app_router.dart` — no changes needed
- [x] 3.3 Primary destinations (Chat, Contexts, Skills, Workflows) + "More" overflow sheet already mapped in `NavShell` — no changes needed
- [ ] 3.4 Verify no `RenderFlex overflowed` error appears on iPhone SE form factor (375 × 667 dp) in the iOS Simulator

## 4. Remote-Only Connection Screen on iOS

- [x] 4.1 `ConnectionScreen` already hides the `SegmentedButton` mode toggle when `EmbeddedServerService.isAvailable` is false — confirmed by code review
- [x] 4.2 URL field default is `http://127.0.0.1:8080` on iOS (`isWebPlatform` is false on iOS) — confirmed by code review
- [ ] 4.3 Verify token is stored via `flutter_secure_storage` in iOS Keychain (use Simulator + Keychain viewer or integration test)

## 5. Xcode Project Configuration

- [x] 5.1 Set `IPHONEOS_DEPLOYMENT_TARGET = 26.0` in `app/ios/Runner.xcodeproj/project.pbxproj` for all build configurations (Debug, Profile, Release)
- [x] 5.2 Bundle identifier already set to `com.cedricziel.assistant` in `project.pbxproj` — kept as-is
- [x] 5.3 `keychain-access-groups` entitlement already present in `DebugProfile.entitlements` and `Release.entitlements` — no changes needed
- [x] 5.4 `Podfile` updated to `platform :ios, '26.0'`
- [x] 5.5 Run `pod install` in `app/ios/` and confirm no dependency conflicts

## 6. CI Integration

- [x] 6.1 Added `build-ios` job to `.github/workflows/flutter.yml` that runs `flutter build ios --no-codesign --release`
- [x] 6.2 Job uses `runs-on: macos-latest` (Xcode required)
- [ ] 6.3 Confirm the job completes successfully in CI on a test PR

## 7. Smoke Testing

- [ ] 7.1 Boot iPhone 15 Simulator and launch the app — confirm connection screen shows remote-only form
- [ ] 7.2 Connect to a running assistant server from the Simulator — confirm chat screen loads
- [ ] 7.3 Boot iPad Pro 13-inch Simulator — confirm `NavigationRail` is shown instead of bottom bar
- [ ] 7.4 Boot iPhone SE (3rd gen) Simulator — confirm no overflow errors
- [ ] 7.5 Terminate and relaunch the Simulator app — confirm saved context persists and chat screen opens directly
