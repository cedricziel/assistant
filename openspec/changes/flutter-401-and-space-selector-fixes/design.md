## Context

The Flutter web app (`app/`) uses a generated `dio` client (`assistant_api`) with no shared interceptors. Every API call goes through a `Dio` instance configured only with `baseUrl`, `connectTimeout`, `receiveTimeout`, and a static bearer token (`api_client.dart:33-44`). The only place that distinguishes 401 today is the SSE conversation list stream (`api_client.dart:274-279`), which throws `ApiAuthException`.

Auth state lives in `activeContextProvider` (an `AsyncNotifier<AssistantContext?>`), which only changes via explicit `activate()` / `deactivate()` calls. The router (`app_router.dart:127-135`) gates `/login` and `/setup` on `!hasContext`. This means a stale JWT — token present but server returns 401 — leaves `hasContext == true`, so no redirect fires; every panel just shows an error card.

`SpaceSelectorScreen` has two race-prone branches inside `space_selector_screen.dart`:

- `_OrgList` and `_SpaceList` `await api.orgs.listOrgs()` / `listSpaces()` from inside `AsyncNotifier.build()`, but if `apiClientProvider` is null they short-circuit to `return []`. The UI cannot tell that apart from a real empty result, and renders `_EmptyCard` ("No organizations found").
- The single-result branch unconditionally returns a spinner and only navigates from a post-frame callback guarded by `current.spaceId == null`. When you revisit the screen with a selection already in memory, the guard fails, the callback no-ops, and the spinner spins forever.

The logout handler in `nav_shell.dart:654-661` clears `activeContextProvider` but leaves `spaceSelectionProvider` untouched, so its in-memory state survives across logout/login.

