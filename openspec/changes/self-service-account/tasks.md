## 1. Auth: bulk refresh-token revocation

- [x] 1.1 Locate the refresh-token store/repo in `crates/auth` and identify how single-token revocation works today (`/oauth/revoke` handler call site).
- [x] 1.2 Add a method `revoke_user_refresh_tokens_except(user_id, except_jti)` (or equivalent) to the token store, returning the count revoked.
- [x] 1.3 Write a unit test: create three refresh tokens for one user, call the new method excepting one, assert the excepted token still validates and the others do not.

## 2. Backend: `/api/users/me` capability

- [x] 2.1 Create `crates/web-ui/src/api/account.rs` with module-level docs listing the three routes (mirror `users.rs:1-11` style).
- [x] 2.2 Define request/response types: `UpdateCurrentUserRequest { name?, email? }`, `UpdateCurrentUserResponse` (a `UserDetail` plus optional `previous_email`), `ChangePasswordRequest { current_password, new_password }`. Use `utoipa::ToSchema` derives.
- [x] 2.3 Implement `get_current_user` handler — read `AuthContext`, look up user, return `UserDetail`. Include `#[utoipa::path]` annotation with `operation_id = "get_current_user"`.
- [x] 2.4 Implement `update_current_user` handler — apply name/email patch, dedupe email within org (return 409), reject empty/invalid email (400), include `previous_email` in response when changed. `operation_id = "update_current_user"`.
- [x] 2.5 Implement `change_password` handler — verify current via `assistant_auth::password::verify_password`, hash new via `hash_password`, persist, then revoke **all** of the user's refresh tokens via `revoke_for_user_except(user_id, "")`. Return `204 No Content`. The calling access JWT survives until natural expiry. `operation_id = "change_password"`.
- [x] 2.6 Add OIDC-mode guard to `update_current_user` and `change_password`: if the user's org has `auth_mode == "oidc"`, return `409` with `{"error": "account managed by identity provider <issuer>"}`. `get_current_user` is unguarded.
- [x] 2.7 Wire the router in `crates/web-ui/src/main.rs` (or wherever `users_api_router` is mounted) under `/api`.
- [x] 2.8 Write handler tests in `account.rs` `#[cfg(test)] mod tests` covering every scenario in `specs/self-service-account/spec.md` for the three endpoints (success paths, OIDC rejection, email collision, wrong current password, empty new password, sibling-tokens revoked + cross-user untouched). Note: "API keys survive password change" and "calling JWT continues to work" are deferred to the §7 integration test, which exercises a real bearer-token / API-key auth flow.

## 3. OpenAPI + generated Flutter client

- [ ] 3.1 Register the three new operations in `crates/web-ui/src/openapi.rs` `paths(...)` and `schemas(...)` sections.
- [ ] 3.2 Run `make dump-openapi` and verify `openapi.json` now contains the three operations with snake_case operationIds.
- [ ] 3.3 Run `make generate-flutter-client` and verify `app/packages/assistant_api/lib/src/api/users_api.dart` (or a new `account_api.dart`) exposes `getCurrentUser`, `updateCurrentUser`, `changePassword`.
- [ ] 3.4 Add a generated-client smoke test under `app/packages/assistant_api/test/` that calls the three operations against a mocked dio and asserts request shapes.

## 4. Web UI: Account section in Settings

