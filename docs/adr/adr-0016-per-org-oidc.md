# ADR 0016: OIDC Is Configured Per Org and Found From the Email

**Status**: Accepted
**Date**: 2026-11-25
**Builds on**: [ADR-0007](adr-0007-multi-user-orgs.md), [ADR-0014](adr-0014-server-and-org-config.md)

## Context

OIDC sign-in was a server-wide switch. Three flags (`--oidc-issuer-url`,
`--oidc-client-id`, `--oidc-client-secret`, with `OIDC_*` env vars) set one IdP
for the whole process. That did not fit the org model:

- Orgs are separate tenants, but they shared one IdP, and one org could not
  stay on passwords while another used SSO.
- The OIDC callback looked users up across all orgs. An IdP account whose
  verified email matched a user in another org was linked to that user and
  signed in to the other org. A tenant's IdP could take over accounts in a
  tenant it did not own.
- Emails are unique per org, not per server. The password form searched every
  org for an email and took the first hit, and the sign-in page had no way to
  say which org the user meant.

## Decision

**Each org owns its IdP settings. The sign-in page finds the org from the
email, and every lookup after that stays inside the org.**

- **Settings live in `org.db`**, table `org_oidc_configs` (migration
  `org/009`), one row per org. An org admin edits them with
  `GET/PUT/DELETE /api/orgs/{id}/oidc`. They are not in `org.toml` and not in
  server flags. The settings change at runtime, by a caller who may not have
  shell access, and they include a secret. An API fits that. `org.toml` is
  read when the org's first space starts, and a flag needs a restart.
- **`auth_mode` guards the settings.** `PATCH /api/orgs/{id}` refuses `oidc`
  with 409 until settings exist. `DELETE` refuses with 409 while `auth_mode` is
  `oidc`. `POST /api/orgs` creates `password` orgs only. An org cannot reach a
  state where sign-in has no method, except by editing the database.
- **Email-first org discovery, slug as fallback.** `GET /oauth/authorize`
  without `org` asks for an email. `POST /oauth/authorize/identify` lists the
  accounts with that email across orgs. One match continues in that org: an
  IdP redirect with `login_hint`, or the password form with the org fixed. No
  match, or matches in several orgs, shows one "enter your organization" form
  that continues at `?org=<slug>`. `?org=<slug>` skips the email step, for
  deep links and bookmarks. The password POST and the device verification
  form require an org slug, and look the user up in that org only. The
  failed-login lockout is keyed by org and email.
- **Lookups are org-scoped.** The callback builds its user store with the org
  that started the login (`OrgOidcUserStore`). It matches by (issuer, subject)
  inside the org, then by email inside the org, then provisions inside the org.
  Linking by email requires `email_verified: true`.
- **The client secret is sealed.** The server encrypts it with `SecretBox`,
  using the key that protects integration credentials (`secrets.key` or
  `ASSISTANT_SECRETS_KEY`). The API never returns it, only `has_client_secret`.
  `PUT` keeps the stored secret when the field is omitted and clears it on
  `null`.
- **Providers are cached per org.** `OidcProviderRegistry` discovers an org's
  IdP on first use and reuses it until the row's `updated_at` changes. A failed
  discovery is not cached.
- **The flags are removed with no compatibility path.** The three flags and
  the `OIDC_*` variables are gone. An operator who used them configures each
  org through the API. This follows the project's rule of removing old paths
  instead of shimming them. A shim would also have had to pick which org the
  old global IdP belonged to, and there is no right answer.

## Accepted trade-off: the identify step reveals account existence

Asking for the email before choosing the sign-in method tells a caller two
things: whether an address has an account, and when exactly one does, whether
its org uses a password or an IdP. A password form that never reveals which
case applied would hide both, but it cannot send an SSO user to their IdP.

We accept this, with limits:

- The identify step shares the per-IP auth rate limit with the other
  credential endpoints (`--auth-rate-limit-per-minute`).
- An address with no account and an address with accounts in several orgs get
  the same form, so the response does not separate those two cases.
- The password POST counts failures for unknown emails too, so lockout does
  not reveal accounts.

A single match still discloses the org's slug and method. Operators who treat
that as sensitive have no switch to turn the identify step off. An org
that is known by its slug can still be reached with `?org=<slug>`.

## Alternatives considered

- **Keep the global flags and add an org field.** One IdP could not serve
  several orgs, and the secret would stay in process arguments and the
  environment.
- **Put the settings in `org.toml`.** ADR-0014 puts org behavior there, but
  org admins have no API for the file, and it holds secrets in plaintext.
- **Slug first, no email step.** Users must know and type an org slug before
  anything else, and the page cannot send them to their IdP without it.
- **Match the IdP login to an org by email domain.** Domains are not unique
  per org and are not proof of ownership.
- **Per-org callback URLs.** Each org would register a different redirect URI
  at its IdP. One fixed `/oauth/callback` with a server-side pending session
  keeps IdP setup the same for every org.

## Consequences

- A server with several orgs can run different IdPs side by side. Each
  registers `{server base URL}/oauth/callback`.
- Losing `secrets.key` makes every stored client secret unreadable. `assistant
backup` includes the key. After a restore without it, re-`PUT` each secret.
- The IdP's JWKS are fetched at discovery. Key rotation at the IdP needs a
  settings update or a restart to take effect.
- A user in an OIDC org has no local password, so the device flow, which asks
  for one, cannot sign them in. They use API keys for the CLI.
- Upgrading from a server that used the flags needs one `PUT` and one `PATCH`
  per org. Until then, orgs sign in with passwords.
- Any new code that finds a user from an IdP identity must take an org ID.
  The store methods for cross-org lookup exist for the identify step only.
