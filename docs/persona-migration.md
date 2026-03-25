# Persona Model Migration Guide

This release introduces a breaking terminology and runtime model migration:

- `Persona` is now the canonical long-lived assistant context term.
- `Subagent Process` is the canonical delegated execution term.
- `A2A Profile` is now Persona-attached configuration.

No backward-compatibility aliases are provided for the renamed CLI and web UI
surfaces listed below.

## Breaking surface changes

### CLI

- `assistant agent ...` -> `assistant persona ...`
- `--agent <id>` -> `--persona <id>`
- `ASSISTANT_AGENT` -> `ASSISTANT_PERSONA`

Examples:

```sh
# old (removed)
assistant agent list

# new
assistant persona list
assistant --persona marketing orchestrator run
```

### Web UI routes

- `/contexts` -> `/personas`
- `/contexts/{id}/use` -> `/personas/{id}/use`

If you have bookmarks, scripts, or reverse-proxy allowlists for the old route,
update them to `/personas`.

### Storage schema

- Table `assistant_agents` is renamed to `personas` via migration `026_personas`.

No manual SQL action is required during normal startup; the embedded migrations
apply automatically.

### A2A profile storage

A2A profile card files are now Persona-scoped:

- old shared location: `~/.assistant/agents/*.md`
- new Persona-scoped location: `~/.assistant/agents/<persona-id>/a2a-profiles/*.md`

## Operator cutover checklist

1. Pull and deploy the new binary.
2. Update CLI scripts from `agent`/`--agent` to `persona`/`--persona`.
3. Update environment files to use `ASSISTANT_PERSONA`.
4. Update Web UI bookmarks and automation from `/contexts` to `/personas`.
5. Verify startup runs migrations successfully.
6. If you use A2A profile cards, verify cards exist under the new Persona-scoped path.

## Verification commands

```sh
assistant persona list
assistant --persona default orchestrator run --no-repl
```

For web UI verification, open `/personas` and switch the active Persona for the
current session.
