# Tasks

Each numbered group is one PR in the stack, in order.

## 1. Fix skill resource traversal (PR 1)

- [x] 1.1 Add a failing test in `crates/mcp-server/tests/dispatch.rs` that reads `skills://<name>/scripts/../../<file>` and expects resource-not-found; confirm red
- [x] 1.2 Reject resolved paths outside the skill directory (canonicalize and check prefix) and verify the test passes

## 2. Workspace rmcp and server handler on stdio (PR 2)

- [x] 2.1 Declare `rmcp = "3.11"` in `[workspace.dependencies]`, point `mcp-client` and `mcp-server` at it (server: `server`, `transport-io`; `macros` is not needed because the handler implements `list_tools`/`call_tool` directly), and verify `cargo check -p assistant-mcp-server -p assistant-mcp-client` passes
- [x] 2.2 Port `tests/dispatch.rs` and `tests/io_loop.rs` to drive the server through an rmcp client over an in-process duplex transport; confirm red against a stub `AssistantMcpServer`
- [x] 2.3 Implement `AssistantMcpServer: ServerHandler` with `get_info`, `list_tools` (deterministic order), `call_tool`, `list_resources`, `list_resource_templates`, `read_resource`, carrying over permission evaluation and the traversal guard; verify the ported tests pass
- [x] 2.4 Add tests that `server/discover` reports `2026-07-28` and `2025-11-25`, that `initialize` at `2025-11-25` succeeds and that `2024-11-05` is rejected; make them pass
- [x] 2.5 Delete `protocol.rs`, `run_io` and the line loop; serve stdio via rmcp in `run`; verify with `cargo test -p assistant-mcp-server` and a `cargo machete --with-metadata` run
- [x] 2.6 Add a test that stdout carries only protocol frames while the handler logs, and make it pass

## 3. Caller identity in the server (PR 3)

- [x] 3.1 Add a failing test: a `tools/call` whose request extensions carry a user `AuthContext` reaches the dispatcher with that user, org and space in `ExecutionContext`; confirm red
- [x] 3.2 Read `AuthContext` from request extensions (stdio injects `AuthContext::system()`), build `ExecutionContext` and `submit_turn` auth from it, and verify the test passes

## 4. Prompts, completions, list changes, progress and cancellation (PR 4)

- [x] 4.1 Add failing tests for `prompts/list`, `prompts/get` with and without `input`, and `completion/complete` for prompt names and the skill template; confirm red
- [x] 4.2 Implement prompts and completions from the skill catalog and verify the tests pass
- [x] 4.3 Add a failing test: a client listening for tool and prompt list changes receives notifications after `install-skill`; implement change fan-out and verify it passes
- [ ] 4.4 Add failing tests for `run-prompt` progress notifications with a progress token and turn cancellation on request cancel (using `StubOrchestrationEngine`); implement and verify they pass

## 5. Audience-bound tokens and `mcp:execute` (PR 5)

- [x] 5.1 Add failing tests in `crates/core` and `crates/auth`: `mcp:execute` parses; a JWT issued with resource `<origin>/mcp` has that `aud`; verification with the API audience rejects it and vice versa; confirm red
- [x] 5.2 Add `ResourceKind::Mcp`, per-call expected audience in `JwtService`, and verify the tests pass
- [x] 5.3 Add failing web-ui tests: `/oauth/authorize` + `/oauth/token` with `resource` issue an MCP-audience token; DCR with a loopback redirect URI and `mcp:execute` succeeds; RFC 8414 metadata lists `mcp:execute`; implement and verify they pass
- [x] 5.4 Document the new `resource` parameter in the utoipa annotations, run `make dump-openapi` and `make generate-flutter-client`, and verify `openapi.json` diff contains only the expected changes

## 6. Streamable HTTP `/mcp` endpoint (PR 6)

- [x] 6.1 Add failing web-ui integration tests: 401 with `resource_metadata` challenge without a token; 403 `insufficient_scope` for a token without `mcp:execute`; 401 for an API-audience JWT; 200 tool list for an MCP-audience token and for an API key with `mcp:execute`; 403 for a foreign `Origin`; confirm red
- [x] 6.2 Add `/.well-known/oauth-protected-resource/mcp` with a failing test first, then implement it with OpenAPI docs
- [x] 6.3 Mount rmcp `StreamableHttpService` at `/mcp` behind an MCP auth middleware that inserts `AuthContext`, enable `server-side-http`, and verify 6.1 passes
- [x] 6.4 Add a test that a space-restricted user cannot reach another space's data through `/mcp`; make it pass
- [x] 6.5 Run `make dump-openapi` and `make generate-flutter-client`; verify the generated client builds with `make lint-flutter`

## 7. Server Tasks (PR 7)

