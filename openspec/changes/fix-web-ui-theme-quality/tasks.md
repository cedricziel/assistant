## Tasks

### Phase 1 — Failing tests + audit baselines

- [ ] Add `app/test/unit/shared/theme/assistant_theme_test.dart` covering: - light/dark `ThemeData` exposes `Inter` as default font family - `colorScheme.tertiary == AssistantColors.accentTertiary` for both brightnesses - `AssistantSpacing.lg == 16` etc. (sanity) - `AssistantTypography.mono.fontFamily == 'JetBrainsMono'`
- [ ] Add `app/test/unit/shared/theme/contrast_test.dart` that computes WCAG ratios for the audited pairs (chat bubble bg vs body, inline code bg vs text, sidebar selected vs label, trace status pills) in **both** themes. The test SHALL fail today for the dark-mode inline-code pair.
- [ ] Add `app/test/golden/theme_smoke_test.dart` rendering a tiny widget tree (Card with body text, mono code span, status pill) in both themes for golden comparison.
- [ ] Add `scripts/check_no_raw_colors.sh` (or equivalent Dart script) that exits non-zero on any `Color(0x` or `Colors.` match outside `app/lib/shared/theme/` and the allow-list. Wire it into `make lint-flutter`.
- [ ] Run `flutter test`; confirm RED on theme + contrast specs.

### Phase 2 — Theme module + vendored fonts

- [ ] Vendor Inter (`Inter-Regular.ttf`, `Inter-Medium.ttf`, `Inter-SemiBold.ttf`) and JetBrains Mono (`JetBrainsMono-Regular.ttf`, `JetBrainsMono-Medium.ttf`) under `app/assets/fonts/`. Add the OFL licence file alongside each family.
- [ ] Declare both families in `app/pubspec.yaml` under `fonts:`.
- [ ] Create `app/lib/shared/theme/assistant_colors.dart` with `brand`, `accentSecondary`, `accentTertiary`, `warning` (light + dark variants where needed).
- [ ] Create `app/lib/shared/theme/assistant_typography.dart` exposing `material()` and `mono`.
- [ ] Create `app/lib/shared/theme/assistant_spacing.dart` with `AssistantSpacing`, `AssistantRadius`, `AssistantElevation`.
- [ ] Create `app/lib/shared/theme/assistant_theme.dart` exposing `assistantLightTheme()` and `assistantDarkTheme()`.
- [ ] Run `flutter test`; theme unit tests GREEN. Contrast test may still fail until consumers migrate.

### Phase 3 — Wire `adaptive_app.dart` to the new theme

- [ ] Replace inline `ColorScheme.fromSeed` in `adaptive_app.dart` with calls to `assistantLightTheme()` / `assistantDarkTheme()`.
- [ ] Run `flutter test`; golden smoke test GREEN.

### Phase 4 — Migrate chat screen tokens

- [ ] In `app/lib/features/chat/chat_screen.dart`, replace inline `EdgeInsets`/`SizedBox` numbers with `AssistantSpacing.*`, inline code background with the theme token, and any raw `Color(0x...)` with theme references.
- [ ] Re-run `flutter test` + the new lint script; confirm green for `chat_screen.dart`.
- [ ] Update Playwright chat baselines.

### Phase 5 — Migrate sidebar / nav shell tokens

- [ ] Same migration in `app/lib/shared/nav_shell.dart`. Watch for the sidebar widths (`_kSidebarExpandedWidth = 240` etc.) — those stay as is (semantic constants, not spacing), but `EdgeInsets`/`SizedBox` migrate.
- [ ] Update Playwright nav-shell baselines.

### Phase 6 — Migrate traces + logs tokens

- [ ] Migrate `app/lib/features/traces/trace_detail_screen.dart` (and `tool_call_span_card.dart` if landed from #265).
- [ ] Migrate `app/lib/features/logs/logs_screen.dart` and `log_entry.dart` (or equivalent).
- [ ] Update Playwright trace + log baselines.

### Phase 7 — Lint enforcement + contrast pass

- [ ] Confirm `make lint-flutter` runs `scripts/check_no_raw_colors.sh` and fails on a deliberately introduced raw `Color(0x...)` outside the theme module.
- [ ] Iterate on `AssistantColors` / `assistantDarkTheme` until the contrast test reports all audited pairs at >= 4.5:1 (body) or >= 3:1 (large text / pills).
- [ ] Re-run `flutter test`; everything GREEN.

### Phase 8 — Docs + wrap-up

- [ ] Add `docs/design/theme.md` covering: token tables (colours, typography, spacing, radius), how to migrate a new screen, allow-list policy, contrast ratios achieved.
- [ ] Note the theme migration in `docs/operations/web-ui-shortcuts.md` if there are user-visible behavioural shifts (there should be none — visual only).
- [ ] `make lint && make format && make test && make lint-flutter && make test-flutter`.
- [ ] PR description: before/after screenshots of the four migrated screens (light + dark), contrast deltas, bundle-size delta from font assets.
