## Context

The router in `app/lib/router/app_router.dart` (line 75) uses a redirect callback that checks `activeContextProvider` state:

1. If `activeContextAsync.isLoading` → redirect to `/loading`
2. If `!hasContext && !onLogin && !onContextSwitcher && !onSetup` → redirect to `/login` (web) or `/contexts` (native)

On a hard reload (browser refresh), `activeContextProvider` loads asynchronously from SharedPreferences. During this window (100-500ms), the redirect fires multiple times:

- First evaluation: `isLoading = true` → redirect to `/loading`, **original destination lost**
- Second evaluation: context resolves → if briefly null, redirect to `/login`

The intended destination (`/chat/some-id`) is never preserved or restored.

## Goals / Non-Goals

**Goals:**

- Deep-links to conversations survive browser refresh and app restart
- The router preserves the intended destination during the async auth loading window
- Once auth resolves, navigate to the originally requested route
- Handle the case where auth fails (no stored context) — still redirect to login, but cleanly

**Non-Goals:**

- Changing the auth flow itself
- Supporting deep-links to unauthenticated routes (already works)
- Persisting deep-links across sessions (e.g. "last visited conversation")

## Decisions

### D1: Store pending redirect target during loading phase

**Choice:** When `activeContextAsync.isLoading` and the current path is an authenticated route, store the intended path in a `Ref`-scoped provider (e.g. `pendingRedirectProvider`) before redirecting to `/loading`. When `/loading` detects auth resolved, read the pending redirect and navigate there.

**Why:** go_router's redirect callback is stateless — it re-evaluates on every navigation and state change. Without external state to remember the intended destination, the information is lost after the first redirect.

**Alternative considered:** Use go_router's `state.uri` in the `/loading` route to pass the original path as a query parameter (e.g. `/loading?redirect=/chat/some-id`). Simpler but fragile — additional redirects can strip query params.

### D2: The `/loading` route watches `activeContextProvider` and redirects on resolve

**Choice:** The loading screen (or the redirect callback's handling of the `/loading` path) watches `activeContextProvider`. When it transitions from loading to data:

- If context exists → navigate to `pendingRedirectProvider` value (or `/chat` if none)
- If context is null → navigate to `/login` (web) or `/contexts` (native)

**Why:** Centralises the "loading complete" transition in one place. The redirect callback only needs to handle the initial "send to /loading" step.

### D3: Don't redirect to `/loading` if already on `/loading`

**Choice:** Add `onLoading` check to the `isLoading` branch to avoid redirect loops.

**Why:** The current code already checks `onLoading` for the `!hasContext` branch but not for the `isLoading` branch (though go_router may de-duplicate). Being explicit is safer.

## Risks / Trade-offs

- **Race with `apiClientProvider`:** Even if the router preserves the deep-link, `ChatScreen` needs the API client to load the conversation. The existing `_onApiClientAvailable` listener (line 112) already handles this — it retries `_loadConversation()` when the API client becomes available. No additional change needed.
- **Multiple re-evaluations:** The redirect callback may fire many times during loading. The pending redirect should be set once (first evaluation) and not overwritten by subsequent `/loading` → `/loading` transitions.

## Migration Plan

No migration. Behavioural change only. No stored state affected.
