## Why

Today users have no way to manage their own account. The `PATCH /api/orgs/{org_id}/users/{id}` endpoint exists but is hidden behind the org-admin tab; the Flutter Settings screen only exposes notification toggles; the CLI has no `account` command. There is **no password-change endpoint at all** — once a user is created, their password is fixed. As `multi-user-orgs` (PR-in-progress) ships and we onboard non-admin users, this becomes a basic blocker.

## What Changes

- Add `GET /api/users/me` returning `UserDetail` for the caller (no `org_id` in URL).
- Add `PATCH /api/users/me` accepting `{ name?, email? }` for self-service profile edits.
- Add `POST /api/users/me/password` accepting `{ current_password, new_password }`. Verifies current with argon2id, hashes new, **revokes all OTHER refresh tokens** for the user (current session keeps working; API keys are untouched).
- For orgs in `auth_mode = "oidc"`: all three endpoints return `409 Conflict` with body `{ "error": "account managed by identity provider <issuer>" }`. The `GET` still works.
- **Web UI / Flutter app**: new "Account" section in Settings with three forms (name, email, change password). Hidden / disabled with an explanatory banner when the org is OIDC-managed.
- **CLI**: new `assistant account` subcommand group — `show`, `set-email <email>`, `set-name <name>`, `change-password` (interactive prompt for current + new + confirm).

Email change is **immediate** (no verification mailer is wired up in the project today); the response includes a `previous_email` field so the UI can display a "we just changed your email from X to Y" confirmation. Adding email-confirmation flow is explicitly deferred.

## Capabilities

### New Capabilities

- `self-service-account`: HTTP API + UI/CLI surfaces letting a user view and modify their own profile (name, email, password) without org-admin privileges.

### Modified Capabilities

<!-- None. The existing org-scoped admin endpoints in crates/web-ui/src/api/users.rs are unchanged; this adds a new /users/me capability alongside them. notification-settings is the only existing settings-related spec and it covers a different concern. -->

## Impact

- **Code**: new handlers in `crates/web-ui/src/api/` (likely `account.rs`); router wire-up in `crates/web-ui/src/main.rs`; new Flutter screen under `app/lib/features/account/` plus a route in `app/lib/router/app_router.dart`; new `cmd_account.rs` in `crates/interface-cli/src/` plus subcommand wiring in `main.rs`.
- **OpenAPI**: three new operations (`get_current_user`, `update_current_user`, `change_password`) added to `openapi.json`; regenerate `app/packages/assistant_api/` via `make generate-flutter-client`.
- **Auth**: needs a way to revoke all refresh tokens for a user except the caller's. `crates/auth` already has revocation primitives (`/oauth/revoke`); needs a bulk-by-user variant.
- **Storage**: no schema changes — `users.password_hash` already exists.
- **Docs**: `docs/authentication.md` gets a "Changing your password" section. **User-facing documentation: YES** — needed for both end-users (settings UI) and operators (CLI command + OIDC behavior).

## Non-goals

- Email-verification flow on email change (no mailer infra exists).
- Password reset for a forgotten password (separate flow, requires email delivery).
- Two-factor authentication / WebAuthn / passkeys.
- Account deletion / self-service org leave (org-admin still owns user lifecycle).
- Username / handle changes (we don't have usernames, only email + name).
- Changing email/password for users in OIDC-managed orgs (delegated to the IdP by design).
