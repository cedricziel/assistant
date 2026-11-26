# MCP

The assistant speaks the [Model Context Protocol](https://modelcontextprotocol.io)
(MCP) in two directions:

- **As a server**, it exposes its tools and skills to MCP clients over stdio
  (`assistant mcp`) and over Streamable HTTP (`/mcp` on the web server).
- **As a client**, it connects to the external MCP servers listed in
  `server.toml` and offers their tools and skills to the Persona.

Both sides support the protocol revisions `2026-07-28` and `2025-11-25`. For
the design behind this, see [ADR-0022](adr/adr-0022-mcp-on-rmcp.md).

External tools can attach an interactive app to their result. The chat shows
it below the tool call. See [MCP Apps](#mcp-apps).

As a server, `run-prompt` can run as a task that the client polls. See
[Run prompts as tasks](#run-prompts-as-tasks).

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
  returns the answer (or a task, see [Run prompts as tasks](#run-prompts-as-tasks)),
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

### Prompts

`prompts/list` returns one prompt per installed skill. The prompt has the
skill's name and description and one optional argument, `input`.

`prompts/get` returns a single user message with the skill's instructions.
If you pass `input`, it follows the instructions after a blank line. An
unknown prompt name fails with an invalid-params error.

### Completions

`completion/complete` completes skill names. It answers for an argument
called `name`, both on a prompt reference and on the resource template
`skills://{name}/{category}/{+path}`. It returns the skill names that start
with the typed value, sorted, at most 100, with the total count and whether
more exist. Any other argument gets an empty list.

### List-changed notifications

The server announces `listChanged` for tools, prompts and resources. After
`install-skill` installs a skill, the server tells every session that is
listening that all three lists changed:

- A `2026-07-28` session gets the notifications on the
  `subscriptions/listen` streams it opened, limited to the kinds it asked
  for.
- A `2025-11-25` session gets the classic `notifications/*/list_changed`
  messages.

Only `install-skill` sends these notifications. A skill added or removed any
other way sends none. Over HTTP, `/mcp` sessions are registered as listeners,
but `install-skill` is unavailable there (see [Known limits](#known-limits)),
so they receive no notifications in practice. Each `assistant mcp` process
notifies only its own session.

### Progress and cancellation

When a `run-prompt` call carries a progress token, the server sends two
progress notifications out of a total of 1: `0` with the message
`Turn started`, and `1` with `Turn finished`.

If the client cancels a `run-prompt` request, the server cancels the
Persona's turn and sends no response for that request.

### Run prompts as tasks

A turn can run for minutes. If the client declares the Tasks extension
(`io.modelcontextprotocol/tasks`) in its `initialize` capabilities, `run-prompt`
returns a task at once instead of blocking:

```json
{ "capabilities": { "extensions": { "io.modelcontextprotocol/tasks": {} } } }
```

The server advertises the extension in its own capabilities. A client without
it gets the blocking result described above. Other tools always answer
directly.

The `tools/call` result is the task itself:

```json
{
  "resultType": "task",
  "taskId": "66dcd908-2a4e-4cfa-9d43-255362e82670",
  "status": "working",
  "ttlMs": 1800000,
  "pollIntervalMs": 1000
}
```

Then:

- Poll `tasks/get` with `{"taskId": "..."}`. `pollIntervalMs` suggests 1
  second.
- `working` means the turn is running. The terminal statuses are `completed`,
  `failed` and `cancelled`.
- A `completed` task carries the `run-prompt` tool result in `result`: the
  Persona's answer as a text block.
- `failed` means the turn errored or the task outlived its TTL.
- `tasks/cancel` with `{"taskId": "..."}` cancels the turn through the
  orchestrator, and the task settles as `cancelled`. Cancelling a finished task
  is accepted and keeps its status.

A task belongs to the user and org that created it. `tasks/get` and
`tasks/cancel` for another identity's task fail with an invalid-params error,
`unknown task: <id>`, the same as for an unknown ID.

Over HTTP, one task store serves the whole `/mcp` endpoint, so you can poll a
task from a later request. Over stdio, tasks live in the `assistant mcp`
process.

Limits:

- A task has a TTL of 30 minutes. A task still running then is marked `failed`
  and its turn is aborted. A finished task stays observable for up to another
  30 minutes, then is removed.
- Tasks live in memory. A server restart loses them.
- There is no `tasks/update` (it returns method-not-found) and no `tasks/list`.
  The server never asks the client for input, so no task enters
  `input_required`.

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

## MCP Apps

An MCP App is a small HTML page that an external MCP server ships with one of
its tools. When the Persona calls that tool, the chat shows the page below
the tool call, for example a weather card or a form. The page receives the
tool's input and result, and it can call its own server's tools. Apps work
for tools from `[mcp]` servers and from integration servers.

### How the assistant finds an app

A tool declares an app by setting `_meta.ui.resourceUri` to a `ui://` URI.
After each call to such a tool, the assistant reads that resource from the
same server and attaches it to the tool call. The resource must meet these
conditions:

- Its MIME type is `text/html;profile=mcp-app`.
- Its content is UTF-8 and at most 1 MiB.

The assistant takes the app's allowed domains (`csp`), device permissions
(`permissions`) and `prefersBorder` from the resource content's `_meta.ui`.
If the content has none, it uses the `_meta.ui` of the matching entry in
`resources/list`.

If the read fails or a condition is not met, the tool call shows as a plain
tool-call chip, and the log has a warning that names the server and URI.

The app is stored with the tool call, so it shows again when you reopen the
conversation.

### What you see

The app appears in a frame below the tool-call chip, labelled
"Third-party app from {server}". The frame starts 160 pixels tall. The app
can resize it to between 48 and 800 pixels.

The app asks you before it does anything outside the frame:

- **Tool calls that can change data.** The dialog asks
  `Allow {server} to run "{tool}"?` and shows the arguments. The
  call runs only if you choose **Run**.
- **Links.** The dialog shows the URL. If you choose **Open**, the link
  opens in your browser. Only `http` and `https` links open.

If you decline, the app gets an error back.

### What an app can do

An app can call tools and read resources only on the server that provided
it. The web API serves these calls at:

- `POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call`
- `POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read`

Both routes need write access to the space and the `conversations:write`
scope. They reach the server with your own access, and request bodies are
limited to 256 KiB.

For a tool call, the server applies these rules:

- The tool must be on the app's server, and its `_meta.ui.visibility` must
  include `"app"`. A tool with no visibility list is visible to apps.
  Any other tool is refused with `403`.
- The call goes through the space's permission policy under the name
  `mcp__{server}__{tool}`. A denied call is refused with `403` and the
  policy's reason.
- The call needs your confirmation if the tool is not annotated
  `readOnlyHint: true`, if the server's trust level is `confirm`, or if the
  policy answers `ask`. Without confirmation the route returns `409`, and the
  chat asks you before it retries.

Resource reads need no confirmation.

### Security boundaries

The app runs in a sandbox that keeps it away from the assistant's session:

- **Web.** The app runs in an `<iframe sandbox="allow-scripts">` loaded from
  `srcdoc`, without `allow-same-origin`. The frame has an opaque origin, so
  it cannot read the assistant's cookies or storage, or call its API.
  Device permissions the app declares become the iframe's `allow` attribute.
- **iOS and macOS.** The app runs in a web view (`webview_flutter`). The web
  view blocks the app from navigating away from its page, and it grants
  camera and microphone access only if the app declared it.
- **Other platforms.** The frame shows "This app cannot be shown on this
  device."

The chat puts a Content Security Policy at the top of every app document,
ahead of the app's own markup. The policy is built only from the domains the
app declares. With no declared domains it is the MCP Apps default:

- no network connections (`connect-src 'none'`) and no nested frames,
- inline scripts and styles only,
- images and media only inline or as `data:` URLs.

A declared domain that is not a plain host source is dropped, so it cannot
add directives to the policy.

The app has no direct line to the assistant's API. The chat forwards its
tool calls and resource reads to the routes above, which run every check.

### Limitations

- Apps that need `allow-same-origin` do not work.
- On web, the frame leaves the page as soon as the chat removes it, so an
  app may not get to handle `ui/resource-teardown` first.
- The host reports its version to apps as `"unknown"`.
- The host reports the time zone as the platform's abbreviation, such as
  `CET`, not as an IANA name.
- Apps show only inline, below the tool call. There is no fullscreen or
  picture-in-picture mode.

## Known limits

- **`install-skill` does not work over HTTP.** It is listed, but a call over
  `/mcp` fails because the HTTP handler has no skill registry to install
  into. Call it over stdio (`assistant mcp`) instead. As a result, `/mcp`
  sessions get no list-changed notifications.
- **Integration servers do not publish skills yet.** Only `[mcp]` servers
  are asked for skills.
- **Slow fallback for `2025-11-25` servers.** The client tries
  `server/discover` first. A `2025-11-25` server that never answers it
  delays the connection by 10 seconds before the client falls back to
  `initialize`. For an integration server this is longer than the pool's
  5-second connection timeout, so the connection fails.
- **No sampling, roots or elicitation.** The client does not offer these to
  servers. A tool that asks for input gets an error result.
