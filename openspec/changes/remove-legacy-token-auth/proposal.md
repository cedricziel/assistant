## Why

The web UI's `/login` page asks users to paste a "token" instead of an email and
password, and the auth middleware accepts a single static string
(`ASSISTANT_WEB_TOKEN`) as a global org-admin bypass. The bypass branch sits in
`resolve_bearer` after JWT validation and API key resolution and grants full
org-admin authority to anyone who knows the value — with no user binding, no
scope, no audit trail. The crate's own doc-comments label it the "legacy" path
and a "migration bridge".

Meanwhile, the system already runs a complete OAuth2 server with email/password
authentication (`POST /oauth/authorize`), argon2id password hashing
(`crates/auth/src/password.rs`), and a user store seeded by the migration
bootstrap (`crates/auth/src/bootstrap.rs`). The legacy bypass is the only
shortcut around it, and the token-paste login form is the only user-facing
affordance that still depends on it. Removing both unifies authentication on
the password flow that already exists.

## What Changes

**Server:**

- Remove `legacy_token` and `legacy_context` fields from `AuthState`
  (`crates/auth/src/middleware.rs`) and `WebAuthConfig`
  (`crates/web-ui/src/auth.rs`).
- Remove the third branch of `resolve_bearer` (the legacy-token match) and
  the matching branch in `AuthExtractor::from_request_parts`.
- Remove `--auth-token` / `ASSISTANT_WEB_TOKEN` from the `assistant webui serve`
  CLI args (`crates/web-ui/src/lib.rs:91`) and the bootstrap seed-password
  path (`crates/auth/src/bootstrap.rs:45`). Always generate a random initial
  password and write it to the existing `admin_credentials_file` (0600).
- **Delete the server-side `/login` page entirely**: GET/POST handlers,
  `LoginForm`, `login_html`, `SESSION_COOKIE` machinery in
  `crates/web-ui/src/auth.rs`. The Flutter SPA owns login UX going forward
  (see Decision 3). Unauthenticated requests to `/login` hit the SPA
  catch-all, which loads the Flutter app, which renders its own login
  screen.

**Flutter app:**

The Flutter `AssistantContext` model has two parallel credential slots
(`authToken` for legacy mode, `oauthCredentials` for OAuth2 mode) selected
by an `authMode` enum. After this change the legacy mode and its slot are
gone — `oauthCredentials` becomes the only credential storage:

- **Collapse `AssistantContext`** (`app/lib/features/contexts/models/context_model.dart`).
  Remove `authToken`, `AuthMode`, the `effectiveToken` branch. `oauthCredentials`
  becomes required. Add `bearerToken` getter as a direct accessor. `fromJson`
  keeps reading the `authMode` key for one release to support migration.
- **`/login`** (`app/lib/features/login/login_screen.dart`) — drop the
  legacy-token toggle, `_tokenCtrl`, `_tokenFocus`, `_tokenVisible`, the
  `token` branch of `_submit`. The "email + password" branch — which
  already calls the OAuth2 PKCE flow — becomes the only path.
- **`/setup`** (`app/lib/features/connection/connection_screen.dart`) — replace
  the token field with email + password fields. The URL field stays.
  Submit runs the OAuth2 PKCE flow against the entered URL. Delete the
  `?_token=` query-param branch and its auto-submit. Keep `?_url=`
  pre-fill (no auto-submit).
- **`/contexts/{id}/edit`** (`app/lib/features/contexts/screens/edit_context_screen.dart`)
  — remove the inline token field. Add a "Re-authenticate" button that
  runs the OAuth2 PKCE flow against the context's stored URL and writes
  fresh `oauthCredentials`. Name and URL stay editable.
- **Migration of existing contexts.** On load, contexts whose persisted
  JSON contains `authMode == "legacyToken"` are surfaced in a degraded
  state (`requiresReauth: true`, `oauthCredentials: null`). The context
  switcher / list shows them with a "Sign in again" affordance that
  triggers the OAuth2 PKCE flow. See Decision 7 in design.md.

**Operations:**

- New CLI: `assistant admin reset-password` — read an email, prompt for a new
  password, update the user. Recovery path for deployments that have lost
  access (or were running pure legacy-token before the upgrade).
- Migration note in release notes pointing operators at the
  `admin_credentials_file` written during the first-run bootstrap.

**E2E test rig (`crates/web-ui/e2e/`):**

The `/setup?_token=…` deep-link goes away (the token field is removed from
ConnectionScreen). Tests skip the UI auth flow entirely by seeding the
active context directly in `localStorage` before navigation.

- Drop `--auth-token test-token` from `playwright.config.ts` webServer cmd.
- Add a Playwright `globalSetup` that, once the server is up, reads the
  admin password from `admin_credentials_file`, performs the OAuth
  password→code→token dance against `/oauth/authorize` + `/oauth/token` to
  obtain a JWT, then `POST /api/users/me/api-keys` to mint an
  `ask_live_…` key. The key is exposed to tests via `process.env.E2E_API_KEY`.
