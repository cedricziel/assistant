## Platform Test Matrix

All verification tasks reference this matrix. Each row is a distinct runtime environment.

| ID  | Target       | Device / Environment                  | TargetPlatform | Expected Style |
| --- | ------------ | ------------------------------------- | -------------- | -------------- |
| M1  | iPhone       | iPhone 16 Simulator                   | iOS            | Cupertino      |
| M2  | iPhone SE    | iPhone SE (3rd gen) Simulator         | iOS            | Cupertino      |
| M3  | iPad         | iPad Pro 13-inch Simulator            | iOS            | Cupertino      |
| M4  | Mac Catalyst | iPad app on Mac ("Designed for iPad") | iOS            | Cupertino      |
| M5  | macOS native | `flutter run -d macos`                | macOS          | Material       |
| M6  | Web          | Chrome via `flutter run -d chrome`    | N/A (kIsWeb)   | Material       |

## TDD Discipline

Every implementation task follows Red → Green → Refactor:

1. **Write a failing test first** that asserts the desired behavior.
2. **Confirm it is red** before writing any implementation.
3. **Write the minimum code** to make it green.
4. **Refactor** under green.

Test tasks are marked with 🔴 (write test, confirm red) and implementation tasks with 🟢 (make it green).

---

## Phase 0: Baselines

Capture the current state on each matrix entry before any changes. These screenshots / observations serve as the "before" reference for regression detection.

### 0.1 Baseline Screenshots

- [ ] 0.1.1 **M1 iPhone**: Screenshot chat screen, settings screen, contexts screen, conversation list, "More" overflow sheet
- [ ] 0.1.2 **M2 iPhone SE**: Screenshot chat screen — verify no overflow errors at 375x667dp
- [ ] 0.1.3 **M3 iPad**: Screenshot chat screen (landscape + portrait), settings, nav rail
- [ ] 0.1.4 **M4 Mac Catalyst**: Screenshot chat screen, settings — document current look of iPad-on-Mac
- [ ] 0.1.5 **M5 macOS native**: Screenshot chat screen, settings, tray menu — confirm embedded server works
- [ ] 0.1.6 **M6 Web**: Screenshot chat screen, settings, navigation bar / navigation rail

### 0.2 Baseline Observations

- [ ] 0.2.1 Document current page transition style on each matrix entry (fade, slide, none)
- [ ] 0.2.2 Document current scroll physics on each matrix entry (bouncing, clamping)
- [ ] 0.2.3 Document current dark mode behavior (expected: none — app is light-only)
- [x] 0.2.4 Run `flutter analyze --fatal-infos` and `flutter test` — record baseline pass/fail (analyze: clean, test: 153/153 pass)

---

## Phase 1: Foundation

### 1.1 Platform Abstraction

