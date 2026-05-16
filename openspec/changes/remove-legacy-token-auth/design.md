## Context

The current bearer-token resolver (`resolve_bearer` in `crates/web-ui/src/auth.rs:83`
and the mirror in `crates/auth/src/middleware.rs:107`) has three branches:

1. JWT validation
2. API key resolution (prefix `ask_live_`)
3. **Legacy static-token match** — if the configured `ASSISTANT_WEB_TOKEN`
   string equals the bearer, return a pre-built org-admin `AuthContext`.

Branch 3 is the bypass. It has no user binding, no scope, no audit identity
beyond `client_id: "legacy"`, and any holder of the string is an org admin.

The same env var doubles as the initial admin-password seed in
`crates/auth/src/bootstrap.rs:45`. Two distinct roles, one variable name.

The web `/login` POST handler (`crates/web-ui/src/auth.rs:394`) accepts a single
`token: String` form field and runs it through `resolve_bearer`. In practice
this means the only thing a fresh deployment can paste there is the legacy
token — JWTs and API keys can be sent on every request directly. So killing
branch 3 without rebuilding `/login` would leave the page broken.

Email+password auth already exists at `POST /oauth/authorize`
(`crates/web-ui/src/oauth/authorize.rs:171`) and uses `argon2id` via
`password::verify_password`. The user store has the hashed password.

## Decisions

### Decision 1: One bundled change, not two

Removing the bypass and rebuilding the login form are technically separate but
operationally inseparable. Doing only A leaves the login form broken; doing
only B leaves the security hole. Bundling avoids a window where users can't
log in via the UI.

### Decision 2: Kill `ASSISTANT_WEB_TOKEN` entirely

Both roles go away:

- **As a bearer bypass** — gone with branch 3.
- **As a seed password** — bootstrap always generates a random 24-char password
  and writes it to `admin_credentials_file` with mode 0600
  (`crates/web-ui/src/install.rs:94`). That file already exists in the migration
  flow; we just remove the `ASSISTANT_WEB_TOKEN` branch in `bootstrap.rs:45`.

Rejected alternative: keep `ASSISTANT_WEB_TOKEN` as seed-only. Adds a code path
nobody will exercise after the first run, and keeping the env var name alive
invites confusion (operators will paste it into Bearer headers expecting it to
work).

### Decision 3: Delete server-side `/login` entirely — Flutter owns login UI

This decision changed mid-proposal once we discovered the full
`AssistantContext` credential model (see Decision 5). The original plan
was to rebuild `/login`'s POST handler to accept email+password and set a
session cookie. That plan no longer fits because:

- The Flutter SPA is the only browser-facing UI. It has its own `/login`
  route rendered client-side by `login_screen.dart`.
- An unauthenticated request to `/login` hits the SPA fallback (the
  catch-all that serves `index.html` for unknown paths), which loads the
  Flutter app, which navigates to its client-side `/login`.
- The Flutter app needs **access + refresh tokens** (to populate
  `OAuthCredentials`), not a session cookie. The right protocol is
  `/oauth/authorize` + `/oauth/token` via PKCE — which already exists and
  is what `login_screen.dart`'s "email + password mode" already calls.

So the server-side `/login` GET and POST handlers, the `LoginForm` struct,
the `login_html` renderer, and the `SESSION_COOKIE` machinery in
`crates/web-ui/src/auth.rs` are **deleted, not rebuilt**. The capability
spec's old "Email + password login form" requirement, which described an
HTML form posted to the server, is replaced by "Flutter login screen uses
OAuth2 PKCE to obtain credentials".

What stays:

- `/oauth/authorize` already renders an email+password form (for browser
  clients that bounce through it during the auth code flow). No change
  needed.
- `/oauth/device/verify` similarly already takes email+password for the
  device-code flow. No change.
- The `assistant_session` cookie semantics — they're set by the
  `/oauth/callback` handler today and remain unchanged.

Rejected alternative: keep `/login` as a thin email+password form that
itself initiates the OAuth flow (server-side redirect to `/oauth/authorize`
on submit). Adds a second login surface for no real user benefit; better
to make the SPA the single owner.

### Decision 4: Recovery via `assistant admin reset-password`

Existing deployments using `ASSISTANT_WEB_TOKEN` need a way to log in once the
bypass is gone. Two cases:

- **Migrated deployment (has an admin user in `org.db`)** — use the
  credentials in `admin_credentials_file`, or run reset-password to pick a new
  one.
- **Never-migrated deployment** — the bootstrap path runs on every fresh
  startup if no admin user exists. After this change, on first boot post-
  upgrade, the bootstrap creates the user and writes the credentials file.