- [ ] 7.1 Add failing tests: a task-declaring client gets `resultType: "task"` for `run-prompt`; a non-declaring client gets a blocking result; polling reaches `completed` with the result; `tasks/cancel` yields `cancelled`; another user's task id is not-found; confirm red
- [ ] 7.2 Wire an owner-scoped `rmcp::TaskManager` into `AssistantMcpServer`, advertise the extension, and verify the tests pass

## 8. Client handler, freshness and content conversion (PR 8)

- [x] 8.1 Add failing tests (rmcp test server over duplex): client capabilities declare Tasks, Skills and Apps and not sampling, roots or elicitation; a tool-list-changed notification makes the next tool listing fresh; a `2025-11-25`-only server connects; confirm red
- [x] 8.2 Implement `AssistantClientHandler`, use it in the stdio manager and integration pool, open a `listen` subscription for `2026-07-28` servers, and verify the tests pass
- [x] 8.3 Add failing `bridge.rs` tests for audio, resource link, embedded text and binary resources, and structured-content-only results; implement conversion and verify they pass
- [x] 8.4 Add a failing test that a tool with visibility `["app"]` is not offered to the Persona; implement and verify it passes

## 9. Client Tasks (PR 9)

- [x] 9.1 Add failing tests: an external tool returning a task is polled at `pollIntervalMs` until completed; turn cancellation sends `tasks/cancel`; the tool timeout bounds polling; confirm red (use `tokio::time::pause`)
- [x] 9.2 Switch the bridge to `call_tool_once` with a polling loop and verify the tests pass

## 10. Client Skills extension (PR 10)

- [x] 10.1 Add failing tests: skills from a Skills-enabled server appear as `{server}__{skill}`; a server without the extension gets no `skills/list`; a digest or size mismatch rejects the skill with a warning; a changed manifest forces re-verification; confirm red
- [x] 10.2 Add `SkillSource::Mcp { server }` and `McpSkillSource` with the verified cache under `~/.assistant/cache/mcp-skills/`, and verify the tests pass
- [x] 10.3 Add a failing test that script tools from a remote skill require confirmation under the permission policy; implement and verify it passes

## 11. MCP Apps resolution and API (PR 11)

- [ ] 11.1 Add failing mcp-client tests: a tool with `_meta.ui.resourceUri` (and the deprecated flat key) attaches `McpApp` after the call; a resource without `text/html;profile=mcp-app` attaches nothing; confirm red
- [ ] 11.2 Implement app resolution in the bridge and verify the tests pass
- [ ] 11.3 Add failing runtime and web-ui tests: the `tool_result` SSE frame and message history carry `app`; implement persistence and projection and verify they pass
- [ ] 11.4 Add failing web-ui tests for `POST .../app/tools-call` and `.../app/resources-read`: same-server only, `app` visibility enforced, permission pipeline applied; implement with OpenAPI docs and verify they pass
- [ ] 11.5 Run `make dump-openapi` and `make generate-flutter-client`; verify the generated models include `app`

## 12. MCP Apps rendering in Flutter (PR 12)

- [ ] 12.1 Add failing unit tests for `McpAppBridge` over a fake port: `ui/initialize` reply, tool input then result after `initialized`, size-change handling, teardown, open-link confirmation, rejection of cross-server calls; confirm red
- [ ] 12.2 Implement `McpAppBridge` and a CSP builder (with unit tests for declared domains and the default policy) and verify the tests pass
- [ ] 12.3 Add `McpAppFrame` in its own file with a widget test using a fake platform view: renders below the chip, labelled with the server name, asks confirmation for non-read-only tool calls; web uses a sandboxed `srcdoc` iframe, iOS and macOS use `webview_flutter`
- [ ] 12.4 Render `McpAppFrame` from `tool_call_chip.dart` for records with an app, extending its widget tests for live and history-reloaded conversations
- [ ] 12.5 Add a Playwright scenario with a fixture app, update screenshot baselines, and document the visual diff in the PR

## 13. Docs and ADR (PR 13)

- [ ] 13.1 Add `docs/adr/adr-0017-mcp-on-rmcp.md` covering the rmcp migration, audience-bound tokens and the opaque-origin app sandbox
- [ ] 13.2 Write `docs/mcp.md` (stdio and HTTP setup, OAuth/DCR and API keys, tasks, remote skills, MCP Apps and their limitation) and update `docs/tool-permissions.md`, `docs/authentication.md` and `docs/operations/configuration.md`; check every claim against the code

## 14. Verification

- [ ] 14.1 Run `make lint && make format && make test && make lint-flutter && make test-flutter` and verify all pass
- [ ] 14.2 Create `.claude/skills/verify/SKILL.md` describing how to start an isolated instance (temp `HOME`, free ports, test credentials) and clean it up, and verify it by running it once
- [ ] 14.3 Run the project `verify` skill against an isolated instance: connect an MCP client over stdio and over `/mcp` with a DCR-registered client, call a tool, poll a `run-prompt` task, and render a fixture MCP App
