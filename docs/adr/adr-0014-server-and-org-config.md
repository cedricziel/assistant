# ADR 0014: Server and Org Configuration Files

**Status**: Accepted
**Date**: 2026-11-25
**Builds on**: [ADR-0007](adr-0007-multi-user-orgs.md), [ADR-0013](adr-0013-per-space-runtimes.md)

## Context

[ADR-0013](adr-0013-per-space-runtimes.md) gave every Space its own runtime,
but every runtime still shared one `AssistantConfig` read from a single
`~/.assistant/config.toml`, and one LLM provider built from it. Two orgs on
one server could not use different LLM providers, memory settings or
messenger credentials.

## Decision

**Configuration is split by scope into two files. `config.toml` is no longer
read.**

- `~/.assistant/server.toml` is the `ServerConfig`
  (`crates/core/src/types/agent.rs`). It holds what the whole process shares:
  `[bus]`, `[mcp]` and `[notifications]` (Web Push keys). It is read once at
  startup.
- `~/.assistant/orgs/{slug}/org.toml` is the `OrgConfig`. It holds what a
  tenant controls: `[llm]`, `[agent]`, `[skills]`, `[observability]`,
  `[memory]`, `[compaction]`, `[learning]`, `[titling]`, `[permissions]`, the
  messenger sections and `[transcription]` / `[tts]`.
- `load_server_config` and `load_org_config` (`crates/core/src/config.rs`)
  return defaults for a missing file and an error naming the file for one
  that cannot be read or parsed. Org slugs are validated like the slugs in
  `OrgPoolFactory`.
- `OrgContexts` (`crates/runtime/src/org_context.rs`) loads an org's
  `org.toml` and builds its LLM client the first time one of its Spaces is
  used, and caches both. `SpaceRuntimes` takes its `OrgContexts` instead of a
  config and an LLM provider, so each `SpaceRuntime` carries the `OrgConfig`
  and LLM client of its own org. Command-line overrides (`--llm-model`,
  `--persona`, …) are applied to every org as it loads.
- `assistant doctor` checks every `server.toml` and `orgs/*/org.toml`.
  `assistant backup` archives them.
- The unused `[self_improvement]` (`MirrorConfig`) section is removed.

The REPL, one-shot commands, messenger interfaces, and the web UI's voice
providers and A2A agent card read the `default` org. Making those per-org is
left open.

There is no backward compatibility: there is no fallback to `config.toml`,
no compatibility read and no migration of the old file.

## Consequences

**Positive**

- Two orgs can use different LLM providers, models and settings on one
  server.
- Each file holds one scope, so an org's file can be handed to that org's
  admin without exposing the bus or MCP configuration.

**Negative**

- Operators with an old `config.toml` must split it by hand
  (`docs/operations/configuration.md`).
- A malformed `org.toml` now fails the first use of that org instead of
  silently falling back to defaults.
