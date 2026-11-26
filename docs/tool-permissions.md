# Tool permissions

Every tool call the LLM makes passes through a permission check before it runs. This page covers the `[permissions]` section of `~/.assistant/orgs/default/org.toml`, the order the checks run in, and how to write rules.

Subagent Processes use the same check as the main turn.

## What changed

Before this feature, tools ran without a check unless a tool asked for confirmation, and that check only applied to interactive turns with a callback registered. Now every call is checked.

With no `[permissions]` section, a call that no rule allows is denied on any turn that cannot prompt a user. Scheduled tasks, Slack, Matrix, the web UI and Subagent Processes all fall in that group. Only the interactive CLI turn can prompt. If a scheduled task used `bash` or `web-fetch` before, it fails until you add an allow rule for it.

## How a call is decided

Checks run in this order. The first one that decides wins.

1. **Deny rules.** Built-in hard-deny rules, then your `deny` rules. A match denies the call.
2. **Confinement.** For `file-read`, `file-write`, `file-edit` and `file-glob`, a path outside the workspace roots is denied. No allow rule overrides this. To open another directory, add it to `workspace_roots`.
3. **Saved deny rules.** The calling user's saved deny rules (see [Approvals and saved rules](#approvals-and-saved-rules)). A match denies the call.
4. **Ask rules.** Built-in ask rules, then your `ask` rules. A match sends the call to a human.
5. **Allow rules.** Built-in allow rules, then your `allow` rules. A match allows the call.
6. **Saved allow rules.** The calling user's saved allow rules. A match allows the call, even for a tool that asks for confirmation.
7. **Decision model.** A call no rule decided goes to the decision model when `[permissions.decision_model] enabled = true`. A call whose tool flags it as requiring confirmation skips the model and goes to a human. The model can approve a call and nothing else.
8. **Human.** Anything still undecided goes to a human. This is also the path when no model is configured.

"Goes to a human" resolves like this:

- On an interactive turn, the owner is asked. In the CLI REPL that is a prompt on stdin. In web chat it is an approval card (see below). The answer decides.
- On any other turn, the call is denied. The reason goes back to the LLM as the tool result, for example `Permission denied for 'bash': approval is required but this turn cannot prompt a user (...). Ask the owner to add an allow rule for this call.`

Scheduled tasks, messenger interfaces (Slack, Matrix, Signal, Mattermost, Nextcloud), A2A, MCP and Subagent Processes never prompt, so an undecided call there is denied. They do read saved rules.

The final outcome is `allow`, `deny` or `ask`. `ask` is the label for "a human decides, or denial if nobody can be asked".

Config rules add to the built-in rules. They cannot remove a built-in deny or ask rule.

## Decision model

The decision model is off by default. When enabled, it sees the calls that static rules leave open and approves the ones that look safe, so a human is asked less often. It never denies. A call it does not approve goes to the human step above.

```toml
[permissions.decision_model]
enabled = true
model = "typesafe/jev-1.13"
threshold = 0.9
timeout_secs = 8
base_url = "https://openrouter.ai/api/alpha/decisions"
api_key_env = "OPENROUTER_API_KEY"
```

| Key            | Default                                     | Meaning                                                               |
| -------------- | ------------------------------------------- | --------------------------------------------------------------------- |
| `enabled`      | `false`                                     | Turns the model on.                                                   |
| `model`        | `typesafe/jev-1.13`                         | Model id. It must carry a version.                                    |
| `threshold`    | `0.9`                                       | Score every asked question must reach. Must be above 0 and at most 1. |
| `timeout_secs` | `8`                                         | Request timeout. A timeout means "not approved".                      |
| `base_url`     | `https://openrouter.ai/api/alpha/decisions` | Decisions endpoint.                                                   |
| `api_key`      | _(none)_                                    | API key. See the lookup order below.                                  |
| `api_key_env`  | `OPENROUTER_API_KEY`                        | Name of the environment variable that holds the API key.              |

`model` must be pinned to a version. `latest` aliases and ids without a version make the config fail to load, so approval behavior never changes without a config change.

The API key is looked up in this order:

1. `api_key` in `[permissions.decision_model]`.
2. `api_key` in `[llm]`, but only when `[llm] provider = "openrouter"`. A key for another provider is never sent to OpenRouter.
3. The environment variable named by `api_key_env`.

If `enabled` is true and none of these gives a key, the model is left out and an error is logged. Undecided calls then go to a human, or are denied on turns that cannot prompt.

### What the model is sent

- The tool call, rendered as one command string and cut at 2,000 characters. For `bash` this is the `command`. For other tools it is the tool name and the parameters as JSON.
- The working directory.
- The agent id.
- The first 1,000 characters of the user's latest message, when there is one.

