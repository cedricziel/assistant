# ADR 0021: Chat Elements Live in a Pure `assistant_ui` Package

**Status**: Accepted
**Date**: 2026-11-26
**Builds on**: [ADR-0009](adr-0009-testability-architecture.md)

## Context

The chat widgets in `app/lib/features/chat/` mix rendering with Riverpod reads,
platform branching and I/O. That makes them hard to show in isolation, hard to
compare pixel for pixel, and hard to extend: nine sibling changes each add chat
widgets and need a stable place and a stable way to plug in.

## Decision

**Chat elements move into `app/packages/assistant_ui/`, a package that renders
and does nothing else.** The app owns state, I/O and platform branching.

- **Package boundary.** `dependencies` are exactly `flutter`,
  `flutter_smooth_markdown` and `material_ui`. Source imports no state
  management, no I/O or platform-channel package, no Cupertino, and never
  branches on the platform. `material_ui`'s `.adaptive` constructors are
  allowed because they branch inside the dependency. The deprecated
  `package:flutter/material.dart` import that markdown styling needs is confined
  to `lib/src/theme/legacy_theme.dart`. `app/test/unit/assistant_ui_boundary_test.dart`
  enforces all of this.
- **Amendment rule.** Another dependency is admitted only if it is pure
  rendering, the "Element package is pure" requirement gets a MODIFIED delta
  naming the new allowlist, and the boundary test changes in the same PR.
  Otherwise the adapter stays in `app/lib`.
- **Pub workspace.** `app/pubspec.yaml` declares `workspace: [packages/assistant_ui]`
  and the member uses `resolution: workspace`. One `flutter pub get` resolves
  everything and `flutter analyze` in `app/` covers the member. Separate
  `pub get` runs would mean separate lockfiles that can drift.
- **Slots first, three scopes.** Elements take constructor slots by default.
  Inherited scopes are reserved for ambient configuration read at several
  nesting levels: `ThreadDensityScope`, `ChatMarkdownScope` and
  `ToolResultRendererScope`. Where an element needs something adaptive or
  I/O-backed, it exposes a slot with a neutral Material default and the app
  injects the real widget.
- **Tokens equal today's values.** `AssistantUiColors` and `AssistantUiSpacing`
  carry the exact values the moved code renders, and the app's
  `AssistantColors.warning` and `AssistantSpacing` are defined in terms of them.
  Moving code therefore moves no pixel. `check_no_raw_colors.sh` scans the
  package and exempts only `lib/src/theme/`.
- **Characterization goldens.** Before a widget is extracted, goldens of the app
  widget are captured. The extracted element must reproduce them byte for byte
  before the app widget is deleted. Goldens are plain `matchesGoldenFile`,
  tagged `golden`, generated on Linux (`make update-goldens`) and compared in CI.
  Local macOS runs skip the tag.

## Consequences

- Every new chat widget has an obvious home and a gallery entry.
- The app gains adapters that inject I/O into elements.
- Golden baselines must be regenerated on Linux, not on a developer's Mac.
- A sibling that wants a heavier dependency must either justify it under the
  amendment rule or keep the code in the app.

## Alternatives considered

- **Move `shared/platform/` into its own package** that `assistant_ui` depends on.
  That is 1,220 lines across 18 files and every importer; it is a separate
  refactor.
- **A single `AssistantUiScope`.** Density changes on every resize and would
  rebuild markdown consumers for nothing.
- **Riverpod inside elements.** Rejected by the boundary.
- **`alchemist` for goldens.** A dependency the plain approach does not need.
