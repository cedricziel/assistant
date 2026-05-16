# app-theme-tokens Specification

## Purpose
TBD - created by archiving change fix-web-ui-theme-quality. Update Purpose after archive.
## Requirements
### Requirement: Theme is centralised in `app/lib/shared/theme/`

The Flutter app SHALL expose all design tokens from `app/lib/shared/theme/`, with these public modules:

- `assistant_theme.dart` — exports `assistantLightTheme()` and `assistantDarkTheme()` `ThemeData` builders.
- `assistant_colors.dart` — colour seed + role tokens (`AssistantColors.brand`, `AssistantColors.accentSecondary`, `AssistantColors.accentTertiary`, `AssistantColors.warning`).
- `assistant_typography.dart` — `AssistantTypography.material()` returns a `TextTheme`; exposes `AssistantTypography.mono` (`TextStyle`) for code surfaces.
- `assistant_spacing.dart` — `AssistantSpacing.xs|sm|md|lg|xl|xxl` (constants), `AssistantRadius.sm|md|lg`, `AssistantElevation.low|medium|high`.

`adaptive_app.dart` SHALL call the builders from `assistant_theme.dart` instead of constructing `ThemeData` inline.

#### Scenario: Adaptive app consumes the theme builders

- **WHEN** `AdaptiveApp` builds
- **THEN** the resolved `MaterialApp.theme` SHALL be the value returned by `assistantLightTheme()` AND `MaterialApp.darkTheme` SHALL be the value returned by `assistantDarkTheme()`

#### Scenario: Tokens are imported from a single module

- **WHEN** any code outside `app/lib/shared/theme/` needs spacing
- **THEN** it SHALL import `AssistantSpacing` from `assistant_spacing.dart` AND SHALL NOT define new module-local spacing constants

### Requirement: Theme defines named accent roles

`assistant_colors.dart` SHALL export at minimum these named roles:

- `brand` — primary brand colour (matches `AssistantColors.brand`).
- `accentSecondary` — secondary accent.
- `accentTertiary` — tertiary accent (success-leaning).
- `warning` — amber-ish, used for `denied` status pills and similar.

`assistantLightTheme()` and `assistantDarkTheme()` SHALL apply these via `ColorScheme.fromSeed(...).copyWith(secondary: accentSecondary, tertiary: accentTertiary)`. The `warning` token SHALL be available as a const since Material 3 has no built-in warning role.

#### Scenario: Tertiary slot uses accentTertiary

- **GIVEN** the light theme is active
- **WHEN** a consumer reads `Theme.of(context).colorScheme.tertiary`
- **THEN** the value SHALL equal `AssistantColors.accentTertiary`

#### Scenario: Warning token is accessible

- **WHEN** a consumer reads `AssistantColors.warning`
- **THEN** the returned `Color` SHALL be non-null and SHALL be one of the documented amber tokens

### Requirement: Typography uses Inter for UI and JetBrains Mono for code

`assistant_typography.dart` SHALL build a `TextTheme` rooted in Inter for all UI text and expose `AssistantTypography.mono` (a `TextStyle` using JetBrains Mono) for code surfaces (inline code in chat, attribute keys in trace cards, log lines). Both families SHALL be loaded via the `google_fonts` package, which fetches them on first use and caches subsequent loads.

#### Scenario: Default text theme uses Inter

- **WHEN** the light theme is active
- **THEN** `Theme.of(context).textTheme.bodyLarge!.fontFamily` SHALL start with `"Inter"` (the package suffixes the cached family name, e.g. `Inter_regular`)

#### Scenario: Inline code uses JetBrains Mono

- **WHEN** an inline-code text style is needed
- **THEN** `AssistantTypography.mono.fontFamily` SHALL start with `"JetBrainsMono"`

#### Scenario: Fonts are cached after first fetch

- **GIVEN** `google_fonts` has fetched Inter and JetBrains Mono once
- **WHEN** the app is reloaded
- **THEN** the fonts SHALL render from cache without an additional network request

### Requirement: Spacing, radius, and elevation tokens replace magic numbers

`AssistantSpacing` SHALL expose `xs=4`, `sm=8`, `md=12`, `lg=16`, `xl=24`, `xxl=32` as `static const double`. The four migrated screens (`chat_screen.dart`, `nav_shell.dart`, `trace_detail_screen.dart`, `logs_screen.dart` and friends) SHALL reference these constants instead of inline numeric literals for `EdgeInsets`, `SizedBox`, `Padding`, and similar widgets.

`AssistantRadius` SHALL expose `sm=8`, `md=12`, `lg=16`. Card / chip / button radii on the migrated screens SHALL reference these.

#### Scenario: Chat bubble uses spacing tokens

- **GIVEN** the chat bubble renders body padding
- **THEN** its `EdgeInsets` SHALL use one of `AssistantSpacing.xs|sm|md|lg|xl|xxl` (not a raw numeric literal)

#### Scenario: Trace card radius references AssistantRadius

- **GIVEN** the trace span card renders rounded corners
- **THEN** its `BorderRadius.circular(...)` SHALL receive an `AssistantRadius.*` constant (not `12`)

### Requirement: Migrated screens meet WCAG AA contrast in both themes

On the migrated screens (`chat_screen.dart`, `nav_shell.dart`, `trace_detail_screen.dart`, logs), foreground / background colour pairs SHALL achieve a contrast ratio of at least:

- `4.5:1` for body text and inline code.
- `3:1` for large text (>= 18 dp regular or 14 dp bold), icons, and chip pills.

Audited pairs:

- Chat bubble background vs. body text — both themes.
- Inline code background vs. inline code text — both themes.
- Sidebar selected-state highlight vs. selected label — both themes.
- Trace status pill text vs. pill background (`ok`, `error`, `denied`, `unknown`) — both themes.

#### Scenario: Light-mode inline code passes AA

- **GIVEN** the light theme is active
- **WHEN** an inline-code span is rendered
- **THEN** the foreground / background contrast ratio SHALL be >= 4.5:1

#### Scenario: Dark-mode chat bubble passes AA

- **GIVEN** the dark theme is active
- **WHEN** an assistant chat bubble renders body text
- **THEN** the foreground / background contrast ratio SHALL be >= 4.5:1

#### Scenario: Trace status pills pass large-text AA

- **GIVEN** either theme is active
- **WHEN** a trace status pill (`ok`, `error`, `denied`, `unknown`) renders
- **THEN** the pill text / pill background contrast ratio SHALL be >= 3:1

### Requirement: Raw colour literals are banned outside the theme module

A CI check SHALL fail when any file under `app/lib/` outside `app/lib/shared/theme/` (and outside test files) matches:

- `Color\(0x` — raw colour literals.
- `\bColors\.` — Material's named colour palette.

Exceptions SHALL be expressed via an allow-list file (e.g. `analysis_lint_allowlist.txt`) committed to the repo; entries SHALL include a comment explaining the exception.

#### Scenario: New file introducing `Colors.red` fails CI

- **GIVEN** a PR adds `final color = Colors.red;` in `app/lib/features/foo/foo_widget.dart`
- **WHEN** the lint check runs
- **THEN** the check SHALL fail AND the error message SHALL reference the offending file/line AND SHALL point to `docs/design/theme.md`

#### Scenario: Theme module is exempt

- **WHEN** the lint scans `app/lib/shared/theme/assistant_colors.dart`
- **THEN** raw `Color(0x...)` literals SHALL be allowed