It asks two questions and gets a score from 0 to 1 for each:

- `reversible`: the call only reads or changes files inside the working directory, or makes read-only requests, and any change can be undone.
- `serves_task`: the call is a reasonable next step toward the user's message. This is asked only when there is task text.

The model is not sent tool output, fetched content or earlier assistant text. Text from those sources cannot argue for its own approval.

### When a call is approved

Every asked question must score at least `threshold`. These outcomes all mean "not approved":

- a score below the threshold,
- an HTTP error,
- a timeout,
- a response that cannot be read,
- a score that is missing or outside 0 to 1.

Not approved goes to a human on an interactive turn and is denied on any other turn. Each undecided call costs one request, so put hot paths under narrow allow rules.

### Network tools

For `web-fetch`, the rendered call includes the URL, the destination host, and whether the host resolves to a private address. Loopback, private, link-local, unique-local, unspecified and CGNAT (`100.64.0.0/10`) ranges count as `private network address`. A host that does not resolve in 2 seconds is `address unknown`. Anything else is `public address`.

This is context for the model. Nothing is blocked on it.

`web-fetch` results report the final URL after redirects, as `URL: <final url>`.

### Limits of the model

Crafted arguments can steer the score. Keep deny rules and file confinement as the boundary, and treat the model as a way to cut prompts. Use narrow allow rules for calls you make often. They are faster and do not depend on a score.

## Configuration

```toml
[permissions]
deny = [
  "bash(git push --force*)",
  "web-fetch(*169.254.169.254*)",
]
ask = ["bash(git push*)"]
allow = [
  "bash(git status*)",
  "bash(cargo test*)",
  "web-fetch(https://docs.rs/**)",
  "web-fetch(http://homeassistant.local:8123/**)",
  "file-read(~/notes/**)",
]
workspace_roots = ["~/notes"]
```

| Key                        | Type          | Meaning                                                                                |
| -------------------------- | ------------- | -------------------------------------------------------------------------------------- |
| `deny`                     | list of rules | Always deny.                                                                           |
| `ask`                      | list of rules | Always send to a human.                                                                |
| `allow`                    | list of rules | Allow, unless a deny, confinement or ask check fires first.                            |
| `workspace_roots`          | list of paths | Directories file tools may use, besides the Persona workspace.                         |
| `limits`                   | table         | Resource limits for `bash` and `process` (below).                                      |
| `decision_model`           | table         | Decision model (above).                                                                |
| `interaction_timeout_secs` | integer       | Seconds an approval or question waits for an answer. Default `600`, range `30..=3600`. |

A rule that does not parse, such as `bash(oops`, stops the config from loading, and the error names the rule. An unknown key under `[permissions]` does too, as does an unpinned `decision_model.model` or a `threshold` outside (0, 1], or an `interaction_timeout_secs` outside `30..=3600`.

### Rule grammar

A rule is either `tool` or `tool(pattern)`.

- `tool` matches every call to that tool, whatever the arguments.
- `tool(pattern)` matches calls whose primary argument matches the pattern.
- The tool part also takes wildcards: `file-*` covers all file tools.
- `*` matches any run of characters, including `/` and nothing. `**` means the same.
- `\*` matches a literal `*`.
- `<workspace>` in a pattern expands to each workspace root.
- A pattern starting with `~/` expands to your home directory.
- The whole argument must match. `git status*` matches `git status -s`, not `echo git status`.

### Primary argument

The pattern is matched against one string per call.

| Tool                                   | Matched against                                                                                                                       |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `bash`                                 | `command`                                                                                                                             |
| `file-read`, `file-write`, `file-edit` | `path`, after resolution (see below)                                                                                                  |
| `file-glob`                            | the literal prefix of `pattern`, up to the first `*`, `?`, `[` or `{`, cut back to the last `/` before it. `.` if that leaves nothing |
| `web-fetch`                            | `url`                                                                                                                                 |
| `web-search`                           | `query`                                                                                                                               |
| any other tool                         | the parameters as compact JSON                                                                                                        |

### Bash and shell metacharacters

A shell command can chain more than one command, so the two directions differ.

