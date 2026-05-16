## Tasks

TDD discipline: every implementation task is preceded by a failing test.

### Backend: `expires_in_days` on create

- [ ] Write failing test: `POST /api/users/me/api-keys` with
      `{"name": "t", "expires_in_days": 30}` → response has
      `expires_at` set to ~30 days from now (±1 minute tolerance).
- [ ] Write failing test: same endpoint with `expires_in_days: null` or
      field absent → `expires_at: null`.
- [ ] Write failing test: `expires_in_days: 0` → 400.
- [ ] Write failing test: `expires_in_days: 401` → 400 (ceiling of 400
      days, matching GitHub).
- [ ] Add `expires_in_days: Option<u32>` to `CreateApiKeyRequest`
      (`crates/web-ui/src/api/api_keys.rs:48`).
- [ ] Compute `expires_at = Utc::now() + Duration::days(n)` in the create
      handler, pass to the store record.
- [ ] Validate the field server-side (`0` and `>400` → 400 with a clear
      error body).
- [ ] Regenerate `openapi.json` via `make dump-openapi`.

### Generated Flutter client

- [ ] Run `make generate-flutter-client`.
- [ ] Verify the only diff in `app/packages/assistant_api/` is the new
      field on `CreateApiKeyRequest` (plus serialization). No unrelated
      churn.

### Flutter: create dialog rebuild

- [ ] Write failing widget test: opening the create dialog renders the
      name field, the scope picker, and the expiry chip row.
- [ ] Write failing widget test: submitting with no scopes selected
      builds a `CreateApiKeyRequest` with `scopes: []` (or omitted).
- [ ] Write failing widget test: selecting `personas:read` and
      `conversations:write` builds a request with those two scope strings.
- [ ] Write failing widget test: selecting the "90 days" chip sends
      `expires_in_days: 90`.
- [ ] Write failing widget test: selecting "No expiry" sends
      `expires_in_days: null` (or omitted).
- [ ] Write failing widget test: tapping "Read everything" preset selects
      all `*:read` scopes and nothing else.
- [ ] Build the scope picker widget (collapsible per-resource groups,
      responsive layout: 1 col on <=640dp, 2 cols on desktop).
- [ ] Build the expiry chip row: 30 / 60 / 90 / 1y / No expiry, default
      90, "No expiry" with subdued warning hint.
- [ ] Build the "Read everything" and "Read + write everything"
      quick-fill buttons.
- [ ] Wire the new fields through `api_keys_provider.dart`
      `createKey({String name, List<String> scopes, int? expiresInDays})`.
- [ ] Keep the existing "key created, copy to clipboard" success state
      unchanged.

### Flutter: list rendering

- [ ] Write failing widget test: tile renders relative `createdAt`
      ("3 days ago"); on hover (desktop) or long-press (mobile) shows
      absolute timestamp.
- [ ] Write failing widget test: tile with `expiresAt` shows "expires in
      N days" with subdued styling; tile with null `expiresAt` shows
      "No expiry".
- [ ] Write failing widget test: tile with <=3 scopes renders them as
      chips; tile with >3 scopes renders "N scopes" with a tap-to-expand.
- [ ] Implement a small `relativeDate(DateTime)` helper in
      `app/lib/shared/time/` (no new package dep).
- [ ] Update `_ApiKeyTile` to render the new subtitle layout.

### Flutter: settings entry

- [ ] Write failing widget test: from the settings landing screen,
      tapping "API keys" navigates to `/api-keys`.
- [ ] If a settings landing screen does not exist, create a minimal one
      under `app/lib/features/settings/` with a single list tile linked
      to `/api-keys`. Register the route.
- [ ] Add the settings entry to the nav shell overflow destinations
      (`_NavDest` in `app/lib/shared/nav_shell.dart`). The API-keys
      screen is _not_ a top-level nav entry — it's reachable through
      Settings.

### Docs

- [ ] `docs/authentication.md`: document `expires_in_days` in the create
      endpoint description; add a "Managing keys in the web UI"
      subsection that points at Settings → API keys.

### Final sweep

- [ ] `make lint && make format && make test`.
- [ ] `cd app && flutter analyze && flutter test`.
- [ ] Smoke test (web): log in, navigate Settings → API keys, create a
      key with 90-day expiry and `personas:read` scope. Verify it appears
      in the list with the right metadata, copy it to clipboard, revoke
      it.
- [ ] Smoke test (CLI): `assistant api-keys list` shows the same keys
      created via the UI; `assistant api-keys revoke <id>` reflects in
      the UI after refresh.
- [ ] Smoke test: create a key with `expires_in_days: 1` via the UI,
      manually fast-forward the system clock (or use a test record), and
      verify `resolve_key` rejects it with 401.
