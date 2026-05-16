# ADR 0008: Conversation Titling via Bus Consumer

**Status**: Accepted
**Date**: 2026-05-16

## Context

Before this ADR, conversations only acquired a "title" when the user's first
message reached a web-ui handler — and even then, the title was a 57-char
truncation of that message. Conversations created by Slack, Matrix,
Mattermost, Nextcloud, Signal, the CLI, MCP clients, and the scheduler all
lived in the database with `title = NULL` and rendered as **"Untitled"**.

We wanted a single mechanism that titles conversations from every interface
once enough material exists for the LLM to summarise. Two design axes
mattered:

1. **Where the titling logic runs** — inside the orchestrator (close to the
   turn) or as a separate consumer (decoupled).
2. **How to prevent retroactive retitling** — once a title is meaningful,
   the system must never change it.

## Decision

### D1: Title generation runs as a bus consumer of `turn.result`

A new `TitleGeneratorWorker` claims messages from
`bus_messages::topic::TURN_RESULT` and, for eligible conversations,
calls the LLM and writes the title via `ConversationStore::update_title`.

`turn.result` is already published by `crates/runtime/src/orchestrator/worker.rs`
on every successful turn from every interface, so subscribing here gives us
cross-interface coverage for free. The same approach is precedented by the
webhook dispatcher (and by `assistant-workflow`), which also fan out off
`turn.result`.

Rejected alternative: `tokio::spawn` inside the orchestrator (the pattern used
by `skill_learner`). Pros: direct access to `TurnContext`. Cons: doesn't
survive process restart, runs in-process only, couples title generation to
turn latency.

Trade-off accepted: one extra DB query per turn to re-fetch history. Negligible
against the LLM round-trip cost.

### D2: New `title_locked` column gates the worker

We added `conversations.title_locked BOOLEAN NOT NULL DEFAULT 0`. The worker
will never overwrite a row where `title_locked = 1`. The flag is set:

- by `ConversationStore::create_conversation` when an explicit title is provided,
- by `ConversationStore::update_title` on every call,
- by the migration `041_conversation_title_locked.sql`, which back-fills
  `title_locked = 1` for every pre-existing row with a non-NULL title.

Rejected alternative: use `title IS NOT NULL` as the lock. Cons: conflates
"explicitly set" with "auto-generated and locked", and complicates future
nullable-title flows.

### D3: Removed legacy auto-truncation and `"New Chat"` default

Old web-ui handlers performed a 57-char truncation of the first user message
in `send_message` and `quick_message`, and `POST /api/conversations` used
`"New Chat"` as a fallback title. All three were deleted:

- `send_message`: no title set, regardless of history depth.
- `quick_message`: creates the conversation with `title = NULL`.
- `create_conversation`: passes `body.title` through unchanged (no default).

The display layer already coerced `NULL` to `"Untitled"`, so the UX is
unchanged for the moment between conversation creation and the worker's
first title.

### D4: Threshold — `min_turns` plus long-first-message escape hatch

A conversation is eligible when **either**:

- `turn >= min_turns` (default `2`), OR
- `turn >= 1` and the first user message exceeds `long_first_message_chars`
  characters (default `200`).

Both are configurable in `[titling]`.

### D5: No retroactive retitling

Per discovery, retitling on scope drift was explicitly rejected. The
`title_locked` flag enforces this at the storage layer; the worker
skips every locked row even if the bus redelivers `turn.result`.

## Consequences

**Positive**:

- Every interface (web, Slack, Matrix, Mattermost, Nextcloud, Signal, CLI,
  MCP, scheduler, A2A) automatically gets titles — by construction.
- Title generation survives process restarts via the bus's at-least-once
  delivery.
- Title work does not affect turn latency.
- `[titling].enabled = false` cleanly disables titling org-wide for
  cost-sensitive deployments.

**Negative**:

- One LLM call per conversation (one-shot, never replays after lock).
- ~80 lines of new worker scaffolding for spawn/lifecycle.
- The worker is best-effort: after `MAX_TURN_REDELIVERIES` (10) of LLM
  failures, the conversation remains Untitled — operator must rename manually.

## Implementation

- `crates/runtime/src/title_generator.rs` — `TitleGeneratorWorker`,
  `generate_title`, `is_eligible`, `spawn_title_generator_worker`.
- `crates/storage/src/conversations.rs` — `ConversationRecord.title_locked`,
  `update_title` sets the lock.
- `migrations/041_conversation_title_locked.sql`.
- `crates/core/src/types.rs` — `TitlingConfig`, embedded on `AssistantConfig`.
- `crates/web-ui/src/main.rs`, `crates/interface-cli/src/main.rs` — spawn
  the worker alongside the orchestrator worker.

## References

- OpenSpec change: `openspec/changes/llm-conversation-titles/`
- Operator guide: `docs/operations/conversation-titling.md`
