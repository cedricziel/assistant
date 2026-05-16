## Tasks

### Phase 1 — Failing tests

- [ ] Add `app/test/unit/shared/sidebar_collapsed_provider_test.dart` covering: - default is `false` when key is absent - reads persisted `true` from a fake `SharedPreferences` - `toggle()` writes the new value through - SharedPreferences write rejection does not throw
- [ ] Add `app/test/widget/nav_shell_test.dart` (or extend existing) covering: - at viewport 1180×820 with `kIsWeb=true`, a top-leading `IconButton` with semantic label `"Collapse sidebar"` is visible - tapping it toggles `sidebarCollapsedProvider` - tooltip flips between `"Collapse sidebar"` and `"Expand sidebar"`
- [ ] Add gesture widget test: simulated touch drag from x=10 → x=80 toggles the provider; drag from x=200 does not.
- [ ] Add Playwright test `app/test_e2e/sidebar_collapse_ipad.spec.ts` at the iPad-landscape viewport (1180×820) that: - captures the expanded baseline - taps the top-leading toggle - captures the collapsed baseline - reloads the page and asserts the sidebar is still collapsed
- [ ] Run `flutter test` and `npm run e2e -- --grep sidebar_collapse_ipad` (or project equivalent) and confirm RED on the new specs.

### Phase 2 — Persistent provider

- [ ] Convert `SidebarCollapsedNotifier` in `app/lib/shared/nav_shell.dart` to an `AsyncNotifier<bool>` backed by `SharedPreferences` under `assistant.sidebarCollapsed`.
- [ ] Update every `ref.watch(sidebarCollapsedProvider)` call to read `AsyncValue.value ?? false` so first-paint defaults to expanded.
- [ ] Run `flutter test`; confirm Phase-1 provider tests now GREEN.

### Phase 3 — Top-leading toggle on Material wide

- [ ] Extract the toggle UI into a `SidebarToggleButton` widget in `app/lib/shared/sidebar_toggle_button.dart` (with its own widget test).
- [ ] In `NavShell._buildBody` Material wide branch, render `SidebarToggleButton` as a `Positioned` overlay at top-leading of the main content `Expanded`, outside any screen-owned `AppBar`.
- [ ] Keep the existing in-sidebar `IconButton`; upgrade the expanded-state copy to read `Collapse` next to the icon.
- [ ] Confirm widget tests GREEN at 1180×820.

### Phase 4 — Swipe-from-left-edge gesture

- [ ] Wrap the main content `Expanded` (Material wide and Apple touch wide) in a `GestureDetector` listening to `onHorizontalDragUpdate` / `onHorizontalDragEnd`.
- [ ] Gate the gesture on touch input: combine `defaultTargetPlatform` (`iOS`, `android`) with a runtime pointer check on web (`PointerDeviceKind.touch`).
- [ ] Edge zone = first 20 logical pixels; threshold = 40 px in either direction.
- [ ] Confirm gesture test GREEN.

### Phase 5 — Playwright + screenshot baselines

- [ ] Update Playwright baselines for the iPad-landscape viewport on every route that renders `NavShell` (chat, traces, logs, skills, personas).
- [ ] Document the intentional baseline movement in the PR description.

### Phase 6 — Wrap-up

- [ ] `make lint-flutter && make test-flutter && make lint && make format`.
- [ ] Manual smoke on web at 1180×820 (Chrome devtools iPad mode): expand → collapse → reload → still collapsed.
- [ ] Manual smoke on native iPad (or Simulator): tap Cupertino toggle → reload → state persists.
- [ ] Add a brief paragraph to `docs/operations/web-ui-shortcuts.md` describing the toggle and the swipe gesture.
