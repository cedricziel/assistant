# ADR 0015: Sessions Are Revoked With a Per-User Token Version

**Status**: Accepted
**Date**: 2026-11-25
**Builds on**: [ADR-0007](adr-0007-multi-user-orgs.md)

## Context

The server issues JWT access tokens (HS256, one hour). Until now a token was
valid for its whole lifetime, whatever happened to its user:

- Logout cleared the cookie and did nothing else. `POST /oauth/revoke` returned
  200 without revoking anything.
- A password change revoked refresh tokens but not the access JWT that made
  the request.
- Deleting a user revoked nothing. A refresh token of a deleted user still
  minted a JWT, because token issuance fell back to a stub identity when the
  user row was missing.
- The token carried the user's org and their role in each Space. A member who
  was demoted or removed kept that access until the token expired.

## Decision

**Every access JWT is checked against `org.db` on every request. A per-user
counter, `users.token_version`, voids all of a user's tokens at once.**

- `users.token_version` (migration `006_user_token_version`) starts at 0. The
  JWT carries it as the `tv` claim, copied from the user row at signing.
- `AuthState::authenticate_jwt` (`crates/auth/src/middleware.rs`) validates the
  signature and expiry, then loads the user's `SessionState` and rejects the
  token when the user is gone or `tv` differs from `token_version`.
- The token no longer carries Space roles. `authenticate_jwt` sets `org_id`
  and `space_roles` from `org.db`, so `SpaceCtx` and the `Access` checks see
  the current membership. Removing or demoting a member takes effect on
  their next request, and no counter bump is needed for it.
- API keys get the same treatment (`AuthState::bind_api_key`). The key's owner
  must still exist, and each Space role is capped at the owner's current role
  and dropped when the owner left the Space.
- `revoke_sessions` (`crates/auth/src/session.rs`) bumps the counter and revokes
  every refresh token of the user. It runs on:
  - `POST /logout` (cookie sessions),
  - `POST /oauth/revoke`, for a refresh token or an access token (the CLI and
    the Flutter app call this on logout),
  - `POST /api/users/me/password`, including the request's own JWT.
- Deleting a user removes the row, which voids their JWTs and API keys. It
  also revokes their refresh tokens. A refresh token whose user is gone now
  fails with `invalid_grant`; the stub-identity fallback is removed.
- Storage failures while checking a token answer 503, not 401, so a transient
  `org.db` error does not look like an expired session to clients that sign out
  on 401.

A token signed before this change has no `tv` claim and fails to deserialize.
Everyone signs in again once. There is no compatibility path.

## Alternatives considered

- **`jti` denylist.** The `jti` claim already exists, but a denylist can only
  revoke tokens it knows about. Revoking "everything for user X" would need a
  record of every token issued, or a second mechanism. It also grows until
  entries expire, and it does nothing about stale roles.
- **`not_before` timestamp per user.** Needs no claim, but `iat` has
  one-second resolution. A token signed in the same second as the revocation
  either survives it or, with a rounded-up cutoff, blocks the next sign-in.
  The counter has neither problem.
- **Short-TTL cache of the lookup.** The check is a primary-key read of
  `users` and an indexed read of `space_memberships` on a local SQLite file.
  A cache would trade that cost for a window in which a removed member still
  has access, plus invalidation code on every membership change.
- **Keep role claims and only add revocation.** Role changes would then need
  to bump the counter, signing the member out. Reading roles live avoids that
  and removes the second source of truth.

## Consequences

- Each authenticated request costs two small `org.db` reads. The legacy static
  token is not checked against `org.db`; task E1 removes it.
- Logout signs the user out of every device and client, because the counter is
  per user, not per session. RFC 7009 permits this. A per-session design would
  need session records, and none exist yet.
- A password change also ends the session that made it. The client has to sign
  in again with the new password.
- The legacy `/login` token form cannot start a cookie session for identities
  without a user row, which includes the legacy static token. Task E1 removes
  that form.
- The schema has no deactivated flag, so deleting a user is the only way to
  disable one. A future flag should call `revoke_sessions`.
- Any new code path that signs a JWT must pass the user's current
  `token_version` to `JwtManager::sign`, and any new path that accepts one must
  go through `AuthState::authenticate_jwt`.
