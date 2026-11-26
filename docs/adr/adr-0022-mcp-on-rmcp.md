# ADR 0022: MCP Runs on rmcp, With Audience-Bound Tokens

**Status**: Accepted
**Date**: 2026-11-26
**Builds on**: [ADR-0007](adr-0007-multi-user-orgs.md), [ADR-0013](adr-0013-per-space-runtimes.md)

## Context

The assistant speaks the Model Context Protocol (MCP) in two directions. As a
server, `assistant mcp` exposes its tools and skills to MCP clients. As a
client, it connects to the external servers listed under `[mcp]` in
`server.toml` and to integration servers, and offers their tools to the
Persona.

Before this change the two sides had little in common:

- `crates/mcp-server` parsed JSON-RPC lines itself. It ran every tool call
  with an empty `ExecutionContext` and every `run-prompt` turn as the system
  identity, whoever the caller was. `resources/read` joined the last URI
  segment onto the skill directory without checking the result.
- The server spoke stdio only. There was no way to reach it over the network.
- `crates/mcp-client` used `rmcp` with no client handler, so it dropped every
  server notification. It kept only text and image content from tool results
  and ignored `_meta`.
- Access tokens carried one fixed audience. Any token the server issued was
  valid on every route.

`rmcp` 3.11 implements protocol revisions `2024-11-05` through `2026-07-28`,
`server/discover`, the Streamable HTTP server as a tower service, the Tasks
extension and the client side of the Skills extension. It does not implement
MCP Apps.

The full proposal is `openspec/changes/mcp-full-spec/`.

## Decision

**The assistant writes no MCP protocol code. Framing, dispatch, version
negotiation, transports, tasks and skills come from `rmcp`. The assistant's
code is the handlers, the authentication glue, and the MCP Apps extension.**

- **One server handler.** `AssistantMcpServer` implements rmcp's
  `ServerHandler` over the existing facades: `SkillCatalog`, `ToolDispatcher`,
  `OrchestrationEngine` and the permission policy. `protocol.rs` and the line
  loop are gone. Stdio and HTTP serve the same handler.
- **Two protocol revisions.** The server advertises `2026-07-28` and
  `2025-11-25`, and rejects older ones. The list is pinned in the server
  crate, so an rmcp upgrade cannot advertise older revisions without a code
  change. The client tries `2026-07-28` through `server/discover` first and
  falls back to `initialize` on `2025-11-25`.
- **Caller identity from request extensions.** Every tool call builds its
  `ExecutionContext` from the caller's `AuthContext`. Over stdio, a transport
  wrapper stamps every request with `AuthContext::system()`. Over HTTP, the
  `/mcp` middleware resolves the credential and puts the `AuthContext` into
  the HTTP request extensions. rmcp copies those into the handler's request
  context. A request without an identity is rejected. It is never treated as
  the system caller.
- **Per-request binding over HTTP.** One `StreamableHttpService` serves
  `/mcp` for every space. The middleware resolves the caller's space and puts
  an `AssistantMcpServer` built from that space's runtime into the request
  extensions. `RequestBoundServer` is a stateless `ServerHandler` that hands
  each request to that bound server. Sessions stay on for `2025-11-25`
  clients; `2026-07-28` requests are stateless.
- **Audience-bound tokens.** `/oauth/authorize` and `/oauth/token` accept the
  RFC 8707 `resource` parameter. Its only accepted value is
  `<issuer>/mcp`. A token issued for it has that URL as `aud` and the single
  scope `mcp:execute`. `/mcp` accepts only that audience, and `/api` rejects
  it. API keys reach `/mcp` when they carry `mcp:execute`.
  `/.well-known/oauth-protected-resource/mcp` publishes the RFC 9728 metadata,
  and a 401 from `/mcp` points to it.
- **Remote skills are verified and cached.** When a connected server
  advertises the Skills extension, the client lists its skills, downloads each
  file the manifest lists with `resources/read`, and checks its size and
  sha256 digest. Verified files go to
  `~/.assistant/cache/mcp-skills/{server}/{skill}/` with a copy of the
  manifest. A changed manifest forces a new download. The skill registry holds
  them in memory as `SkillSource::Mcp { server }`, named `{server}__{skill}`.
  A built-in ask rule makes any `bash` or `process` command that names
  `/mcp-skills/` wait for a human.
- **MCP Apps render in an opaque-origin sandbox (planned).** This part is not
  implemented. The client already declares the MCP Apps extension and hides
  tools whose visibility excludes the model, but no app is resolved or
  rendered. The plan: on web, an `<iframe sandbox="allow-scripts" srcdoc=…>`
  without `allow-same-origin`, so the frame has an opaque origin and cannot
  reach the assistant's cookies, storage or API. On iOS and macOS,
  `webview_flutter` loads the same document.

## Alternatives considered

- **rmcp's `#[tool_router]` macros, one function per tool.** The tool set is
  dynamic: it is the executor's specs plus `run-prompt` and `install-skill`.
  The handler implements `list_tools` and `call_tool` directly instead.
- **Accept any valid JWT on `/mcp`.** The MCP authorization spec requires
  audience validation. Separate audiences also stop a token taken from one
  surface from being replayed on the other.
- **The spec's separate-origin sandbox proxy for MCP Apps.** It needs a second
  served origin, and a self-hosted single-host install does not have one. An
  opaque-origin `srcdoc` frame gives the same isolation from the host's
  credentials. Apps that need `allow-same-origin` will not work.

## Consequences

- MCP behavior follows rmcp releases. `rmcp = "3.11"` is declared once in
  `[workspace.dependencies]` and bumped on purpose.
- `assistant mcp` keeps its name and stdio behavior. Desktop clients that
  launch it need no configuration change, other than speaking `2025-11-25` or
  newer.
- HTTP access is new and opt-in. A client registers through
  `/oauth/register` (RFC 7591) and asks for `resource=<issuer>/mcp`.
- `legacy_session_mode` keeps per-session state for `2025-11-25` clients. It
  can go once clients move to `2026-07-28`.
- Over HTTP, `install-skill` is listed but fails: the HTTP handler has no
  skill registry to install into.
- A `2025-11-25` server that never answers `server/discover` delays each
  connection by rmcp's 10-second discovery timeout before the fallback.
- Remote skills run third-party code. They stay in memory only, are withdrawn
  when their server disconnects, and their scripts always ask first.
- Server-side Tasks for `run-prompt`, prompts and completions, and MCP Apps
  rendering are later steps of the same OpenSpec change. Server tasks will
  live in memory only and will not survive a restart.
