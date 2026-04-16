## Context

The Flutter app uses go_router with a `redirect` callback that guards routes based on whether an active context exists in `activeContextProvider`. On cold start the provider is `AsyncLoading` (reading from `SharedPreferences`); the redirect fires before it settles. The workaround introduced in the previous session returns `null` from redirect while loading, which avoids the wrong redirect but still leaves the initial route visible for ~1-3 s before the router re-evaluates. Users see a brief flash of whatever the initial route renders before landing on the correct screen.

The `_RouterRefreshNotifier` already triggers a redirect re-evaluation whenever the provider's `AsyncValue` changes, so the mechanism for self-correcting is already in place — we just need an explicit "waiting" destination.

## Goals / Non-Goals

**Goals:**

- Show a deterministic, intentional loading screen while `activeContextProvider` is `AsyncLoading`.
- Eliminate the flash of an intermediate screen (chat or context-switcher) on cold start.
- Keep the fix entirely within the Flutter router layer — no Rust changes, no API changes.

**Non-Goals:**

- Animating or branding the loading screen beyond a basic spinner + label.
- Handling slow network connections to the backend (separate concern).
- Persisting last-visited route across restarts.

## Decisions

### 1. Dedicated `/loading` route instead of `null` redirect

**Decision:** Redirect to `/loading` (a new route) while the provider is loading, rather than returning `null` and letting the current route render.

**Why:** Returning `null` means the app renders whichever route happens to be active — on first launch this is `AppRoutes.chat` (the `initialLocation`). Flutter renders it for up to 3 s before the provider settles and the redirect corrects the route. A dedicated `/loading` route gives users clear feedback and avoids accidentally rendering a protected screen before auth is confirmed.

**Alternative considered:** Wrapping the entire `MaterialApp` in an `AsyncValue.when()` to delay router creation. Rejected because it delays the entire widget tree including the navigator, causing a blank screen rather than a spinner, and it's harder to integrate with go_router's `refreshListenable`.

### 2. `LoadingScreen` lives outside `NavShell`

**Decision:** The `/loading` route is registered as a top-level `GoRoute` (sibling of `/setup`), not inside the `ShellRoute`.

**Why:** The nav shell renders the icon rail, top bar, and bottom tabs. Showing those chrome elements during loading would be confusing — they'd briefly flash navigation items before the user has an active context.

### 3. Remove the `isLoading` early-return guard

**Decision:** Replace the `if (activeContextAsync.isLoading) return null;` workaround with `return AppRoutes.loading;`.

**Why:** The `/loading` route is the correct, explicit state for "we don't know yet." Returning `null` was a temporary guard; now that there is a proper destination, the guard is redundant.

### 4. `LoadingScreen` is a simple stateless widget

**Decision:** `LoadingScreen` is a `StatelessWidget` with a `Scaffold` containing a centred `CircularProgressIndicator` and a "Starting…" text label. No `ConsumerWidget`, no state.

**Why:** The router handles navigation away from `/loading` automatically when `_RouterRefreshNotifier` fires. The screen itself has nothing to do — it just needs to look intentional.

## Risks / Trade-offs

- **Redirect loop risk** → The `/loading` route must be exempt from the redirect (like `/setup` and `/contexts`). If it isn't, the redirect will send a loading user to `/loading` which re-triggers the redirect indefinitely. Mitigation: add `final onLoading = state.fullPath == AppRoutes.loading` to the exempt condition.
- **Deep-link loss** → If a user opens a deep link while the app is cold-starting, the redirect sends them to `/loading`, and after the provider settles they land on `/chat` or `/contexts` — not the original deep-link target. Accepted trade-off for this change; restoring the deep-link target after loading is a future enhancement.

## Open Questions

_(none)_
