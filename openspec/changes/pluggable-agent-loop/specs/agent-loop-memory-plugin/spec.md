## ADDED Requirements

### Requirement: MemoryPlugin injects persistent memory files into context via transform_context

`MemoryPlugin` SHALL implement the `Plugin` trait and wrap `MemoryLoader` (from `assistant-core`) to inject the assembled system-prompt content — SOUL.md, IDENTITY.md, USER.md, MEMORY.md, AGENTS.md, TOOLS.md, and daily notes — as a `System`-role `ChatHistoryMessage` prepended to the message list on every turn. `MemoryLoader` has no heavy dependencies (pure filesystem I/O in `assistant-core`), so `MemoryPlugin` SHALL live in the base `assistant-agent-loop` crate with no feature flag.

#### Scenario: Memory content prepended before LLM call

- **WHEN** `MemoryPlugin` is registered and memory files exist on disk
- **THEN** `transform_context` prepends a `System`-role message containing the assembled memory content before the LLM call

#### Scenario: No memory files — context unchanged

- **WHEN** `MemoryLoader::load_system_prompt()` returns an empty string (no files present)
- **THEN** `transform_context` returns the message list unmodified; no empty `System` message is prepended

#### Scenario: Memory content respects existing size caps

- **WHEN** individual memory files exceed `BOOTSTRAP_MAX_CHARS_PER_FILE` or the total exceeds `BOOTSTRAP_MAX_CHARS_TOTAL`
- **THEN** the assembled content is truncated by `MemoryLoader` exactly as today; `MemoryPlugin` does not add additional truncation logic

### Requirement: MemoryPlugin runs the BOOT.md startup hook on session start

`MemoryPlugin::on_session_start` SHALL read BOOT.md from the configured path (if it exists and is non-empty after stripping HTML comments) and run it as a turn against the loop — replicating the existing `Orchestrator::run_boot` behaviour. Failure is non-fatal: errors SHALL be logged at `warn` level and the session SHALL proceed.

#### Scenario: BOOT.md hook executes at session start

- **WHEN** `MemoryPlugin` is registered and BOOT.md exists with non-empty content
- **THEN** `on_session_start` submits BOOT.md content as a system turn before any user input is processed

#### Scenario: Missing BOOT.md is silently skipped

- **WHEN** BOOT.md does not exist at the configured path
- **THEN** `on_session_start` completes without error and without running any turn

#### Scenario: BOOT.md failure does not abort the session

- **WHEN** the BOOT.md turn fails (e.g. LLM error)
- **THEN** a `warn!` is logged and the session continues normally

### Requirement: MemoryPlugin handles BOOTSTRAP.md self-deletion on first run

`MemoryPlugin::on_session_start` SHALL check whether BOOTSTRAP.md exists. If present, it is included in the assembled memory content for that session (handled by `MemoryLoader`) and then deleted from disk, so it is only shown once. This replicates the existing self-deleting onboarding behaviour.

#### Scenario: BOOTSTRAP.md included and deleted on first session

- **WHEN** BOOTSTRAP.md exists and `on_session_start` runs
- **THEN** BOOTSTRAP.md content appears in the system prompt for that session AND the file is deleted so it is not included in subsequent sessions

#### Scenario: Absent BOOTSTRAP.md has no effect

- **WHEN** BOOTSTRAP.md does not exist
- **THEN** `on_session_start` proceeds without attempting deletion

### Requirement: MemoryPlugin is constructed with a MemoryLoader

`MemoryPlugin::new(loader: MemoryLoader) -> Self` SHALL be the constructor. `MemoryLoader` is constructed from `AssistantConfig` or `MemoryConfig` by the caller before being passed in.

#### Scenario: Plugin works without storage or LLM

- **WHEN** `MemoryPlugin` is registered without `StoragePlugin` or any LLM-backed component
- **THEN** memory files are loaded and injected correctly — no database or embedding access is required

### Requirement: MemoryIndexer remains a background task independent of the plugin pipeline

`MemoryIndexer` (chunking and embedding memory files into SQLite) SHALL NOT be converted to a plugin. It runs as a `tokio::spawn` background task at application startup, unchanged from today's behaviour, and is relocated from `crates/runtime/src/memory_indexer/` to `crates/storage/src/memory_indexer/`. The plugin pipeline has no dependency on the indexer.

#### Scenario: MemoryPlugin and MemoryIndexer operate independently

- **WHEN** both `MemoryPlugin` and the background `MemoryIndexer` are running
- **THEN** they operate on the same files without coordination — `MemoryPlugin` reads files synchronously per-turn; `MemoryIndexer` chunks and embeds asynchronously in the background
