## 1. Failing tests first (TDD red)

- [x] 1.1 Add `app/test/widget/space_selector_close_button_test.dart`: pump `SpaceSelectorScreen` inside a `GoRouter` test harness with `/spaces` and `/chat` routes. Assert the close affordance is visible. Tap it → assert the router is on `/chat` AND `spaceSelectionProvider` retains its prior state.
- [x] 1.2 Add `app/test/widget/space_selector_change_org_visibility_test.dart`: override `orgsProvider` with one org → assert "Change organization" is absent. Override with two orgs → assert it's present.
- [x] 1.3 Run `flutter test` — confirm both new tests fail (close button doesn't exist; "Change organization" is unconditionally rendered).

## 2. Implement the close affordance

- [x] 2.1 In `SpaceSelectorScreen.build`, add a `Row` at the top of the existing column with the headline on the left and an `IconButton(icon: Icon(Icons.close), tooltip: 'Close', onPressed: () => GoRouter.of(context).go(AppRoutes.chat))` on the right.
- [x] 2.2 Confirm test 1.1 turns green.

## 3. Conditional "Change organization"

- [x] 3.1 In `_SpaceList`, wrap the existing `TextButton.icon("Change organization")` in a `Consumer` that watches `orgsProvider`. Render an empty `SizedBox.shrink()` unless `orgsAsync.value != null && orgsAsync.value!.length > 1`.
- [x] 3.2 Confirm test 1.2 turns green.

## 4. Smoke + ship

- [x] 4.1 `flutter analyze --fatal-infos` → 0 issues.
- [x] 4.2 `flutter test` → all green.
- [x] 4.3 Manual smoke against schorschvm: open chat → click switcher → see close (X) button + no "Change organization" link → tap close → land on `/chat` with selection intact.
- [ ] 4.4 PR: `fix(app): close affordance + hide change-org for single-org users`. Body links the four scenarios.
- [ ] 4.5 Merge and deploy via apt update.
- [ ] 4.6 Archive: `openspec archive space-selector-usability`.
