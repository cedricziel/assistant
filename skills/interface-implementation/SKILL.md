---
name: interface-implementation
description: >
  Checklist and architectural rules for implementing new assistant interfaces
  (CLI, Slack, web, etc.). Every interface MUST use the Orchestrator to ensure
  consistent behaviour: system prompt, tools, skills, memory, and the full
  ReAct loop.  Use this skill when adding, reviewing, or fixing an interface.
license: MIT
metadata:
  tier: docs
  mutating: "false"
  confirmation-required: "false"
---

# Interface Implementation Guide

All assistant interfaces **MUST** route user messages through the
`Orchestrator` (`assistant-runtime`). Direct `LlmProvider::chat_streaming()`
calls bypass the system prompt, tools, skills, memory, and ReAct loop —
producing a "dumb chatbot" instead of the full assistant.

## Architecture invariant

```
User message
  -> Interface (transport layer)
    -> Orchestrator.submit_turn()        <-- REQUIRED
      -> Worker dispatches to one of:
         run_turn()                      (default)
         run_turn_streaming()            (with token sink)
         run_turn_with_tools()           (with extension tools)
      -> System prompt (MemoryLoader)
      -> Tool specs (ToolExecutor + extensions)
      -> Skills XML (SkillRegistry)
      -> ReAct loop (multi-iteration tool calling)
    <- TurnResult { answer, attachments }
  <- Interface renders / delivers response
```

An interface that calls `llm.chat_streaming()` directly violates this
invariant and MUST be fixed.

---

## Checklist for a new interface

### 1. Add an `Interface` variant

In `crates/core/src/types.rs`, add a variant to the `Interface` enum:

```rust
pub enum Interface {
    Cli,
    Signal,
    Mcp,
    Slack,
    Mattermost,
    Web,           // <-- new
    Scheduler,
}
```

### 2. Bootstrap the full dependency chain

Every interface needs these components, in order:

```
AssistantConfig        (load from ~/.assistant/config.toml)
  -> StorageLayer      (SQLite, migrations)
  -> SkillRegistry     (load embedded + dir-scanned skills)
  -> LlmProvider       (Ollama / Anthropic / OpenAI)
  -> ToolExecutor      (storage, llm, registry, config)
  -> MessageBus        (storage.message_bus())
  -> Orchestrator      (llm, storage, executor, registry, bus, config)
  -> executor.set_subagent_runner(orchestrator)   // break init cycle
```

The `assistant_runtime::bootstrap` module provides shared helpers:

- `load_config(path)` — loads config TOML
- `skill_dirs(config, project_root)` — returns skill search directories
- `AutoDenyConfirmation` — for non-interactive interfaces

### 3. Spawn the worker

The orchestrator's bus-based processing requires a background worker:

```rust
let worker_orch = orchestrator.clone();
tokio::spawn(async move {
    worker_orch.run_worker("web-worker").await;
});
```

Without this, `submit_turn()` will publish to the bus but nothing will
claim and process the request.

### 4. Manage conversations

Each logical conversation needs a stable `Uuid`:

| Interface  | Key                             | Strategy                    |
| ---------- | ------------------------------- | --------------------------- |
| CLI        | session                         | `Uuid::new_v4()` at startup |
| Slack      | `(channel_id, thread_ts)`       | HashMap                     |
| Mattermost | `(channel_id, root_post_id)`    | LRU cache (10k)             |
| Signal     | sender phone number             | HashMap                     |
| Web UI     | conversation UUID from database | `ConversationStore` rows    |

The orchestrator creates the conversation record lazily inside
`prepare_history()` via `create_conversation_with_id()` (upsert).

### 5. Choose a delivery mode

| Mode                | Registration call             | Worker method           | Use case                  |
| ------------------- | ----------------------------- | ----------------------- | ------------------------- |
| **Streaming**       | `register_token_sink(id, tx)` | `run_turn_streaming()`  | CLI, Signal, Web UI (SSE) |
| **Extension tools** | `register_extensions(id, ..)` | `run_turn_with_tools()` | Slack, Mattermost         |
| **Fire-and-forget** | (none)                        | `run_turn()`            | Scheduler, MCP            |

For web interfaces, streaming via `register_token_sink` is the natural
fit — pipe tokens to SSE events.

### 6. Submit the turn

