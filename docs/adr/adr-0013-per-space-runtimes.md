# ADR 0013: Every Space Has Its Own Runtime

**Status**: Accepted
**Date**: 2026-11-25
**Builds on**: [ADR-0007](adr-0007-multi-user-orgs.md)

## Context

[ADR-0007](adr-0007-multi-user-orgs.md) gave every Space its own database
(`orgs/{slug}/spaces/{space}/space.db`). The processes still opened one
database at startup, chosen by `--db-path` or `[storage] db_path`, and built
one orchestrator, tool executor, skill registry and message bus on it. Every
request and every background worker ran against that one Space, whatever
Space the caller belonged to.

## Decision

**Each Space gets its own runtime, built on that Space's database. The single
startup database is removed.**

- `SpaceRuntime` (`crates/runtime/src/space_runtime.rs`) bundles a skill
  registry, tool executor, message bus and orchestrator bound to one Space's
  `StorageLayer`. `SpaceRuntimes` builds them lazily per (org slug, space
  slug) and caches them, building each one once even under concurrent
  requests. The LLM provider, config, confirmation callback, audio store and
  MCP tools are shared by all Spaces. MCP tools are registered in every
  Space, including Spaces built later.
- `ScopedMessageBus` (`crates/storage/src/scoped_bus.rs`) prefixes topics
  with `{org}.{space}`, so Spaces sharing one NATS broker do not see each
  other's messages. The SQLite bus lives in each Space's own database and
  needs no prefix.
- `SpaceSupervisor` (`crates/runtime/src/space_supervisor.rs`) lists every
  Space in `org.db`, builds its runtime and runs a caller-supplied spawner
  once per Space. It reconciles every 30 seconds: it starts tasks for new
  Spaces, and stops tasks and evicts the runtime of deleted Spaces.
- The web server (`assistant webui serve`) holds `SpaceRuntimes` in
  `ApiState`. Data-plane handlers resolve the caller's Space through the
  `SpaceCtx` extractor (`crates/web-ui/src/space.rs`) and run on that
  Space's runtime. The workflow runner, schedule adapter and event adapter
  run once per Space; assistant-turn actions use that Space's orchestrator as
  `AuthContext::system_in(org, space)`.
- The orchestrator service (`assistant orchestrator run`) supervises every
  Space and runs the worker plan (turn workers, scheduler, title generator,
  memory indexer) once per Space. A messenger interface runs in the Space its
  interface instance in `org.db` (`interface_instances.space_id`) is bound
  to. If several instances of one type exist, the oldest wins. With no
  instance it runs in the default Space (`default/default`). The REPL, MCP
  server and `worker` roles serve the default Space.
- The public workflow webhook URL carries the Space:
  `/workflow-hooks/{org_slug}/{space_slug}/{id}/{token}`.
- `--db-path` and `[storage] db_path` are removed. The install root is
  always `~/.assistant`. `/ready` checks `org.db`.

There is no backward compatibility: old webhook URLs stop working, and a
custom database path can no longer be configured.

## Consequences

**Positive**

- A turn, schedule or workflow in one Space reads and writes only that
  Space's data.
- New Spaces get workers within one reconcile interval, without a restart.
- All Spaces can share one NATS broker.

**Negative**

- Memory and open file handles grow with the number of Spaces, since each
  active Space holds a runtime and, in the orchestrator service, its own
  workers.
- Operators must re-copy workflow webhook URLs.

**Known gaps**

These are still process-global, not per Space:

- Persona workspace files under `~/.assistant/agents/<id>/`, including
  `HEARTBEAT.md` and memory files.
- The `AgentStore` A2A Profile directory.
- The active Persona id (`PersonaApiState.agent_id`): one value per process.
- The install-wide `{root}/skills` directory. Only the skill registry
  metadata is per Space; the skill files on disk are shared by all Spaces.
