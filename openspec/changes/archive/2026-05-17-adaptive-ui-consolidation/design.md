## Context

The Flutter app at `app/` ships to web (rust-embed SPA), macOS desktop, and iOS 26+ (Designed for iPad on Apple Silicon). A home-grown adaptive façade exists at `app/lib/shared/platform/` with five wrappers (`AdaptiveApp`, `AdaptiveScaffold`, `AdaptiveNavBar`, `showAdaptiveConfirmDialog`, `AdaptiveMediaContextMenu`) keyed off a three-bucket `AppPlatformStyle` enum (`cupertino` / `material` / `macos`). The previous `cupertino-chrome` and `adaptive-widgets` capabilities established the façade pattern and the screens that should adopt iOS-native chrome.

Today, despite the façade, 13 feature screens import `flutter/material.dart` and `flutter/cupertino.dart` directly. Inventory shows the bypasses fall into four patterns:

- **Pattern A (9 screens)** — `CupertinoSliverNavigationBar` inline + Material `Scaffold` fallback. Wrapper does not exist.
- **Pattern B (1 screen — settings)** — `CupertinoListSection` / `CupertinoListTile` / `CupertinoSwitch` inline. Wrappers do not exist.
- **Pattern C (1 screen — chat, 2229 LOC)** — `CupertinoTextField`, `CupertinoNavigationBar` + Material `Switch`, `Slider`, `SnackBar`, `Drawer`, `Dialog`. Multiple wrappers missing.
- **Pattern D (1 screen — nav_shell, 1207 LOC)** — `CupertinoSidebar`, `CupertinoModalPopup`, `CupertinoActionSheet` + Material `NavigationBar`, `NavigationRail`. The Liquid Glass tab bar would land here.

The `adaptive_platform_ui` package (pub.dev v0.1.107) embeds real UIKit views on iOS 26 to provide Liquid Glass nav bars and tab bars that Flutter Cupertino does not yet fully match. Web and macOS are not covered. The package is solo-published by `medialyra.com` and one of its widgets (`iOS26NativeSearchTabBar`) is documented as broken.

## Goals / Non-Goals

**Goals:**

- Feature code in `app/lib/features/**` and `app/lib/shared/**` (except `lib/shared/platform/`) MUST NOT import `package:flutter/cupertino.dart`. Currently 18 files do.
- Feature code MUST NOT inline `if (isAppleTouch) … else …` rendering branches; that logic lives in the façade.
- iOS 26 chrome (nav bar, tab bar) on iPhone/iPad renders with real UIKit views via `adaptive_platform_ui`, achieving visible Liquid Glass blur/tab pill morph.
- Web, macOS, and Android continue to render Flutter native widgets. No package code paths execute outside iOS.
- No visual regression on web/macOS/Android. No behaviour change at feature-screen level — pure refactor through Phase 2.
- A lint rule enforces the façade so the bypass pattern cannot grow back.

**Non-Goals:**

- Visual redesign of any screen.
- macOS-native rendering pass (the `AppPlatformStyle.macos` bucket continues to alias to Material output; addressed in a future change).
- Adopting `iOS26NativeSearchTabBar` from the package (upstream broken).
- Replacing Flutter widgets with package widgets on web/macOS/Android.
- Touching `app/lib/main.dart`'s root `MaterialApp` / `CupertinoApp` selection beyond what `AdaptiveApp` already does.

## Decisions

### Decision 1: Façade-first sequencing — wrappers before sweep, sweep before package

Order is Phase 1 (add wrappers, pure Flutter) → Phase 2 (migrate screens, one stacked PR each) → Phase 3 (drop package in behind wrappers) → Phase 4 (lint).

**Why:** Each phase is independently shippable and reversible. Phases 1 and 2 produce value even if `adaptive_platform_ui` is never adopted. Phase 3 changes only `lib/shared/platform/` internals — the migration sweep cannot be invalidated by a later package failure. Phase 4 prevents regression but is itself non-blocking work.

**Alternatives considered:**

- Drop the package in first, then migrate. Rejected: couples 13 screen migrations to a v0.1 dependency. If the package proves unstable mid-sweep, we are in a worse state than today.
- Big-bang single PR. Rejected: 13 screens + new wrappers + dep + lint rule is unreviewable, and the user explicitly prefers stacked atomic PRs (memory: `feedback_stacked_prs.md`).

