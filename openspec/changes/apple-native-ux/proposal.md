## Why

The Flutter assistant app compiles and runs on iOS/iPadOS (see `ios-ipados-app` change), but it looks and feels like a Material/Android app in an iOS shell. Every widget — navigation bars, toggles, dialogs, transitions, text fields — uses Material Design. iOS users immediately sense this friction: no edge-swipe-to-go-back, no Cupertino toggles, no translucent nav bars, no dark mode, hard-coded light-only colors.

Making the app feel native on Apple touch platforms (iPhone, iPad, Mac via "Designed for iPad") is critical for user trust and retention. Users don't consciously notice good platform fidelity, but they immediately notice its absence.

## What Changes

- Introduce a three-bucket platform abstraction (`cupertino` / `material` / `macos`) that routes UI decisions at the structural level.
- Replace the app root with `CupertinoApp.router` on Apple touch platforms, unlocking iOS page transitions, scroll physics, text selection, and system font (SF Pro) for free.
- Replace structural chrome (navigation bars, tab bars, page scaffolds) with Cupertino equivalents on Apple touch platforms.
- Add dark mode support across all platforms (Material dark theme + CupertinoThemeData brightness).
- Replace hard-coded colors with semantic color tokens that work in both light and dark modes.
- Swap interactive widgets (switches, spinners, dialogs) to their `.adaptive` or Cupertino counterparts on Apple platforms.
- Adopt iOS interaction patterns: CupertinoSliverNavigationBar (large collapsing titles), CupertinoTextField in chat input, haptic feedback on key actions, CupertinoAlertDialog for destructive confirmations.

## Capabilities

### New Capabilities

- `platform-abstraction`: Three-bucket platform enum (`cupertino` / `material` / `macos`) with convenience getters. Used by all adaptive widgets to decide rendering. The `macos` bucket is a forward-looking seam — macOS native stays Material for now but has a dedicated code path for a future macOS-native design pass.
- `adaptive-shell`: Platform-aware navigation shell that renders CupertinoTabBar (compact) or Cupertino sidebar (regular width) on Apple touch platforms, and the existing NavigationBar/NavigationRail on Material platforms.
- `cupertino-chrome`: CupertinoPageScaffold, CupertinoNavigationBar, and CupertinoSliverNavigationBar wrappers for page-level chrome on Apple touch platforms.
- `dark-mode`: Dark theme configuration for both Material (ThemeData with Brightness.dark) and Cupertino (CupertinoThemeData), plus a cleanup of all hard-coded color references to use semantic tokens.
- `adaptive-widgets`: Platform-adaptive interactive widgets — Switch.adaptive, CircularProgressIndicator.adaptive, CupertinoAlertDialog, CupertinoTextField for chat input, haptic feedback.

### Modified Capabilities

- `ios-navigation` (from `ios-ipados-app`): The adaptive navigation shell from the prior change is superseded by the richer `adaptive-shell` capability here, which adds Cupertino styling rather than just responsive layout.

## Impact

- `app/lib/main.dart` — Root widget becomes platform-adaptive (CupertinoApp.router vs MaterialApp.router). Dark theme added.
- `app/lib/shared/nav_shell.dart` — Delegates to platform-aware tab bar / sidebar. Major rewrite of mobile and desktop branches.
- `app/lib/shared/platform/` — New directory with platform detection and adaptive widget helpers.
- `app/lib/features/chat/chat_screen.dart` — CupertinoNavigationBar, CupertinoTextField in input row, semantic colors replacing hard-coded values.
- `app/lib/features/settings/settings_screen.dart` — CupertinoListSection.insetGrouped, Switch.adaptive, CupertinoSliverNavigationBar.
- `app/lib/features/contexts/screens/context_switcher_screen.dart` — CupertinoAlertDialog for delete confirmation, adaptive scaffold.
- `app/lib/features/chat/conversation_list.dart` — CupertinoAlertDialog for delete confirmation, semantic colors.
- All list screens (skills, personas, traces, logs, webhooks, agents, workflows) — Adaptive scaffold with CupertinoSliverNavigationBar on Apple platforms.
- `app/pubspec.yaml` — No new dependencies expected (cupertino_icons already present, Cupertino widgets are in Flutter SDK).

## Scope Boundaries

**In scope:**

- iOS, iPadOS, and Mac Catalyst ("Designed for iPad") — all report `TargetPlatform.iOS`
- Dark mode for all platforms (Material and Cupertino)
- Semantic color cleanup (removing hard-coded Colors.black38 etc.)

**Out of scope:**

- macOS native design pass (toolbar, NSOutlineView-style sidebar, menu bar) — deferred to a future change, but the platform abstraction accommodates it
- Web-specific improvements
- New features or screens — this is purely a UX/rendering change
- Push notifications, deep linking, or other iOS capabilities
