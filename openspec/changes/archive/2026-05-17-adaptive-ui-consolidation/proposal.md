## Why

The Flutter app has a working adaptive façade at `app/lib/shared/platform/` (5 wrappers, 389 LOC), but 13 feature screens bypass it and import both `flutter/material.dart` and `flutter/cupertino.dart` directly. An inventory shows the façade is under-built (no `AdaptiveSliverNavBar`, no `AdaptiveListSection`, etc.), not wrong — screens go dual-import because the right wrapper does not exist. At the same time, the iOS deployment target is locked to 26.0, so iOS 26 Liquid Glass is the live visual target, and Flutter's Cupertino widgets only partially track it. We need to close the wrapper gaps, sweep feature code onto the façade, and adopt the `adaptive_platform_ui` package strictly inside the façade for the iOS-26-native chrome it provides — without leaking it into feature code.

## What Changes

- **Expand the home-grown façade** at `app/lib/shared/platform/` with the missing wrappers identified by inventory: `AdaptiveSliverNavBar`, `AdaptiveListSection`, `AdaptiveListTile`, `AdaptiveSwitch`, `AdaptiveSwitchTile`, `AdaptiveActionSheet`, `AdaptiveButton`, `AdaptiveIcons`, `AdaptiveTextField`, `AdaptiveSlider`, `AdaptiveSnackBar`. Implementations use plain Flutter Cupertino + Material — no new dependency in this phase.
- **Migrate 13 feature screens** off direct `flutter/cupertino.dart` and `flutter/material.dart` imports onto the façade. One stacked PR per screen; no behavior change per screen. Order: `error_screen` (trivial) → 8 sliver-nav list screens → `settings_screen` → `chat_screen` → `nav_shell`.
- **Adopt `adaptive_platform_ui` inside façade wrappers only.** On iOS 26 the `AdaptiveNavBar`, `AdaptiveSliverNavBar`, and nav-shell tab bar render the package's UIKit-embedded chrome. Web, macOS, and Android branches remain on Flutter native widgets. Pin to an exact version (no caret). **EXPLICITLY DO NOT** adopt `iOS26NativeSearchTabBar` (upstream README flags it as broken — lifecycle, navigation, hot-reload, memory leaks). For input widgets (`AdaptiveTextField`, `AdaptiveSwitch`, `AdaptiveSlider`), A/B against Flutter Cupertino under iOS 26 and adopt only where the package visibly beats Flutter — otherwise stay on Flutter.
- **Enforce the façade via custom_lint.** Add a rule that bans `import 'package:flutter/cupertino.dart'` and `import 'package:flutter/material.dart'` from any file outside `app/lib/shared/platform/` and the root `app/lib/main.dart`. Wire into `make lint-flutter` and CI.
- **No visual redesign.** Every screen should look identical to today, modulo the iOS 26 chrome upgrade on iOS hardware.
- **macOS native rendering pass stays out of scope.** The `AppPlatformStyle.macos` bucket continues to fall through to Material; a future change will address it.

## Capabilities

### New Capabilities

- `platform-facade-discipline`: A lint-enforced rule that all platform-conditional rendering in `app/lib/` flows through `app/lib/shared/platform/`. Feature code does not import `flutter/cupertino.dart` directly and does not branch on `defaultTargetPlatform`/`Platform.is*` for rendering decisions.

### Modified Capabilities

- `cupertino-chrome`: Add `AdaptiveSliverNavBar` as a first-class façade widget (currently REQ-3 mandates `CupertinoSliverNavigationBar` on list screens without providing a wrapper, forcing dual-imports). Add a requirement that on iOS 26 the chrome wrappers render the `adaptive_platform_ui` UIKit-embedded nav bars, while web/macOS/Android branches stay on Flutter natives.
- `adaptive-widgets`: Expand the widget catalogue beyond switches/dialogs/text-field/spinner to include list sections, list tiles, switch tiles, action sheets, sliders, snack bars, and buttons. Mandate that every adaptation lives behind a façade wrapper rather than as scattered `.adaptive` calls or inline platform branches in feature code.

## Impact

- **Code**: ~13 feature screens edited (Phase 2 sweep, one stacked PR each); ~11 new wrapper files under `app/lib/shared/platform/`; existing `AdaptiveNavBar`/`AdaptiveScaffold`/`AdaptiveDialog` internals modified in Phase 3 to gate on iOS 26.
- **Dependencies**: Adds `adaptive_platform_ui` (pinned exact version) and `custom_lint` + a local lint package for the import-discipline rule. No removals.
- **Build & CI**: `make lint-flutter` gains the custom_lint pass. iOS golden-test baselines re-captured once after Phase 3 lands (platform-view embedding may shift pixels). Existing `flutter analyze --fatal-infos` and `flutter test` stay green throughout.
- **Risk**: `adaptive_platform_ui` is v0.1.x and solo-published (medialyra.com); the change mitigates this by (a) pinning exact version, (b) confining usage to façade internals so a swap is local, (c) skipping the upstream-broken `iOS26NativeSearchTabBar`, and (d) leaving Flutter natives as the fallback on every non-iOS-26 path.
- **No runtime regressions for older iOS** — iOS 26 is already the deployment-target floor.
- **No web/macOS regressions** — those branches remain on Flutter widgets; the package is never invoked there.
