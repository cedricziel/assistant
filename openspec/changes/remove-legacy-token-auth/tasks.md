## Tasks

TDD discipline: every implementation task is preceded by a failing test.

### Server: delete `/login` HTML page

- [ ] Write failing test: request to `GET /login` returns the SPA
      fallback (200, content-type text/html with the Flutter app bundle),
      not a server-rendered login form.
- [ ] Delete `LoginForm`, `login_page`, `login_submit`, `login_html`,
      `logout`, `SESSION_COOKIE`, and `redirect_with_cookie` from
      `crates/web-ui/src/auth.rs`.
- [ ] Unregister the `/login` GET and POST routes (and `/logout`) in
      `crates/web-ui/src/lib.rs`. The SPA catch-all handles unauthenticated
      paths.
- [ ] Delete the existing `login_*` tests in `crates/web-ui/src/auth.rs`
      (they exercise the deleted handlers).
- [ ] Verify `/oauth/authorize` and `/oauth/device/verify` are unaffected;
      they have their own credential forms and continue to function.

### Server: remove legacy bypass

- [ ] Write failing test (or modify existing): `AuthState` no longer has
      `legacy_token` / `legacy_context` fields — tests that construct it must
      compile without those fields.
- [ ] Delete the legacy match branch in `crates/auth/src/middleware.rs`
      `AuthExtractor::from_request_parts` and its mirror in
      `crates/web-ui/src/auth.rs:resolve_bearer`.
- [ ] Delete `legacy_token` and `legacy_context` fields from `AuthState`
      and `WebAuthConfig`. Update `WebAuthConfig::new` signature.
- [ ] Delete `extract_legacy_token` and
      `bearer_legacy_token_produces_admin_context` tests.
- [ ] Update remaining tests to drop the legacy-context arguments.

### Server: remove `ASSISTANT_WEB_TOKEN` plumbing

- [ ] Delete `--auth-token` / `ASSISTANT_WEB_TOKEN` arg from
      `crates/web-ui/src/lib.rs` (`Args`, `legacy_token` resolution, threading
      into `WebAuthConfig::new`).
- [ ] Delete the `ASSISTANT_WEB_TOKEN` seed-password branch in
      `crates/auth/src/bootstrap.rs:45`; always generate.
- [ ] Delete the "Password: (your ASSISTANT_WEB_TOKEN value)" branch in
      `crates/web-ui/src/lib.rs:241` and the matching line in
      `crates/interface-cli/src/main.rs:1501`.

### Recovery CLI: `assistant admin reset-password`

- [ ] Write failing test: integration test that runs the CLI against a temp
      `org.db` with a seeded user, supplies a new password via stdin, then
      verifies the new password against the updated hash and the old one
      fails.
- [ ] Implement subcommand in `crates/interface-cli/src/main.rs` under an
      `admin` group: parses `<email>`, reads the password from TTY via
      `rpassword`, opens `org.db` directly via `OrgPoolFactory`, calls
      `password::hash_password`, updates the row.
- [ ] Add `--org` flag (defaults to `default`); fail clearly if the user is
      not found.

### Flutter app: collapse `AssistantContext` credential model

- [ ] Write failing unit test: `AssistantContext` constructor signature
      takes required `OAuthCredentials`; no `authToken` parameter; no
      `authMode` parameter.
- [ ] Write failing unit test: `bearerToken` getter returns
      `oauthCredentials.bearerToken` directly (no mode branching).
- [ ] Write failing unit test: `needsTokenRefresh` returns
      `oauthCredentials.isExpired()` (no mode guard).
- [ ] Write failing unit test: `toJson` does not include `authMode` or
      `authToken` keys; `fromJson` reading a payload with `authMode ==
  "legacyToken"` produces a context with `requiresReauth: true`,
      `oauthCredentials: null`.
- [ ] Write failing unit test: `fromJson` reading a payload with no
      `authMode` key (post-migration) produces a normal context.
- [ ] Remove `authToken`, `AuthMode`, `effectiveToken` from
      `app/lib/features/contexts/models/context_model.dart`. Make
      `oauthCredentials` required in the constructor (or keep optional
      paired with `requiresReauth`).
- [ ] Add `bearerToken` getter as a direct field access.
- [ ] Add `requiresReauth: bool` in-memory flag (default false, never
      serialised).
- [ ] Update `copyWith`: drop `authToken` and `clearAuthToken`; keep
      `clearOAuthCredentials`; add `requiresReauth`.
- [ ] Update `AssistantContext.create` factory: take required
      `oauthCredentials` instead of optional `authToken` / `authMode`.

### Flutter app: legacy context migration

- [ ] Write failing test: `ContextRepository.loadContexts` reading a
      stored row with `authMode == "legacyToken"` returns a context with
      `requiresReauth: true` and `oauthCredentials: null`. The next
      `saveContext` rewrites the persisted JSON without `authMode`.
- [ ] Write failing widget test: contexts list / switcher renders a
      `requiresReauth: true` context with a "Sign in again" affordance
      instead of the normal connect button.
