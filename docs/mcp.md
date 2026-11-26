# MCP

The assistant speaks the [Model Context Protocol](https://modelcontextprotocol.io)
(MCP) in two directions:

- **As a server**, it exposes its tools and skills to MCP clients over stdio
  (`assistant mcp`) and over Streamable HTTP (`/mcp` on the web server).
- **As a client**, it connects to the external MCP servers listed in
  `server.toml` and offers their tools and skills to the Persona.

Both sides support the protocol revisions `2026-07-28` and `2025-11-25`. For
the design behind this, see [ADR-0022](adr/adr-0022-mcp-on-rmcp.md).

<!-- TODO: server tasks / MCP Apps once merged -->

## Connect an MCP client over stdio

`assistant mcp` serves MCP on stdin and stdout until the client disconnects.
Logs go to stderr, so stdout carries only protocol messages.

Configure your MCP client to launch it. For a client that uses the common
`mcpServers` format:

```json
{
  "mcpServers": {
    "assistant": {
      "command": "assistant",
      "args": ["mcp"]
    }
  }
}
```

A stdio caller acts as the local system identity in the `default` space of
the `default` org. It needs no credentials. The `mcp` command is part of the
default build; a binary built without the `mcp` feature does not have it.

## Connect an MCP client over HTTP

The web server (`assistant webui serve`) serves MCP at `<public base URL>/mcp`.
The base URL is the OAuth issuer: `--public-url`, or derived from the listen
address, with no trailing slash. In the examples below it is
`https://assistant.example.com`.

Every request needs a bearer token. Two kinds are accepted:

- a JWT issued for the MCP endpoint (see [Sign in with OAuth](#sign-in-with-oauth)),
- an API key with the `mcp:execute` scope (see [Use an API key](#use-an-api-key)).

A JWT issued for the web API (`/api`) is rejected on `/mcp`, and an MCP token
is rejected on `/api`.

### Sign in with OAuth

An MCP client that supports MCP authorization finds everything on its own.
The steps it takes:

1. It sends a request to `/mcp` without a token. The server answers `401`
   with this header:

   ```http
   WWW-Authenticate: Bearer scope="mcp:execute", resource_metadata="https://assistant.example.com/.well-known/oauth-protected-resource/mcp"
   ```

2. It reads the protected resource metadata (RFC 9728) at that URL:

   ```json
   {
     "resource": "https://assistant.example.com/mcp",
     "authorization_servers": ["https://assistant.example.com"],
     "scopes_supported": ["mcp:execute"],
     "bearer_methods_supported": ["header"]
   }
   ```

3. It reads the authorization server metadata (RFC 8414) at
   `/.well-known/oauth-authorization-server`.

4. It registers itself at `POST /oauth/register` (RFC 7591). The server
   accepts `https` redirect URIs, loopback `http` redirect URIs
   (`http://127.0.0.1:<port>/...`, `http://localhost`, `http://[::1]`) and
   private-use schemes such as `com.example.app:/callback`.

5. It runs the authorization code flow with PKCE (`S256` only) and sends
   `resource=https://assistant.example.com/mcp` to both `/oauth/authorize`
   and `/oauth/token`.

The token it gets back has `aud` set to the MCP URL and carries exactly one
scope, `mcp:execute`, whatever scopes the client asked for. Refresh tokens
keep the same resource. The token is never set as the browser session cookie.

`resource` accepts only the canonical MCP URL. Any other value fails with
`invalid_target`. For the details, see
[MCP tokens](authentication.md#mcp-tokens-resource-and-mcpexecute).

### Use an API key

For a client that cannot run an OAuth flow, create an API key with the
`mcp:execute` scope:

```sh
assistant api-keys create --name "MCP client" --scopes "mcp:execute"
```

Send it as the bearer token:

```http
Authorization: Bearer ask_live_...
```

### Select a space

Each request runs in one space. If the caller has a role in exactly one
space, that space is used. Otherwise, send the space ID in the `x-space-id`
header, the same selector the web API uses:

```http
x-space-id: spc_a1
```

The caller must have a role in that space or be an org admin. A request runs
against that space's tools, skills and permission settings.

### Errors

| Status | Cause                                                                                                     |
| ------ | --------------------------------------------------------------------------------------------------------- |
| `401`  | No token, an unknown or expired token, or a JWT whose audience is not the MCP URL.                        |
| `403`  | The token lacks `mcp:execute` (`error="insufficient_scope"` in `WWW-Authenticate`).                       |
| `403`  | The request has an `Origin` header that does not match `Host`. Requests without `Origin` pass this check. |
| `400`  | The caller has roles in several spaces and sent no `x-space-id`.                                          |
| `403`  | The caller has no role in the selected space.                                                             |
| `503`  | The server could not check the credential.                                                                |

## What the server exposes

The same handler serves stdio and HTTP.

### Tools

`tools/list` returns:

- every tool of the space's tool executor, except tools that need a rich
  client to render their output,
- `run-prompt`, which sends a prompt through a full turn of the Persona and
  returns the answer,
- `install-skill`, which installs a skill from a local path or from GitHub
  (`owner/repo[/path]`).

Every `tools/call` except `run-prompt` passes the space's permission policy
first. An MCP caller cannot be asked for approval, so a call that no rule
allows is refused. The refusal comes back as a tool result with
`isError: true`. `run-prompt` runs a normal turn, so each tool the turn calls
passes the policy there. See [Tool permissions](tool-permissions.md#mcp-server).

A caller with the `viewer` role in the space cannot call tools.

### Resources

| URI                                 | Content                                                                           |
| ----------------------------------- | --------------------------------------------------------------------------------- |
| `skills://list`                     | JSON index of all skills.                                                         |
| `skills://<name>`                   | The skill's `SKILL.md`.                                                           |
| `skills://<name>/<category>/<file>` | A file bundled with the skill. `category` is `scripts`, `references` or `assets`. |

`resources/templates/list` returns the template
`skills://{name}/{category}/{+path}`. A path that resolves outside the skill's
directory returns resource-not-found.

<!-- TODO: prompts and completions once merged -->

## Connect external MCP servers

List external servers in `~/.assistant/server.toml` under `[mcp]`. The server
reads the file once at startup, and the servers are available in every space.

```toml
# Spawn a local process and talk to it over stdio.
[[mcp.servers]]
name = "github"
command = ["npx", "-y", "@modelcontextprotocol/server-github"]
trust = "confirm"
[mcp.servers.env]
GITHUB_TOKEN = "${GITHUB_TOKEN}"

# Connect to a remote server over Streamable HTTP.
[[mcp.servers]]
name = "remote-db"
url = "https://db.example.com/mcp"
[mcp.servers.headers]
Authorization = "Bearer ${DB_TOKEN}"
```

| Key                 | Default     | Meaning                                                                                              |
| ------------------- | ----------- | ---------------------------------------------------------------------------------------------------- |
| `name`              | (required)  | Lowercase letters, digits and `-`. Must not start with `org_`, `space_` or `user_`.                  |
| `command`           |             | Program and arguments for a stdio server. If both `command` and `url` are set, `command` wins.       |
| `url`               |             | Endpoint of an HTTP server.                                                                          |
| `headers`           | `{}`        | HTTP headers sent with every request to an HTTP server.                                              |
| `env`               | `{}`        | Environment variables for a stdio server. Not sent to HTTP servers.                                  |
| `enabled`           | `true`      | Set to `false` to skip the server.                                                                   |
| `trust`             | `"confirm"` | `"confirm"` makes every tool of the server require confirmation. `"trust"` lets them run without it. |
| `tool_timeout_secs` | `120`       | Longest a tool call to the server may take, in seconds, task polling included. Must be at least 1.   |

Values in `env` and `headers` can reference environment variables as
`${VAR}`. A name that starts with a reserved prefix, or a `tool_timeout_secs`
of 0, makes `server.toml` fail to load. A name with other invalid characters
is logged and skipped. `assistant doctor` lists the configured servers.

### Tools from external servers

Each tool appears to the Persona as `mcp__{server}__{tool}`. Calls pass the
permission policy like any other tool call, and `trust = "confirm"` marks
them as requiring confirmation.

The assistant keeps each server's tool list current:

- When a server announces that its tools or prompts changed, the assistant
  lists its tools again and replaces the registered ones. For a
  `2026-07-28` server it opens a subscription for these announcements.
- Every 30 seconds a health check finds disconnected servers, removes their
  tools and reconnects them with back-off (2 seconds, doubling up to 60).

A tool whose MCP Apps visibility (`_meta.ui.visibility`) does not include
`"model"` is meant for apps only and is not offered to the Persona.

Tool results are converted like this:

- Text blocks become the tool output text.
- Images, audio and embedded binary resources become attachments.
- Resource links and embedded text resources go into the text with their URI.
- `structuredContent` becomes the output text when the result has no text
  block, and always comes along as the tool's data.

### Tasks from external tools

A server can answer a tool call with a task instead of a result. The
assistant then polls the task until it ends:

- It waits the `pollIntervalMs` the server suggests, or 1 second if it
  suggests none. Each wait is at least 100 ms and at most 10 seconds.
- A completed task returns its result like a normal call.
- A failed or cancelled task, or one that needs input, returns an error
  result. The assistant cannot provide input.
- The whole call, polling included, is bounded by the server's tool timeout.
  When the timeout expires, or the turn is cancelled, the assistant sends
  `tasks/cancel`.

The tool timeout applies to every external tool call. It is 120 seconds
unless the server's `tool_timeout_secs` sets another value.

### Skills from external servers

When a server advertises the MCP Skills extension, the assistant offers its
skills in every space:

- **Naming.** A remote skill is named `{server}__{skill}`. Local skill names
  are kebab-case and contain no underscores, so a remote skill cannot replace
  a local one.
- **Verification.** The assistant downloads every file the skill's manifest
  lists with `resources/read` and checks its size and sha256 digest.
  `SKILL.md` must parse, and its `name` and `description` must match the
  manifest. A skill that fails any check is rejected with a warning in the
  log. A skill whose manifest says `"dynamic"` lists no files. Only its
  `SKILL.md` is fetched and its frontmatter checked, and it is never cached.
- **Cache.** Verified files are stored in
  `~/.assistant/cache/mcp-skills/{server}/{skill}/`, with the manifest they
  were checked against. They are read from the cache only while the manifest
  is unchanged and the files still match it. Otherwise they are downloaded
  and checked again. `cache/` is not backed up.
- **Refresh.** The assistant syncs remote skills at startup and every 30
  seconds. A disconnected server's skills are withdrawn. Remote skills live
  in memory only and cannot be edited or deleted through the skill API.
- **Scripts ask first.** A built-in ask rule matches any `bash` or `process`
  command that contains `/mcp-skills/`. Such a call always goes to a human,
  whatever your allow rules say, and is denied on turns that cannot prompt.
  See [Tool permissions](tool-permissions.md#ask).

### Integration servers

Integrations connect to MCP servers over Streamable HTTP through a separate
connection pool, one connection per integration and user. Their tools are
named `mcp__{layer}_{slug}__{tool}`, where `layer` is `org`, `space` or `user`.
That is why `[mcp]` server names cannot start with those prefixes. These connections handle list-change
announcements, tasks and content the same way as `[mcp]` servers. Their tool
timeout is always 120 seconds.

## Known limits

- **`install-skill` does not work over HTTP.** It is listed, but a call over
  `/mcp` fails because the HTTP handler has no skill registry to install
  into. Call it over stdio (`assistant mcp`) instead.
- **Integration servers do not publish skills yet.** Only `[mcp]` servers
  are asked for skills.
- **Slow fallback for `2025-11-25` servers.** The client tries
  `server/discover` first. A `2025-11-25` server that never answers it
  delays the connection by 10 seconds before the client falls back to
  `initialize`. For an integration server this is longer than the pool's
  5-second connection timeout, so the connection fails.
- **No sampling, roots or elicitation.** The client does not offer these to
  servers. A tool that asks for input gets an error result.