### Decision 2: Three-bucket platform model preserved (`cupertino` / `material` / `macos`)

`AppPlatformStyle` stays as the single source of truth. Every new wrapper branches on `platformStyle`, not on `Platform.is*` or `defaultTargetPlatform`. The `macos` bucket continues to fall through to Material today but remains distinct so a future macOS-native pass is a localised change.

**Why:** The package collapses macOS into "not handled". Adopting that collapse would lose information we already pay for, and would force a global rewrite the day macOS-native rendering becomes a priority.

**Alternatives considered:**

- Collapse `macos` into `material`. Rejected: irreversible loss of intent in 5 wrappers + every future wrapper.

### Decision 3: `adaptive_platform_ui` lives inside wrappers, never imported by feature code

Feature code only ever imports from `app/lib/shared/platform/`. The package becomes an implementation detail of `AdaptiveNavBar`, `AdaptiveSliverNavBar`, and the tab bar inside `nav_shell`'s façade extraction.

**Why:** Confinement makes the dep swappable. If `adaptive_platform_ui` is abandoned or breaks on a future iOS release, we change wrapper internals; feature code is untouched.

**Alternatives considered:**

- Re-export the package's widgets directly. Rejected: leaks package types into feature code, breaks confinement, makes the `macos` bucket awkward.

### Decision 4: iOS 26 native chrome gated by `Platform.isIOS` only — no version check

`IPHONEOS_DEPLOYMENT_TARGET = 26.0` is the floor. There is no older iOS to fall back to. The gate is simply "is this an iOS runtime", which is `platformStyle == AppPlatformStyle.cupertino` on a non-web build.

**Why:** Adding a runtime iOS version check would be dead code today. If the deployment target ever drops below 26, the version check is a localised addition inside wrappers.

**Alternatives considered:**

- `iOS >= 26` runtime gate. Rejected: no behaviour difference today, complicates wrapper code.

### Decision 5: Package version pinned exact (no caret), Phase-3-only

`adaptive_platform_ui` is added in Phase 3 only, with an exact pin (e.g. `adaptive_platform_ui: 0.1.107`, not `^0.1.107`). Phase 1 and Phase 2 use only Flutter built-ins.

**Why:** Solo publisher, v0.x, 12 days old. A pin freezes the API surface; bumping is a deliberate act with its own PR and golden-test re-baseline.

**Alternatives considered:**

- Vendor the package source. Rejected for now: too much code to maintain proactively. Reconsidered only if upstream goes silent.

### Decision 6: Skip `iOS26NativeSearchTabBar`

The package README documents lifecycle, navigation, hot-reload, and memory-leak issues with this widget. We use the package's regular `iOS26NativeTabBar` if appropriate, and otherwise stay on Flutter Cupertino for the search-tab pattern.

**Why:** Verified-broken widget in v0.1.107.

**Alternatives considered:**

- Try it anyway. Rejected: documented memory leaks are not worth the iteration cost.

### Decision 7: Enforce façade with `custom_lint`

Phase 4 adds a `custom_lint` rule that bans `import 'package:flutter/cupertino.dart'` and `import 'package:flutter/material.dart'` outside `app/lib/shared/platform/**` and `app/lib/main.dart`. Wired into `make lint-flutter` and the existing flutter CI workflow.

**Why:** Without enforcement the bypass pattern grows back. Lint produces a fast, mechanical failure with a clear error message pointing at the façade.

**Alternatives considered:**

- Code review discipline only. Rejected: 30+ active OpenSpec changes touch the Flutter app; relying on reviewers to spot direct imports is brittle.
- A shell-script grep gate in `make lint-flutter`. Rejected: works but produces worse diagnostics; `custom_lint` integrates with the IDE and `flutter analyze`.

### Decision 8: Material ban implemented via re-export shim (Option C)

Feature code is genuinely Material-heavy: 299 `Icons.*` references, 233 `Theme.of(context)` reads, 78 `IconButton`, 28 `Card`, 22 `Divider`, plus `InkWell`, `FloatingActionButton`, `Tooltip` — many of which have no meaningful iOS equivalent. Building Adaptive wrappers for every one of them would invent fake adaptivity for Material-only concepts (a `Card` is just a visual surface trick; iOS has no equivalent). Banning `flutter/material.dart` directly in features and rebuilding all of these would be weeks of mostly-cosmetic work for no rendering improvement.

