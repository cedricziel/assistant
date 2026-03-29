# Quickstart: Skill Management

**Feature**: 003-skill-management
**Date**: 2026-03-29

## For implementers

### Key files to touch

| File                                                 | Change                                                                                                             |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `migrations/028_skill_body.sql`                      | New: add `body_text` to `skills`                                                                                   |
| `migrations/029_persona_skill_access.sql`            | New: `skill_access_mode` on personas + `persona_skill_list` table                                                  |
| `crates/storage/src/lib.rs`                          | Register migrations 028 + 029                                                                                      |
| `crates/storage/src/registry.rs`                     | Extend: persist `body_text`; add `create_user_skill`, `update_user_skill`, `delete_user_skill`, `list_for_persona` |
| `crates/storage/src/persona_skill_access.rs`         | New: `PersonaSkillAccessStore`                                                                                     |
| `crates/storage/src/personas.rs`                     | Extend: `skill_access_mode` in `PersonaRecord`; expose `set_skill_mode`                                            |
| `crates/storage/src/lib.rs`                          | Expose `PersonaSkillAccessStore` convenience constructor                                                           |
| `crates/interface-cli/src/main.rs`                   | Add `Command::Skill` and `PersonaCommand` extensions                                                               |
| `crates/web-ui/src/skills/mod.rs`                    | New: skill CRUD router + handlers                                                                                  |
| `crates/web-ui/src/skills/pages.rs`                  | New: Askama template handlers                                                                                      |
| `crates/web-ui/src/main.rs`                          | Add `registry: Arc<SkillRegistry>` to `AppState`; wire skills router                                               |
| `crates/web-ui/templates/skills/`                    | New: list, show, new, edit HTML templates                                                                          |
| `crates/web-ui/templates/personas/skill_access.html` | New: persona skill access page                                                                                     |
| `skills/agentskills-spec/SKILL.md`                   | New: embedded agentskills.io spec builtin                                                                          |

### Build / test commands

```sh
# Full build
make build

# Fast check (no codegen)
make check

# Unit tests
cargo test -p assistant-storage
cargo test -p assistant-web-ui
cargo test -p assistant-cli

# Lint + format (required before commit)
make lint && make format

# Run web UI locally
ASSISTANT_WEB_TOKEN=dev assistant webui serve --listen 127.0.0.1:8080 --llm-provider anthropic
```

### Key patterns to follow

**New migration**: copy the pattern from `crates/storage/src/lib.rs` migrations slice. File goes in `migrations/<NNN>_<name>.sql`. Register in the `migrations` array in order.

**New storage struct**: follow `crates/storage/src/personas.rs` — `pub struct FooStore { pool: SqlitePool }`, `impl FooStore { pub fn new(pool: SqlitePool) -> Self }`, async methods returning `anyhow::Result<_>`. Expose convenience constructor from `StorageLayer`.

**New web UI page**: follow `crates/web-ui/src/a2a/pages.rs` — define state struct, Askama template structs, handlers, router fn. Register router in `main.rs` under `protected_routes`.

**CLI subcommand**: follow existing `PersonaCommand` pattern in `crates/interface-cli/src/main.rs`.

### Persona skill filtering integration point

`SkillRegistry::list_for_persona(persona_id: &str, pool: &SqlitePool)` should be called by the Orchestrator's skill-loading path (in `crates/runtime`) when assembling the system prompt context. Search for where `registry.list()` is called in `crates/runtime` to find the integration point.
