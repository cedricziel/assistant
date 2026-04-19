## Context

The Flutter assistant app is 100% Material Design — every screen uses `Scaffold`, `AppBar`, `NavigationBar`, `NavigationRail`, `AlertDialog`, `TextField`, `SwitchListTile`, and `CircularProgressIndicator` from `flutter/material.dart`. There are zero Cupertino imports. The theme uses a Google Blue seed color (`#1A73E8`), has no dark mode, and hard-codes colors like `Colors.black38` and `Colors.red.shade50` throughout.

The `ios-ipados-app` change made the app compile and run on iOS, but the UX is indistinguishable from an Android app. This change makes it feel native on Apple touch platforms.

## Goals / Non-Goals

**Goals:**

- Ship a Cupertino-native experience on iOS, iPadOS, and Mac Catalyst.
- Support dark mode on all platforms.
- Eliminate hard-coded colors in favor of semantic tokens.
- Establish a platform abstraction that cleanly separates Apple touch, macOS native, and Material (web) rendering without per-widget `if` statements scattered everywhere.
- Preserve the existing Material experience on web and macOS native — no regressions.

**Non-Goals:**

- macOS-native design language (toolbar, NSOutlineView sidebar, menu bar) — the platform abstraction leaves a seam for this, but implementation is deferred.
- Redesigning screens or adding features — this is a rendering/UX change, not a functional one.
- Adding new dependencies — Cupertino widgets ship with the Flutter SDK, and `cupertino_icons` is already in pubspec.yaml.

## Decisions

### Decision 1: Three-bucket platform enum, not a boolean

**Chosen**: An `AppPlatformStyle` enum with three values: `cupertino`, `material`, `macos`.

```dart
enum AppPlatformStyle { cupertino, material, macos }

AppPlatformStyle get platformStyle {
  if (kIsWeb) return AppPlatformStyle.material;
  if (defaultTargetPlatform == TargetPlatform.macOS) return AppPlatformStyle.macos;
  if (defaultTargetPlatform == TargetPlatform.iOS) return AppPlatformStyle.cupertino;
  return AppPlatformStyle.material;
}
```

**Rationale**: A simple `isApple` boolean would conflate iOS touch (CupertinoTabBar, edge-swipe transitions) with macOS desktop (toolbar, permanent sidebar, no tab bar). These are different design languages even though both are Apple. The three-bucket enum lets macOS native fall through to Material today while reserving a clean insertion point for a future macOS design pass — no refactor needed later.

**Alternative considered**: Two-value enum (`apple` / `material`) — rejected because macOS native and iOS touch have fundamentally different navigation paradigms. Forcing them into one bucket would require a second split later.

### Decision 2: CupertinoApp.router as root on Apple touch platforms

**Chosen**: Conditionally use `CupertinoApp.router` when `platformStyle == cupertino`, and `MaterialApp.router` otherwise.

**Rationale**: Switching the root widget to `CupertinoApp.router` unlocks several iOS-native behaviors for free, with no per-screen changes required:

- **Edge-swipe-to-go-back** on all GoRouter routes (CupertinoPageRoute transitions)
- **Bouncing scroll physics** as the default
- **iOS text selection handles** (the teardrop-shaped selection grips)
- **SF Pro system font** (already the default on iOS, but CupertinoApp makes it explicit)

These are the behaviors iOS users notice most when they're absent.

**Alternative considered**: Keep `MaterialApp.router` everywhere and override transitions per-route — rejected because it requires touching every `GoRoute` and doesn't cover scroll physics or text selection.

### Decision 3: Adapt the shell, not every widget

**Chosen**: Platform adaptation happens at 5-6 structural points (app root, navigation shell, page scaffold, top bar, dialogs, chat input). Content widgets (ListTile, buttons, cards, custom chat bubbles) stay as-is.

```
STRUCTURAL (platform-adaptive):        CONTENT (stay Material):
├── App root                            ├── ListTile
├── Navigation shell (tab bar/sidebar)  ├── ElevatedButton, TextButton
├── Page scaffold                       ├── Card
├── Top navigation bar                  ├── Custom chat bubbles
├── Dialogs / action sheets             ├── Divider
└── Chat text input                     └── Most layout widgets
```

**Rationale**: The "shell vs content" split minimizes the number of files that need platform branching (roughly 6 structural files vs 30+ screens). Material content widgets look acceptable on iOS — it's the structural chrome (navigation, transitions, top bar) that signals "wrong platform" to users. This approach gives 90% of the native feel with 20% of the code changes.

**Alternative considered**: Full Cupertino replacement of every widget — rejected as disproportionate effort with diminishing returns. Also considered `flutter_platform_widgets` package — rejected because it adds a dependency for something achievable with thin wrappers.

### Decision 4: Adaptive widget file organization

**Chosen**: New `app/lib/shared/platform/` directory with focused files:

```
app/lib/shared/platform/
  platform.dart              # AppPlatformStyle enum + getters
  adaptive_app.dart          # CupertinoApp.router or MaterialApp.router builder
  adaptive_scaffold.dart     # CupertinoPageScaffold or Scaffold
  adaptive_nav_bar.dart      # CupertinoNavigationBar or AppBar
  adaptive_tab_shell.dart    # CupertinoTabBar or NavigationBar (bottom nav)
  adaptive_dialog.dart       # showAdaptiveConfirmDialog helper
```

**Rationale**: Each file handles one structural decision. Screens import only what they need. The `platform.dart` file is the single source of truth for platform detection — no `defaultTargetPlatform` checks scattered across feature code.

**Alternative considered**: Single `adaptive_widgets.dart` mega-file — rejected because it would grow unwieldy and create import coupling.

