# Spec Delta

## Purpose
Define the Streamable HTTP MCP endpoint served by the web server and how it is protected as an OAuth 2.0 resource server.

## ADDED Requirements

### Requirement: Streamable HTTP endpoint
The web server SHALL serve the MCP server at `/mcp` using the Streamable HTTP transport of each supported revision. It SHALL reject requests whose `Origin` header is present and does not match the server's own host with 403, and SHALL return 405 for methods the revision does not define.

#### Scenario: Tool call over HTTP
- **WHEN** an authorized client POSTs a `tools/call` request to `/mcp`
- **THEN** the server responds with the result as JSON or a request-scoped event stream

#### Scenario: Foreign origin
- **WHEN** a browser page on another origin POSTs to `/mcp`
- **THEN** the server responds 403 without dispatching the request

### Requirement: Protected resource metadata
The server SHALL publish OAuth protected resource metadata at `/.well-known/oauth-protected-resource/mcp` naming the canonical `/mcp` URL as the resource, the assistant's own authorization server, and the supported scopes.

#### Scenario: Client discovers the authorization server
- **WHEN** a client fetches the protected resource metadata
- **THEN** `authorization_servers` contains the assistant's issuer and `scopes_supported` contains `mcp:execute`

### Requirement: Authentication challenges
A request to `/mcp` without a valid credential SHALL get 401 with a `WWW-Authenticate: Bearer` header carrying `resource_metadata` and the required scope. A valid credential lacking `mcp:execute` SHALL get 403 with `error="insufficient_scope"`.

#### Scenario: Missing token
- **WHEN** a client POSTs to `/mcp` without `Authorization`
- **THEN** the response is 401 and its challenge points at the protected resource metadata

#### Scenario: Token without MCP scope
- **WHEN** a client presents a token that has only `conversations:read`
- **THEN** the response is 403 with `error="insufficient_scope"` and `scope="mcp:execute"`

### Requirement: Audience-bound tokens
The authorization server SHALL accept the `resource` parameter on authorization and token requests and SHALL issue tokens whose audience is that resource. `/mcp` SHALL accept only tokens whose audience is its canonical URL, and API keys carrying `mcp:execute`. Tokens issued for any other audience SHALL be rejected with 401.

#### Scenario: Token issued for the web API
- **WHEN** a client presents a token issued without `resource=<origin>/mcp`
- **THEN** `/mcp` responds 401

#### Scenario: API key with MCP scope
- **WHEN** a client presents an `ask_live_` key that has `mcp:execute`
- **THEN** the request is served as the key's user

### Requirement: Dynamic client registration for MCP clients
The authorization server SHALL let MCP clients register through dynamic client registration, advertised in its metadata, accepting loopback and custom-scheme redirect URIs and the `mcp:execute` scope.

#### Scenario: Desktop MCP client registers
- **WHEN** a client registers with redirect URI `http://127.0.0.1:33418/callback` and scope `mcp:execute`
- **THEN** registration succeeds and the client can complete an authorization-code flow with PKCE for the `/mcp` resource

### Requirement: Caller identity
Every HTTP request SHALL execute as the authenticated user in the org and space the credential resolves to, and tool calls SHALL be evaluated against that user's permissions.

#### Scenario: Space-restricted user
- **WHEN** a user without access to space B calls a tool through `/mcp`
- **THEN** the tool cannot read or change data in space B