- [x] 1.1.1 🔴 Write unit test `app/test/unit/platform/platform_test.dart`: assert `AppPlatformStyle` enum has three values (`cupertino`, `material`, `macos`). Assert convenience getters (`isAppleTouch`, `isMacOS`, `isMaterial`) return expected values for each style. Test that `platformStyle` returns `material` when `kIsWeb` is true. Confirm tests fail (class doesn't exist yet).
- [x] 1.1.2 🟢 Create `app/lib/shared/platform/platform.dart` with `AppPlatformStyle` enum, `platformStyle` getter, and convenience getters. Make tests green.
- [x] 1.1.3 Refactor if needed under green.

### 1.2 Adaptive App Root

- [x] 1.2.1 🔴 Write widget test `app/test/widget/platform/adaptive_app_test.dart`: assert that `AdaptiveApp` renders a `MaterialApp` when `platformStyle` is `material`. Assert it renders a `CupertinoApp` when `platformStyle` is `cupertino` (may require overriding `debugDefaultTargetPlatformOverride` in test). Confirm tests fail.
- [x] 1.2.2 🟢 Create `app/lib/shared/platform/adaptive_app.dart` — widget that renders `CupertinoApp.router` when `isAppleTouch`, `MaterialApp.router` otherwise. Make tests green.
- [x] 1.2.3 🟢 Update `main.dart` to use `AdaptiveApp` instead of `MaterialApp.router`.
- [x] 1.2.4 Verify existing tests still pass (`flutter test`). (169/169 pass)

### 1.3 Dark Mode

- [x] 1.3.1 🔴 Write widget test: render `AdaptiveApp` with `MediaQuery` set to `Brightness.dark`. Assert that the resulting `MaterialApp` has a non-null `darkTheme`. Assert that text using semantic color tokens is legible (not white-on-white or black-on-black). Confirm tests fail. (dark_mode_test.dart — 2 tests)
- [x] 1.3.2 🟢 Add `darkTheme` to `MaterialApp.router` using `ColorScheme.fromSeed(brightness: Brightness.dark)`.
- [x] 1.3.3 🟢 Configure `CupertinoThemeData` to respect system brightness. (CupertinoApp inherently resolves brightness from MediaQuery — no explicit darkTheme needed)
- [x] 1.3.4 🔴 Write a lint-style test that greps `app/lib/` for hard-coded `Colors.black` and `Colors.red` usage — assert zero matches (excluding test files). Confirm it fails. (no_hardcoded_colors_test.dart — 3 tests: black, red, white)
- [x] 1.3.5 🟢 Audit and replace all hard-coded `Colors.black*` references with semantic `colorScheme` tokens.
- [x] 1.3.6 🟢 Audit and replace all hard-coded `Colors.red*` references with semantic `colorScheme.error*` tokens.
- [x] 1.3.7 Confirm lint-style test is now green. (3/3 pass)

### 1.4 Phase 1 Verification — Per Matrix Entry

- [ ] 1.4.1 **M1 iPhone**: App launches with CupertinoApp.router. Page transitions are iOS slide. Scroll physics are bouncing. Compare to baseline 0.1.1.
- [ ] 1.4.2 **M1 iPhone (dark)**: Toggle dark mode in Simulator settings. All screens legible — chat bubbles, empty states, error banners, streaming dots. No hard-coded white/black text.
- [ ] 1.4.3 **M2 iPhone SE**: App launches. No overflow errors at 375x667dp. Dark mode legible.
- [ ] 1.4.4 **M3 iPad**: App launches with CupertinoApp.router. Landscape and portrait both work. Dark mode legible.
- [ ] 1.4.5 **M4 Mac Catalyst**: App launches. Cupertino transitions active. Dark mode follows system.
- [ ] 1.4.6 **M5 macOS native**: App launches with MaterialApp.router (no change from baseline). Embedded server still starts. Tray menu works. Dark mode works.
- [ ] 1.4.7 **M6 Web**: App launches with MaterialApp.router (no change from baseline). Dark mode respects `prefers-color-scheme`. PWA update listener still works.
- [x] 1.4.8 Run `flutter analyze --fatal-infos` — zero issues ✓
- [x] 1.4.9 Run `flutter test` — all tests pass (174/174) ✓

---

## Phase 2: Navigation Shell

### 2.1 CupertinoTabBar (Compact)

- [x] 2.1.1 🔴 Write widget test `app/test/widget/platform/adaptive_tab_shell_test.dart`: pump `NavShell` with `debugDefaultTargetPlatformOverride = TargetPlatform.iOS` and screen width 375dp. Assert a `CupertinoTabBar` is found in the widget tree. Assert 5 tab items exist (4 primary + More). Confirm tests fail.
- [x] 2.1.2 🔴 Write widget test: same setup, assert tapping the "More" item triggers `showCupertinoModalPopup` (or finds an overflow menu with the expected destinations). Confirm tests fail.
- [x] 2.1.3 🟢 Create `app/lib/shared/platform/adaptive_tab_shell.dart` — renders `CupertinoTabBar` on Apple touch compact, `NavigationBar` on Material compact. Wire destinations and "More" overflow. Make tests green.

### 2.2 Cupertino Sidebar (Regular Width)

- [x] 2.2.1 🔴 Write widget test: pump `NavShell` with `debugDefaultTargetPlatformOverride = TargetPlatform.iOS` and screen width 1024dp. Assert no `CupertinoTabBar` is found. Assert a sidebar `ListView` with all destinations (primary + overflow) is present. Confirm tests fail.
- [x] 2.2.2 🟢 Implement sidebar list for Apple touch regular-width (>= 768dp) with all destinations. Style to match Apple iPad sidebar pattern. Make tests green.

### 2.3 Non-Regression Tests

- [x] 2.3.1 🔴 Write widget test: pump `NavShell` with `debugDefaultTargetPlatformOverride = TargetPlatform.macOS` and screen width 375dp. Assert `NavigationBar` (Material) is found, not `CupertinoTabBar`. Pump with width 1024dp — assert `NavigationRail` is found. Confirm tests pass (should be green immediately if Material path is untouched — if not, fix).
- [x] 2.3.2 🟢 Update `nav_shell.dart` to delegate to platform-specific navigation based on `platformStyle`.

### 2.4 Phase 2 Verification — Per Matrix Entry

- [ ] 2.4.1 **M1 iPhone**: CupertinoTabBar visible at bottom with 5 items (Chat, Contexts, Skills, Workflows, More). Tapping "More" shows Cupertino-styled popup. Active tab highlighted correctly on all routes including overflow.
- [ ] 2.4.2 **M2 iPhone SE**: CupertinoTabBar fits without overflow at 375dp width.
- [ ] 2.4.3 **M3 iPad (landscape)**: Sidebar visible with all destinations. No bottom tab bar. Selected destination highlighted.
- [ ] 2.4.4 **M3 iPad (portrait)**: Verify correct layout — sidebar or tab bar depending on width.
- [ ] 2.4.5 **M4 Mac Catalyst**: Sidebar visible (regular width). All destinations accessible. Compare to baseline 0.1.4.
- [ ] 2.4.6 **M5 macOS native**: NavigationRail unchanged from baseline. No visual regression.
- [ ] 2.4.7 **M6 Web**: NavigationBar (compact) and NavigationRail (wide) unchanged from baseline. No visual regression.
- [x] 2.4.8 Run `flutter analyze --fatal-infos` — zero issues
- [x] 2.4.9 Run `flutter test` — all tests pass (390/390)

---

## Phase 3: Page Chrome

### 3.1 Adaptive Scaffold & Nav Bar

- [ ] 3.1.1 🔴 Write widget test `app/test/widget/platform/adaptive_scaffold_test.dart`: pump `AdaptiveScaffold` with iOS platform override — assert `CupertinoPageScaffold` found. Pump with macOS override — assert `Scaffold` found. Confirm tests fail.
- [ ] 3.1.2 🔴 Write widget test `app/test/widget/platform/adaptive_nav_bar_test.dart`: pump `AdaptiveNavBar` with iOS override — assert `CupertinoNavigationBar` found. Pump with macOS override — assert `AppBar` found. Confirm tests fail.
- [ ] 3.1.3 🟢 Create `app/lib/shared/platform/adaptive_scaffold.dart` and `adaptive_nav_bar.dart`. Make tests green.

### 3.2 Large Title Screens

- [ ] 3.2.1 🔴 Write widget test: pump `SettingsScreen` with iOS platform override inside a `CustomScrollView`-compatible ancestor. Assert `CupertinoSliverNavigationBar` is found with `largeTitle` text "Settings". Confirm test fails.
- [ ] 3.2.2 🟢 Convert Settings screen to use `CupertinoSliverNavigationBar` on Apple touch. Make test green.
- [ ] 3.2.3 🔴 Write parameterized widget test for remaining list screens (Skills, Personas, Traces, Logs, Contexts, Webhooks, Agents, Analytics, Workflows): for each, assert `CupertinoSliverNavigationBar` found on iOS and `AppBar` found on macOS/web. Confirm tests fail.
- [ ] 3.2.4 🟢 Convert Skills screen to use large title. Make its test green.
- [ ] 3.2.5 🟢 Convert Personas screen. Green.
- [ ] 3.2.6 🟢 Convert Traces screen. Green.
- [ ] 3.2.7 🟢 Convert Logs screen. Green.
- [ ] 3.2.8 🟢 Convert Contexts screen. Green.
- [ ] 3.2.9 🟢 Convert Webhooks screen. Green.
- [ ] 3.2.10 🟢 Convert Agents screen. Green.
- [ ] 3.2.11 🟢 Convert Analytics screen. Green.
- [ ] 3.2.12 🟢 Convert Workflows screen. Green.

### 3.3 Chat Screen Chrome

- [ ] 3.3.1 🔴 Write widget test: pump `ChatScreen` with iOS override. Assert `CupertinoNavigationBar` found (not `CupertinoSliverNavigationBar` — no large title). Assert `AppBar` is NOT found. Confirm test fails.
- [ ] 3.3.2 🟢 Update Chat screen to use `CupertinoNavigationBar` on Apple touch. Make test green.
- [ ] 3.3.3 🔴 Write widget test: pump `ChatScreen` with macOS override. Assert `AppBar` found (Material, unchanged). Confirm test passes immediately (non-regression).

### 3.4 Phase 3 Verification — Per Matrix Entry

- [ ] 3.4.1 **M1 iPhone**: Every list screen shows large title that collapses on scroll. Chat screen shows compact translucent nav bar. Back button appears on detail screens. Edge-swipe-to-go-back works.
- [ ] 3.4.2 **M1 iPhone (dark)**: All screens legible in dark mode with new Cupertino chrome.
- [ ] 3.4.3 **M2 iPhone SE**: Large titles render without overflow. Scroll collapse works on small screen.
- [ ] 3.4.4 **M3 iPad**: Large titles on list screens. Chat screen with sidebar and compact nav bar. Detail screens have back button.
- [ ] 3.4.5 **M4 Mac Catalyst**: Large titles work. Edge-swipe-to-go-back works (if trackpad gesture supported).
- [ ] 3.4.6 **M5 macOS native**: All screens use Scaffold + AppBar unchanged. No visual regression from baseline.
- [ ] 3.4.7 **M6 Web**: All screens use Scaffold + AppBar unchanged. No visual regression from baseline.
- [ ] 3.4.8 Run `flutter analyze --fatal-infos` — zero issues
- [ ] 3.4.9 Run `flutter test` — all tests pass

---

## Phase 4: Widget Polish

### 4.1 Adaptive Switches & Spinners

- [ ] 4.1.1 🔴 Write widget test: pump `SettingsScreen` with iOS override. Assert `CupertinoSwitch` widgets are found (rendered by `SwitchListTile.adaptive`). Pump with macOS override — assert Material `Switch` found. Confirm tests fail.
- [ ] 4.1.2 🟢 Replace 3 `SwitchListTile` in Settings with `SwitchListTile.adaptive`. Make tests green.
- [ ] 4.1.3 🔴 Write widget test: pump a screen that shows a loading state with iOS override. Assert `CupertinoActivityIndicator` found (rendered by `CircularProgressIndicator.adaptive`). Confirm test fails.
- [ ] 4.1.4 🟢 Replace all `CircularProgressIndicator` with `CircularProgressIndicator.adaptive`. Make tests green.

### 4.2 Adaptive Dialogs

- [ ] 4.2.1 🔴 Write unit test `app/test/unit/platform/adaptive_dialog_test.dart`: call `showAdaptiveConfirmDialog` in a test harness with iOS override. Assert `CupertinoAlertDialog` is shown. Call with macOS override — assert `AlertDialog` is shown. Confirm tests fail.
- [ ] 4.2.2 🟢 Create `app/lib/shared/platform/adaptive_dialog.dart` with `showAdaptiveConfirmDialog` helper. Make tests green.
- [ ] 4.2.3 🟢 Update delete context confirmation to use adaptive dialog.
- [ ] 4.2.4 🟢 Update delete conversation confirmation to use adaptive dialog.

### 4.3 Chat Input

- [ ] 4.3.1 🔴 Write widget test: pump `ChatScreen` (or `_InputRow` directly) with iOS override. Assert `CupertinoTextField` found. Pump with macOS override — assert Material `TextField` found. Confirm tests fail.
- [ ] 4.3.2 🟢 Update `_InputRow` to render `CupertinoTextField` on Apple touch (rounded-rect, placeholder). Make tests green.
- [ ] 4.3.3 🔴 Write widget test: with iOS override, enter text in `CupertinoTextField`, trigger submit action. Assert `onSend` callback fires. Verify multiline expansion (set text with newlines, assert `maxLines` behavior). Confirm tests pass (functional parity).

### 4.4 Settings Screen

- [ ] 4.4.1 🔴 Write widget test: pump `SettingsScreen` with iOS override. Assert `CupertinoListSection` found. Pump with macOS override — assert no `CupertinoListSection` (Material layout). Confirm tests fail.
- [ ] 4.4.2 🟢 Update Settings screen to use `CupertinoListSection.insetGrouped` on Apple touch. Make tests green.

### 4.5 Haptic Feedback

- [ ] 4.5.1 🔴 Write test: mock `HapticFeedback` channel. On iOS override, trigger message send. Assert `HapticFeedback.lightImpact` was called. On web, assert it was NOT called. Confirm tests fail.
- [ ] 4.5.2 🟢 Add `HapticFeedback.lightImpact()` on message send, conversation selection, switch toggles (Apple touch only).
- [ ] 4.5.3 🟢 Add `HapticFeedback.mediumImpact()` on destructive actions (delete). Make all haptic tests green.

### 4.6 Phase 4 Verification — Per Matrix Entry

- [ ] 4.6.1 **M1 iPhone**: CupertinoSwitch in settings. iOS spinner. CupertinoAlertDialog on delete. CupertinoTextField in chat input (rounded-rect). Haptic feedback fires on send, select, toggle, delete.
- [ ] 4.6.2 **M1 iPhone (dark)**: All adaptive widgets legible in dark mode. CupertinoTextField, CupertinoAlertDialog, CupertinoListSection all render correctly.
- [ ] 4.6.3 **M2 iPhone SE**: CupertinoTextField input row fits at 375dp width. Keyboard doesn't obscure input.
- [ ] 4.6.4 **M3 iPad**: All adaptive widgets render Cupertino. Settings uses grouped inset list. Dialogs render CupertinoAlertDialog.
- [ ] 4.6.5 **M4 Mac Catalyst**: Adaptive widgets render Cupertino. Haptic feedback is silent (no Taptic Engine — verify no crash).
- [ ] 4.6.6 **M5 macOS native**: All widgets remain Material. No visual regression.
- [ ] 4.6.7 **M6 Web**: All widgets remain Material. No visual regression from baseline.
- [ ] 4.6.8 Run `flutter analyze --fatal-infos` — zero issues
- [ ] 4.6.9 Run `flutter test` — all tests pass

---

## Final Acceptance

- [ ] FA.1 Compare final screenshots to Phase 0 baselines for M5 (macOS native) and M6 (Web) — confirm zero visual regression
- [ ] FA.2 Compare final screenshots to Phase 0 baselines for M1-M4 — confirm Cupertino improvements are visible
- [ ] FA.3 Full dark/light mode walkthrough on M1 (iPhone) — every screen, both modes
- [ ] FA.4 Full dark/light mode walkthrough on M6 (Web) — every screen, both modes
- [ ] FA.5 CI green: `flutter analyze --fatal-infos`, `flutter test`, `flutter build ios --no-codesign`, `flutter build web`, `flutter build macos`