Instead, we enforce the import boundary while keeping Material widgets where they belong: add `app/lib/shared/platform/widgets.dart` as a barrel file that re-exports a **curated subset** of `flutter/material.dart` (plus all `flutter/widgets.dart` neutral widgets, plus all adaptive wrappers, plus `CupertinoIcons` from `flutter/cupertino.dart`). Feature code imports the barrel and gets one stable surface; the Phase 4 lint bans direct `flutter/material.dart` and `flutter/cupertino.dart` imports outside the façade.

The `show` list in the barrel is the **discipline boundary** — adding a widget to it is a deliberate act. As Adaptive wrappers grow, re-exports are removed (e.g. when `AdaptiveIconButton` lands, `IconButton` drops out of the barrel). The boundary moves; the lint never weakens.

**Why:**

- Gives the lint enforcement immediately, before the migration sweep finishes.
- Avoids inventing Cupertino variants of Material-only concepts (`Card`, `InkWell`, `FAB`).
- Single import surface for feature code (`import 'package:assistant_app/shared/platform/widgets.dart';`) — also re-exports the adaptive wrappers so feature files need only one import.
- Iterative ratchet: every Adaptive wrapper added in Phase 3 or later removes one re-export. Visibility into "how much Material is leaking" is one grep through the barrel's `show` list.
- `CupertinoIcons` lives in `flutter/cupertino.dart` but is just a constants class, not a widget. Re-exporting it from the barrel lets feature code reference it for `AdaptiveIcon(cupertino: CupertinoIcons.refresh, material: Icons.refresh)` without importing `cupertino.dart` directly.

**Alternatives considered:**

- **Option A — full façade pass.** Build ~20 more wrappers (`AdaptiveCard`, `AdaptiveInkWell`, `AdaptiveIconButton`, …) plus an `AppTheme.of(context)` shim. Migrate ~700 call sites. Rejected: weeks of work, much of it inventing iOS variants of Material-only concepts.
- **Option B — `flutter/widgets.dart` baseline.** Feature code imports only the neutral widgets layer; everything styled comes through the façade. Rejected: even bigger scope than A (need to reimplement `Card`, `InkWell`, theme machinery from scratch).
- **Option D — theme-only ban.** Narrow the lint to `Theme.of(context).colorScheme.*` access. Rejected: doesn't address widget-level fragmentation (the original problem statement was "13 screens mix Material + Cupertino directly").

**Show-list scope:**

Initial barrel re-exports include: `Card`, `InkWell`, `Material` (widget), `Icons` (constants), `CupertinoIcons` (constants, from cupertino.dart), `Theme`, `ThemeData`, `ColorScheme`, `TextTheme`, `Brightness`, `IconButton`, `FloatingActionButton`, `Tooltip`, `Divider`, `CircularProgressIndicator`, `MaterialPageRoute`, `ScaffoldMessenger`, `Drawer`, `TabBar`, `Tab`, `PopupMenuButton`, `PopupMenuItem`, `OutlinedButton`. Notably **NOT** re-exported (force migration to wrappers): `Scaffold`, `AppBar`, `ListTile`, `SwitchListTile`, `Switch`, `TextField`, `Slider`, `SnackBar`, `AlertDialog`, `FilledButton`, `TextButton`. Also **NOT** re-exported (already gated): `Colors`.

### Decision 9: Phase 4 enforcement via shell script, not custom_lint

The original Phase 4 plan called for a `custom_lint` plugin. In practice, the discipline boundary is simple enough — "no direct cupertino.dart / material.dart imports outside the allowlist" — that a shell-script gate (`scripts/check_facade_imports.sh`) modelled on the existing `scripts/check_no_raw_colors.sh` is the pragmatic choice. It ships immediately, requires no separate Dart package or analyzer setup, and integrates into `make lint-flutter` exactly like the existing colour-token check.

**Why:** The custom_lint plugin would have given IDE integration and richer diagnostics, but at the cost of a separate package + analyzer wiring + a unit-test harness. The shell script matches the project's existing enforcement style and gets us 95% of the value for 10% of the work.

**Alternatives considered:**

- Full `custom_lint` plugin. Rejected for now as over-engineering; can be added later if the shell-script diagnostics prove insufficient.

**Phase 4 special-case allowlists:**