### Decision 5: Dark mode via ColorScheme.fromSeed with brightness

**Chosen**: Add `darkTheme` to `MaterialApp.router` and set `CupertinoThemeData` brightness based on `MediaQuery.platformBrightness`. Replace all hard-coded color references with semantic `colorScheme` tokens.

Hard-coded colors to replace:

- `Colors.black38` → `colorScheme.onSurfaceVariant`
- `Colors.black54` → `colorScheme.onSurfaceVariant`
- `Colors.black45` → `colorScheme.onSurfaceVariant`
- `Colors.black26` → `colorScheme.outlineVariant`
- `Colors.black12` → `colorScheme.outlineVariant`
- `Colors.red` / `Colors.red.shade50` / `Colors.red.shade700` → `colorScheme.error` / `colorScheme.errorContainer` / `colorScheme.onErrorContainer`

**Rationale**: Dark mode is table stakes on iOS — users expect it. Hard-coded light-only colors break catastrophically in dark mode (invisible text, blinding white backgrounds). Semantic tokens from `ColorScheme` automatically adapt to both brightness modes.

**Alternative considered**: Manual dark color palette — rejected; `ColorScheme.fromSeed` with `Brightness.dark` generates a coherent dark palette from the same seed color, keeping light and dark themes visually consistent.

### Decision 6: CupertinoSliverNavigationBar for list screens

**Chosen**: List-style screens (Settings, Skills, Personas, Traces, Logs, Webhooks, Agents, Analytics, Workflows) use `CupertinoSliverNavigationBar` with `largeTitle` on Apple touch platforms. This gives the signature iOS "large title that collapses on scroll" pattern.

**Rationale**: This is the most recognizable iOS navigation pattern — used in Settings, Messages, Mail, Music, App Store. It signals "this is a native iOS app" more than any other single element.

**Screens affected**: Settings, Skills, Personas, Traces, Logs, Webhooks, Agents, Analytics, Workflows, Contexts. Chat screen uses a regular `CupertinoNavigationBar` (no large title — it's a conversation view, not a list).

### Decision 7: Navigation shell — CupertinoTabBar + sidebar

**Chosen**: On Apple touch platforms:

- **Compact width (< 768dp)**: `CupertinoTabBar` at bottom with 4 primary destinations + "More"
- **Regular width (>= 768dp)**: Sidebar list with all destinations (like iPad Settings / Mail)

On Material platforms: existing `NavigationBar` and `NavigationRail` unchanged.

**Rationale**: This matches Apple's own multi-tab iPad apps. `CupertinoTabBar` on iPhone is the expected pattern. On iPad/Catalyst, a sidebar list (not `NavigationRail`) looks native. The existing 768dp breakpoint is preserved.

**Alternative considered**: Keep `NavigationRail` on wide Apple screens — rejected because `NavigationRail` is a Material widget that looks non-native on iPad. A simple `ListView` with styled list tiles in a sidebar matches the Apple paradigm better.

## Phased Migration Plan

### Phase 1: Foundation

Platform abstraction + `CupertinoApp.router` + dark mode + semantic color cleanup.

**Impact**: iOS transitions, scroll physics, text selection, and dark mode all work. Every screen benefits from the root widget change without any per-screen modifications.

### Phase 2: Navigation Shell

`CupertinoTabBar` on compact, Cupertino sidebar on regular width. Existing Material navigation preserved for web.

**Impact**: Navigation feels native on iPhone and iPad.

### Phase 3: Page Chrome

`CupertinoPageScaffold` + `CupertinoNavigationBar` on all screens. `CupertinoSliverNavigationBar` (large titles) on list screens.

**Impact**: Every screen has native-looking chrome.

### Phase 4: Widget Polish

`.adaptive` switches/spinners, `CupertinoTextField` in chat input, `CupertinoAlertDialog` for confirmations, haptic feedback, pull-to-refresh with `CupertinoSliverRefreshControl`.

**Impact**: Interaction-level native feel.

## Risks / Trade-offs

- **Mixing Material content inside CupertinoApp** — Flutter officially supports this. Material widgets inherit theme data correctly inside `CupertinoApp` via `MaterialBasedCupertinoThemeData`. Tested pattern in Flutter documentation.
- **GoRouter + CupertinoApp.router compatibility** — GoRouter explicitly supports `CupertinoApp.router`. Page transitions are automatically Cupertino-style when the root is `CupertinoApp`.
- **Increased code in nav_shell.dart** — The navigation shell gains a platform branch. Mitigated by extracting the Cupertino path into `adaptive_tab_shell.dart`.
- **Regression risk on web/macOS** — All changes are guarded by `platformStyle`. Web and macOS native code paths are untouched. CI runs `flutter analyze` and `flutter test` on all platforms.
- **CupertinoSliverNavigationBar requires CustomScrollView** — List screens currently use `Scaffold(body: ListView(...))`. On Apple platforms, they'll need `CupertinoPageScaffold(child: CustomScrollView(slivers: [...]))`. The content is the same, just wrapped differently.

## Open Questions

- Should the seed color change on Apple platforms? `#1A73E8` (Google Blue) is very "Google." `CupertinoColors.systemBlue` would be more native, but a custom brand color could work too.
- Should the chat input use `CupertinoTextField` with a rounded pill shape (iMessage-style) or keep the outlined rectangle? The pill shape is more iOS-native but diverges from the Material version more significantly.
- For the iPad sidebar: should it use `CupertinoListSection` (grouped inset style) or a plain list? Apple's own apps vary — Settings uses grouped, Mail uses plain.
