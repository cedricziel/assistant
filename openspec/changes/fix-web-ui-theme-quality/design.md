## Context

Today's theme story:

- `adaptive_app.dart` calls `ColorScheme.fromSeed(seedColor: Color(0xFF1A73E8))` once for Material and once for Cupertino, with `useMaterial3: true` and nothing else customised.
- No top-level typography overrides — Flutter's default Roboto applies on web/Android and SF on iOS via Cupertino.
- Screens inline ad-hoc colour values (chat bubble `surfaceContainerLowest`, trace cards via `colorScheme.outlineVariant`, sidebar items with literal `IconButton` styling) and ad-hoc spacing (`EdgeInsets.all(12)`, `EdgeInsets.all(16)`, `SizedBox(height: 6)`).
- Dark mode is enabled but no surface-tint contrast verification has happened.

The risk of "just fix the theme" turning into endless scope creep is real, so this change defines a strict perimeter: theme module + four screens + lint enforcement. Other surfaces stay default-themed and will adopt tokens incrementally in follow-up work.

## Goals / Non-Goals

**Goals**

- A single source of truth for colour, typography, spacing, radius, and elevation in `app/lib/shared/theme/`.
- Both light and dark themes meet WCAG AA contrast on the four migrated screens.
- A lint that prevents new code from regressing the policy (raw `Color` literals, `Colors.X` references) outside the theme module.
- Token names are stable; future screens can adopt them without renames.

**Non-goals**

- Visual identity / brand redesign.
- Migrating every screen at once (only chat, sidebar, traces, logs).
- iOS-native (Cupertino) restyling beyond what the shared Material theme cascades through.
- A user-facing theme picker UI.

## Decisions

### D1: Named-seed palette instead of single seed

**Choice:** Build `ColorScheme.fromSeed` with explicit overrides:

```dart
ColorScheme.fromSeed(
  seedColor: AssistantColors.brand,            // primary
  brightness: brightness,
).copyWith(
  secondary: AssistantColors.accentSecondary,
  tertiary:  AssistantColors.accentTertiary,
  // Surface tints kept algorithmic.
);
```

Three intentional accent roles (success, info, warning) are derived from `tertiary`, `secondary`, and a new `warning` token (Material 3 doesn't have a built-in warning role).

**Why:** Lets us match accent colours to semantic meaning (success-tertiary, warning-amber) rather than whatever the seed algorithm produces. Keeps Material 3's surface tint generation since that part works well.

### D2: Typography — Inter + JetBrains Mono, vendored

**Choice:** Vendor two open-licence fonts under `app/assets/fonts/`: Inter (UI) and JetBrains Mono (code / monospace). Declare them in `pubspec.yaml`. Build a `TextTheme` from `Typography.material2021()` with Inter as the default and JetBrains Mono on `bodySmall.copyWith(fontFamily: 'JetBrainsMono')` exposed via `AssistantTypography.mono`.

**Why:** No network-loaded fonts (PWA / offline correctness). Both fonts have permissive licences (OFL). Inter is the de-facto modern UI font; JetBrains Mono renders code well at all sizes.

**Alternative considered:** Google Fonts package — rejected because it does a runtime fetch on first use; that breaks offline-first.

### D3: Spacing tokens — multiplicative, 4 dp base

**Choice:** Define `AssistantSpacing` as a class with named constants:

```dart
abstract class AssistantSpacing {
  static const double xs = 4;
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
  static const double xl = 24;
  static const double xxl = 32;
}
```

Plus a small set of radius and elevation constants. All values are multiples of 4 dp — Material's grid base.

**Why:** Token names communicate intent; magic numbers don't. A future change can adjust the base unit without touching every screen.

### D4: Lint enforcement

**Choice:** Add a custom lint rule via `custom_lint` (or, if that's heavyweight, a CI-side `grep` check that fails on `Color(0x` / `Colors\.` matches outside `app/lib/shared/theme/`). Allow-list the theme module.

**Why:** Without enforcement, "use the tokens" decays in three PRs. The grep version is dirt-cheap and good enough — we can upgrade to `custom_lint` later.

### D5: Contrast audit — top four offenders

**Choice:** Run a contrast audit on:

1. Chat bubble background vs. body text (light + dark).
2. Inline code background vs. inline code text (uses `surfaceContainerLowest` today).
3. Sidebar selected-state highlight vs. label.
4. Trace status pill text vs. pill background (the chips this change introduces — verify against AA before merging #265 dependencies).

For each failing pair, adjust the affected token (not the consumer). Document the resulting contrast ratios in `docs/design/theme.md`.

**Why:** Auditing every surface is a lot. The four above are the highest-density user-facing surfaces. Other regressions can be filed as follow-ups.

### D6: No Cupertino theme override beyond what cascades

**Choice:** The Cupertino branch still gets its own `CupertinoApp.router` with the default Cupertino theme. The `Theme(data: materialTheme, child: Material(...))` wrapper inside `CupertinoApp.builder` (`adaptive_app.dart:60`) gives any Material widgets (which we use heavily inside the chat shell) the new tokens automatically.

**Why:** Keeping system styling on Apple touch was a deliberate decision in the `adaptive-shell` spec. We don't want to fight it here.

## Risks / Trade-offs

- **Token churn:** Renaming a token after release means a coordinated PR. The proposed names (`xs`/`sm`/`md`/`lg`, `brand`/`accentSecondary`/`accentTertiary`/`warning`) are conventional enough that we don't expect churn, but we accept the risk.
- **Vendored fonts inflate bundle size:** Inter + JetBrains Mono add ~600 KB to the web bundle (after gzip). We accept this for offline-first; if it becomes a problem we can subset.
- **Custom lint maintenance:** `custom_lint` adds build complexity. We start with a grep-based CI check (zero dep) and revisit if violations slip through.
- **Theme work blocks other UI changes:** While this change is in flight, other contributors will collide on the four migrated screens. We sequence #266 to land **after** #619, #620, #265 — those three touch the same files but with focused diffs. This change picks up cleanly afterwards.

## Migration Plan

- No data migration. UI only.
- Bundle size impact is on first deploy; subsequent updates re-use the cached fonts via service worker.
- Document the token system in `docs/design/theme.md`. Reference it from the `ux-principles` skill so future agents land on the new patterns.