- `scripts/facade_cupertino_allowlist.txt` (2 entries): `lib/shared/nav_shell.dart` (navigation orchestrator combining `cupertino_sidebar` package + `CupertinoTabBar` / `CupertinoActionSheet` / `CupertinoModalPopup` + Material's `NavigationBar` / `NavigationRail`); `lib/shared/error_screen.dart` (bootstrap widget rendered before the normal widget tree). Both have hybrid platform semantics that wrapping would either complicate or visually regress.
- `scripts/facade_material_allowlist.txt` (~44 entries on day one): all files currently importing `flutter/material.dart` that haven't yet migrated to the barrel. Each entry is marked for follow-up; every subsequent migration removes one allowlist entry. This is the ratchet that lands Phase 4 immediately without blocking on the full feature-code migration.

## Risks / Trade-offs

- **Package v0.1.x solo publisher abandons.** → Mitigation: exact pin, confinement to wrapper internals, swap-back to Flutter Cupertino is a localised change in `lib/shared/platform/`. Source is MIT-licensed and small enough to vendor if needed.
- **Platform-view embedding shifts golden-test pixels.** → Mitigation: re-baseline iOS goldens in the Phase 3 PR and note explicitly in `tasks.md`. Web/macOS goldens unaffected because the package is never invoked there.
- **Hot-reload caveats from package README.** → Mitigation: dev-docs note in `app/README.md` (Phase 3 PR). Affects developer-machine workflow only, not shipping behaviour.
- **`chat_screen.dart` is 2229 LOC** — likely the messiest Phase 2 PR. → Mitigation: split internal extraction (e.g. composer, message list) ahead of the import sweep if needed; flagged as a single task that may produce a small stack of sub-PRs.
- **`nav_shell.dart` is 1207 LOC and owns navigation** — touching it risks routing regressions. → Mitigation: do this PR last in Phase 2, behind the others; ensure widget tests exercise both compact (bottom bar) and regular (sidebar) breakpoints on iOS and Material before/after.
- **Custom_lint adds a build step.** → Mitigation: only ship the lint rule in Phase 4, after Phase 2 has eliminated all current violations. The rule turning red on Phase-4 land would mean a regression slipped through.
- **iOS 26 chrome looks different on iPad-on-Mac vs iPhone hardware.** → Mitigation: visual QA on physical iPhone, iPad, and Apple Silicon Mac as part of Phase 3 acceptance.

## Migration Plan

1. **Phase 1** (one PR): add 11 new wrappers under `app/lib/shared/platform/`, each ~40–60 LOC, each with a widget test. No feature code touched. Backwards compatible — old direct imports continue to work.
2. **Phase 2** (13 stacked PRs, one per screen, in dependency order from `error_screen` outward to `nav_shell`): each PR replaces direct `flutter/cupertino.dart` and/or `flutter/material.dart` imports with façade imports. No behaviour change per PR; widget tests and goldens stay green.
3. **Phase 3** (one PR or a small stack): add `adaptive_platform_ui` to `pubspec.yaml` (exact pin). Modify `AdaptiveNavBar`, `AdaptiveSliverNavBar`, and the nav-shell tab bar internals to render the package's UIKit widgets on iOS. Re-baseline iOS goldens. Adopt selected input widgets (text field, switch, slider) only after A/B vs Flutter Cupertino on iOS 26.
4. **Phase 4** (one PR): land `custom_lint` rule + plugin package. Add allowlist for `lib/shared/platform/**` and `lib/main.dart`. Run on CI.

**Rollback:** Each phase is independently revertible. Phase 3 swap-back is a `flutter pub remove adaptive_platform_ui` plus revert of wrapper internals — feature code stays put. Phase 4 rollback is a `dart_pre_commit` allowlist toggle.

## Open Questions

- Does the nav-shell tab bar match the package's `iOS26NativeTabBar` API closely enough to drop in without restructuring `nav_shell.dart`'s route plumbing? Investigation needed in Phase 3 spike.
- Should `AdaptiveIcons` be a thin enum-keyed lookup (e.g. `AdaptiveIcons.delete`) or a builder receiving both `cupertinoIcon` and `materialIcon`? The existing `AdaptiveContextMenuAction` uses the builder pattern; consistency argues for builder. Decide in Phase 1.
- Are there iPad-specific affordances (split view, hover) that the package handles differently from Flutter Cupertino? Out of scope for this change but flag for follow-up.