```rust
// For streaming:
orchestrator.register_token_sink(conversation_id, token_tx).await;

// For extension tools:
orchestrator.register_extensions(conversation_id, tools, attachments).await;

// Then submit (always):
orchestrator.submit_turn(&user_text, conversation_id, Interface::Web).await?;
```

`submit_turn()` publishes to the message bus and blocks (with a 10-minute
timeout) until the worker publishes a `TurnResult`.

### 7. Render the result

The `TurnResult` contains:

- `answer: String` — the final text to show the user
- `attachments: Vec<Attachment>` — any files produced

For streaming interfaces, tokens arrive via the `mpsc::Receiver<String>`
in real time. The `TurnResult.answer` is the authoritative final text
(use it for persistence, not the concatenated tokens).

### 8. Run BOOT.md on startup (optional)

```rust
orchestrator.run_boot(conversation_id, Interface::Web).await?;
```

This reads `~/.assistant/BOOT.md` and submits it as a silent turn if
non-empty. Useful for per-session initialization tasks.

---

## What the orchestrator provides (that raw LLM calls do not)

| Feature                      | Orchestrator | Raw `chat_streaming()` |
| ---------------------------- | :----------: | :--------------------: |
| System prompt (MemoryLoader) |     Yes      |           No           |
| AGENTS.md, SOUL.md, etc.     |     Yes      |           No           |
| BOOTSTRAP.md (first-run)     |     Yes      |           No           |
| Skills XML catalog           |     Yes      |           No           |
| Tool specs + execution       |     Yes      |           No           |
| ReAct loop (multi-turn)      |     Yes      |           No           |
| History sanitization         |     Yes      |           No           |
| OTel tracing spans           |     Yes      |           No           |
| Confirmation gate            |     Yes      |           No           |
| Error recovery in history    |     Yes      |           No           |

---

## Anti-patterns

1. **Direct LLM call** — `llm.chat_streaming(SYSTEM_PROMPT, &history, &[], ...)`
   bypasses everything. The assistant won't know its identity, won't have
   tools, and can't execute skills.

2. **Hardcoded system prompt** — `"You are a helpful assistant."` instead
   of the MemoryLoader's composed prompt. The assistant loses its
   personality, workspace context, and memory.

3. **Empty tool list** — `&[]` as the tool spec. The assistant can't read
   files, run commands, search the web, or use any skill.

4. **Missing worker** — calling `submit_turn()` without a spawned
   `run_worker()` task causes a 10-minute timeout.

5. **Skipping `set_subagent_runner()`** — subagent spawning (the
   `agent-spawn` tool) will fail at runtime.

6. **Double-persisting user messages** — if the interface saves the user
   message to the database AND the orchestrator also saves it (via
   `prepare_history`), the conversation ends up with duplicate user
   entries. Let the orchestrator own persistence; the interface should
   only render the message for display.

7. **`StorageLayer` is not `Clone`** — wrap it in `Arc<StorageLayer>`
   early and share via `Arc::clone()`. The `pool: SqlitePool` inside
   it _is_ `Clone`, so you can still extract it for direct DB access
   (conversation listing, titling, etc.).

---

## Message persistence ownership

The orchestrator's `prepare_history()` saves the user message and the
ReAct loop saves all assistant / tool messages. Interfaces MUST NOT
duplicate this by saving user or assistant messages themselves.

For web interfaces where the user expects to see their message
immediately, render the HTML from the form data in memory — do not
round-trip through the database. Use a shared pending-message map
(`Arc<RwLock<HashMap<Uuid, String>>>`) to pass user text from the
message-send endpoint to the streaming endpoint.

---

## `parse_interface()` update

When adding a new `Interface` variant, also update `parse_interface()`
in `crates/runtime/src/orchestrator.rs`. This function deserialises
the variant from the message bus; a missing entry silently falls back
to `Interface::Cli`.

---

## Reference implementations

- **CLI** — `crates/interface-cli/src/main.rs` (streaming mode)
- **Slack** — `crates/interface-slack/src/runner.rs` (extension tools mode)
- **Mattermost** — `crates/interface-mattermost/src/runner.rs` (extension tools)
- **Signal** — `crates/interface-signal/src/runner.rs` (streaming mode)
- **Web UI** — `crates/web-ui/src/main.rs` + `crates/web-ui/src/chat/mod.rs` (streaming mode, SSE)

All five go through `orchestrator.submit_turn()`.