- [ ] Write failing widget test: tapping "Sign in again" launches the
      OAuth2 PKCE flow against the context's stored URL; on success the
      context's `requiresReauth` flips to false and `oauthCredentials`
      is populated.
- [ ] Implement the legacy detection in
      `app/lib/features/contexts/data/context_repository.dart`.
- [ ] Update `AssistantContext.fromJson` to detect `authMode ==
  "legacyToken"` and emit `requiresReauth: true`.
- [ ] Implement the "Sign in again" affordance in the contexts
      list/switcher.

### Flutter app: `/login` screen

- [ ] Write failing widget test: login screen renders email + password
      inputs only (no token TextField, no credential-type toggle).
- [ ] Write failing widget test: submitting with valid credentials runs
      the OAuth2 PKCE flow (mocked `OAuthClient`) and stores the
      resulting `OAuthCredentials` on the active `AssistantContext`.
- [ ] Remove the legacy-token toggle, `_tokenCtrl`, `_tokenFocus`,
      `_tokenVisible`, and the `token` branch of `_submit` in
      `app/lib/features/login/login_screen.dart`.
- [ ] The submit handler keeps the existing OAuth2 PKCE call path (it
      already exists for the "email + password" branch); only the dead
      legacy branch is removed.

### Flutter app: `/setup` screen (ConnectionScreen)

- [ ] Write failing widget test: ConnectionScreen in remote mode renders
      URL + email + password fields (no token field).
- [ ] Write failing widget test: navigating to `/setup?_url=https://x`
      pre-fills the URL field but does NOT auto-submit.
- [ ] Write failing widget test: `/setup?_token=foo` does NOT pre-fill or
      submit anything (the query param is ignored).
- [ ] Write failing widget test: submitting valid credentials runs the
      OAuth2 PKCE flow against the entered URL and saves an
      `AssistantContext` with `oauthCredentials` populated.
- [ ] Replace `_tokenController` and the token TextFormField with
      `_emailController` + `_passwordController` and matching fields in
      `app/lib/features/connection/connection_screen.dart`.
- [ ] Delete the `?_token=` query-param branch in `initState` and the
      `WidgetsBinding.instance.addPostFrameCallback` auto-submit. Keep
      `?_url=` pre-fill.