- Add a Playwright `test.beforeEach` helper that uses `page.evaluate()` to
  write an `AssistantContext` (with `oauthCredentials.accessToken` = the
  minted API key) into `localStorage` under the `shared_preferences` keys
  the Flutter app reads from, then navigates straight to `/chat`. Replaces
  every existing `page.goto('/setup?_token=...')` call.
- Replace `const AUTH_TOKEN = "test-token"` in each spec with a read of
  `process.env.E2E_API_KEY` (via the shared helper).
- `apiGet()` continues unchanged — it sends `Authorization: Bearer <token>`
  directly and the value is now the minted API key.

**Docs:**

- `docs/authentication.md` — delete "Quick start (single-token mode)" section,
  drop step 4 from "Auth middleware resolution order", add a password-login
  quick start.
- `docs/web-ui.md` — fix Quick start, CLI options table, plain-HTTP example.
- `docs/multi-user.md` — drop legacy-token bullets from "Backward
  compatibility".
- `docs/siri-shortcut.md` — replace `--auth-token` prereq with API-key
  instructions.
- `README.md` — fix Quick start and Docker examples.
- `crates/web-ui/src/openapi.rs:104,187` — update bearer-auth descriptions;
  regenerate `openapi.json` via `make dump-openapi`.
- New ADR documenting the removal (ADR-0007 stays as historical record).

## Capabilities

### Modified

- `web-login`: the Flutter SPA owns all login UI; the server-side `/login`
  HTML form is removed. Login is performed via OAuth2 PKCE against
  `/oauth/authorize` + `/oauth/token`. `/setup` and edit-context screens
  use the same flow. Migration: existing `AssistantContext` rows with
  `authMode == "legacyToken"` are surfaced as needing re-authentication.

## Impact

**Code:**

- `crates/auth/src/middleware.rs` — drop legacy fields and branch
- `crates/auth/src/bootstrap.rs` — drop `ASSISTANT_WEB_TOKEN` seed path
- `crates/web-ui/src/auth.rs` — delete `LoginForm`, `login_page`,
  `login_submit`, `login_html`, `SESSION_COOKIE`, `logout`. Drop legacy
  bearer-resolution branch. Drop legacy fields from `WebAuthConfig`.
- `crates/web-ui/src/lib.rs` — drop `--auth-token` arg + threading; drop
  legacy-context construction; unregister the `/login` GET/POST routes
- `crates/web-ui/src/openapi.rs` — update auth descriptions; remove
  `/login` from documented paths
- `crates/interface-cli/src/main.rs` — drop the legacy-token-as-seed log
  line; add `admin reset-password` subcommand
- `app/lib/features/contexts/models/context_model.dart` — collapse to
  OAuth2-only: remove `authToken`, `AuthMode`, `effectiveToken` branch;
  add `bearerToken` getter; keep `requiresReauth` migration flag
- `app/lib/features/contexts/data/context_repository.dart` — detect and
  flag legacy contexts on load; rewrite persisted JSON without `authMode`
- `app/lib/features/login/login_screen.dart` — drop token toggle + field;
  OAuth2 PKCE becomes only path
- `app/lib/features/connection/connection_screen.dart` — replace token
  field with email + password; submit triggers OAuth2 PKCE; drop
  `?_token=` auto-submit; keep `?_url=` pre-fill
- `app/lib/features/contexts/screens/edit_context_screen.dart` — remove
  token field; add "Re-authenticate" button that runs OAuth2 PKCE
- `app/lib/router/app_router.dart` — `/setup` redirect-guard logic stays;
  the route is _not_ removed (only its credential input changes)
- `app/test/unit/contexts/*.dart`, `app/test/unit/auth/*.dart`,
  `app/test/widget/contexts/edit_context_test.dart`,
  `app/test/widget/login/login_screen_test.dart` — sweep ~40
  `authToken:` references; rewrite to use `oauthCredentials:`
- `crates/web-ui/e2e/playwright.config.ts` — drop `--auth-token` from
  webServer cmd; add `globalSetup` to mint an API key
- `crates/web-ui/e2e/tests/_helpers/seedContext.ts` (new) — writes an
  `AssistantContext` into `localStorage` so tests skip the UI auth flow
- `crates/web-ui/e2e/tests/*.spec.ts` — replace every
  `page.goto('/setup?_token=...')` with `seedContext(page)` + direct
  navigation; replace `const AUTH_TOKEN = "test-token"` with a read of
  `process.env.E2E_API_KEY`
- `openapi.json` — regenerated via `make dump-openapi`

**Operational:**

- Deployments using `ASSISTANT_WEB_TOKEN` as their only credential must use
  the admin user created at migration time (already in `org.db`) or run
  `assistant admin reset-password`. Surface this prominently in release
  notes.
- schorschvm is in this category — verify the admin credentials file exists
  on disk before rollout, or run reset-password as part of the cutover.

**Out of scope:**

- API keys (`ask_live_…`) — kept; they're scoped programmatic credentials
- OIDC mode — kept; delegated auth is still username/password at the IdP
- OAuth2 authorize / device flows — kept; already password-based
- JWT issuance / refresh tokens — kept; these are the result of password
  auth, not an alternative
