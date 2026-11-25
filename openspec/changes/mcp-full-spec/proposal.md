# Proposal

## Why

Our MCP server is a hand-rolled stdio JSON-RPC loop pinned to `2024-11-05`, without prompts, subscriptions, HTTP or authorization, and its `resources/read` joins an unchecked filename. The client uses only tools and drops content other than text and images. MCP is now at `2026-07-28`, and the Tasks, Apps and Skills extensions offer features we cannot use.

## What Changes

- **BREAKING** Replace the hand-rolled MCP server with an `rmcp` server handler. No JSON-RPC parsing, framing, version negotiation or transport code of our own remains; extension wire formats `rmcp` lacks (MCP Apps `_meta.ui`) are the only protocol code we write.
- Serve protocol `2026-07-28`, with `2025-11-25` (initialize-era) clients still accepted, over stdio (`assistant mcp`) and Streamable HTTP at `/mcp` in the web server.
- Protect `/mcp` as an OAuth resource server: protected resource metadata, `WWW-Authenticate` challenges, tokens bound to the MCP resource by audience, a new `mcp:execute` scope, and API keys carrying that scope.
- Run every server tool call as the authenticated caller, under the same permission policy as other interfaces.
- Server Tasks extension: `run-prompt` returns a task, polled with `tasks/get` and cancelled with `tasks/cancel`.
- Client: upgrade to `rmcp` 3.11 at the latest protocol, keep tool lists fresh through `subscriptions/listen`, convert all content types (audio, resource links, embedded resources, structured content), and consume Tasks when external servers return them.
- Client Skills extension: discover skills that external servers publish and offer them to Personas after verifying each manifest's digests.
- MCP Apps host: render `ui://` resources returned by external servers' tools inline in the chat, sandboxed, with the postMessage bridge.
- **Fix** path traversal in `resources/read`.

## Capabilities

### New Capabilities

- `mcp-server`: protocol revisions, features and stdio serving of the assistant as an MCP server.
- `mcp-http-endpoint`: Streamable HTTP `/mcp` endpoint and its OAuth resource-server protection.
- `mcp-tasks`: Tasks extension as server and as client.
- `mcp-client`: protocol behaviour toward external MCP servers, including content conversion and list freshness.
- `mcp-client-skills`: Skills extension consumption from external MCP servers.
- `mcp-apps-host`: rendering MCP Apps returned by external tools.

### Modified Capabilities

- `tool-call-display`: a tool call whose tool declares an MCP App renders the app below its chip.

## Non-goals

- Authoring our own MCP Apps or Skills-extension serving.
- Advertising sampling, roots or elicitation to external servers (sampling and roots are deprecated; elicitation UI is a follow-up).
- Exposing external servers' prompts to users.
- Legacy HTTP+SSE transport and protocol versions older than `2025-11-25`.

## Impact

- Crates: `mcp-server` (rewritten), `mcp-client`, `web-ui` (new route and auth layer), `auth` (audience, scope), `core` (scope kind), `interface-cli`.
- Dependencies: `rmcp` 3.11 with `server`, `server-side-http`, `macros`; Flutter gains a webview dependency for native targets.
- API: new `/mcp`, `/.well-known/oauth-protected-resource/mcp`; `openapi.json` and the Dart client regenerate for the `resource` parameter.
- **Visual change**: tool-call bubbles may contain an embedded app frame; Playwright baselines move.
- **Documentation**: required — a new `docs/mcp.md` (connecting clients, auth, apps, skills) and updates to `docs/tool-permissions.md` and `docs/authentication.md`. New ADR for the rmcp migration.
