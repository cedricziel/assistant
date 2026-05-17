## Cupertino Page Chrome

### Summary

Page-level structural widgets (scaffold, navigation bar) use Cupertino equivalents on Apple touch platforms, giving each screen a native iOS look.

### Requirements

- **REQ-1**: Provide an `AdaptiveScaffold` widget that renders `CupertinoPageScaffold` on Apple touch and `Scaffold` on Material/macOS.
- **REQ-2**: Provide an `AdaptiveNavBar` widget that renders `CupertinoNavigationBar` on Apple touch and `AppBar` on Material/macOS. Must support: title, leading widget, trailing actions.
- **REQ-3**: List-style screens (Settings, Skills, Personas, Traces, Logs, Webhooks, Agents, Analytics, Workflows, Contexts) use `CupertinoSliverNavigationBar` with `largeTitle` on Apple touch platforms. The large title collapses on scroll.
- **REQ-4**: The Chat screen uses a regular (non-large-title) `CupertinoNavigationBar` on Apple touch platforms, since it is a conversation view, not a list.
- **REQ-5**: `CupertinoNavigationBar` should use translucent background (the default blur effect).
- **REQ-6**: On Material platforms, all screens continue to use `Scaffold` + `AppBar` unchanged.
- **REQ-7**: Back navigation buttons are handled automatically by `CupertinoNavigationBar` (no manual back button needed on Apple touch).
- **REQ-8**: Screens that use `CupertinoSliverNavigationBar` must convert their body from `ListView` to `CustomScrollView` with `SliverList` (or equivalent sliver widgets).

### Acceptance Criteria

- List screens on iPhone Simulator show large title that collapses on scroll.
- Chat screen on iPhone Simulator shows compact (non-large-title) nav bar with blur.
- Back button appears automatically when navigating to detail screens.
- Web browser shows existing AppBar / Scaffold unchanged.
- No visual regression on macOS native build.
## Requirements
### Requirement: AdaptiveSliverNavBar wrapper

The façade at `app/lib/shared/platform/` SHALL provide an `AdaptiveSliverNavBar` widget that renders `CupertinoSliverNavigationBar` (with `largeTitle`) on Apple touch platforms, a `SliverAppBar` on Material/macOS, and supports the same title/leading/trailing surface as the existing `AdaptiveNavBar`. Screens that need the iOS large-title-collapse-on-scroll pattern SHALL use this wrapper instead of importing `package:flutter/cupertino.dart` directly.

#### Scenario: List screen uses the wrapper on iOS

- **WHEN** a list screen (Traces, Logs, Personas, Skills, Webhooks, Agents, Analytics, Workflows, Contexts) is built on an iPhone or iPad running iOS 26
- **THEN** the rendered top chrome SHALL be a `CupertinoSliverNavigationBar` with the large title visible at rest
- **THEN** scrolling the body downward SHALL collapse the large title into the standard nav bar height
- **THEN** the screen file SHALL NOT import `package:flutter/cupertino.dart`

#### Scenario: Same screen on web

- **WHEN** the same list screen is built in a desktop browser
- **THEN** the rendered top chrome SHALL be a `SliverAppBar` with Material styling
- **THEN** the screen file SHALL NOT import `package:flutter/cupertino.dart`

#### Scenario: Same screen on macOS native

- **WHEN** the same list screen is built in the `flutter build macos` app
- **THEN** the rendered top chrome SHALL be a `SliverAppBar` (Material fallback for the `macos` bucket)
- **THEN** no regression vs the pre-change Material rendering is visible

### Requirement: iOS 26 native chrome via adaptive_platform_ui

On Apple touch platforms, `AdaptiveNavBar` and `AdaptiveSliverNavBar` SHALL render the `adaptive_platform_ui` package's UIKit-embedded native iOS 26 navigation bars (Liquid Glass blur). The package SHALL only be invoked from within `app/lib/shared/platform/`. The `iOS26NativeSearchTabBar` widget from the package SHALL NOT be used.

#### Scenario: iPhone running iOS 26 sees Liquid Glass nav bar

- **WHEN** any screen using `AdaptiveNavBar` or `AdaptiveSliverNavBar` is built on iOS 26 hardware
- **THEN** the nav bar SHALL render with the iOS 26 Liquid Glass blur backdrop produced by the package's platform view
- **THEN** the underlying widget tree SHALL be the `adaptive_platform_ui` native nav bar, not Flutter's `CupertinoNavigationBar`

#### Scenario: Web browser ignores the package

- **WHEN** the same screen is built in a desktop browser
- **THEN** the package code path SHALL NOT execute
- **THEN** the rendered chrome SHALL be `AppBar` or `SliverAppBar` (Material)

#### Scenario: macOS native ignores the package

- **WHEN** the same screen is built via `flutter build macos`
- **THEN** the package code path SHALL NOT execute
- **THEN** the rendered chrome SHALL be `AppBar` or `SliverAppBar` (Material fallback for the `macos` bucket)

#### Scenario: SearchTabBar is not used

- **WHEN** the codebase is grepped for `iOS26NativeSearchTabBar`
- **THEN** no references SHALL exist in `app/lib/`
- **THEN** the nav-shell tab bar SHALL use a non-search-pill variant on iOS

### Requirement: Feature code does not import flutter/cupertino directly

No file under `app/lib/features/**` or `app/lib/shared/**` (excluding `app/lib/shared/platform/**`) SHALL import `package:flutter/cupertino.dart`. The façade is the only place that may import Cupertino widgets directly.

#### Scenario: Lint catches a stray Cupertino import

- **WHEN** a developer adds `import 'package:flutter/cupertino.dart';` to a file under `app/lib/features/` or `app/lib/shared/` (outside `lib/shared/platform/`)
- **THEN** `make lint-flutter` SHALL fail with a clear diagnostic naming the file and pointing to the façade
- **THEN** the CI Flutter workflow SHALL block merge

#### Scenario: Wrapper file is allowed

- **WHEN** a file under `app/lib/shared/platform/` imports `package:flutter/cupertino.dart`
- **THEN** the lint SHALL accept it without error

