## Why

When a user reloads a deep-link to a conversation (e.g. `/chat/some-id`), they end up on the conversation list or login screen instead of the conversation. This is caused by a race condition in the router's redirect logic: `activeContextProvider` loads asynchronously from SharedPreferences, and while it's loading, the redirect guard sends the user to `/loading` → then to `/login` (web) or `/contexts` (native), losing the intended destination.

## What Changes

- Preserve the intended route during the async loading window so it can be restored once auth state resolves
- Prevent the redirect guard from discarding authenticated deep-links while `activeContextProvider` is still loading
- Ensure the `/loading` → authenticated transition navigates to the originally requested path, not the default route

## Capabilities

### Modified Capabilities

- `deep-link-navigation`: Deep-links to conversations (and other authenticated routes) survive browser refresh / app restart
- `auth-redirect-guard`: Redirect logic preserves the target route during async auth loading

## Impact

- `app/lib/router/app_router.dart` — Modify redirect callback to store the pending destination when `activeContextAsync.isLoading`, restore it once auth resolves
- No backend changes
- No data model changes
