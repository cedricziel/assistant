## Context

User-account management today is split awkwardly across the codebase:

- `crates/web-ui/src/api/users.rs` exposes `PATCH /api/orgs/{org_id}/users/{id}` with name + email, but it's only reachable through the org-admin Admin screen and requires the caller to know its own `user_id` and `org_id`.
- `crates/auth/src/password.rs` has working `hash_password` / `verify_password` (argon2id) but the only call site is user creation in `users.rs:179`. There is no change-password endpoint.
- `crates/interface-cli/src/cmd_login.rs` already establishes the pattern of self-service via `/api/users/me/...` (used for API keys), and `crates/web-ui/src/api/api_keys.rs` shows how an authenticated handler reads the caller from `AuthContext` and operates on "me" without an ID in the URL.
- The Flutter `app/lib/features/settings/settings_screen.dart` only contains notification toggles. There is no `/account` route in `app_router.dart`.

The `multi-user-orgs` change (in-progress, 99/110 tasks) is bringing real non-admin users to the system. Without self-service account management, every name/email/password change requires an org-admin — which is the wrong default for a deployed multi-user instance.

## Goals / Non-Goals

**Goals:**

- One coherent `/api/users/me` capability covering view, edit, and password change.
- Surface it identically across web UI, Flutter app, and CLI so the user's mental model is the same regardless of client.
- Do not lock a user out of their own session when they change their password.
- Cleanly degrade for OIDC-managed orgs: the controls are invisible/disabled and the API rejects writes with a clear message pointing at the IdP.

**Non-Goals:**

