# Set up OIDC sign-in for an org

This page is for an org admin. It moves one org from password sign-in to an
external identity provider (IdP) such as Keycloak, Auth0, or Google Workspace.
Other orgs on the server are not affected. For the full behavior, see
[Per-org OIDC](../authentication.md#per-org-oidc).

You need:

- A bearer token or API key for an admin of the org.
- Permission to register an OAuth client at the IdP.
- The org's ID and slug.

## 1. Register the client at the IdP

Create an OpenID Connect client with the authorization code flow. Set its
redirect URI to:

```text
{server base URL}/oauth/callback
```

The server requests the scopes `openid email profile`. Note the client ID,
the client secret if the client is confidential, and the issuer URL. The
issuer URL must match the `issuer` field of the IdP's
`/.well-known/openid-configuration` exactly, including any trailing slash.

## 2. Save the settings

```sh
curl -X PUT "$SERVER/api/orgs/$ORG_ID/oidc" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "issuer_url": "https://idp.example.com/realms/main",
    "client_id": "assistant",
    "client_secret": "s3cret",
    "auto_provision": true,
    "allowed_email_domains": ["example.com"]
  }'
```

The server accepts `https` issuers, and `http` only for `localhost` and
loopback addresses. The response shows `has_client_secret: true` and never
the secret. To change other fields later, send the `PUT` again without
`client_secret` and the stored secret stays. Send `"client_secret": null` to
clear it.

Set `auto_provision` to `false` to let in only users who already exist. An
empty `allowed_email_domains` allows any domain.

## 3. Switch the org to OIDC

```sh
curl -X PATCH "$SERVER/api/orgs/$ORG_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"auth_mode": "oidc"}'
```

This returns 409 if step 2 did not happen. Switching back to `password`
works at any time, and only then can you `DELETE` the OIDC settings.

Test it before you sign out. Open the web app in a private window, enter an
email from the org, and confirm the IdP login page appears. Keep your admin
token valid until a sign-in works.

The device flow behind `assistant login` takes a local password, which
accounts in an OIDC org do not have. Use an API key for the CLI and scripts.

## What users see

A user opens the app and enters their email. If one account has that email,
the server sends them to the org's IdP with the email as a login hint. After
they sign in there, they land back in the app.

An address with no account, or accounts in several orgs, gets a form asking
for the organization name. Entering the slug continues the sign-in for that
org.

The server matches the IdP user to an account in this org, by IdP identity
first and then by email. Linking an existing account by email needs the IdP
to send `email_verified: true`. With `auto_provision` on, a user with no
match gets a new account.

## Troubleshooting

| Symptom at sign-in                                           | Cause                                                                                                                                                                 |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 502 "the identity provider is unavailable"                   | Discovery failed. The next attempt retries. Check `issuer_url` against the IdP.                                                                                       |
| 500 "single sign-on is not configured for this organization" | The org is in `oidc` mode but has no settings. `PUT` them.                                                                                                            |
| Login fails after the IdP redirect                           | Email domain not in `allowed_email_domains`, unverified email matching an existing account, or `auto_provision` is off for a new user. The server log has the reason. |
| Logins fail after the IdP rotated its signing keys           | The server fetched the keys at discovery. `PUT` the settings again or restart.                                                                                        |

## Migrating from the server-wide flags

The `--oidc-issuer-url`, `--oidc-client-id`, and `--oidc-client-secret` flags
and the `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`, and `OIDC_CLIENT_SECRET`
environment variables no longer exist. The server rejects the flags at
startup, and the environment variables have no effect.

To keep the same IdP:

1. Remove the flags and variables from your service definition.
2. For each org that should use the IdP, do steps 2 and 3 above with the
   values you used before. Use the same issuer URL so existing users keep
   matching by IdP identity.
3. The redirect URI is the same as before, `{server base URL}/oauth/callback`,
   so the IdP client needs no change.

Until an org is switched, it uses password sign-in.
