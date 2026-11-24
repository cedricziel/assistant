# Tool permissions

Every tool call the LLM makes passes through a permission check before it runs. This page covers the `[permissions]` section of `~/.assistant/config.toml`, the order the checks run in, and how to write rules.

Subagent Processes use the same check as the main turn.

## What changed

Before this feature, tools ran without a check unless a tool asked for confirmation, and that check only applied to interactive turns with a callback registered. Now every call is checked.

With no `[permissions]` section, a call that no rule allows is denied on any turn that cannot prompt a user. Scheduled tasks, Slack, Matrix, the web UI and Subagent Processes all fall in that group. Only the interactive CLI turn can prompt. If a scheduled task used `bash` or `web-fetch` before, it fails until you add an allow rule for it.

## How a call is decided

Checks run in this order. The first one that decides wins.

1. **Deny rules.** Built-in hard-deny rules, then your `deny` rules. A match denies the call.
2. **Confinement.** For `file-read`, `file-write`, `file-edit` and `file-glob`, a path outside the workspace roots is denied. No allow rule overrides this. To open another directory, add it to `workspace_roots`.
3. **Ask rules.** Your `ask` rules. A match sends the call to a human.
4. **Allow rules.** Built-in allow rules, then your `allow` rules. A match allows the call.
5. **No rule matched.** The call goes to a human.

"Goes to a human" resolves like this:

- On an interactive turn, the CLI prompts you. Your answer decides.
- On any other turn, the call is denied. The reason goes back to the LLM as the tool result, for example `Permission denied for 'bash': approval is required but this turn cannot prompt a user (...). Ask the owner to add an allow rule for this call.`

Only the CLI REPL turn is interactive.

The final outcome is `allow`, `deny` or `ask`. `ask` is the label for "a human decides, or denial if nobody can be asked".

Config rules add to the built-in rules. They cannot remove a built-in deny.

### Coming next

A decision model, pinned to one version, will be added in a follow-up. It will run between allow rules and the human fallback, and can only approve a call or pass it on. Deny stays a static-rule decision. Until then there is no `decision_model` key, and `[permissions]` rejects unknown keys.

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

| Key               | Type          | Meaning                                                        |
| ----------------- | ------------- | -------------------------------------------------------------- |
| `deny`            | list of rules | Always deny.                                                   |
| `ask`             | list of rules | Always send to a human.                                        |
| `allow`           | list of rules | Allow, unless a deny, confinement or ask check fires first.    |
| `workspace_roots` | list of paths | Directories file tools may use, besides the Persona workspace. |

A rule that does not parse, such as `bash(oops`, stops the config from loading, and the error names the rule. An unknown key under `[permissions]` does too.

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

Before a path is matched or checked, it is resolved: `~/` is expanded, relative paths are made absolute, `..` is applied, and symlinks are followed. A symlink inside the workspace that points outside it resolves to the outside location and is denied. A path that does not exist yet keeps its missing parts as written, so `file-write` to a new file works.

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

### Allow

These tools run without a rule of yours:

- `memory-get`, `memory-search`, `memory-append`
- `list-skills`, `load-skill`
- `list-tasks`, `schedule-task`, `cancel-task`
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

Rules match the URL string as the LLM wrote it. A hostname that resolves to a blocked address, or a redirect to one, does not match a deny rule on the IP. Prefer narrow allow rules over a broad allow plus a few denies.

## Audit events

Each decision logs one `tracing` event at `info` level on the target `assistant::permission`, with the message `tool permission decision`.

| Field         | Value                                                                |
| ------------- | -------------------------------------------------------------------- |
| `tool`        | Tool name.                                                           |
| `decider`     | Which stage decided. Today always `static-rules`.                    |
| `rule`        | The matching rule text, `workspace-roots` for confinement, or empty. |
| `confidence`  | Unset until a decision model exists.                                 |
| `outcome`     | `allow`, `deny` or `ask`.                                            |
| `interactive` | Whether the turn could prompt a user.                                |

`outcome` is the pipeline result before the human step. An `ask` event does not record whether the user then agreed or the call was denied for lack of a prompt. Arguments are not logged.

The default console filter is `info`, so the events show up without extra setup. See `docs/opentelemetry.md` for `RUST_LOG`.

## Known gap

The MCP server entry point (`crates/mcp-server`) calls the tool executor directly. It does not go through this check, so none of the rules on this page apply to calls that arrive over MCP.