- [ ] Update `_connect` to run the OAuth2 PKCE flow (re-using
      `login_screen.dart`'s existing helpers) against the entered URL,
      then build the `AssistantContext` with the resulting
      `oauthCredentials`.
- [ ] Verify the embedded-mode branch (macOS bundled binary) is unchanged.

### Flutter app: edit-context screen

- [ ] Write failing widget test: edit-context screen renders name + URL
      fields and a "Re-authenticate" button (no token input).
- [ ] Write failing widget test: tapping "Re-authenticate" launches the
      OAuth2 PKCE flow against `context.serverUrl`; on success the
      context's `oauthCredentials` is replaced.
- [ ] Remove `_tokenCtrl`, `_tokenVisible`, and the token TextFormField
      from `app/lib/features/contexts/screens/edit_context_screen.dart`.
- [ ] Add the "Re-authenticate" affordance that triggers the same
      OAuth2 PKCE flow used by `/login`.

### Flutter app: test sweep

- [ ] Sweep `app/test/unit/contexts/context_model_test.dart` (~25
      references): replace `makeCtx(authToken: 'tok')` with
      `makeCtx(oauthCredentials: ...)`. Delete the `effectiveToken
  returns authToken for legacyToken mode` scenario.
- [ ] Sweep `app/test/unit/contexts/context_repository_test.dart` and
      `context_repository_upsert_test.dart` (~15 references): same
      pattern.
- [ ] Sweep `app/test/unit/auth/auth_provider_test.dart` (~5
      references): same pattern.
- [ ] Update `app/test/widget/contexts/edit_context_test.dart` (drop
      `authToken: 'my-token'` fixture; assert no token field).
- [ ] Delete legacy-mode scenarios from
      `app/test/widget/login/login_screen_test.dart` (lines 43, 85
      and similar — see the file's `_switchToLegacy()` helper).
- [ ] `grep -rn "authToken\|AuthMode\|legacyToken\|effectiveToken" app/lib app/test`
      — must return zero hits outside the migration code path in
      `context_model.dart` and `context_repository.dart`.

### Flutter app: cleanup

- [ ] Run `flutter analyze --fatal-infos`.
- [ ] Run `flutter test`.

### E2E test rig (`crates/web-ui/e2e/`)

- [ ] Add `globalSetup.ts` that polls `/health`, reads
      `~/.assistant/orgs/default/admin_credentials.txt`, runs the OAuth
      password→code→token dance, mints an `ask_live_…` API key via
      `POST /api/users/me/api-keys`, and writes the key to
      `process.env.E2E_API_KEY`.
- [ ] Wire `globalSetup` into `playwright.config.ts`. Remove
      `--auth-token test-token` from the `webServer.command`.
- [ ] Inspect `app/lib/features/contexts/data/context_repository.dart` to
      determine the exact `shared_preferences` key names used for the
      contexts list and the active-context id on web.
- [ ] Add `tests/_helpers/seedContext.ts` exporting `seedContext(page)`.
      Uses `page.evaluate()` to write a JSON-encoded `AssistantContext`
      (with `authToken = process.env.E2E_API_KEY`) into `localStorage`
      under the keys identified above, and marks it as the active
      context. Throws clearly if `E2E_API_KEY` is unset.
- [ ] Add `tests/_helpers/auth.ts` exporting `getAuthToken()` that returns
      `process.env.E2E_API_KEY` (used by `apiGet` and any direct REST
      callers in tests).
- [ ] Replace every `page.goto('/setup?_token=...')` call in the four
      specs with `await seedContext(page); await page.goto('/chat');`.
- [ ] Replace `const AUTH_TOKEN = "test-token"` in:
      `visual-regression.spec.ts`, `trace-tool-call-rendering.spec.ts`,
      `platform-io-web.spec.ts`, `sidebar-collapse-ipad.spec.ts` with
      `getAuthToken()`.
- [ ] Run the suite locally end-to-end: `npm run test` from
      `crates/web-ui/e2e/`. Verify all four specs pass without ever
      visiting `/login` or `/setup`.
- [ ] Document the new e2e bootstrap flow in
      `crates/web-ui/e2e/README.md` (create if absent), including the
      `seedContext` mechanism and how to regenerate the admin password
      if the credentials file is missing.

### OpenAPI

- [ ] Update bearer-auth descriptions in `crates/web-ui/src/openapi.rs:104,187`
      to drop `--auth-token` / `ASSISTANT_WEB_TOKEN` references.
- [ ] Run `make dump-openapi` to regenerate `openapi.json`.
- [ ] Run `make generate-flutter-client`; verify no diff beyond the auth-doc
      text.

### Docs

- [ ] `docs/authentication.md`: delete "Quick start (single-token mode)"
      section, drop step 4 from "Auth middleware resolution order", add a
      password-login quick start under "Quick start".
- [ ] `docs/web-ui.md`: fix Quick start example (L11-12), the "enter the
      server URL and token" copy (L18-20), the `--auth-token` row in the CLI
      table (L132), the plain-HTTP example (L152).
- [ ] `docs/multi-user.md`: rewrite the "Backward compatibility" bullets
      (L183-184) to drop legacy-token mentions.
- [ ] `docs/siri-shortcut.md`: replace the `--auth-token` prereq (L8) with
      an API-key prereq.
- [ ] `README.md`: fix L318 (web-ui quick start) and L433 (Docker example).
- [ ] Add `docs/adr/adr-0012-remove-legacy-token-auth.md` (or next available
      number) documenting the decision, with a backlink from ADR-0007.

### Final sweep

- [ ] `rg "ASSISTANT_WEB_TOKEN|--auth-token|legacy.token|legacy_context|test-token"` —
      must return zero hits outside this change's own files and the new ADR.
- [ ] `rg "authToken|AuthMode|legacyToken|effectiveToken|LoginForm|login_html|login_submit"`
      across `crates/` and `app/lib/` — must return zero hits outside the
      Flutter migration code path (`context_model.dart`, `context_repository.dart`).
- [ ] `make lint && make format && make test`.
- [ ] `cd app && flutter analyze && flutter test`.
- [ ] Smoke test (server): fresh deployment (no `org.db`), confirm bootstrap
      creates the admin user and writes `admin_credentials_file`. Confirm
      `GET /login` returns the SPA (not a server-rendered form).
- [ ] Smoke test (Flutter web): on a fresh deployment, navigate to the
      Flutter login screen, enter email + password, and confirm the OAuth2
      PKCE flow completes and the app lands on `/chat`.
- [ ] Smoke test: existing deployment (with admin user), confirm Bearer
      auth still works with JWTs and API keys.
- [ ] Smoke test: `assistant admin reset-password admin@localhost` updates
      the password and a subsequent Flutter-app sign-in succeeds.
- [ ] Smoke test (Flutter web): `/setup` accepts URL + email + password and
      saves an active context with populated `oauthCredentials`. Verify
      `?_url=https://x.example.com` pre-fills the URL but does not
      auto-submit.
- [ ] Smoke test (Flutter web): `/setup?_token=anything` does not pre-fill
      or submit a token (the query param is silently ignored).
- [ ] Smoke test (Flutter web): edit-context's "Re-authenticate" flow runs
      the OAuth2 PKCE flow against the context's URL and refreshes the
      stored credentials.
- [ ] Smoke test (migration): with a context written under the legacy
      model (`authMode == "legacyToken"`), confirm the app surfaces it
      as needing re-authentication, the "Sign in again" affordance
      launches OAuth2 PKCE, and the next save persists JSON without
      `authMode`.
