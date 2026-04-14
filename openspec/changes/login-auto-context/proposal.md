## Why

On the web, the app opens directly to the context switcher (or chat if no context), forcing users to manually create a context pointing to the server they're already browsing from — this is unnecessary friction when the server URL is always `window.location.origin`. Users need a simple "enter your token" login screen that auto-registers the context, persists the session across refreshes, and provides a visible logout path.

## What Changes

- Add a **web login screen** (`/login`) that shows only a token input field, pre-filling the server URL from `window.location.origin`. On submit it creates/updates the local-host context and activates it automatically.
- Update the **router redirect logic** so that on web (when no active context exists) users land on `/login` instead of `/contexts`.
- Persist the active context selection so that a **hard refresh** on web restores the previously active context without re-prompting.
- Add a **logout button** to the nav-rail trailing section (desktop) and the "More" sheet (mobile) that deactivates the current context and returns to `/login`.
- **Hide the Contexts nav entry on web**: the context switcher screen is replaced by the login flow on web, so multi-context management is not exposed to avoid CORS-setup confusion.

## Capabilities

### New Capabilities

- `web-login`: Dedicated token-only login screen for the web platform. Auto-detects server URL from `window.location.origin`, creates/activates a context on submit, and handles redirect back to `/chat` on success.

### Modified Capabilities

- `contexts`: On web platform, hide the contexts nav destination and switcher screen; restrict multi-context management to native (desktop/mobile) builds only.

## Impact

- **`app/lib/router/app_router.dart`**: Add `/login` route; update redirect logic to send unauthenticated web users to `/login` (not `/contexts`).
- **`app/lib/features/connection/`** (or new `app/lib/features/login/`): New `LoginScreen` widget for web.
- **`app/lib/shared/nav_shell.dart`**: Add logout icon button to the nav-rail trailing section; suppress contexts button on web.
- **`app/lib/features/contexts/providers/context_providers.dart`**: Ensure `activeContextProvider` persists selection across refreshes (already uses shared_preferences — verify and document).
- **`app/lib/features/pwa/`** or platform utils: Add `kIsWeb` guards for contexts navigation visibility.
- No Rust/backend changes required.
