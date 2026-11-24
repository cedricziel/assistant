# AGENTS.md — tool-executor

## Adding a Builtin Tool

1. Create `crates/tool-executor/src/builtins/<name>.rs` with a handler struct.
2. Implement `#[async_trait] impl ToolHandler` -- 4 required methods: `name()`, `description()`,
   `params_schema()`, `run()`. Optionally override `is_mutating()`, `requires_confirmation()`,
   or `output_schema()`.
3. Export from `crates/tool-executor/src/builtins/mod.rs`.
4. Register in `ToolExecutor::register_builtins()` in `executor.rs`.
5. Optionally add `skills/<name>/SKILL.md` for documentation.
