## Context

The API key surface is mostly built. Inventory:

| Layer    | Status                                                                                                                                         |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Storage  | `ApiKeyRecord` has `scopes: Vec<Scope>` and `expires_at: Option<DateTime>`                                                                     |
| Resolver | `resolve_key` rejects expired keys (`crates/auth/src/api_keys.rs:102-105`) and applies scopes when building `AuthContext`                      |
| REST     | `POST/GET/DELETE /api/users/me/api-keys`. `CreateApiKeyRequest` has `name` and `Option<Vec<String>>` scopes — but no expiry field              |
| OpenAPI  | Documented; security scopes `api_keys:read` and `api_keys:write` defined                                                                       |
| CLI      | `assistant api-keys list / create --scopes / revoke`                                                                                           |
| Flutter  | `ApiKeysScreen` (list, create-name-only, revoke); `api_keys_provider.dart` (AsyncNotifier); route `/api-keys` registered but unlinked from nav |

The only backend code change in this proposal is adding `expires_in_days`
to `CreateApiKeyRequest`. Everything else is UI work and one settings link.

## Decisions

### Decision 1: Expiry shape — `expires_in_days` not `expires_at`

The API accepts `expires_in_days: Option<u32>` rather than an absolute
`expires_at: Option<DateTime<Utc>>`.

Reasons:

- Matches how users think ("expires in 90 days"), not how the database
  stores it.
- Avoids client/server clock-skew bugs at boundary values.
- Server computes `expires_at = Utc::now() + Duration::days(n)` at create
  time; the absolute timestamp is what's stored and returned in `ApiKeySummary`.
- Trivially encodes "never" as `None` without overloading sentinel dates.

Open hand: this rules out "expires at a specific calendar date" (e.g.,
"end of fiscal year"). Acceptable — GitHub doesn't offer that either at
PAT-create time, and a follow-up can add an absolute variant if real
demand emerges.

### Decision 2: Preset chips for expiry, not a date input

The Flutter create dialog offers `30 days / 60 days / 90 days / 1 year /
No expiry` as chip-style choices. No free-form numeric input, no calendar
picker.

Reasons:

- 95% of PAT users pick a preset (GitHub, Slack, GitLab all bear this
  out).
- Free-form numeric input invites typos that go undetected ("3" instead
  of "30").
- Preset chips compress to a single row on mobile breakpoints.
- "No expiry" gets a subdued warning hint so it's not the silent default.

The default selection on the screen is `90 days`.

### Decision 3: Scope picker — multi-select chips against the canonical set

The scope picker enumerates the `ResourceKind × Action` combinations
documented in `docs/authentication.md`:

| Resource        | Actions allowed                      |
| --------------- | ------------------------------------ |
| `personas`      | `read`, `write`, `delete`            |
| `conversations` | `read`, `write`, `delete`            |
| `messages`      | `read`, `write`                      |
| `skills`        | `read`, `write`, `delete`, `execute` |
| `interfaces`    | `read`, `write`, `manage`            |
| `bindings`      | `read`, `write`                      |
| `users`         | `read`, `write`, `manage`            |
| `org`           | `read`, `manage`                     |
| `api_keys`      | `read`, `write`                      |
| `spaces`        | `read`, `write`, `manage`            |

Rendered as collapsible per-resource groups (10 groups, ~30 scopes total).
On viewport <= 640 dp the groups render as accordion sections; on
desktop they render as a 2-column grid.

The picker submits `scopes: Vec<String>` in `"resource:action"` form,
which is what the backend already expects.

Default selection: empty (key has no scopes, falls back to user's space
roles). This matches today's behaviour. Add a "Read everything" preset
and a "Read + write everything" preset as quick-fills.

### Decision 4: Nav discoverability — Settings entry, not top-level

The screen does not deserve a top-level nav slot. Most users will create
a key once and never visit the screen again. Surface it via
Settings → API keys (or wherever the existing Settings landing routes).

If no Settings landing screen exists, add a minimal one with a single
list tile for now. Future settings entries (themes, notifications,
personas-mgmt) can land beside it.

### Decision 5: Date formatting — relative, with absolute on hover/tap

`createdAt` and `expiresAt` render as relative strings ("3 days ago",
"in 87 days"). On hover (desktop) or tap (mobile) the absolute timestamp
is shown via tooltip / bottom sheet.

Use `package:intl` (already a transitive dep via `flutter_localizations`)
for absolute formatting. For relative, prefer a small in-app helper —
`timeago` is a small package but adding a dep for ~10 lines of formatting
isn't justified.

### Decision 6: Keep the terminology "API key"

Don't rename to "Personal access token" anywhere in code, models, OpenAPI,
or CLI. The codebase consistently uses "API key" across all layers and
the generated Flutter client uses `ApiKeySummary` etc. Renaming would
touch ~50 identifiers and the generated client without changing behaviour.

User-facing description copy can include "personal access token" as
clarification — e.g., "API keys are personal access tokens for
programmatic access to your account" — but the noun stays "API key".

## Test surface

**Backend:**

- `POST /api/users/me/api-keys` with `expires_in_days: 30` → response has
  `expires_at` ~30 days in the future (±1 minute).
- `POST /api/users/me/api-keys` with `expires_in_days: None` →
  `expires_at: null`.
- `POST /api/users/me/api-keys` with `expires_in_days: 0` → 400 (don't
  silently mint pre-expired keys).
- `POST /api/users/me/api-keys` with `expires_in_days` larger than some
  ceiling (e.g., 366) → 400. (Optional — GitHub allows 400 days; we can
  match.)
- Regression: existing tests that don't set `expires_in_days` continue to
  pass with `expires_at: null`.

**Flutter:**

- Widget: create dialog renders the scope groups, the expiry chips, and
  the name field; submitting builds the right `CreateApiKeyRequest`.
- Widget: list tile renders relative dates and scope chips.
- Widget: tapping "API keys" in Settings navigates to `/api-keys`.

## Migration

No data migration. New field is optional on the request; existing keys
stay as they are. Generated Dart client picks up the new field via
`make generate-flutter-client`.

## Risks

- **Picker complexity.** The scope picker with 10 resources × ~3 actions
  each could overwhelm users who just want "a key that works". Mitigated
  by the two presets ("Read everything" / "Read + write everything") and
  by leaving "no scopes" as the default (falls back to space role).
- **Backend mismatch on key submit.** If a user picks a scope that isn't
  legal for their space role, the backend currently accepts it but the
  key won't work in practice. This is pre-existing — out of scope here.
  File a follow-up to validate scope subset at create time.
- **Generated-client drift.** `expires_in_days` is a new field; any
  Flutter callers of `createApiKey` outside `api_keys_provider.dart` need
  to be checked. There shouldn't be any — `api_keys_provider.dart` is
  the only call site as of writing.

## Out of scope

- `last_used_at` tracking. Wants a separate change because the write
  amplification is non-trivial (every authenticated request touches the
  row); we'd need a batched update strategy.
- Copy-prefix-to-clipboard button on each row.
- Expiry warning banners ("expires in 3 days").
- Email notification when a key is about to expire.
- A "regenerate" affordance (revoke + create new with same name and
  scopes) — common in PAT UIs.
- Bulk revoke / select-all.
