## Context

The assistant has multiple user-facing interfaces: a CLI REPL, a Flutter web UI, and five messenger adapters (Slack, Mattermost, Matrix, Nextcloud, Signal). Today, the CLI has six hardcoded commands (`/skills`, `/review`, `/install`, `/model`, `/help`, `/quit`) implemented as string matching in `main.rs`. Messenger interfaces have zero command support — every message goes straight to the orchestrator.

The `ChannelRunner` in `assistant-runtime` drives all messenger adapters through a shared dispatch loop: receive message → resolve conversation → acquire per-conversation lock → dispatch to orchestrator. This is the natural interception point for commands.

The orchestrator already has per-turn `CancellationToken` support (used by `submit_turn` timeout logic) and a full context compaction engine (`compaction.rs`) with LLM-driven chunked summarization. Both can be leveraged directly.

## Goals / Non-Goals

**Goals:**

- Unified command dispatch: all interfaces share the same command definitions and execution path.
- 6 built-in commands: `/new`, `/stop`, `/model`, `/compact`, `/status`, `/help`.
- Commands are visible to users in the conversation timeline but never included in LLM context.
- Per-conversation config overrides (initially: model selection).
- Slack-style autocomplete in the Flutter web UI.
- REST API for command listing and execution.

**Non-Goals:**

- Plugin/skill-registered custom commands.
- Inline directives (e.g. `/think high explain X`).
- Command authorization or per-user permissions.
- Command history or analytics.

## Decisions

### 1. Command types live in `assistant-core`, implementations in `assistant-runtime`

`CommandDef` (name, description, args schema, category) and `CommandResult` (ack text, side effects enum) are defined in `assistant-core` so they're available to all crates. The `CommandRegistry` struct and built-in command implementations live in `assistant-runtime` alongside `ChannelRunner` and the orchestrator, where they have access to conversation state, the LLM, and storage.

**Alternative considered:** Putting everything in `assistant-core`. Rejected because command execution needs `Orchestrator`, `ConversationStore`, and `LlmProvider` — pulling those into core would invert the dependency graph.

### 2. Interception in `ChannelRunner::run()` before dispatch

When a `ChannelMessage` with `ChannelContent::Text(t)` arrives and `t` starts with `/`, the runner routes it to `CommandRegistry::execute()` instead of `dispatch()`. The command result's `ack_text` is sent back via `adapter.send()`.

```
stream.next() → msg
  │
  ├─ text starts with "/" → CommandRegistry::execute(cmd, args, ctx)
  │   └─ adapter.send(ack_text)
  │
  └─ otherwise → existing dispatch path (resolve conv, lock, orchestrator)
```

For the CLI, the existing `strip_prefix('/')` dispatch in `main.rs` is replaced with a call to the same `CommandRegistry`. This eliminates the hardcoded match block and gives the CLI all commands automatically.

**Alternative considered:** Adding a `ChannelAdapter::on_command()` hook so each adapter handles commands differently. Rejected because the whole point is uniform behavior — adapter-specific rendering (thread replies, reactions) is already handled by `adapter.send()`.

### 3. Locking semantics per command

Commands fall into three categories based on their concurrency needs:

| Category  | Commands                           | Behavior                                                                                     |
| --------- | ---------------------------------- | -------------------------------------------------------------------------------------------- |
| Immediate | `/new`, `/stop`, `/model`, `/help` | No lock needed. Execute instantly, even while a turn is running.                             |
| Read-only | `/status`                          | No lock. Reads conversation state without mutation.                                          |
| Mutating  | `/compact`                         | Acquires the per-conversation lock. Waits for any in-flight turn to finish before executing. |

`/stop` is the critical case — it must execute while a turn is in progress. It finds the active `CancellationToken` for the conversation's current turn and cancels it. The orchestrator already checks this token between tool-call iterations (`worker.rs:265`).

**Implementation detail for `/stop`:** The `ChannelRunner` needs to track which `request_id` is active for each conversation. A new `Arc<RwLock<HashMap<Uuid, Uuid>>>` maps `conv_id → active_request_id`. The request ID is looked up in `Orchestrator::turn_cancellations` to find the token to cancel.

### 4. Per-conversation config via `ConversationConfig`

A new struct stored in-memory (in the `ChannelRunner` or `Orchestrator`) and optionally persisted to a `conversation_config` table:

```rust
pub struct ConversationConfig {
    pub model_override: Option<String>,
    // Future: reasoning_level, persona_override, etc.
}
```

When the orchestrator builds a turn, it checks for a conversation-level model override before falling back to the global config. The override is set by `/model` and cleared by `/new`.