- **Allow rules** never match a command that contains any of `;` `|` `&` newline `` ` `` `$` `(` `)` `<` `>`. `bash(git status*)` allows `git status -s` but not `git status; rm file`. Such a command falls through to the human fallback.
- **Deny and ask rules** match the whole command, and also each piece you get by splitting it at those characters. `bash(rm -rf /)` catches `echo hi; rm -rf /`.

### Paths and workspace roots

File tools may touch the workspace roots and nothing else. The roots are:

- the runtime workspace directory, when one is set,
- the Persona's workspace,
- every entry in `workspace_roots`.

The file tools enforce the same roots themselves, so the limit also holds for callers that bypass the permission check (see Known gap). `workspace_roots` is the only way to widen it. Before a path is matched or checked, it is resolved: `~/` is expanded, relative paths are made absolute, `..` is applied, and symlinks are followed. A symlink inside the workspace that points outside it resolves to the outside location and is denied. A path that does not exist yet keeps its missing parts as written, so `file-write` to a new file works.

## Resource limits

The `bash` and `process` tools enforce these limits themselves, whatever the permission check decided. Set them under `[permissions.limits]`.

```toml
[permissions.limits]
bash_max_timeout_secs = 600
bash_max_output_chars = 200000
process_ttl_secs = 3600
process_max_sessions = 16
process_max_line_chars = 4000
```

| Key                      | Default  | Meaning                                                                                                              |
| ------------------------ | -------- | -------------------------------------------------------------------------------------------------------------------- |
| `bash_max_timeout_secs`  | `600`    | Upper bound for the `timeout_secs` argument of `bash`. Larger values are clamped.                                    |
| `bash_max_output_chars`  | `200000` | Output kept per stream. `bash` reads output as the command runs and keeps the tail, so memory stays near this value. |
| `process_ttl_secs`       | `3600`   | A `process` session with no `poll`, `log`, `write`, `kill` call for this long is killed and removed.                 |
| `process_max_sessions`   | `16`     | Maximum number of running `process` sessions. `start` fails at the limit.                                            |
| `process_max_line_chars` | `4000`   | Stored output lines longer than this are cut and end with `[line truncated]`.                                        |

`bash` attaches an image to the reply only when its path, with symlinks resolved, lies inside a workspace root. Before this change it attached any image path printed to stdout.

## Built-in rules

### Deny

These are always on.

Destructive filesystem commands:

- `rm -rf` and `rm -fr` with the target `/`, `/*`, `~`, `~/`, `~/*`, or `$HOME*`
- `mkfs*`
- `dd *of=/dev/*`
- `:(){*` (fork bomb)
- `chmod -R 777 /*`

Privilege escalation and host control:

- `sudo *`, `su`, `su *`
- `shutdown*`, `reboot*`, `poweroff*`, `halt`

Remote code piped into a shell:

- `*curl *| *sh*`, `*curl *| *bash*`, and the same two with `wget`

Credential stores. `bash` is denied when the command contains one of these paths, and `file-*` is denied when the path matches:

- `/.ssh/`, `/.gnupg/`, `/.aws/credentials`, `/.kube/config`, `/.netrc`, `/etc/shadow`
- for `bash` only, the `~` spellings: `~/.ssh`, `~/.gnupg`, `~/.aws/credentials`, `~/.kube/config`, `~/.netrc`

The credential rules for file tools cover reads too, so `file-read` of `~/.ssh/id_ed25519` is denied.

### Ask

These always go to a human, whatever your allow rules say:

- `bash(*/mcp-skills/*)` and `process(*/mcp-skills/*)`

Skills that external MCP servers publish are cached under `~/.assistant/cache/mcp-skills/` and are untrusted, so a command that runs one of their scripts asks first. On a turn that cannot prompt, the call is denied. The rule matches the command text, so a command that never names that directory is left to the rest of the checks. See [MCP](mcp.md#skills-from-external-servers).

### Allow

These tools run without a rule of yours:

- `memory-get`, `memory-search`, `memory-append`
- `list-skills`, `load-skill`
- `list-tasks`, `schedule-task`, `cancel-task`
- `plan-update`
- `agent-spawn`, `agent-status`, `agent-terminate`
- `self-analyze`, `voice-response`
- `web-search`
- `file-read`, `file-write`, `file-edit` and `file-glob` on paths inside a workspace root

Everything else needs a rule of yours or a human. That includes `bash`, `web-fetch` and `process`.

## Network destinations

Rules do not block network destinations. Local and LAN access is a core use case for the assistant, so nothing rejects loopback, private or link-local addresses by default. `web-fetch` is not in the built-in allow list, so a fetch is undecided until you write a rule.

Govern it with `web-fetch(<url pattern>)` rules:

```toml
[permissions]
allow = ["web-fetch(http://homeassistant.local:8123/**)"]
deny = ["web-fetch(*169.254.169.254*)"]
```

Rules match the URL string as the LLM wrote it. A hostname that resolves to a blocked address, or a redirect to one, does not match a deny rule on the IP. Prefer narrow allow rules over a broad allow plus a few denies. When the decision model is on, it also sees whether the host is a private address (see Decision model).

## Approvals and saved rules

A turn that someone is watching asks that person before it runs a call the pipeline left undecided. The request is an _approval_. It names the tool and its arguments and waits for the answer. Approvals are rolling out with web chat: until web chat shows approval cards, web turns still deny an undecided call.

### The four answers

| Answer        | Effect                                                                |
| ------------- | --------------------------------------------------------------------- |
| Allow once    | The call runs. Nothing is saved.                                      |
| Allow always  | The call runs and an allow rule is saved for you in this space.       |
| Reject once   | The call does not run. The LLM is told you denied it.                 |
| Reject always | The call does not run and a deny rule is saved for you in this space. |

An approval that nobody answers expires after `interaction_timeout_secs` (default 10 minutes, range `30..=3600`). The call does not run and the LLM is told the request expired. The CLI prompt offers only the once answers.

### When "always" is withheld

The card offers only the once answers when saving a rule would be too broad or is not yours to grant:

- The command contains a shell metacharacter (`|`, `;`, `&`, `>`, `` ` ``, `$(`, and so on).
- It starts with a shell or interpreter (`sh`, `bash`, `zsh`, `python`, `node`, `ruby`, `perl`, and similar) or a wrapper that runs other commands (`env`, `xargs`, `eval`, `exec`, `nohup`, `time`, `watch`).
- It starts with a destructive command (`rm`, `mv`, `chmod`, `chown`, `dd`, `truncate`, `shred`).
- A configured `ask` rule produced the approval. That rule means a human decides every time.

Otherwise the saved rule is narrow. `git status --short` saves `bash(git status*)`. `npm -s run build` saves `bash(npm -s run*)`. `web-fetch` saves the scheme and host, and every other tool saves its bare name. The server derives the rule from the stored request. The client never sends a pattern.

### What a saved rule covers

A saved rule belongs to one user in one space. It applies to every turn that carries your identity in that space, whatever the interface. That includes your scheduled tasks and your messages from Slack or Matrix, which never prompt: a saved allow lets a matching call run unattended, and a saved deny blocks it. A call that no saved rule matches is still denied on those turns. Other users, and you in another space, are not affected.

Saved rules never outrank configuration. A configured or built-in deny wins over a saved allow, and a configured `ask` rule still asks. The saved allow also uses the shell-metacharacter guard of allow rules, so `bash(git status*)` does not allow `git status; rm -rf x`.

You can list and revoke your saved rules through `/api/tool-permission-rules`. Space admins and org admins can list and revoke every member's rules in a space. They cannot create or edit a rule or answer a member's approval.

### Messenger interfaces

Slack, Matrix, Signal, Mattermost and Nextcloud turns cannot show an approval. They deny an undecided call as before, and they follow saved rules.

## Audit events

Each decision logs one `tracing` event at `info` level on the target `assistant::permission`, with the message `tool permission decision`.

| Field         | Value                                                                                                   |
| ------------- | ------------------------------------------------------------------------------------------------------- |
| `tool`        | Tool name.                                                                                              |
| `decider`     | Which stage decided. `static-rules`, `user-rules` or `decision-model`.                                  |
| `rule`        | The matching rule text, `workspace-roots` for confinement, the model id for `decision-model`, or empty. |
| `confidence`  | The lowest score from the decision model. Unset for static rules and when the model failed.             |
| `outcome`     | `allow`, `deny` or `ask`.                                                                               |
| `interactive` | Whether the turn could prompt a user.                                                                   |

`outcome` is the pipeline result before the human step. An `ask` event does not record whether the user then agreed or the call was denied for lack of a prompt. Arguments are not logged.

Each answer to an approval logs a second event, `tool approval answered`, on the same target. It carries `user`, `tool`, `interaction` (the approval id), `decision` and `rule_id` (empty unless a rule was saved).

The default console filter is `info`, so the events show up without extra setup. See `docs/opentelemetry.md` for `RUST_LOG`.

## MCP server

Tool calls that arrive over MCP (`tools/call`, including `install-skill`) pass the same policy, over stdio (`assistant mcp`) and over HTTP (`/mcp`). An MCP caller cannot be prompted, so a call no rule allows is refused. The refusal comes back as a tool result with `isError: true` and the reason as text. `run-prompt` runs a normal turn, so the tools it calls pass the orchestrator's check.

The call runs as the caller. Over stdio that is the local system identity in the `default` space. Over HTTP it is the user behind the token or API key, in the space the request selects, and the `[permissions]` of that space's org apply. A caller with the `viewer` role cannot call tools at all. See [MCP](mcp.md).
