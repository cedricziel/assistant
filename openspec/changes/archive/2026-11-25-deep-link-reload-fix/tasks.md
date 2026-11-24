## Tasks

- [x] Create `pendingRedirectProvider` (simple `StateProvider<String?>`) to store the intended route during async loading
- [x] In `app_router.dart` redirect callback: when `activeContextAsync.isLoading` and the current path is an authenticated route, store path in `pendingRedirectProvider` before redirecting to `/loading`
- [x] When auth resolves (loading → data), read `pendingRedirectProvider` and redirect to stored path (clear it after use)
- [x] Handle auth failure: if context resolves to null, redirect to `/login` (web) or `/contexts` (native) as before
- [x] Add guard against overwriting `pendingRedirectProvider` on subsequent redirect evaluations
- [ ] Test: navigate to `/chat/{id}`, hard reload browser, verify conversation loads
- [ ] Test: navigate to `/chat/{id}` when not authenticated, verify redirect to login
- [ ] Test: navigate to `/chat` (no ID), hard reload, verify conversation list loads
- [ ] Test: navigate to `/traces`, hard reload, verify traces screen loads
