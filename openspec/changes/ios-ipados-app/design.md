## Context

The Flutter app already has an `app/ios/` Xcode project scaffold generated at project init time but has never been published or built in CI. The app supports two server modes: **embedded** (Rust binary bundled inside the macOS app bundle) and **remote** (user-specified URL + token). iOS cannot run arbitrary native binaries, so only the remote mode is supported on iOS/iPadOS.

Current pain points that must be addressed before the iOS target is viable:

- `tray_manager` and `window_manager` are imported unconditionally; they crash at runtime on iOS.
- `EmbeddedServerService` and `EmbeddedServerProvider` are referenced unconditionally in `ConnectionScreen`, which will fail at compile time on iOS because the underlying FFI surface doesn't exist.
- Navigation is implemented as a macOS-style `NavigationRail` with no bottom-bar alternative for iPhone.

## Goals / Non-Goals

**Goals:**

- Ship a working Flutter iOS/iPadOS app that connects to a remote assistant server.
- Reuse 100% of existing feature code (chat, personas, traces, logs, skills, contexts) unchanged.
- Adapt connection onboarding to skip the embedded-server option on iOS.
- Adapt navigation shell to use `BottomNavigationBar` on iPhone and `NavigationRail` on iPad.
- Guard all macOS-only code so the iOS build compiles and passes CI.
- Add `flutter build ios --no-codesign` to CI.

**Non-Goals:**

- Embedded/local server mode on iOS — not technically feasible.
- Push notifications — out of scope for this change.
- App Store submission / code signing setup for distribution — placeholder entitlements only; actual signing is an operator concern.
- Dark-mode or theming overhaul — existing theme unchanged.
- Deep-linking / universal links — out of scope.

## Decisions

### Decision 1: Platform guards via `defaultTargetPlatform` (not separate entry points)

**Chosen**: Use `kIsWeb` and `defaultTargetPlatform == TargetPlatform.macOS` guards throughout existing files rather than forking `main.dart` or using `dart:io Platform`.

**Rationale**: The codebase already uses `kIsWeb` consistently. Adding analogous macOS guards follows the same pattern. A separate iOS entry point would duplicate router and provider setup. `dart:io Platform` is unavailable on web; `defaultTargetPlatform` works on all platforms.

**Alternative considered**: Separate `main_ios.dart` — rejected; duplicates bootstrap logic and creates two diverging code paths.

### Decision 2: Conditional import for macOS-only packages

**Chosen**: Wrap `tray_manager` and `window_manager` initialisation in `if (Platform.isMacOS)` blocks inside `main.dart` and the tray service, guarded by a `!kIsWeb` pre-check.

**Rationale**: Both packages define stubs for non-macOS platforms in their `pubspec.yaml` platform map, so they won't be compiled into the iOS binary. The guard is defensive runtime protection.

**Alternative considered**: Remove packages from `pubspec.yaml` and add them back under `flutter.plugin.platforms` — too invasive, breaks the macOS build.

### Decision 3: Adaptive navigation shell

**Chosen**: Introduce a thin `AdaptiveShell` widget that renders `BottomNavigationBar` when `defaultTargetPlatform == TargetPlatform.iOS` and the screen width is < 600 dp, and `NavigationRail` otherwise (existing macOS/iPad behaviour).

**Rationale**: Flutter's adaptive patterns recommend this split at the 600 dp breakpoint (material.io guidance). Keeps a single widget tree; no conditional routing.

**Alternative considered**: `NavigationDrawer` for both — too different from current macOS UX; rejected.

### Decision 4: Secure token storage on iOS

**Chosen**: `flutter_secure_storage` (already in `pubspec.yaml`) stores tokens in the iOS Keychain via the existing `ContextStore`. No changes to storage layer needed — it is already platform-aware.

**Rationale**: `flutter_secure_storage` already handles iOS Keychain natively. The same code path that works on macOS works on iOS.

### Decision 5: Minimum deployment target iOS 26

**Chosen**: Set `IPHONEOS_DEPLOYMENT_TARGET = 26.0` in the Xcode project.

**Rationale**: iOS 26 (released September 2025) allows the app to use the latest platform APIs without compatibility shims. Targeting the current major release is acceptable for a new app with no existing user base — there is no legacy install base to protect. `flutter_secure_storage` 10.x requires iOS 12+, so no conflict.

## Risks / Trade-offs

- **Keychain entitlement on iOS** → Mitigation: Add `keychain-access-groups` entitlement in `Runner.entitlements`; CI builds with `--no-codesign` so it won't block PRs, only distribution.
- **tray_manager crash on iOS if guard is missing** → Mitigation: CI `flutter build ios --no-codesign` will catch any unguarded import at compile time.
- **NavigationRail overflow on short iPhone screens** → Mitigation: `AdaptiveShell` uses `BottomNavigationBar` on phones; overflow issue (already fixed on macOS in a prior commit) does not apply.
- **No codesigning in CI** → Mitigation: `--no-codesign` flag accepted; actual device testing is a developer responsibility. Document in README.

## Migration Plan

1. Add platform guards to `main.dart`, `tray/`, and `embedded_server/`.
2. Introduce `AdaptiveShell` widget; wire into `app_router.dart`.
3. Update `ConnectionScreen` to hide the mode toggle on iOS.
4. Configure Xcode project: deployment target, bundle ID placeholder, entitlements.
5. Add CI job `flutter build ios --no-codesign`.
6. Smoke-test on iOS Simulator (iPhone 15 + iPad Pro 13-inch).

Rollback: all changes are additive platform guards; removing them reverts to current behaviour on macOS/web.

## Open Questions

- Should the app support iOS 15 (adds ~5% of devices)? Currently targeting 16 — revisit if user demand is clear.
- Should iPad use a `NavigationDrawer` instead of `NavigationRail` in portrait mode? Deferred to a follow-up UX iteration.
