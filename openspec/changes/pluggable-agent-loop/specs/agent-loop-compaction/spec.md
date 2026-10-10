## ADDED Requirements

### Requirement: Plugin trait gains compaction lifecycle hooks

The `Plugin` trait SHALL gain two compaction hooks (following pi-mono's `before_compact` / `compact` event pattern):

```rust
/// Called before the loop compacts the message history.
/// Return `CompactionOutcome::Cancel` to abort compaction (e.g. a plugin that
/// owns an external history store and wants to delay until its flush completes).
async fn before_compact(
    &self,
    ctx: &SessionContext,
    messages: &[ChatHistoryMessage],
) -> CompactionOutcome {
    CompactionOutcome::Proceed
}

/// Called after compaction has been applied. `summary` is the text that
/// replaced the compacted messages. Plugins use this to persist the summary
/// to storage, write it back to memory files, or emit a bus event.
async fn on_compact(
    &self,
    ctx: &SessionContext,
    summary: &str,
    retained_messages: &[ChatHistoryMessage],
) {
}
```

`CompactionOutcome` SHALL be an enum: `Proceed`, `Cancel`.

Default implementations SHALL be no-ops (`Proceed` for `before_compact`, empty body for `on_compact`).

#### Scenario: Plugin cancels compaction

- **WHEN** a plugin's `before_compact` returns `CompactionOutcome::Cancel`
- **THEN** the message list is returned unchanged and `on_compact` is NOT called on any plugin

#### Scenario: All plugins proceed — compaction fires

- **WHEN** all registered plugins return `CompactionOutcome::Proceed` from `before_compact`
- **THEN** compaction runs, message history is summarized, and `on_compact` is called on every plugin with the summary text and retained messages

---

### Requirement: CompactionPlugin implements context history compaction

`CompactionPlugin` SHALL be provided in the base `assistant-agent-loop` crate (no feature flag — `LlmProvider` is already a base dep). It summarizes old messages via the LLM when the estimated history size exceeds a configurable threshold.

```rust
pub struct CompactionPlugin {
    provider: Arc<dyn LlmProvider>,
    /// Maximum number of messages before compaction triggers.
    /// Default: 40.
    threshold_messages: usize,
    /// Number of recent messages to retain verbatim after compaction.
    /// Default: 10.
    keep_recent: usize,
}

impl CompactionPlugin {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self;
    pub fn with_threshold(mut self, threshold_messages: usize) -> Self;
    pub fn with_keep_recent(mut self, keep_recent: usize) -> Self;
}
```

`transform_context` SHALL:

1. Count `messages.len()`. If `len <= threshold_messages`, return unmodified.
2. Partition into `to_compact = messages[..len - keep_recent]` and `to_retain = messages[len - keep_recent..]`.
3. Call `plugins.before_compact(ctx, &to_compact)` — if any plugin returns `Cancel`, return messages unmodified.
4. Call the LLM with a fixed summarization prompt and `to_compact` as context; produce a single `System`-role summary message.
5. Call `plugins.on_compact(ctx, &summary_text, &to_retain)`.
6. Return `[summary_message] + to_retain`.

The summarization prompt is fixed: `"Summarize the following conversation history concisely, preserving all decisions, tool outputs, and key facts. Output plain text."` — not configurable by callers. Callers who need custom prompts implement their own plugin.

Compaction is also triggerable manually via a `compact` slash-command registered by `CompactionPlugin::tools()` (contributed via the `Plugin::tools()` method), allowing users to type `/compact` to force compaction at any point.

#### Scenario: History below threshold — no compaction

- **WHEN** `messages.len() <= threshold_messages`
- **THEN** `transform_context` returns messages unmodified; no LLM call is made

#### Scenario: History exceeds threshold — automatic compaction

- **WHEN** `messages.len() > threshold_messages`
- **THEN** old messages are summarized; returned list is `[System(summary)] + keep_recent messages`

#### Scenario: Manual /compact command

- **WHEN** the user sends `/compact` (or the LLM calls the `compact` tool)
- **THEN** compaction fires immediately regardless of current message count

#### Scenario: Compaction LLM failure is non-fatal

- **WHEN** the LLM call during compaction fails
- **THEN** a `warn!` is logged, the original uncompacted message list is returned, and the session continues

#### Scenario: StoragePlugin persists pre-compaction history

- **WHEN** `StoragePlugin` is registered alongside `CompactionPlugin`
- **THEN** full message history is persisted to `ConversationStore` before compaction replaces it in-context; no conversation history is permanently lost

---

### Requirement: AgentBus gains CompactionStarted and CompactionCompleted events

Two new `AgentEvent` variants SHALL be added:

```rust
CompactionStarted  { session_id: Uuid, messages_compacted: usize },
CompactionCompleted { session_id: Uuid, summary: String, messages_retained: usize },
```

`CompactionPlugin` emits `CompactionStarted` after `before_compact` passes (all `Proceed`) and `CompactionCompleted` after `on_compact` returns. Interface crates (web-ui, CLI) can display "Compacting context…" indicators.

#### Scenario: Web-ui shows compaction indicator

- **WHEN** `CompactionStarted` is received on the `AgentBus`
- **THEN** the SSE endpoint emits a `compact_start` event; the browser renders a "Compacting context…" indicator until `CompactionCompleted` arrives

---

### Requirement: MemoryPlugin compacts memory files when truncation is detected

`MemoryPlugin` SHALL optionally compact MEMORY.md and daily notes when `MemoryLoader::load_system_prompt()` produces truncated output (i.e., any file exceeded `BOOTSTRAP_MAX_CHARS_PER_FILE` or the total exceeded `BOOTSTRAP_MAX_CHARS_TOTAL`). This requires `Arc<dyn LlmProvider>` — it is opt-in via a builder method:

```rust
impl MemoryPlugin {
    /// Enable LLM-based memory file compaction when size caps are hit.
    /// Compaction runs in on_session_end if truncation was detected this session.
    pub fn with_compaction(mut self, provider: Arc<dyn LlmProvider>) -> Self;
}
```

When enabled, `on_session_end` SHALL:

1. Re-run `load_system_prompt()` with truncation detection. If no file was truncated, skip.
2. For each file that was truncated (MEMORY.md, daily notes), call the LLM to summarize the file content.
3. Write the compacted summary back via `MemoryLoader::update_file("memory", ..., "replace")` or the equivalent path.
4. Log at `info!` level with file name and old/new character counts.
5. Any LLM failure is non-fatal: log `warn!` and leave the file untouched.

Memory file compaction is NOT triggered on every session — only when truncation is detected. This prevents unnecessary LLM calls when memory files are within caps.

#### Scenario: MEMORY.md exceeds cap — compacted at session end

- **WHEN** MEMORY.md exceeds `BOOTSTRAP_MAX_CHARS_PER_FILE` during a session AND `with_compaction()` was set
- **THEN** `on_session_end` compacts MEMORY.md to a shorter summary via LLM and writes it back

#### Scenario: No truncation — no compaction call

- **WHEN** all memory files fit within caps
- **THEN** `on_session_end` skips compaction; no LLM call is made

---

### Requirement: MemoryPlugin supports session reflection (write-back)

`MemoryPlugin` SHALL optionally extract learnings from the session and persist them to memory files at `on_session_end`. This is the "distillation" path — the inverse of loading. Opt-in via builder:

```rust
impl MemoryPlugin {
    /// Enable LLM-based session reflection: after each session, extract key
    /// facts and append them to MEMORY.md and today's daily notes.
    pub fn with_reflection(mut self, provider: Arc<dyn LlmProvider>) -> Self;
}
```

When enabled, `on_session_end` SHALL:

1. Call the LLM with the full session message history and the fixed prompt:
   `"Extract key facts, decisions, and learnings from this session worth remembering. Be concise. Output as a bulleted markdown list."`
2. If the LLM returns non-empty output, append it to MEMORY.md via `MemoryLoader::update_file("memory", ..., "append")`.
3. Also append a timestamped entry to today's daily notes via `MemoryLoader::append_daily_note(Some("session"), ...)`.
4. LLM failure or empty output is non-fatal: log `warn!` and skip.

#### Scenario: Session ends with reflection enabled

- **WHEN** `with_reflection()` was set and the session had at least one user turn
- **THEN** key facts are extracted by LLM and appended to MEMORY.md and today's daily notes

#### Scenario: Empty session — no reflection

- **WHEN** the session had no user turns (e.g. only a BOOT.md execution)
- **THEN** reflection is skipped; no LLM call is made
