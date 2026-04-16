## 1. LoadingScreen Widget

- [x] 1.1 Create `app/lib/shared/loading_screen.dart` with a `LoadingScreen` `StatelessWidget` — `Scaffold` containing a centred `Column` with `CircularProgressIndicator` and a `Text('Starting…')` label
- [x] 1.2 Run `flutter analyze` — zero issues after adding the file

## 2. Router Changes

- [x] 2.1 Add `static const loading = '/loading'` to `AppRoutes` in `app/lib/router/app_router.dart`
- [x] 2.2 Register a top-level `GoRoute(path: AppRoutes.loading, builder: …)` as a sibling of `/setup` (outside the `ShellRoute`) that renders `LoadingScreen`
- [x] 2.3 Update the `redirect` callback: replace `if (activeContextAsync.isLoading) return null;` with `if (activeContextAsync.isLoading) return AppRoutes.loading;`
- [x] 2.4 Add `final onLoading = state.fullPath == AppRoutes.loading;` to the exempt-routes check so `/loading` is never redirected to itself
- [x] 2.5 Run `flutter analyze` — zero issues

## 3. Tests

- [x] 3.1 Add widget test: loading state → `CircularProgressIndicator` and `Text('Starting…')` are present, `NavShell` chrome is absent
- [x] 3.2 Add widget test: provider transitions `AsyncLoading → AsyncData(context)` → router navigates to `/chat`
- [x] 3.3 Add widget test: provider transitions `AsyncLoading → AsyncData(null)` → router navigates to `/contexts`
- [x] 3.4 Run `flutter test` — all tests pass

## 4. Visual Regression Baselines

- [x] 4.1 Add a Playwright test for the `/loading` route — skipped: `/loading` is a transient router state; once Flutter WASM initialises, go_router redirects away immediately. The widget test (3.1) already covers the visual content of `LoadingScreen`.
- [x] 4.2 No new Playwright baselines needed (see 4.1).

## 5. Cleanup

- [x] 5.1 Confirm there are no remaining `if (activeContextAsync.isLoading) return null;` workaround guards left in `app_router.dart` after task 2.3
- [x] 5.2 Run `flutter analyze` and `flutter test` — both clean
