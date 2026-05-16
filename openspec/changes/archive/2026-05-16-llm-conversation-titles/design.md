## Context

The current title situation, mapped during exploration:

- `Orchestrator::prepare_history` (runtime/orchestrator/mod.rs:1329) creates every conversation with `title = None`. Every interface goes through this path.
- Only three web-ui handlers ever set a title: `POST /api/conversations` (uses `body.title` or static `"New Chat"`), `POST /api/conversations/{id}/messages` (auto-truncates the user's first message to 57 chars when history is empty), and `POST /quick-message` (same truncation rule).
- Messenger interfaces (Slack/Mattermost/Matrix/Nextcloud/Signal), CLI, MCP, scheduler-driven conversations, and A2A all bypass these handlers and remain `title = NULL`. Read sites coerce NULL to the string `"Untitled"`.
- A durable, topic-based `MessageBus` already exists (`assistant-core::bus`, SQLite + NATS implementations). `topic::TURN_RESULT` is **already published from `orchestrator/worker.rs` on every successful and failed turn** (lines 306/332/491/516/551). Webhook dispatch and the workflow engine are already wired to it. The envelope (`TurnResult`) carries `conversation_id`, `turn`, `had_errors`, and `message_id`.
- A precedent for post-turn LLM evaluation exists in `runtime/src/skill_learner.rs` — it uses `tokio::spawn` inside the orchestrator with direct `TurnContext` access, gated behind `LearningConfig`.
- `ConversationStore::update_title` already emits `ConversationUpserted` on the in-memory `ConversationBroadcaster`, which the Flutter app consumes via SSE for reactive list updates.

## Goals / Non-Goals

**Goals:**

- Every conversation, regardless of originating interface, eventually gets an LLM-generated title once enough material exists.
- Title generation must survive process restarts (a crash mid-LLM-call must not leave a permanently-untitled conversation).
- Title generation must not block turn responses or be observable in turn latency.
- Manual rename via PATCH must never be overwritten by the worker.
- Idempotent on bus message redelivery — at-least-once delivery must not produce duplicate LLM calls or overwrites.

**Non-Goals:**

- Backfilling existing Untitled conversations (operator concern; can be addressed via a one-shot CLI subcommand later if desired).
- Retroactive retitling when conversation scope drifts.
- Per-user titling preferences (org-level config only).
- Streaming the title as it generates — the worker writes once and broadcasts the final value.

## Decisions

### D1: Bus consumer pattern over in-process spawn

**Chosen**: A new `TitleGeneratorWorker` in `crates/runtime/src/title_generator.rs` consumes `turn.result` from the `MessageBus` via `claim_filtered`, mirroring the pattern used by webhook dispatch and the workflow engine.

**Alternatives considered**:

- _`tokio::spawn` from the orchestrator (skill-learner pattern)_: simpler but doesn't survive restart, runs in-process only, and would not fire if the orchestrator process crashed after publishing `turn.result` but before the spawn dispatched. Skill-learner can tolerate this because skill creation is an opportunistic enhancement; missing a title leaves the conversation permanently Untitled.
- _Synchronous title generation inside `send_message`_: adds 500ms+ of LLM latency to the turn response. Rejected.

**Rationale**: `turn.result` is the canonical "a turn completed" event, already published from every interface. The bus already provides at-least-once delivery, claim filters, redelivery caps (`MAX_TURN_REDELIVERIES`), and SQLite/NATS backend portability. Re-fetching conversation history adds one DB query (~negligible vs the LLM call). The worker can run in a separate process in distributed mode.

### D2: `title_locked` column for idempotency and rename respect

**Chosen**: Add `title_locked BOOLEAN NOT NULL DEFAULT 0` to the `conversations` table. Set to `1` whenever any title is written — by the worker, by the explicit `PATCH /api/conversations/{id}` handler, or by an explicit title in `POST /api/conversations`. Worker skips any conversation where `title_locked = 1`.

**Alternatives considered**:

- _Use `title IS NOT NULL` as the lock signal_: simpler but conflates "explicitly set" with "auto-generated and locked", and would force the worker to skip conversations that received an empty/placeholder title via API.
- _Separate `auto_titled_at TIMESTAMP` column_: more data but no functional gain over a boolean.

**Rationale**: Explicit boolean is queryable for audit ("how many conversations were auto-titled?"), survives schema introspection, and decouples the lock semantics from any future change to NULL handling. Adding a single integer column to SQLite is cheap; existing rows default to `0`.

### D3: Removal of the 57-char truncation and `"New Chat"` default

**Chosen**: Delete the truncation logic in `send_message` (web-ui/api/mod.rs:879–895) and `quick_message` (mod.rs:1591–1595). Change `create_conversation` to pass `body.title` directly without the `"New Chat"` fallback, so `title` is `None` until the worker writes one. Read paths already coerce `NULL` → `"Untitled"` consistently.

**Alternatives considered**:

- _Keep truncation as a placeholder until the LLM title lands_: violates "no retroactive change" weakly (placeholder → real is arguably retroactive), introduces visual flicker, and duplicates the threshold logic in two places.
- _Keep `"New Chat"` as the initial title for web-created conversations_: it would block the worker (because `title_locked` would be set on create), so existing web conversations would never get LLM titles. Rejected.

**Rationale**: A clean `NULL → Untitled → <LLM title>` state machine is easier to reason about, prevents placeholder/lock interactions, and makes the system uniform across all interfaces.

### D4: Threshold — turn count plus long-first-message escape hatch

**Chosen**: The worker considers a conversation eligible when **either**:

- `turn >= min_turns` (default `2` — meaning after the second assistant message; user has spoken twice), OR
- `turn >= 1 AND first_user_message_length > long_first_message_chars` (default `200`).

Both thresholds configurable per-org in `[titling]`.

**Alternatives considered**:

- _Fixed message-count threshold (e.g., always after 4 messages)_: misses the "one long, specific question" case where a great title is already derivable.
- _LLM-based eligibility judge_: doubles the LLM cost; not worth it for v1.
- _Token-count threshold_: more accurate but requires running the tokenizer; turn count is a usable proxy.

### D5: LLM provider selection — reuse conversation's provider with optional override

**Chosen**: The worker uses the conversation's primary LLM provider for the title call. A per-org model override was considered but is **deferred** — `LlmProvider::chat` doesn't accept a model name parameter today, so honouring an override would require either threading it through every provider impl or building a duplicate provider instance at startup. Neither is justified by current evidence; revisit when an operator hits real cost pain.

**Rationale**: Keeps cost/latency predictable per org. Override exists for orgs who want to force a cheap model (Haiku, llama3:8b) while the main conversation runs on a larger model.

### D6: Error handling — fail-open, bounded retry

**Chosen**: On LLM error, return `nack_delayed` with exponential backoff (mirror the orchestrator's worker schedule). After `MAX_TURN_REDELIVERIES` (10, ~28 minutes of retry), the bus permanently fails the message. The conversation remains Untitled — operator can rename or, future, re-enqueue.

**Rationale**: Title generation is best-effort. We never want a hung title worker to block other downstream consumers of `turn.result` (webhook dispatch, workflow triggers).

## Risks / Trade-offs

- **Risk**: LLM provider outage during a busy period → many conversations stay Untitled, then all flood retries simultaneously.
  **Mitigation**: Bus redelivery backoff is per-message, not synchronised; the staggered turn arrival times naturally space retries.

- **Risk**: A very fast user sends 3+ turns before the LLM responds → worker processes the first `turn.result`, then `title_locked = 1`, so the second/third `turn.result` events are no-ops.
  **Mitigation**: That's the desired outcome. The eligibility check on `title_locked` is the natural deduplication.

- **Risk**: Title contains PII from the conversation (e.g., the user named a real person in a sensitive context).
  **Mitigation**: Same trust boundary as the LLM provider itself. Document in operator docs; orgs that need stronger guarantees can disable titling.

- **Risk**: The 200-char threshold for "long first message" is arbitrary and could be wrong in either direction.
  **Mitigation**: Configurable. Default chosen because typical "Hey can you help me…" greetings are <200 chars while substantive technical questions usually exceed it.

- **Trade-off**: Adding a column requires a migration. SQLite ALTER TABLE ADD COLUMN is cheap on the existing org/space DB layout, but it does mean every org DB migrates on next startup.

- **Trade-off**: Bus consumer plumbing is more code than `tokio::spawn`. ~80 lines of new worker scaffolding vs ~30 — paid back the first time the assistant crashes mid-title.

## Migration Plan

1. Migration `NNNN_add_title_locked.sql` adds the column with `DEFAULT 0`. Existing rows get `0`, so the worker will consider them eligible — but D3 below prevents that.
2. **Existing conversations with non-null titles**: a single `UPDATE conversations SET title_locked = 1 WHERE title IS NOT NULL` inside the migration locks all currently-titled conversations. The worker will not touch them.
3. **Existing conversations with NULL titles**: stay unlocked. They will _not_ be retroactively titled by the worker because the worker only consumes new `turn.result` events from the bus — it does not scan the database. (If the operator wants to backfill, that's a future `assistant migrate retitle` subcommand — out of scope.)
4. Rollback: revert the binary. The column is harmless on older binaries (they ignore it). If the migration must be reversed, `ALTER TABLE conversations DROP COLUMN title_locked` is safe but unnecessary.

## Open Questions

- Should the worker include the assistant's first response in the prompt, or just the user message(s)? Including the assistant response gives better titles for "answer-y" conversations but adds tokens. **Tentative: include up to ~500 tokens of recent messages.** Confirm during implementation by spot-checking title quality.
- Should `[titling]` config also support a per-persona override? Probably yes, but defer to a follow-up — getting org-level right first.
- Naming: `title_locked` vs `auto_title_locked` vs `title_frozen`. `title_locked` reads cleanest. Open to bikeshed.
