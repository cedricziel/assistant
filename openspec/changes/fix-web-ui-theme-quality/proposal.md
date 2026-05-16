## Why

The Flutter app's Material theme is currently a one-line `ColorScheme.fromSeed(seedColor: 0xFF1A73E8)` (`app/lib/shared/platform/adaptive_app.dart:39-83`) with no token customisation, no typography overrides, and no surface tinting strategy. The result is exactly what issue #266 reports:

- Surface colours derived from the seed look generic and read as "default Flutter app", not "this product".
- Typography defaults to `Roboto` everywhere — no display / body distinction, no monospace pairing for code.
- Several places (chat bubbles, sidebar items, trace cards) inline custom colours and spacing values, so the look is inconsistent across screens.
- Light mode contrast is acceptable; dark mode contrast on `surfaceContainerLowest` (used for inline code, message backgrounds) drops below WCAG AA in several spots.

This is the kind of polish that compounds: every other UI change after this lands on better foundations.

## What Changes

- Define an explicit `AssistantTheme` module in `app/lib/shared/theme/` that exports light + dark `ThemeData` built from:
  - A small, named seed palette (primary, secondary, tertiary) rather than a single seed colour, so accents are intentional rather than algorithmically derived.
  - A typography ramp that pairs Inter (or Geist) for UI with JetBrains Mono for code blocks and trace attributes.
  - Centralised spacing / radius / elevation tokens (`AssistantSpacing`, `AssistantRadius`) replacing magic numbers in the screens.
- Replace inline `Color(0x...)` / hard-coded `EdgeInsets.all(16)` usages in the highest-traffic screens (chat, sidebar, traces, logs) with token references. Other screens stay on the new theme defaults for now.
- Audit dark-mode contrast on the chat bubble, inline code, sidebar selection state, and trace status pills. Fix the four worst offenders identified in the audit.
- Lock the theme by adding a static-analysis lint that fails CI when a non-test file imports `Colors` or uses raw `Color(0x...)` literals outside `app/lib/shared/theme/`.

## Non-goals

- A full visual redesign — typography, base palette, and shape feel stay roughly familiar.
- New iconography or illustration work.
- Restyling Cupertino chrome (we keep system styling on iOS / iPadOS).
- Adding theme switcher UI (already exists at OS level; in-app toggle is a later change).
- Touching every screen — only the four high-traffic surfaces are migrated in this change.

## Capabilities

### Added Capabilities

- `app-theme-tokens` (new spec) — defines the theme module's shape (named tokens, accessibility floor, file location, lint enforcement).

## Impact

- New files:
  - `app/lib/shared/theme/assistant_theme.dart` — light + dark `ThemeData` builders.
  - `app/lib/shared/theme/assistant_colors.dart` — colour roles (seed + tints).
  - `app/lib/shared/theme/assistant_typography.dart` — text styles.
  - `app/lib/shared/theme/assistant_spacing.dart` — spacing / radius / elevation constants.
- Refactor `app/lib/shared/platform/adaptive_app.dart` to consume the new theme builders.
- Touch the high-traffic screens to use tokens:
  - `app/lib/features/chat/chat_screen.dart`
  - `app/lib/shared/nav_shell.dart`
  - `app/lib/features/traces/trace_detail_screen.dart`
  - `app/lib/features/logs/`
- Add `pubspec.yaml` font assets for Inter + JetBrains Mono (vendored under `app/assets/fonts/`).
- Add `analysis_options.yaml` lint rule (custom or via `linter_extras`) banning raw `Color` outside the theme module.
- Update Playwright baselines on every screen we touched.

## Visual / UI change

Yes — substantial. Typography ramp moves, surface tints change, spacing on chat bubbles and trace cards shifts. Playwright baselines on the four high-traffic screens will move significantly. All other screens retain Material 3 defaults applied through the new theme.

## User-facing documentation

- Add `docs/design/theme.md` describing the token system, the named seed palette, and how to add a new screen that consumes the tokens (so future contributors don't fall back to raw `Color` values).
- Update the existing `ux-principles` skill notes if any token names are referenced there.