- Password reset for forgotten passwords (needs email delivery infra we don't have).
- Email-verification round-trip on email change (same reason).
- 2FA / WebAuthn / passkeys.
- Account deletion or self-service org leave.
- Username/handle as a separate field.

## Decisions

### Decision 1: New `/api/users/me` capability instead of reusing org-scoped routes

**Choice**: Add `GET`, `PATCH /api/users/me` and `POST /api/users/me/password` rather than reusing or generalising the existing `/api/orgs/{org_id}/users/{id}` routes.

**Why**:

- The CLI already needs a way to act without knowing its own `org_id`; `/users/me` is the existing pattern (see `api_keys.rs`, `templates.rs`).
- Keeps admin authorization (`is_org_admin()` checks) cleanly separated from self-service authorization (just "is logged in as this user").
- The org-scoped route survives unchanged for the Admin screen — no risk of regressing it.

**Alternative considered**: Reuse `PATCH /api/orgs/{org_id}/users/{id}` with caller-equals-target as the authorization rule. Rejected because every client would need to round-trip through `GET /me` first to learn its own IDs, and we'd still have to add the password endpoint somewhere new.

### Decision 2: Password change verifies current password and revokes other refresh tokens

**Choice**: `POST /api/users/me/password` requires `{ current_password, new_password }`. On success: hash + persist new, then revoke all of the user's refresh tokens **except** the one underlying the current request. API keys are not touched.

**Why**:

- Verifying current password defends against a stolen access token being used to take over the account permanently (attacker doesn't know the current password).
- Revoking other sessions is the user-expected outcome ("I changed my password, my old laptop is now logged out").
- Keeping the current session alive avoids the kick-myself-out footgun (Web UI would need to re-login after every change).
- API keys are a separate trust path (named, scoped, individually revocable); coupling them to password changes would surprise users.

**Alternative considered**: Revoke everything including current session, force re-login. Rejected as user-hostile — the user just proved they know the password.

**Alternative considered**: Don't revoke other tokens. Rejected because the typical reason for changing a password is suspected compromise; not revoking would defeat the purpose.

### Decision 3: OIDC orgs reject self-service writes with `409 Conflict`

**Choice**: When the caller's org has `auth_mode = "oidc"`, the `PATCH` and password endpoints return `409 Conflict` with `{ "error": "account managed by identity provider <issuer>" }`. `GET /api/users/me` still works (read-only is always safe).

**Why**:

- Email and password live at the IdP for OIDC users; we have no way to push them upstream.
- `409` (rather than `403`) signals "the resource is in a state where this operation can't apply", which is exactly the case here. `403` would imply "you're not allowed", which is misleading.
- The error body names the IdP so the UI can render an actionable link.

**Alternative considered**: Hide endpoints entirely (404). Rejected because clients (CLI, app) need a deterministic way to detect "managed elsewhere" without out-of-band knowledge.

### Decision 4: Email change is immediate; no verification mailer

**Choice**: A `PATCH /api/users/me` with a new email writes immediately and returns the new value. The response body includes a `previous_email` field so clients can render a confirmation banner ("Email changed from X to Y").

**Why**:

- The codebase has no SMTP/transactional-email integration. Adding one is a much larger scope.
- Self-hosted single-org deployments (the most common deployment today per `~/.assistant/` layout) generally don't need verification — the operator knows the user.
- The proposal explicitly defers email-verification.

**Mitigation**: The Settings UI shows a clear "this takes effect immediately" hint and the response shape lets us layer verification on later without breaking the API.

### Decision 5: CLI uses `assistant account` subcommand group

**Choice**: New top-level subcommand `account` with children `show`, `set-email <email>`, `set-name <name>`, `change-password`. Implementation in `crates/interface-cli/src/cmd_account.rs`, mirroring `cmd_login.rs` style and reusing its `authenticated_client` helper.

**Why**:

- Mirrors existing `api-keys` subcommand layout exactly (see `main.rs:142-147`), so users who know one know the other.
- `change-password` is interactive (TTY prompts using `rpassword` — the standard Rust hidden-input crate) since accepting passwords on the command line leaks them into shell history.

**Alternative considered**: Flat commands like `assistant set-email`. Rejected as cluttering the top-level namespace and inconsistent with `api-keys`/`persona`/`skill`.

### Decision 6: Flutter app gets a new `/account` route nested under Settings

**Choice**: New screen at `app/lib/features/account/account_screen.dart` reachable via a tile in `settings_screen.dart` ("Account" → arrow). Route `/settings/account` registered in `app_router.dart`.

**Why**:

- Settings is the natural home; surfacing it as a sub-screen avoids overloading the Settings root with three forms.
- Cupertino sub-page navigation is the iOS-native pattern (`CupertinoPageRoute`); Material gets a standard pushed route.
- A separate screen keeps form state, validation, and "managed by IdP" banner logic isolated from notification toggles.

## Risks / Trade-offs

- **[Risk] Email-change immediate-write enables a typo lockout** → Mitigation: client-side double-entry confirmation in all three UIs; `previous_email` in response so an admin can manually correct via the org-scoped PATCH if needed.
- **[Risk] Password-change without rate limiting could enable brute-forcing the _current_ password via the API** → Mitigation: add a per-user rate limit (e.g. 5 attempts / 15 min) at the handler level. Already a TODO for `/oauth/authorize`; consolidate.
- **[Risk] Refresh-token revocation logic gets it wrong and kicks the user out anyway** → Mitigation: integration test that asserts the calling refresh token survives while a sibling token is revoked. `crates/integration-tests` is the right place.
- **[Trade-off] No email verification means a malicious actor with a stolen access token could change the user's email and lock them out (then change password)** → Accepted because they'd need both the access token AND the current password; verifying current password is the primary defence. Adding email verification is tracked as a non-goal but a future capability.
- **[Trade-off] OIDC orgs see read-only Account screen, which may confuse users who don't realise their org is OIDC-managed** → Mitigation: explanatory banner naming the IdP issuer.

## Migration Plan

1. Ship the new endpoints first behind no flag — they're additive and unauthenticated callers get `401` from the existing auth middleware.
2. Regenerate `app/packages/assistant_api/` and merge in the same PR (per `make generate-flutter-client` discipline).
3. Ship UI/CLI surfaces in follow-up PRs — backend works standalone for `curl` users.
4. Rollback is `git revert`; no data migration, no schema change.

## Open Questions

- Should the CLI `account show` print the full `UserDetail` (including `created_at`/`updated_at`) or a trimmed view? Leaning trimmed by default with `--json` for the full payload, matching `api-keys list`'s style.
- Where does refresh-token-by-user revocation actually live in `crates/auth`? Need to confirm whether to add a new method on the token store or compose existing single-token revocation. Will be answered while implementing.
