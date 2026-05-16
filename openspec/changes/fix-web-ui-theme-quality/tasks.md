## Tasks

### Phase 1 — Failing tests + audit baselines

- [x] Add `app/test/unit/shared/theme/assistant_theme_test.dart` covering: - light/dark `ThemeData` declares the `Inter` family on default text - `colorScheme.tertiary == AssistantColors.accentTertiary` for both brightnesses - `AssistantSpacing` constants match the documented scale - `AssistantTypography.mono.fontFamily == 'JetBrainsMono'`
- [x] Add `app/test/unit/shared/theme/contrast_test.dart` computing WCAG ratios for `onSurface` vs `surface`, `onSurface` vs `surfaceContainerLowest`, `onPrimary` vs `primary`, and accent-on-surface pairs in both themes.
- [x] Add `app/scripts/check_no_raw_colors.sh` that exits non-zero on any `Color(0x...)` or `Colors.X` (except `Colors.transparent`) outside `lib/shared/theme/` and the allow-list. Wire it into `make lint-flutter`.
- [x] Run `flutter test`; confirm RED on theme + contrast specs.

### Phase 2 — Theme module

- [x] Create `assistant_colors.dart` with `brand`, `accentSecondary`, `accentTertiary`, `warning`.
- [x] Create `assistant_typography.dart` exposing `material()` and `mono`, plus `preloadFonts()` that wires `google_fonts` for Inter + JetBrains Mono.
- [x] Create `assistant_spacing.dart` with `AssistantSpacing`, `AssistantRadius`, `AssistantElevation`.
- [x] Create `assistant_theme.dart` exposing `assistantLightTheme()` / `assistantDarkTheme()`.
- [x] Add `google_fonts: ^6.3.2` to `app/pubspec.yaml`.
- [x] Theme + contrast tests GREEN.

### Phase 3 — Wire `adaptive_app.dart`

- [x] Replace inline `ColorScheme.fromSeed` in `adaptive_app.dart` with calls to `assistantLightTheme()` / `assistantDarkTheme()`.
- [x] Call `AssistantTypography.preloadFonts()` from `main()` before `runApp`.

### Phase 4 — Migrate chat screen

- [x] Replace `Colors.amber.shade*` in the queue-depth badge with `AssistantColors.warning`.
- [x] Replace `EdgeInsets.symmetric(horizontal: 16, vertical: 4)` with token references on the same row.

### Phase 5 — Migrate sidebar / nav shell tokens

- [ ] Migrate `app/lib/shared/nav_shell.dart` spacing literals. (Allow-listed in this PR for follow-up.)

### Phase 6 — Migrate traces + logs tokens

- [x] Migrate `app/lib/features/traces/trace_detail_screen.dart`: `EdgeInsets.all(16)` → `AssistantSpacing.lg`; `BorderRadius.circular(12)` → `AssistantRadius.md`; `EdgeInsets.all(12)` → `AssistantSpacing.md`.
- [x] Migrate `app/lib/features/traces/tool_call_span_card.dart`: pane padding + outer card padding + border radius.
- [ ] Migrate `app/lib/features/logs/logs_screen.dart`. (Allow-listed.)

### Phase 7 — Lint enforcement + contrast pass

- [x] `make lint-flutter` runs `scripts/check_no_raw_colors.sh` and fails on any new raw colour outside the theme module.
- [x] Allow-list documents pre-existing offenders as known follow-up migrations.
- [x] Contrast test reports all audited pairs at >= 4.5:1 (body) or >= 3:1 (large text / pills).

### Phase 8 — Docs + wrap-up

- [x] Add a "Theme" section to `docs/web-ui.md` covering token tables, migration recipe, allow-list policy, and the contrast test reference.
- [ ] `make lint && make format && make test && make lint-flutter && make test-flutter` (run by the pre-commit hook on commit).
- [ ] PR description: before/after screenshots of the chat queue-depth badge and trace detail page (light + dark) for visual review.

### Deferred (follow-up changes)

- Full token migration of remaining screens listed in `app/scripts/theme_color_allowlist.txt`. Each can land as its own PR.
- WCAG contrast verification on every screen (this PR audits the load-bearing pairs only).
- Replacing the `google_fonts` runtime fetch with vendored `.ttf` files in `app/assets/fonts/` if offline-first PWA usage becomes a hard requirement.
