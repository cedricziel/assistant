## Why

The `remove-legacy-token-auth` change makes API keys (`ask_live_…`) the only
mechanism for programmatic access to the server (alongside short-lived JWTs
from OAuth flows). After that change, the question "how does a user create or
revoke a key from a browser?" becomes more pressing.

Most of the API key surface already exists:

- Backend: full CRUD at `/api/users/me/api-keys` (list, create, delete),
  scope-aware via `Scope` from `assistant-core::identity`, expiry-aware
  (`resolve_key` rejects expired keys at
  `crates/auth/src/api_keys.rs:102-105`).
- CLI: `assistant api-keys list / create --scopes / revoke`.
- Flutter: `ApiKeysScreen` + `api_keys_provider.dart`, route registered at
  `app_router.dart:374` as `AppRoutes.apiKeys = '/api-keys'`.

What is missing, in order of severity:

1. **The screen has no nav entry.** It's reachable only by typing `/api-keys`
   into the address bar.
2. **The create dialog only asks for a name.** There is no scope picker and
   no expiry picker, even though the backend supports both. Every key is
   created with default scopes and no expiry.
3. **`CreateApiKeyRequest` has no `expires_at` field.** The store model and
   `resolve_key` honour `expires_at`, but the create endpoint can't set it.
4. **Date columns render raw RFC3339 strings.** "Created
   `2026-04-12T13:22:09.181Z`" is not a useful UX for revocation decisions.

This proposal closes those gaps at MVP scope. "Last used at" tracking,
copy-prefix affordances, expiry warning banners, and scope-pretty-printing
are out of scope and can ship as follow-ups if needed.

## What Changes

**Backend (`crates/web-ui/src/api/api_keys.rs`):**

- Add `expires_in_days: Option<u32>` to `CreateApiKeyRequest`. When set,
  the handler computes `expires_at = now + days` and persists it. When
  `None`, key has no expiry (current behaviour).
- Add `expires_at` to the underlying create call. No other backend changes
  required — `resolve_key` already enforces expiry.

**Flutter (`app/lib/features/api_keys/`):**

- `_CreateApiKeyDialog`: add a scope picker (multi-select chips backed by
  the canonical scope set) and an expiry picker (preset chips: 30 days /
  60 days / 90 days / 1 year / no expiry). The "no expiry" option must
  show a subdued warning hint.
- `_ApiKeyTile`: render `createdAt` and `expiresAt` as relative timestamps
  (`intl` `DateFormat.yMd().add_jm()` or `timeago` if already a workspace
  dep). Render scope summary as chips when there are <=3, "N scopes" when
  more.
- Settings landing screen: add an "API keys" entry that navigates to
  `/api-keys`. The screen is too power-user for a top-level nav slot; gate
  discovery through Settings.

**Generated client:**

- `make dump-openapi` after the `CreateApiKeyRequest` change.
- `make generate-flutter-client` so the Dart `CreateApiKeyRequest` builder
  picks up the new field.

**Docs:**

- `docs/authentication.md`: document the `expires_in_days` field in the
  create endpoint section. Add a one-paragraph "Managing keys in the web
  UI" section pointing at Settings → API keys.

## Capabilities

### Added

- `api-key-management`: a user can view, create, and revoke their own API
  keys from the Flutter web/macOS app. Creation supports scope and expiry
  selection.

## Impact

**Code:**

- `crates/web-ui/src/api/api_keys.rs` — add `expires_in_days`, compute and
  pass `expires_at` on create
- `app/lib/features/api_keys/api_keys_screen.dart` — scope picker, expiry
  picker, relative dates, scope chips
- `app/lib/features/api_keys/api_keys_provider.dart` — pass new fields
  through `createKey`
- `app/lib/features/settings/settings_screen.dart` (or equivalent
  Settings landing) — link to `/api-keys`
- `openapi.json` — regenerated
- `app/packages/assistant_api/` — regenerated

**Out of scope (deliberate, can ship as follow-ups):**

- `last_used_at` tracking — needs schema migration plus middleware writes;
  worth a separate change with its own write-amplification analysis.
- Copy-prefix button on rows.
- Expiry warning banners ("3 days until expiry").
- Scope-pretty-printing (`personas:read` → "Read personas").
- Renaming the noun to "Personal access token" — codebase consistently
  uses "API key" across model, OpenAPI, CLI, screen title; renaming is a
  separate bikeshed.

**Sequencing relative to `remove-legacy-token-auth`:**

- This change does NOT block `remove-legacy-token-auth`. The minimal
  existing ApiKeysScreen + CLI is sufficient for users who lose the legacy
  bypass.
- Ship `remove-legacy-token-auth` first (security cleanup); ship this
  second.
- The two changes share the same review surface for the create flow
  (Flutter dialog), so coordinating PR order avoids merge churn.