`assistant admin reset-password` is a small but necessary new affordance: a
CLI that opens `org.db` directly, finds the user, prompts for a new password
via TTY, and updates the `password_hash`. No JWT auth required because it
runs as the OS user with file access to the SQLite database. Mirrors the
existing `account change-password` CLI but without needing a session.

### Decision 5: Collapse `AssistantContext` to OAuth2-only credentials

`AssistantContext` is a richer model than initially scoped. It carries:

```dart
enum AuthMode { legacyToken, oauth2 }

class AssistantContext {
  final String?            authToken;        // legacy slot
  final OAuthCredentials?  oauthCredentials; // oauth slot
  final AuthMode           authMode = legacyToken;  // default!
  String? get effectiveToken { /* branches on authMode */ }
  bool    get needsTokenRefresh => /* oauth-only guard */;
}
```

`AuthMode.legacyToken` is "ye olde" on the client side — the same era and
mental model as the server's `ASSISTANT_WEB_TOKEN` bypass. `AuthMode.oauth2`
already has full plumbing (refresh tokens, expiry checks, the 401
interceptor's refresh logic). The honest endpoint is to remove the dead
mode, not merely narrow how it's filled:

```
BEFORE                              AFTER
─────────────────────────           ─────────────────────────────

class AssistantContext {            class AssistantContext {
  String?        authToken;           // authToken: REMOVED
  OAuthCreds?    oauthCreds;          OAuthCreds  oauthCreds;  // now required
  AuthMode       authMode;            // authMode: REMOVED
  effectiveToken {…branches…}        String get bearerToken =>
}                                      oauthCredentials.bearerToken;
                                     bool get needsTokenRefresh =>
                                       oauthCredentials.isExpired();
                                   }
```

Knock-on cleanups:

- `effectiveToken` collapses to a direct field access — rename to
  `bearerToken` to drop the misleading prefix while we're touching it.
- `needsTokenRefresh` loses its mode guard.
- `copyWith`'s `clearAuthToken` flag is removed; `clearOAuthCredentials`
  stays (signals "log out this context but keep it listed").
- `toJson` / `fromJson` drop the `authMode` key on write. `fromJson` keeps
  reading it for one release to support migration (see Decision 7).
- ~40 references in `app/test/unit/contexts/` and `app/test/unit/auth/`
  switch from `authToken: 'tok'` to `oauthCredentials: OAuthCredentials(...)`.

**login_screen.dart.** Drops the legacy-token toggle and the `_tokenCtrl`
path. The "email + password" branch — which already calls the OAuth2 PKCE
flow — becomes the only path.

**ConnectionScreen.** Keeps the URL field. The token field is replaced
with email + password. On submit, the screen runs the OAuth2 PKCE flow
against the chosen server's `/oauth/authorize` + `/oauth/token`, captures
the access + refresh tokens, and saves the context with `oauthCredentials`
populated. The `?_token=` query-param handling is deleted; `?_url=`
continues to pre-fill the URL field only and no longer auto-submits.

**Edit-context.** The inline token field is removed. The screen shows
name + URL (editable) plus a "Re-authenticate" button that launches the
OAuth2 PKCE flow against the context's stored URL; on success it writes
fresh `oauthCredentials`. No `authToken` to edit.

**The embedded-server path on macOS** does not need credentials at all
(server is local, started by the app, no auth). ConnectionScreen's
existing embedded-mode branch is unchanged.

**OIDC mode interaction.** `/oauth/authorize` already redirects to the
upstream IdP when the server is configured in OIDC mode. The Flutter
screens don't need to know — they kick off PKCE, the browser does the
IdP dance, and the callback delivers tokens.

### Decision 6: E2E rig skips the UI auth flow via localStorage seed

The Playwright suite under `crates/web-ui/e2e/` depends on the legacy token
in three places:

- `playwright.config.ts:82-83` — server boot uses `--auth-token test-token`
- 4 test specs use `page.goto('/setup?_token=test-token')` — relying on
  ConnectionScreen's `?_token=` auto-submit which is being removed
- The `apiGet` helper sends `Authorization: Bearer test-token` directly

The migration plan needs to account for the `?_token=` auto-submit being
deleted. Three options were considered:

- (a) Have tests log in via the new email+password form. Reliable but adds
  4 form interactions per spec and exercises auth UX in visual-regression
  tests that don't care about it.
- (b) Pre-seed `AssistantContext` directly into `localStorage` via
  `page.evaluate()` before navigation. Skips auth UI entirely.
- (c) Set `Authorization` via Playwright `extraHTTPHeaders`. Doesn't work —
  the Flutter dio client builds the header from the stored context, not
  from the page's headers.

Option (b) wins. The end-to-end migration:

1. Drop `--auth-token` from the webServer command.
2. Add a Playwright `globalSetup` that:
   - Polls `/health` until the server is up,
   - Reads the admin password from `~/.assistant/orgs/default/admin_credentials.txt`
     (written by the bootstrap on first start, mode 0600),
   - Runs the OAuth password→code→token dance against `/oauth/authorize`
     and `/oauth/token` to obtain a short-lived JWT,
   - `POST /api/users/me/api-keys` with that JWT to mint an `ask_live_…`
     key scoped to whatever the e2e suite needs,
   - Writes the key to `process.env.E2E_API_KEY`.
3. Add `tests/_helpers/seedContext.ts` exporting `seedContext(page)`. It
   uses `page.evaluate()` to write a JSON-encoded `AssistantContext` (with
   `authToken = process.env.E2E_API_KEY`) into `localStorage` under the
   `shared_preferences` keys the Flutter app reads. The exact key names
   come from `app/lib/features/contexts/data/context_repository.dart`.
4. Replace every `page.goto('/setup?_token=...')` with
   `await seedContext(page); await page.goto('/chat');`.
5. `apiGet` is unchanged — same Bearer shape, value comes from
   `process.env.E2E_API_KEY`.

After this, e2e exercises the real production auth path (API key through
`resolve_bearer` branch 2) instead of a bypass that doesn't exist for real
users. The `globalSetup` adds ~2 seconds to test start; visual-regression
tests already do `FLUTTER_SETTLE_MS = 3000` per spec so it's noise.

Rejected alternative: add an env-gated `ASSISTANT_TEST_SEED_API_KEY` flag
to the bootstrap that mints a deterministic key on first start. Faster (no
OAuth round-trip) but bakes a test-only code path into the production
binary; rejected on principle.

### Decision 7: Migrate existing legacy-token contexts on first launch

Local installs in the wild may have one or more `AssistantContext` rows
persisted in `localStorage` (web) or `flutter_secure_storage` (native)
with `authMode == "legacyToken"` and `authToken` populated. After the
model collapses, those rows have no usable credentials.

Three options were considered:

- (a) **Drop legacy contexts silently on launch.** Surprising — users
  lose servers they configured and don't know why.
- (b) **Keep rows; mark them as needing re-authentication.** Surface
  them in the contexts list with a "Sign in again" affordance. Users see
  what they had and re-auth on demand.
- (c) **Upgrade in place.** If `authToken` happens to decode as a JWT,
  mint a refresh token server-side and write to `oauthCredentials`. Too
  clever; doesn't work for the common case (a real static legacy token);
  not worth the code path.

Option (b) wins. The migration:

1. `AssistantContext.fromJson` keeps reading the `authMode` key for one
   release. When it sees `"legacyToken"`, it constructs a context with
   `oauthCredentials: null` and an additional in-memory flag
   `requiresReauth: true` (similar in spirit to `credentialsCorrupted`).
2. `ContextRepository.loadContexts` runs this conversion on every read.
   The persisted JSON is rewritten without `authMode` and without any
   secret slot the next time `saveContext` runs.
3. The contexts list / switcher screen shows rows with
   `requiresReauth == true` in a degraded state ("Sign in again" instead
   of "Connect").
4. Selecting such a row routes through the OAuth2 PKCE flow against the
   context's stored URL.
5. After a successful re-auth, the row gets fresh `oauthCredentials` and
   `requiresReauth` flips to false.

The persisted JSON maintains **forward** compatibility for new clients:
`fromJson` ignores unrecognised keys (`authMode`, etc.) gracefully.

## Auth resolution: before / after

```
BEFORE                              AFTER
─────────────────────────────       ─────────────────────────────

Authorization: Bearer <X>           Authorization: Bearer <X>
        │                                   │
        ▼                                   ▼
  ┌──────────┐                        ┌──────────┐
  │   JWT?   │─yes→ ctx               │   JWT?   │─yes→ ctx
  └──────────┘                        └──────────┘
        │                                   │
        ▼                                   ▼
  ┌──────────┐                        ┌──────────┐
  │ ask_live │─yes→ ctx               │ ask_live │─yes→ ctx
  └──────────┘                        └──────────┘
        │                                   │
        ▼                                   ▼
  ┌──────────┐                            401
  │  legacy  │─yes→ admin
  └──────────┘
        │
       401
```

## Login surface: before / after

```
BEFORE                              AFTER
─────────────────────────────       ─────────────────────────────

Server-rendered /login form         No server-rendered login form.
  POST /login token=<string>         Flutter SPA fallback serves
        │                            index.html for /login; SPA
        ▼                            renders its own login UI.
  resolve_bearer() →                       │
   JWT / API key / legacy                  ▼
        │                            Flutter login UI runs
        ▼                            OAuth2 PKCE against
  jwt_manager.sign + Set-Cookie       /oauth/authorize + /oauth/token
                                            │
                                            ▼
                                     OAuthCredentials persisted
                                     in flutter_secure_storage
```

The server keeps `/oauth/authorize` and `/oauth/token` unchanged. The
`/login` GET, POST, `LoginForm`, `login_html`, and the `SESSION_COOKIE`
machinery in `crates/web-ui/src/auth.rs` are deleted.

## Test surface

**Server tests to add (failing first, TDD):**

- Bearer-only requests with no token → 401 (regression: same as today minus
  branch 3).
- `resolve_bearer` no longer has a legacy branch — unit test asserts only
  JWT + API key paths exist.
- Request to `GET /login` → SPA fallback (200 with index.html), not the old
  server-rendered HTML form.

**Server tests to delete:**

- `crates/auth/src/middleware.rs:393` — `extract_legacy_token`.
- `crates/web-ui/src/auth.rs:608` — `bearer_legacy_token_produces_admin_context`.
- All `crates/web-ui/src/auth.rs` tests that exercise `LoginForm`,
  `login_submit`, `login_page`, or `logout` (the entire suite around the
  deleted handlers).
- Any test that constructs `AuthState` with `legacy_token: Some(...)`.

**Flutter tests to add:**

- Widget test: login screen renders only email + password fields (no token
  toggle, no token field). Submit triggers OAuth PKCE — verify via mocked
  `OAuthClient`.
- Widget test: ConnectionScreen in remote mode renders URL + email +
  password fields. Submit triggers OAuth PKCE against the entered URL.
- Widget test: `/setup?_url=https://x` pre-fills URL only; no auto-submit.
- Widget test: `/setup?_token=x` ignores the param entirely.
- Widget test: edit-context renders no token field; "Re-authenticate"
  button triggers OAuth PKCE.
- Unit test: `AssistantContext` has no `authToken` field; constructor takes
  required `oauthCredentials`.
- Unit test: `AssistantContext.fromJson` reading legacy JSON with
  `authMode == "legacyToken"` and `authToken` → context with
  `requiresReauth: true`, `oauthCredentials: null`.
- Unit test: `AssistantContext.toJson` does not write `authMode` or
  `authToken`.

**Flutter tests to delete/rewrite:**

- `app/test/widget/login/login_screen_test.dart` — delete the "legacy
  token mode" scenarios (lines 43, 85 and similar).
- `app/test/unit/contexts/context_model_test.dart` — replace every
  `makeCtx(authToken: 'tok')` with `makeCtx(oauthCredentials: ...)`.
  Delete the `effectiveToken returns authToken for legacyToken mode`
  scenario (line 187) and similar legacy-mode assertions. Estimated ~25
  references.
- `app/test/unit/contexts/context_repository_test.dart`,
  `context_repository_upsert_test.dart` — same sweep, ~15 references.
- `app/test/unit/auth/auth_provider_test.dart` — same sweep, ~5
  references.
- `app/test/widget/contexts/edit_context_test.dart` — drop the
  `authToken: 'my-token'` fixture path; assert that the screen has no
  token field.

## Migration sequencing

1. Land the change behind a feature-detection-friendly migration: on first
   startup after the upgrade, if no admin user exists in `org.db`, run the
   bootstrap and log the credentials file path prominently.
2. Release notes call out: "If you were using `ASSISTANT_WEB_TOKEN`, look up
   your admin password in `~/.assistant/orgs/{slug}/admin_credentials.txt`
   or run `assistant admin reset-password user@host`."
3. For schorschvm specifically: verify the admin credentials file before
   restarting the service (covered by the existing multi-org-cutover babysit
   procedure documented at `docs/operations/multi-org-cutover.md`).

## Risks

- **Operator lockout.** If a deployment never ran the migration (legacy
  single-user setup with `assistant.db` and `ASSISTANT_WEB_TOKEN` only), and
  also never wrote `admin_credentials_file`, the operator has no way in. The
  reset-password CLI is the safety net but they need SSH access. Acceptable
  risk because (a) bootstrap writes the file on the very first migration, (b)
  pre-existing migrations already happened in 2026-04, so most installs are
  through it, and (c) reset-password runs as a local OS user.
- **Forgotten doc surface.** Search before merge: `rg "ASSISTANT_WEB_TOKEN|--auth-token|legacy.token"`.
- **OpenAPI consumers.** Anyone reading `openapi.json` will see different
  bearer-auth wording. No breaking change to the protocol, just to the prose.

## Out of scope

- API keys — kept as-is. They're scoped programmatic credentials with audit
  identity, not "olde tokens".
- OIDC mode — kept. Username/password is still the user experience at the IdP.
- OAuth2 authorize / device flows — kept; already email+password.
- JWT format, issuer, signing key rotation — unchanged.
