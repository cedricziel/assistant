## Purpose

Stores and uses the secrets that integrations need: static headers and OAuth
tokens, including tokens obtained through MCP OAuth with Dynamic Client
Registration. Secrets are kept encrypted and are never returned to clients.

## ADDED Requirements

### Requirement: Credentials are encrypted at rest and never returned

Integration credentials (header values, OAuth access and refresh tokens, and
dynamically registered client secrets) SHALL be stored encrypted with an
authenticated cipher, under a server key. The ciphertext SHALL be bound to the
integration and the subject it belongs to, so a ciphertext copied to another row
fails to decrypt. The server key comes from `ASSISTANT_SECRETS_KEY` or, if that
is unset, from `~/.assistant/secrets.key`. The key file is created with mode
`0600` on first start. No API response, log line or trace attribute may
contain a credential value. Responses only report whether a credential is
present, and when it was last updated.

#### Scenario: Header credential round trip

- **WHEN** a user saves header value `ghp_abc` for their GitHub integration
- **THEN** the stored row does not contain `ghp_abc` in plain text, and the API
  returns `{"has_credential": true}` with no value

#### Scenario: Wrong key

- **GIVEN** credentials written under key K1
- **WHEN** the server starts with key K2
- **THEN** affected integrations report status `auth_expired`, the server keeps
  running, and an error names the key mismatch without printing either key

### Requirement: Header credentials fill the catalog template

For an integration whose auth kind is `header` or `mixed`, a caller allowed to
set its credential SHALL be able to submit a value for the template placeholder.
The server SHALL then send the header formed by the template, such as
`Authorization: Bearer <value>`, on every MCP request. Custom-URL integrations
SHALL accept an explicit header name and value instead of a template.

#### Scenario: Bearer template

- **GIVEN** a catalog entry with template `Authorization: Bearer {pat}`
- **WHEN** the user stores `ghp_abc` and the integration connects
- **THEN** each MCP request carries `Authorization: Bearer ghp_abc`

### Requirement: OAuth connection with discovery and dynamic registration

For an integration whose auth kind is `oauth` or `mixed`, or whose server
answers an unauthenticated request with `401` and a `WWW-Authenticate` header
pointing at protected-resource metadata, the server SHALL offer an OAuth
connection that follows the MCP authorization specification:

1. Discover the protected-resource metadata (RFC 9728) and the authorization
   server metadata (RFC 8414 or OpenID Connect discovery).
2. Obtain a client: reuse one registered earlier for the same authorization
   server, use a client id entered by an admin, or register one with Dynamic
   Client Registration (RFC 7591) when the server advertises a
   `registration_endpoint`.
3. Run the authorization-code flow with PKCE (`S256`), a single-use `state`
   that expires after 10 minutes, and the `resource` parameter (RFC 8707) set to
   the MCP server URL.
4. Exchange the code at the token endpoint, and store the access token, the
   refresh token and the expiry against the integration and the subject.

The redirect URI SHALL be `{server.public_url}/api/integrations/oauth/callback`.
When `server.public_url` is unset, OAuth connection MUST fail with a clear
error, and header auth remains available.

#### Scenario: Connect with DCR

- **GIVEN** an MCP server whose authorization server advertises a
  `registration_endpoint` and no client is registered yet
- **WHEN** a user calls `POST /api/integrations/{id}/oauth/start`
- **THEN** the server registers a client, and the response contains an
  `authorization_url` with `code_challenge_method=S256`, a `state` and a
  `resource` parameter

#### Scenario: Callback completes connection

- **WHEN** the provider redirects to the callback with a valid `code` and `state`
- **THEN** tokens are stored for that integration and subject, the state is
  consumed, the integration's status becomes `ready`, and the browser is sent to
  the app's integrations screen

#### Scenario: Replayed or expired state

- **WHEN** the callback receives a `state` that was already used, or is older
  than 10 minutes
- **THEN** no tokens are stored and the user sees an error page

#### Scenario: No DCR and no client id

- **GIVEN** an authorization server without a `registration_endpoint`
- **WHEN** OAuth start is requested and no admin has entered a client id
- **THEN** the response is `422` with a message asking an admin to enter a
  client id

### Requirement: OAuth subject follows credential mode

The subject of an OAuth connection SHALL be the integration itself for `shared`
mode, and the calling user for `per_user` mode. Only callers allowed to manage
the integration's layer may start a `shared` connection. Any member who can see
a `per_user` integration may connect their own account to it. Disconnecting
SHALL delete the subject's tokens and attempt token revocation (RFC 7009) when
the authorization server supports it.

#### Scenario: Member connects to a shared integration

- **GIVEN** a space-layer integration with mode `shared`
- **WHEN** a `member` of that space calls `oauth/start`
- **THEN** the response is `403`

#### Scenario: Member connects to a per-user integration

- **GIVEN** an org-layer integration with mode `per_user`
- **WHEN** a member calls `oauth/start`
- **THEN** the flow starts, and the resulting tokens are usable only in that
  member's turns

### Requirement: Tokens are refreshed transparently

Before using an access token that is expired or expires within 60 seconds, the
server SHALL refresh it with the stored refresh token, and persist any rotated
refresh token. Concurrent turns MUST NOT issue concurrent refreshes for the same
subject. If refresh fails with `invalid_grant`, the tokens SHALL be marked
invalid, the integration's status for that subject becomes `auth_expired`, and
its tools are omitted from turns until the subject reconnects.

#### Scenario: Expired token during a turn

- **GIVEN** an access token that expired one minute ago and a valid refresh token
- **WHEN** a turn uses the integration
- **THEN** the token is refreshed once, the turn's tool calls succeed, and the
  new tokens are stored

### Requirement: Outbound MCP URLs are restricted

Integrations SHALL connect only to `https` URLs. Custom URLs, and every URL
discovered during OAuth (metadata, registration, authorization and token
endpoints), MUST NOT resolve to loopback, link-local, or private address ranges.
The only exception is a host listed in
`[integrations] allow_private_hosts`.

#### Scenario: Custom URL to an internal address

- **GIVEN** `allow_custom_urls = true` and no `allow_private_hosts`
- **WHEN** an org admin installs a custom integration at `https://10.0.0.5/mcp`
- **THEN** the response is `422`
