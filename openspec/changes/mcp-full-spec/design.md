# Design

## Context

- `crates/mcp-server` parses JSON-RPC lines itself (`server.rs` `handle_request`). Tool calls run with an empty `ExecutionContext`; `run-prompt` runs as `AuthContext::system()`. `resources/read` joins the URI's last segment onto the skill directory unchecked.
- `crates/mcp-client` holds `RunningService<RoleClient, ()>`: no client handler, so notifications are dropped. `bridge.rs` keeps only text and image blocks and ignores `_meta`. `Cargo.toml` says `3.1.4` but the lock already resolves `rmcp` 3.11.0.
- `rmcp` 3.11 implements revisions `2024-11-05` through `2026-07-28`, `server/discover`, MRTR, `subscriptions/listen`, Streamable HTTP server as a tower service, `TaskManager` (Tasks extension), and client `list_skills`/`get_skill` (Skills extension). It has no MCP Apps support.
- The web server already runs the OAuth authorization server (`/oauth/*`, RFC 8414 metadata, `/oauth/register`). JWTs carry one fixed `aud`; scopes are `resource:action` parsed in `crates/auth/src/jwt.rs`.
- The Flutter app targets web, iOS and macOS and has no webview or iframe rendering today.

## Goals / Non-Goals

**Goals:**

- Zero hand-written MCP protocol code: framing, dispatch, version negotiation, transports, tasks and skills come from `rmcp`. Our code is handlers, auth glue, and the MCP Apps extension.
- One server handler shared by stdio and HTTP.

**Non-Goals:**

- Persisting server tasks across restarts.
- A separate-origin sandbox proxy for web app rendering (see Decision 7).

## Decisions

### 1. One `rmcp` `ServerHandler` over the existing trait facades

`AssistantMcpServer` implements `ServerHandler` and holds `Arc<dyn SkillCatalog>`, `Arc<dyn ToolDispatcher>`, `Arc<dyn OrchestrationEngine>` and the permission policy. Stdio serves it with `rmcp::transport::stdio`; HTTP serves it with `StreamableHttpService`. The existing `tests/dispatch.rs` stubs keep working against the new handler through an in-process duplex transport. `protocol.rs`, `run_io` and the line loop are deleted.
_Alternative:_ `#[tool_router]` macros per tool. Rejected: our tool set is dynamic (executor specs plus skills), so the handler implements `list_tools`/`call_tool` directly.

### 2. Caller identity through request extensions

An axum middleware on `/mcp` resolves the credential to `AuthContext` and inserts it into the HTTP request extensions. `rmcp` copies `http::request::Parts` into the handler's `RequestContext` extensions, and the handler reads `AuthContext` from there. Stdio inserts `AuthContext::system()`. Every tool call builds its `ExecutionContext` from that identity.

### 3. Stateless HTTP mode, dual era

`StreamableHttpServerConfig` with `legacy_session_mode` on so `initialize`-era (`2025-11-25`) clients keep working, `json_response` off so progress can stream. Origin validation: a request with no `Origin` (non-browser clients) passes; a present `Origin` must match the request's `Host`, the same rule `require_same_origin_mutation` applies.

### 4. Audience-bound tokens and `mcp:execute`

- Add `ResourceKind::Mcp`, so `mcp:execute` parses like other scopes.
- `/oauth/authorize` and `/oauth/token` accept RFC 8707 `resource`. When it equals the canonical `/mcp` URL the JWT's `aud` is that URL; otherwise the existing audience. `JwtService` verifies against an expected audience passed per call, so `/api` keeps rejecting MCP tokens and `/mcp` rejects API tokens.
- API keys are first-party credentials and are accepted on `/mcp` when they carry `mcp:execute`.
- DCR stays the registration path; `scopes_supported` in RFC 8414 metadata gains `mcp:execute`. Client ID Metadata Documents are a follow-up.
  _Alternative:_ accept any valid JWT on `/mcp`. Rejected: the spec requires audience validation, and audience isolation stops token replay between surfaces.

### 5. Tasks via `rmcp::TaskManager`