- [ ] 4.1 Identify where the existing web Settings page lives (web-ui crate's HTML templates if applicable, or note that the Web UI = the Flutter web build and §5 covers it).
- [ ] 4.2 If a separate Web UI HTML/template exists, add an "Account" section with name, email (with confirm), and change-password (current + new + confirm) forms; otherwise mark this group as N/A and document why.
- [ ] 4.3 Wire OIDC banner display when `auth_mode == "oidc"` (read from `GET /api/users/me` + `GET /api/orgs/<id>` or equivalent).
- [ ] 4.4 Manually test in browser: change name, change email (verify previous-email banner), change password (verify other browser session is logged out, current stays in).

## 5. Flutter app: Account screen

- [ ] 5.1 Create `app/lib/features/account/account_provider.dart` with an `AsyncNotifier` exposing the current user and methods `updateName`, `updateEmail`, `changePassword` that call the generated client.
- [ ] 5.2 Create `app/lib/features/account/account_screen.dart` with three sections: Name (text field + save), Email (new + confirm + save), Password (current + new + confirm + save). Use adaptive widgets per project convention.
- [ ] 5.3 Render OIDC banner (read-only fields, hidden password section, "Managed by <issuer>" text) when the org's `auth_mode == "oidc"`.
- [ ] 5.4 Add an "Account" tile to `app/lib/features/settings/settings_screen.dart` with `Icons.person_outline` / `CupertinoIcons.person` that navigates to `/settings/account`.
- [ ] 5.5 Register the `/settings/account` route in `app/lib/router/app_router.dart` (after the existing `/settings` entry).
- [ ] 5.6 Add a widget test under `app/test/widget/account_screen_test.dart` covering: form rendering, OIDC banner toggle, email-confirmation mismatch blocks submit, successful save shows confirmation.

## 6. CLI: `account` subcommand

- [ ] 6.1 Add `rpassword` to `[workspace.dependencies]` in root `Cargo.toml` and depend on it from `crates/interface-cli/Cargo.toml`.
- [ ] 6.2 Promote `authenticated_client` from private inside `cmd_login.rs` to a `pub(crate)` helper (or move to `credentials.rs`) so `cmd_account.rs` can reuse it.
- [ ] 6.3 Create `crates/interface-cli/src/cmd_account.rs` with four functions: `cmd_account_show`, `cmd_account_set_email`, `cmd_account_set_name`, `cmd_account_change_password`.
- [ ] 6.4 Implement `cmd_account_show` — `GET /api/users/me`, print email/name/org_id/auth_mode in a friendly block; support `--json` for raw `UserDetail`.
- [ ] 6.5 Implement `cmd_account_set_email` and `cmd_account_set_name` — `PATCH /api/users/me`, on success print `<Field> changed from <old> to <new>.` (use `previous_email` when present).
- [ ] 6.6 Implement `cmd_account_change_password` — use `rpassword::prompt_password` for current + new + confirm; reject mismatch locally; submit to `POST /api/users/me/password`; on `409` (OIDC), print actionable message naming issuer; on success print `Password changed.`.
- [ ] 6.7 Wire up `Account { command: AccountCommand }` enum and `AccountCommand::{Show, SetEmail, SetName, ChangePassword}` variants in `crates/interface-cli/src/main.rs` and dispatch to the new handlers.
- [ ] 6.8 Add CLI parsing tests in `main.rs` `#[cfg(test)] mod tests` for `account show`, `account set-email foo@bar`, `account set-name "X"`, `account change-password` (mirror existing `parses_api_keys_*` tests).

## 7. Integration test (cross-cutting)

- [ ] 7.1 Add `crates/integration-tests/tests/account.rs` (or extend `smoke.rs`) covering: log in, get-me, patch email, patch name, change password, verify old refresh token is revoked but current session continues, verify API keys still work.

## 8. Documentation

- [ ] 8.1 Add a "Changing your account" section to `docs/authentication.md` describing the three endpoints, OIDC behavior, and refresh-token revocation semantics.
- [ ] 8.2 Add a CLI usage block to whatever README/CLI doc covers `assistant login` (likely `crates/interface-cli/README.md` or top-level `README.md`).
- [ ] 8.3 Note the new Account screen in user-facing app documentation (if `docs/` has an app-tour section, otherwise skip).

## 9. Polish & verification

- [ ] 9.1 Run `make lint` and `make format` — fix any issues.
- [ ] 9.2 Run `make test` and `make test-flutter` — all green.
- [ ] 9.3 Run `make precommit` — all hooks pass.
- [ ] 9.4 Manually exercise CLI happy path against a local `assistant webui serve` instance: `assistant account show`, `set-email`, `set-name`, `change-password`. Verify the next `assistant login` (or session refresh) on a second machine fails until re-login.
