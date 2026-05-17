## 1. Phase 1 — Expand the home-grown façade (one PR)

- [x] 1.1 Write failing widget test for `AdaptiveSliverNavBar` covering Cupertino, Material, and macOS branches
- [x] 1.2 Implement `app/lib/shared/platform/adaptive_sliver_nav_bar.dart` — minimum code to pass 1.1
- [x] 1.3 Write failing widget test for `AdaptiveListSection` (Cupertino insetGrouped vs Material section header)
- [x] 1.4 Implement `app/lib/shared/platform/adaptive_list_section.dart`
- [x] 1.5 Write failing widget test for `AdaptiveListTile` (CupertinoListTile vs ListTile)
- [x] 1.6 Implement `app/lib/shared/platform/adaptive_list_tile.dart`
- [x] 1.7 Write failing widget test for `AdaptiveSwitch` (CupertinoSwitch vs Material Switch)
- [x] 1.8 Implement `app/lib/shared/platform/adaptive_switch.dart`
- [x] 1.9 Write failing widget test for `AdaptiveSwitchTile` (CupertinoListTile + Switch vs SwitchListTile)
- [x] 1.10 Implement `app/lib/shared/platform/adaptive_switch_tile.dart`
- [x] 1.11 Write failing widget test for `AdaptiveActionSheet` (CupertinoActionSheet vs modal bottom sheet)
- [x] 1.12 Implement `app/lib/shared/platform/adaptive_action_sheet.dart`
- [x] 1.13 Write failing widget test for `AdaptiveButton` (CupertinoButton vs FilledButton/TextButton variants)
- [x] 1.14 Implement `app/lib/shared/platform/adaptive_button.dart`
- [x] 1.15 Write failing widget test for `AdaptiveIcon` lookup (Cupertino vs Material icon mapping)
- [x] 1.16 Implement `app/lib/shared/platform/adaptive_icon.dart` — chose builder-with-both-icons pattern (constructor takes `cupertino` + `material` IconData; matches existing `AdaptiveContextMenuAction` style)
- [x] 1.17 Write failing widget test for `AdaptiveTextField` (CupertinoTextField vs Material TextField with rounded decoration)
- [x] 1.18 Implement `app/lib/shared/platform/adaptive_text_field.dart`
- [x] 1.19 Write failing widget test for `AdaptiveSlider` (CupertinoSlider vs Material Slider)
- [x] 1.20 Implement `app/lib/shared/platform/adaptive_slider.dart`
- [x] 1.21 Write failing widget test for `AdaptiveSnackBar` (Cupertino-style overlay vs Material SnackBar)
- [x] 1.22 Implement `app/lib/shared/platform/adaptive_snack_bar.dart`
- [x] 1.23 Run `make lint-flutter && make test-flutter` — green (933 tests pass, dart_pre_commit clean)
- [x] 1.24 Run `cd app && flutter analyze --fatal-infos` — zero issues
- [x] 1.25 Open Phase 1 PR with stacked-PR-base flag set; merge before Phase 2 (PR #789)

## 2. Phase 2.1 — Trivial sweep (1 PR)

- [ ] 2.1.1 ~~Remove the vestigial `package:flutter/material.dart` import from `app/lib/shared/error_screen.dart`~~ **Deferred.** Original inventory was wrong: the file legitimately uses `Material()` widget (background colour), `Theme.of(context)`, `Icons.error_outline`, and `ExpansionTile`. It's a special-case bootstrap widget rendered via `ErrorWidget.builder` before the app's normal widget tree may be available. Decision: add `app/lib/shared/error_screen.dart` to the Phase 4 lint allowlist as a known exception. Migration to `AdaptiveScaffold` + `AdaptiveIcon` is possible but requires a small visual change (AppBar on Material path) and is out of scope here.

## 2b. Phase 2.0 — Façade barrel re-export shim (1 PR, see Decision 8)

- [x] 2b.1 Create `app/lib/shared/platform/widgets.dart` barrel that re-exports `flutter/widgets.dart` (everything), a curated subset of `flutter/material.dart` via `show` clause, `CupertinoIcons` from `flutter/cupertino.dart`, and all `adaptive_*.dart` wrappers + `platform.dart`. Excluded from material's show list: any widget with an Adaptive wrapper (Scaffold, AppBar, ListTile, etc.) and Colors (already gated).
- [x] 2b.2 Write a widget/compile test that imports only the barrel and verifies the curated widgets are reachable. Catches accidental show-list regressions.
- [x] 2b.3 Update `traces_screen.dart` (migrated in PR #793) to import the barrel instead of `flutter/material.dart` directly — barrel surfaced 3 more uses to fix: `Scaffold` → `AdaptiveScaffold`, `Colors.green` → `colorScheme.tertiary` (the success token), `FilledButton` → `AdaptiveButton.filled`. The barrel's `show` list correctly forced all three through the façade.
- [ ] 2b.5 Add a "How to add a façade wrapper" section to `app/lib/shared/platform/README.md` (create if absent) explaining the ratchet: every new Adaptive wrapper drops one re-export from the barrel. Defer to Phase 4.

## 3. Phase 2.2 — Sliver-nav list screens (9 stacked PRs, one per screen)

- [x] 3.1 `traces_screen.dart`: replace inline `if (isAppleTouch) CupertinoSliverNavigationBar` with `AdaptiveSliverNavBar`; drop `package:flutter/cupertino.dart` import; existing tests stay green. Unified both platform paths into a single CustomScrollView with the wrapper. Material visual change: SliverAppBar.large (collapsing) replaces fixed AppBar, and the explicit `IconButton(arrow_back) → /chat` is dropped (top-level nav screen — sidebar/tab bar already provides home access; matches the existing iOS path which had no back button).
- [x] 3.2 `logs_screen.dart`: same treatment. Switched to the barrel and `AdaptiveSliverNavBar`. Surfaced `Colors.{orange,blue,grey,purple}.shade*` for the severity chip — extracted into `lib/shared/theme/log_severity_colors.dart` as a design token helper (inside the theme dir so it's exempt from `check_no_raw_colors.sh`). Also `Scaffold` → `AdaptiveScaffold`, `TextField` → `AdaptiveTextField` (with the inline progress spinner moved from `suffixIcon` to the nav-bar actions), `FilledButton` → `AdaptiveButton.filled`. Same Material visual change as traces: fixed AppBar → SliverAppBar.large, explicit back button dropped.
- [x] 3.3 `agents_screen.dart`: switched to barrel + `AdaptiveSliverNavBar`. Cupertino's `CupertinoButton(child: Icon(CupertinoIcons.refresh))` collapsed to plain `IconButton(Icons.refresh)` — both paths now use the same Material refresh icon (icon glyph is visually identical across platforms; no AdaptiveIcon pair needed). Added `CircleAvatar` to the barrel show-list (Material-only, used in 8 feature files). Also: `Scaffold` → `AdaptiveScaffold`, `ListTile` → `AdaptiveListTile`, `FilledButton` → `AdaptiveButton.filled`.
- [x] 3.4 `workflows_screen.dart`: switched to barrel + AdaptiveSliverNavBar. The active/inactive badge previously used `Colors.green.withAlpha(...)` and `Colors.grey.withAlpha(...)` directly — migrated to `colorScheme.tertiary` (success token) and `colorScheme.onSurfaceVariant` (dim outline), with `tertiaryContainer` and `surfaceContainerHighest` for the soft fills. FAB stays as `FloatingActionButton` (already in barrel; no Cupertino equivalent). Added a 80px-tall trailing SliverToBoxAdapter so the FAB doesn't cover the last row.
- [ ] 3.5 `personas_screen.dart`: same treatment
- [ ] 3.6 `skills_screen.dart`: same treatment
- [ ] 3.7 `webhooks_screen.dart`: same as 3.3
- [ ] 3.8 `analytics_screen.dart`: same as 3.3
- [ ] 3.9 `contexts/screens/context_switcher_screen.dart`: same treatment + adopt `AdaptiveActionSheet` for delete confirmation

## 4. Phase 2.3 — Settings screen (1 PR)

- [ ] 4.1 `settings_screen.dart`: replace `CupertinoListSection`/`CupertinoListTile`/`CupertinoSwitchTile` with `AdaptiveListSection`/`AdaptiveListTile`/`AdaptiveSwitchTile`; drop `package:flutter/cupertino.dart` import; update Playwright screenshot baseline if visual diff is non-zero on web

## 5. Phase 2.4 — Chat screen (likely small stack inside one PR)

- [ ] 5.1 Spike: read `chat_screen.dart` (2229 LOC) and decide whether to extract composer + message list into separate files before the import sweep; record decision in PR description
- [ ] 5.2 If extracted: move composer to `app/lib/features/chat/chat_composer.dart` with widget test; verify chat screen still renders identically
- [ ] 5.3 If extracted: move message list to `app/lib/features/chat/chat_messages_view.dart` with widget test
- [ ] 5.4 Replace `CupertinoTextField` with `AdaptiveTextField`
- [ ] 5.5 Replace `CupertinoNavigationBar` with `AdaptiveNavBar` (regular, non-large-title — per `cupertino-chrome` REQ-4)
- [ ] 5.6 Replace Material `Switch`, `Slider`, `SnackBar` with `AdaptiveSwitch`/`AdaptiveSlider`/`AdaptiveSnackBar`
- [ ] 5.7 Drop both `package:flutter/cupertino.dart` and any unnecessary `package:flutter/material.dart` imports
- [ ] 5.8 Update widget tests; update Playwright baselines if visual diff is non-zero on web

## 6. Phase 2.5 — Nav shell (1 PR, last in Phase 2)

- [ ] 6.1 Write widget tests for both compact (bottom bar) and regular (sidebar) breakpoints on iOS and Material if not already present
- [ ] 6.2 Replace inline `CupertinoSidebar`, `CupertinoModalPopup`, `CupertinoActionSheet` with façade wrappers (`AdaptiveActionSheet` for the "More" overflow sheet per `adaptive-shell` REQ-4)
- [ ] 6.3 Replace inline `CupertinoButton`/`CupertinoIcons`/`CupertinoColors` with `AdaptiveButton`/`AdaptiveIcons`
- [ ] 6.4 Drop `package:flutter/cupertino.dart` from `nav_shell.dart`
- [ ] 6.5 Verify route-based active-state highlighting still works on iPhone, iPad, web, and macOS (per `adaptive-shell` REQ-6)
- [ ] 6.6 Update Playwright baselines for nav_shell screenshots if diff is non-zero

## 7. Phase 3 — Wire adaptive_platform_ui inside façade (one PR or small stack)

- [ ] 7.1 Add `adaptive_platform_ui` to `app/pubspec.yaml` with an exact pin (e.g. `adaptive_platform_ui: 0.1.107` — no caret); run `flutter pub get`
- [ ] 7.2 Spike: build a minimal iOS-only test page that renders the package's native nav bar + tab bar; verify on iPhone Simulator running iOS 26 and on Apple Silicon Mac (Designed for iPad). Record findings in PR description
- [ ] 7.3 Modify `app/lib/shared/platform/adaptive_nav_bar.dart` internals: on `isAppleTouch`, render the package's native iOS 26 nav bar (do NOT use `iOS26NativeSearchTabBar`). Material/macOS branches unchanged
- [ ] 7.4 Modify `app/lib/shared/platform/adaptive_sliver_nav_bar.dart` internals: same treatment
- [ ] 7.5 Replace nav_shell's iOS tab bar with the package's `iOS26NativeTabBar` (or equivalent non-search variant). EXPLICITLY skip `iOS26NativeSearchTabBar` (upstream broken)
- [ ] 7.6 A/B test `AdaptiveTextField` vs Flutter Cupertino on iOS 26 hardware; if package version wins, swap; record decision in wrapper docstring + this task
- [ ] 7.7 A/B test `AdaptiveSwitch` vs Flutter Cupertino on iOS 26; same protocol as 7.6
- [ ] 7.8 A/B test `AdaptiveSlider` vs Flutter Cupertino on iOS 26; same protocol
- [ ] 7.9 A/B test `AdaptiveSnackBar` vs Flutter Cupertino on iOS 26; same protocol
- [ ] 7.10 Re-baseline iOS goldens; document the visual diff in PR description
- [ ] 7.11 Add a "developer notes" section to `app/README.md` documenting the package's hot-reload caveats
- [ ] 7.12 Verify web + macOS builds: no `adaptive_platform_ui` code path executes; `flutter analyze --fatal-infos` passes for both targets
- [ ] 7.13 Run `make lint-flutter && make test-flutter`

## 8. Phase 4 — Lint enforcement (one PR)

- [ ] 8.1 Add `custom_lint` to `app/dev_dependencies` (exact pin)
- [ ] 8.2 Create a local lint package at `app/packages/platform_facade_lints/` with a single rule `avoid_direct_platform_imports` that fires on `package:flutter/cupertino.dart`, `package:flutter/material.dart`, and `package:adaptive_platform_ui/*` imports outside the allowlist
- [ ] 8.3 Configure the rule's allowlist to `app/lib/shared/platform/**` and `app/lib/main.dart`
- [ ] 8.4 Wire `custom_lint` into `app/analysis_options.yaml`
- [ ] 8.5 Update `make lint-flutter` to invoke `dart run custom_lint` and fail on any violation
- [ ] 8.6 Update `.github/workflows/flutter.yml` to run the lint
- [ ] 8.7 Write a failing unit test for the lint rule against a fixture file with a banned import; confirm red
- [ ] 8.8 Make the test pass
- [ ] 8.9 Run `make lint-flutter` against the codebase post-Phase-2/3 — must report zero violations
- [ ] 8.10 Document the rule and how to add new façade wrappers in `app/lib/shared/platform/README.md` (create if absent)

## 9. Final verification

- [ ] 9.1 Grep `app/lib/` for `package:flutter/cupertino.dart` — must return only files under `app/lib/shared/platform/`
- [ ] 9.2 Grep `app/lib/features/` and `app/lib/shared/` (excluding `lib/shared/platform/`) for `if (Platform.isIOS)`, `Platform.isMacOS` in widget files, and `defaultTargetPlatform` — must return zero rendering-related hits (service-layer hits remain permitted)
- [ ] 9.3 Run `make lint && make format` (Rust side untouched, but the precommit hook covers the whole tree)
- [ ] 9.4 Run `make lint-flutter && make test-flutter`
- [ ] 9.5 Run `flutter analyze --fatal-infos` in `app/`
- [ ] 9.6 Manual visual QA on iPhone (iOS 26), iPad (iOS 26 / iPadOS), Apple Silicon Mac (Designed for iPad), Chrome browser, and `flutter run -d macos`
- [ ] 9.7 Update `app/lib/shared/platform/README.md` with the final wrapper catalogue and the discipline rules