The handler owns a `TaskManager` keyed by owner (`AuthContext` user plus org). `run-prompt` from a task-declaring client spawns the turn in the manager and returns `CreateTaskResult`. `tasks/get` and `tasks/cancel` look up only the caller's tasks. State is in memory with a 30-minute TTL; after a restart unknown ids return not-found, which the spec allows.
On the client, `call_tool_once` is used; a `task` result is polled with `get_task` at `pollIntervalMs` (default 1 s, capped at 10 s) until terminal or the tool timeout, and `cancel_task` runs when the turn's cancellation token fires.

### 6. Client handler

`AssistantClientHandler` implements `ClientHandler`: `get_info` declares the Tasks, Skills and Apps extensions (Apps with `mimeTypes: ["text/html;profile=mcp-app"]`), and `on_tool_list_changed` / `on_prompt_list_changed` mark the connection's caches stale. For `2026-07-28` servers the connection opens one `listen` subscription for list changes. Both the stdio manager and the integration connection pool use the handler.

### 7. MCP Apps

- **Resolution (Rust).** `McpToolHandler` keeps the tool's `_meta.ui` (`resourceUri`, accepting the deprecated flat key, and `visibility`). After a call it reads the `ui://` resource from the same server, checks the MIME type and attaches an `McpApp { server, resource_uri, html, csp, permissions }` to `ToolOutput.data`. The runtime persists it with the tool result, and the `tool_result` SSE frame and message history gain an optional `app` field.
- **App-initiated calls.** New routes `POST /api/conversations/{id}/tool-calls/{tool_call_id}/app/tools-call` and `.../app/resources-read` look up the app's server from the stored record, enforce `app` visibility and same-server, and run through the permission pipeline. A tool without `readOnlyHint: true` requires confirmation in the Flutter UI before the request is sent.
- **Rendering (Flutter).** An `McpAppFrame` widget. On web: `HtmlElementView` hosting an `<iframe sandbox="allow-scripts" srcdoc=…>`. Without `allow-same-origin` the frame gets an opaque origin, so it cannot reach the assistant's cookies, storage or API. CSP is injected as a `<meta http-equiv>` built from `_meta.ui.csp`. On iOS and macOS: `webview_flutter` loads the same document; its JavaScript channel carries the bridge.
- **Bridge.** A Dart `McpAppBridge` implements the host side of the postMessage JSON-RPC (`ui/initialize`, tool-input, tool-result, size-changed, open-link, teardown). It is pure Dart over an abstract message port so it is unit-testable without a browser.
  _Alternative:_ the spec's separate-origin sandbox proxy. Rejected for now: it needs a second served origin, which self-hosted single-host installs do not have. An opaque-origin `srcdoc` frame gives the same isolation from the host's credentials. Apps that need `allow-same-origin` are not supported, and the limitation is documented.

### 8. Remote skills

`McpSkillSource` lists skills per Skills-enabled connection, downloads each manifest's files with `resources/read`, verifies size and sha256, and writes them to `~/.assistant/cache/mcp-skills/{server}/{skill}/`. The skill registry loads them as `SkillSource::Mcp { server }` named `{server}__{skill}`. A manifest whose digests change invalidates the cache entry. The `cache/` directory is already excluded from backups.

### 9. Prompts and completions

`list_prompts` maps installed skills to prompts with one optional `input` argument; `complete` answers prompt names and the `skills://{name}/…` template argument from the catalog.

## Risks / Trade-offs

- [`rmcp` APIs shift between minors] → Declare `rmcp = "3.11"` once in `[workspace.dependencies]` and bump deliberately.
- [`legacy_session_mode` keeps per-session state for old clients] → Rely on rmcp's session expiry; drop the legacy mode once clients move to `2026-07-28`.
- [Web apps that rely on `allow-same-origin` break] → Documented limitation; revisit with a sandbox origin.
- [Remote skills run third-party scripts] → They are always untrusted: script tools require confirmation, and verification is bound to manifest digests.
- [In-memory tasks are lost on restart] → Clients see not-found, which the spec allows; TTL bounds memory use.
- [`webview_flutter` adds native code to iOS and macOS] → Isolated in one widget behind a platform check; widget tests use a fake port.

## Migration Plan

`assistant mcp` keeps its name and stdio behaviour, so desktop MCP clients that launch it need no configuration change beyond accepting the newer protocol. HTTP access is new and opt-in: clients register through DCR. Rollback is reverting the stack; no data migration is involved.
