## Why

When the Flutter app starts, `activeContextProvider` is in `AsyncLoading` state while it reads from `SharedPreferences`. The go_router redirect fires synchronously before the provider settles, sees `value == null`, and redirects the user to the context-switcher screen — even though a valid context already exists in storage. The current workaround (skip redirect while loading) leaves a visible flash before the correct route renders. A proper loading scaffold eliminates the flash and makes the intent explicit in the UI.

## What Changes

- Add a `/loading` route that renders a full-screen loading scaffold (spinner + "Starting…" label).
- Update the go_router redirect to send the app to `/loading` while `activeContextProvider.isLoading`, instead of returning `null` (which lets whatever route happens to be active render first).
- Once the provider settles the `_RouterRefreshNotifier` fires, the redirect re-evaluates, and the router navigates to the correct destination (`/chat` or `/contexts`).
- Remove the `isLoading` early-return guard added as a workaround in the previous fix (replaced by the `/loading` route strategy).

## Capabilities

### New Capabilities

- `router-loading-scaffold`: Full-screen loading route shown while async providers initialise, preventing premature redirects and visible UI flashes on cold start.

### Modified Capabilities

_(none — no existing spec-level requirements change)_

## Impact

- `app/lib/router/app_router.dart` — new `/loading` route + updated redirect logic.
- `app/lib/shared/loading_scaffold.dart` (new file) — simple `LoadingScreen` widget.
- `app/lib/router/app_router.dart` `AppRoutes` constants — new `loading` entry.
- No API changes, no Rust changes, no pubspec changes.
