# Configuration

This page lists the configuration files the `assistant` server reads, what
each one controls, and how to check them. It is for operators.

Configuration is split in two. `server.toml` holds settings for the whole
process. Each organization has its own `org.toml` for everything else. A
missing file means defaults.

## File layout

All paths are relative to `~/.assistant/`.

| File                       | Scope             | Sections                                                                                                                                                                                                                                               |
| -------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `server.toml`              | The whole process | `[bus]`, `[mcp]`, `[notifications]`                                                                                                                                                                                                                    |
| `orgs/<org-slug>/org.toml` | One organization  | `[llm]` (including `[llm.embeddings]`), `[agent]`, `[skills]`, `[observability]`, `[memory]`, `[compaction]`, `[learning]`, `[titling]`, `[permissions]`, `[signal]`, `[slack]`, `[mattermost]`, `[nextcloud]`, `[matrix]`, `[transcription]`, `[tts]` |

What the `server.toml` sections do:

- `[bus]` selects the message bus backend, `sqlite` or `nats`.
- `[mcp]` lists external MCP servers. Their tools are registered in every space. A server name must not start with `org_`, `space_` or `user_`: those prefixes belong to integration tools, and the file fails to load otherwise.
- `[notifications]` holds the Web Push VAPID keys. The web UI generates them on
  first start and writes them back to this file.

`[agent]` in `org.toml` sets the Persona id. `[observability]` has
`trace_content`.

Org slugs may contain only letters, digits, `-` and `_`. Anything else is
rejected.

Annotated examples are `server.toml` and `org.toml` at the repo root. Release
archives and packages include them, and Docker images put them at
`/etc/assistant/server.toml.example` and `/etc/assistant/org.toml.example`.

### Embeddings

`[llm] embedding_model` is optional. Unset, the chat provider's default
applies: Ollama `nomic-embed-text`, OpenAI `text-embedding-3-small`,
OpenRouter `openai/text-embedding-3-small`.

`[llm.embeddings]` picks a separate embedding provider: `ollama`,
`openai`, `openrouter` or `voyage`, with optional `model`, `base_url` and
`api_key`. See [OpenRouter embeddings](../openrouter.md#embeddings).

## How files are loaded

- The server reads `server.toml` once at startup.
- The server reads an org's `org.toml` the first time a space of that org is
  used, then caches it until the process exits. Restart to pick up edits.
- A space takes its LLM provider and settings from the org it belongs to. Two
  orgs can use different providers.
- A file that cannot be read or parsed is an error that names the file. For
  `server.toml` startup fails. For `org.toml` the first use of that org fails.

The first org is `default`. The `assistant` CLI REPL, one-shot commands and the
messenger interfaces run under the `default` org.

Two web UI settings are still read from the `default` org only, not per org:
the process-wide voice providers (`[transcription]`, `[tts]`) and the
Persona behind the A2A agent card.

## Overrides

These flags override the matching setting of every org:

| Flag             | Command           | Overrides            |
| ---------------- | ----------------- | -------------------- |
| `--llm-provider` | web UI            | `[llm]` provider     |
| `--llm-model`    | web UI            | `[llm]` model        |
| `--llm-base-url` | web UI            | `[llm]` base URL     |
| `--agent`        | web UI            | `[agent]` Persona id |
| `--persona`      | `assistant` (CLI) | `[agent]` Persona id |

## Checking your configuration

```sh
assistant doctor
```

The `Configuration` check parses `server.toml` and every `orgs/*/org.toml`.
It reports each file that fails, with the parse error. If it finds no config
files, it warns that the server is using defaults.

`assistant backup` archives `server.toml` and every `orgs/*/org.toml`.

## Moving from config.toml

Earlier releases read one `~/.assistant/config.toml`. The server no longer
reads it. There is no fallback and no automatic migration, and a leftover file
is ignored. Split it by hand:

1. Copy `[bus]`, `[mcp]` and `[notifications]` into `~/.assistant/server.toml`.
2. Copy everything else into `~/.assistant/orgs/default/org.toml`.
3. Delete `~/.assistant/config.toml`.
4. Run `assistant doctor` and restart the server.

The `[self_improvement]` section is gone. It had no effect, so drop it.

## See also

- `docs/backup.md` for `assistant backup` and `assistant restore`.
- `docs/adr/adr-0007-multi-user-orgs.md` for the org and space model.
