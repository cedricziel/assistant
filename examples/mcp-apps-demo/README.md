# MCP Apps demo

An MCP server with one MCP App, built on the official SDKs:
`@modelcontextprotocol/server` 2.4.0 (the v2 TypeScript SDK, which replaces
`@modelcontextprotocol/sdk`) and `@modelcontextprotocol/ext-apps` 2.0.4.
`package.json` pins both exactly.

| Tool            | Who can call it | Notes                                                                                                      |
| --------------- | --------------- | ---------------------------------------------------------------------------------------------------------- |
| `get-forecast`  | the model       | Read-only. Its app is `ui://demo/forecast.html`, served as `text/html;profile=mcp-app`.                    |
| `save-favorite` | the app only    | Visibility `["app"]`. Not read-only, so the assistant asks the user to confirm before the app's call runs. |

The app page (`src/app.js`) shows the tool input and result the host sends. Its
"Save as favorite" button calls `save-favorite` through the host.

## Run it in the assistant

You need Node 20 or newer. The server speaks MCP over stdio only.

1. Build the app page. This bundles the SDK into one HTML file,
   `dist/app.html`, which the server reads when a host reads the resource.

   ```sh
   cd examples/mcp-apps-demo
   npm ci && npm run build
   ```

2. List the server in `~/.assistant/server.toml`:

   ```toml
   [[mcp.servers]]
   name = "demo"
   command = ["node", "/path/to/assistant/examples/mcp-apps-demo/server.mjs"]
   trust = "trust"
   ```

3. Allow the tool in `~/.assistant/orgs/default/org.toml`. With
   `trust = "trust"` the server does not ask for confirmation, but a web UI turn
   cannot prompt, so the permission policy denies the call without an allow rule.

   ```toml
   [permissions]
   allow = ["mcp__demo__get-forecast"]
   ```

4. Restart the assistant. It reads `server.toml` once, at startup.

5. In the web chat, ask the Persona for the weather in a city. The tool shows
   as `mcp__demo__get-forecast`, and the app renders below the tool call.

See [Connect external MCP servers](../../docs/mcp.md#connect-external-mcp-servers)
and [Tool permissions](../../docs/tool-permissions.md).

## Tests

The Playwright scenario `crates/web-ui/e2e/tests/mcp-app.spec.ts` runs against
this server. The e2e harness writes the config above
(`crates/web-ui/e2e/tests/support/e2e-server.mjs`) and needs the build from step 1.