Stakeholders: anyone using the web UI (currently the schorschvm operator). The recent JWT `org_id` fix (#676) made the symptom more obvious because pre-fix tokens silently mapped every user to `org_id="default"`; post-fix users with stale tokens now hit real 401s that the client cannot recover from.

## Goals / Non-Goals

**Goals:**

- A 401 from any `assistant_api` endpoint MUST result in either a successful refresh-and-retry, or — if refresh fails — deactivation of the active context and redirect to `/login`. No silent error states for auth failures.
- The space selector MUST distinguish loading from empty. While `apiClientProvider` is null, the UI shows a spinner, not "No organizations found".
- Revisiting `/spaces` after a single-org / single-space auto-select MUST either bounce the user back to `/chat` or render a usable list — never a stuck spinner.
- Logout MUST reset `spaceSelectionProvider` so the next session starts clean.
- All four behaviors covered by Flutter widget/unit tests so they cannot regress.

**Non-Goals:**

- Proactive token refresh based on `expires_at`. Refresh stays reactive (only on 401).
- Multi-tab session sharing or cross-tab logout broadcasts.
- Refactoring `OAuthService` or PKCE flow.
- Backend changes — the Rust `assistant-web-ui` already returns 401 correctly.
- Persisting `spaceSelectionProvider` to localStorage.

## Decisions

### Decision 1: Add a single `Dio` interceptor in `ApiClient` that owns 401 handling

The interceptor implements the well-known "refresh once, retry once, otherwise deactivate" pattern:

1. On `onError` with `response?.statusCode == 401`:
   - If the failed request already has the `x-retried` extra flag → fall through to deactivate (avoid loops).
   - Else attempt `OAuthService.refresh(refreshToken: …, clientId: …)` using the active context's `oauthCredentials`.
   - On success: persist the new credentials via `contextsProvider.notifier.saveContext(updated)`, update the bearer header, mark the request as retried, and re-issue it. Return the retry's response.
   - On failure (refresh throws, or no refresh token available): call `activeContextProvider.notifier.deactivate()`, then `handler.next(err)` so the original caller still sees the 401 (router redirect handles the visible state).
2. A `Mutex`/single-flight guard ensures only one refresh is in flight at a time; concurrent 401s wait on the same future.

**Why a Dio interceptor instead of wrapping each provider?** Centralizes the rule — every endpoint added in the future inherits it for free. Avoids polluting `OrgsNotifier`, `SpacesNotifier`, `ConversationsNotifier`, etc. with try/catch boilerplate.

**Why reactive refresh instead of proactive?** The token expiry from the server is authoritative; clocks drift. A reactive scheme means we always trust the server's "this is expired" signal. The trade-off is one extra round trip per cold-cache 401, which is negligible.

**Alternatives considered:**

- Per-call try/catch using existing `refreshIfNeeded(ref, ctx)` → noisy, easy to forget on new endpoints. Rejected.
- Periodic refresh timer → adds a clock dependency, fights the test harness, and wastes the refresh budget while idle. Rejected.

### Decision 2: Plumb a Riverpod `Ref` into `ApiClient` so the interceptor can call `deactivate()`

`ApiClient` is built by `apiClientProvider`, which already has a `Ref`. We pass `ref` (or a narrow callback bag — `onAuthExpired: () async { … }, onTokenRefreshed: (creds) async { … }`) into the constructor. The interceptor calls those callbacks; the provider wires them to `activeContextProvider.notifier.deactivate()` and `contextsProvider.notifier.saveContext(...)` respectively.

**Why callbacks instead of passing `Ref` directly?** Keeps `ApiClient` testable without a full ProviderContainer — a unit test can assert "interceptor called `onAuthExpired` after refresh failed" with two mock closures.

### Decision 3: `OrgsNotifier` / `SpacesNotifier` await the connection instead of returning `[]`

Replace `if (api == null) return [];` with `if (api == null) { state = const AsyncLoading(); return await ref.watch(...).future; }` semantics — concretely, watch `serverProfileProvider.future` to wait for the connection to settle before issuing the API call. If `serverProfileProvider` resolves with `profile == null` (genuinely logged out), then return `[]` — at that point empty is the correct answer.

**Why this over a UI-side check?** Keeps the AsyncNotifier's state machine honest (loading → data, never the data:[] → data:[1] flicker). The UI doesn't need a special case.

### Decision 4: `_SpaceList` revisit — show the list, don't spin

When `spaces.length == 1` AND the selection already has `spaceId == spaces.first.id`, the user is intentionally revisiting the selector. Render the list (single Card) with the current space marked as selected so they can confirm. The auto-navigate-to-`/chat` post-frame callback ONLY fires on the path where `spaceId == null` (first-time auto-select).

`_OrgList` is left as-is (the parent screen short-circuits to `_SpaceList` once `orgId` is set, so the same race doesn't happen there in practice).

**Alternative considered:** Unconditionally `go(/chat)` from the post-frame callback when `spaces.length == 1` regardless of prior selection. Rejected because it makes the switcher useless when the user is on `/chat` and clicks it intentionally — they'd just bounce back.

### Decision 5: Logout resets `spaceSelectionProvider`

`nav_shell.dart` logout handler adds `ref.read(spaceSelectionProvider.notifier).clear()` before `deactivate()`. Same hook fires in the `onAuthExpired` callback path so an interceptor-driven logout also clears selection.

## Risks / Trade-offs

- **Refresh loop on bad refresh token** → mitigated by the `x-retried` extras flag plus single-flight mutex; on second 401 we always fall through to deactivate.
- **Concurrent 401s during page load** (chat + sidebar + traces fetch in parallel) all hit the interceptor → mitigated by single-flight refresh; only one refresh request leaves the client.
- **`onAuthExpired` triggered while user is mid-typing in chat** → expected behavior is they get bounced to `/login`. Drafts are not preserved (out of scope; could be a follow-up).
- **Test coverage of the loading-vs-empty distinction is fragile** (timing-dependent) → use `await tester.pumpAndSettle()` plus explicit container overrides for `apiClientProvider`; assert on the rendered widget, not on internal state transitions.
- **Breaking the `_SpaceList` revisit behavior for genuinely multi-space users** → covered by both the single-space-revisit and multi-space test cases.

## Migration Plan

This is a client-only change with no backend or schema implications.

1. Land the interceptor + selector fixes behind no flag (these are bug fixes, the old behavior is broken).
2. Bump the version (`pubspec.yaml` in `app/`), rebuild the embedded web bundle (`make build` or `flutter build web --release`), and ship in the next `assistant` release.
3. After deploy on schorschvm: have the operator clear browser storage once to drop any pre-existing stale tokens, then verify the four behaviors listed in `tasks.md` Phase 5.
4. **Rollback**: revert the PR. No data migrations to undo. Old binary will still serve the old web bundle; users who already loaded the new bundle keep behavior until they hard-reload.

## Open Questions

- Should the interceptor preserve the user's pending route and restore it post-login? Currently they always land on `/chat` after login. Suggest deferring to a follow-up — out of scope for this fix.
- Should `space_selector_screen.dart` highlight the currently-selected space when rendering the list during revisit? Default to yes, using `Theme.colorScheme.primaryContainer` on the matching tile.
