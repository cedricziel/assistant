## Context

The Flutter web app is served by the same Rust process the user wants to connect to. Because the origin is always `window.location.origin`, forcing the user to navigate to Contexts and type the URL manually is pure ceremony. The existing context system (Riverpod `activeContextProvider` backed by `shared_preferences` → `localStorage` on web) already persists the active context across hard refreshes — the UX just hasn't been wired up to take advantage of it.

Current redirect flow:

```
load → /loading → no context? → /contexts (full CRUD screen)
```

Target flow on web:

```
load → /loading → no context? → /login (token-only form, URL pre-filled)
                → has context? → /chat (restored from localStorage, no prompt)
```

## Goals / Non-Goals

**Goals:**

- Web visitors see a focused "Enter your token" login screen on first visit.
- Server URL is auto-filled from `window.location.origin` — user cannot edit it on web.
- Submitting a valid token creates a context named "Local" (or `window.location.host`) and activates it.
- Hard-refreshing while authenticated lands directly on `/chat` without prompting.
- Nav rail has a visible logout button (deactivates context, returns to `/login`).
- Contexts nav entry and switcher screen are hidden on web platform.

**Non-Goals:**

- Multi-server management on web (CORS complexity deferred; native apps retain full context switcher).
- Token validation against the server before saving (optimistic save; existing behavior).
- Changing the Rust backend or auth mechanism.
- SSO / OAuth flows.

## Decisions

### D1 — Separate `/login` route instead of repurposing `/contexts`

**Decision**: Add a new `/login` route and `LoginScreen` widget. Do not modify `ContextSwitcherScreen` for the web path.

**Rationale**: `ContextSwitcherScreen` serves a different purpose (manage multiple contexts) and is still needed on native. Keeping them separate avoids branching inside the existing screen and makes testing straightforward.

**Alternative considered**: `kIsWeb` guards inside `ContextSwitcherScreen` to switch it into login mode. Rejected — mixes concerns and complicates the widget.

### D2 — `kIsWeb` compile-time guard for platform branching

**Decision**: Use Flutter's `kIsWeb` constant from `foundation.dart` at the router redirect level and in `NavShell` to switch behaviour.

**Rationale**: `kIsWeb` is tree-shaken by the compiler; no runtime overhead. Avoids a separate platform-detection provider.

**Alternative considered**: A Riverpod provider that detects web at runtime via `UniversalPlatform`. Rejected — unnecessary indirection for a compile-time constant.

### D3 — Auto-fill URL from `Uri.base` (not hardcoded `localhost`)

**Decision**: Pre-fill the server URL field from `Uri.base.origin` (`dart:html` equivalent: `window.location.origin`). In the Flutter web context this is `Uri.base` from `dart:core`, which works without importing `dart:html`.

**Rationale**: Works in dev (`http://localhost:8080`) and production (`https://assistant.example.com`) without any configuration.

### D4 — Context name defaulted to `Uri.base.host`

**Decision**: On login, create the context with `name = Uri.base.host` (e.g. `localhost:8080` or `assistant.example.com`). Do not ask the user to name it.

**Rationale**: Reduces friction. On web there is only ever one context; the name is informational only.

### D5 — Logout button replaces Contexts button in nav rail on web

**Decision**: In `NavShell`, on web (`kIsWeb`), replace the contexts `IconButton` with a logout `IconButton` (icon: `Icons.logout`). The contexts button is hidden entirely.

**Rationale**: Context management on web is not useful (single-server, CORS constraints). A logout button at that position is the natural affordance users expect.

### D6 — Token field is optional (matches existing behavior)

**Decision**: Token input is optional — an empty token creates a context with `authToken: null`. This mirrors the current `_CreateContextDialog` behavior.

**Rationale**: Some deployments may run without authentication. Do not regress that case.

## Risks / Trade-offs

- **`Uri.base` on native platforms**: `Uri.base` returns `file://` on native. The `/login` route will never be shown on native (router guards use `kIsWeb`), but any accidental use of `Uri.base` in the login widget on native would produce an invalid URL. Mitigation: `LoginScreen` is only reachable on web; add `assert(kIsWeb)` in `initState`.

- **`localStorage` cleared by user**: If the user clears browser storage, the active context is lost and they see the login screen again. This is expected and acceptable behavior.

- **Single-context limitation on web**: If a user needs to connect to multiple servers from the same web origin, they cannot. Mitigation: documented non-goal; power users can use the native app.

## Migration Plan

1. Add `/login` route to router; update redirect guard.
2. Create `LoginScreen` widget.
3. Update `NavShell` with `kIsWeb` guards.
4. No data migration needed — existing `shared_preferences` data is unchanged.
5. Deploy: existing users with a saved context are unaffected (redirect goes to `/chat`). New users see `/login`.

## Open Questions

- None blocking implementation.