**Alternative considered:** Storing overrides in the existing `conversations` table as a JSON column. Rejected because conversation records are currently minimal (id, created_at) and managed by `ConversationStore` — adding config there mixes concerns. A separate lightweight map keeps things clean.

**Persistence decision:** `ConversationConfig` is stored in-memory only for now (in a `HashMap<Uuid, ConversationConfig>` on the `CommandRegistry` or `ChannelRunner`). If the process restarts, overrides reset to defaults. This is acceptable for an initial implementation — persisting to SQLite is a future enhancement.

### 5. `command_events` table for durable command records

```sql
CREATE TABLE command_events (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    command         TEXT,
    payload         TEXT,
    ack_text        TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);
CREATE INDEX idx_command_events_conv ON command_events(conversation_id);
```

Command invocations are stored here, never in the `messages` table. The web UI timeline merges both tables by `created_at` for rendering. The orchestrator's `prepare_history` never queries this table — commands are invisible to the model.

### 6. REST API endpoints

**`GET /api/commands`** — returns the command registry for frontend autocomplete.

```json
[
  {
    "name": "new",
    "description": "Start a new conversation",
    "category": "session",
    "args": []
  },
  {
    "name": "model",
    "description": "Switch the model for this conversation",
    "category": "config",
    "args": [
      {
        "name": "model_name",
        "required": true,
        "completions_endpoint": "/api/models"
      }
    ]
  }
]
```

**`POST /api/conversations/{id}/command`** — execute a command.

Request: `ExecuteCommandRequest { command: String, args: Vec<String> }`
Response: `CommandEventResponse { id: Uuid, event_type: String, command: String, payload: Value, ack_text: String, created_at: DateTime }`

Returns 200 on success, 400 if command is unknown or args are invalid.

**`GET /api/conversations/{id}/events`** — list events for timeline rendering.

Returns a bare JSON array of `CommandEventResponse` sorted by `created_at`.

### 7. Flutter autocomplete popup

Triggered when the user types `/` as the first character in the input field. The popup:

- Fetches `GET /api/commands` (cached after first load).
- Filters commands as the user types (e.g. `/mo` shows only `/model`).
- For commands with `completions_endpoint`, fetches argument completions when the user starts typing the argument.
- Selecting a command with no required args submits immediately.
- Selecting a command with args fills the input and waits for the user to complete.
- Pressing Escape or Backspace past `/` dismisses the popup.

Timeline rendering: events from `GET /api/conversations/{id}/events` are interleaved with messages by timestamp and rendered with a distinct visual style (system-event appearance, not a chat bubble).

### 8. `/compact` triggers existing compaction engine directly

The existing `maybe_compact()` function in `compaction.rs` always performs compaction when called — the threshold check lives in `should_compact()`, which callers invoke before calling `maybe_compact()`. The `/compact` command simply calls `maybe_compact()` directly, bypassing the `should_compact()` guard:

```rust
// Normal turn path (threshold-gated):
if compaction::should_compact(&history, cfg) {
    compaction::maybe_compact(&mut history, llm, cfg, conv_store).await;
}

// /compact command (always runs):
compaction::maybe_compact(&mut history, llm, cfg, conv_store).await;
```

No changes to the `maybe_compact()` signature are needed. All other logic (chunking, persistence) remains unchanged.

## Risks / Trade-offs

**[`/stop` may not halt tool execution immediately]** → The cancellation token is checked between tool-call iterations, not during a tool's execution. A long-running tool (e.g. `bash` with a slow command) won't be interrupted mid-execution. Mitigation: acceptable for v1 — the user sees "Stopped" immediately and the turn is discarded when the current tool finishes.

**[`/compact` blocks on conversation lock]** → If a turn is running, `/compact` waits until it finishes. The user sees "Compacting..." but nothing happens until the lock is released. Mitigation: send an immediate ack ("Compaction queued, waiting for current turn to finish...") and run compaction in a background task.

**[`/new` in threaded platforms is ambiguous]** → In Slack, a conversation maps to a channel/thread. `/new` could mean "forget context in this thread" or "start a new thread." Mitigation: `/new` always means "forget context" — it evicts the conversation key from the LRU cache so the next message gets a fresh UUID. It does not create new platform threads.

**[In-memory `ConversationConfig` lost on restart]** → Model overrides reset when the process restarts. Mitigation: acceptable for v1. Users can re-issue `/model` after restart. Persistence can be added later.

**[Command parsing edge cases]** → Messages like `/new-york pizza` should not be treated as commands. Mitigation: only exact command name matches (after splitting on whitespace) are dispatched. `/new-york` does not match `/new`.
