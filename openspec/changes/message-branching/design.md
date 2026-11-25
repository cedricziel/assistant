# Design: message-branching

## Context

What exists today:

- **Schema.** `messages(id, conversation_id, role, content, skill_name, turn, created_at, tool_calls_json, sender_user_id)` (`migrations/001_conversations.sql:10`, `005_tool_calls.sql`, `040_message_sender.sql`). `conversation_id` cascades on delete (`001_conversations.sql:12`). Nothing links a message to its predecessor. `PRAGMA foreign_keys=ON` is set at startup (`crates/storage/src/lib.rs:277`). Space migrations are registered by hand in the list at `crates/storage/src/lib.rs:297-474`.
- **Ordering and `turn`.** `load_history` sorts by `turn ASC, created_at ASC` (`crates/storage/src/conversations.rs:465`). `prepare_history` sets `base_turn = prior.len()`, the row count (`crates/runtime/src/orchestrator/mod.rs:1554`), and saves the user row with `turn = base_turn` (`mod.rs:1562-1565`). Every row of one loop iteration (tool calls, tool results, thinking, final text) gets the same `turn = base_turn + iteration + 1` (`mod.rs:966`, `977`, `1117`, `1331`; `dispatch.rs:60-80` takes the shared turn). So `turn` is an iteration index, not a depth, and it is not unique.
- **Save sites.** The user message (`mod.rs:1565`), thinking rows (`mod.rs:1120`, `1459`), tool-call rows (`dispatch.rs:99`), tool results (`dispatch.rs:252`, `443`, `turn_control.rs:186`) and final text (`mod.rs:1336`, `turn_control.rs:49`, `72`). `save_message` is two statements without a transaction, and its INSERT is `ON CONFLICT(id) DO NOTHING` (`conversations.rs:413-450`).
- **Compaction is destructive.** `persist_compacted` (`crates/runtime/src/compaction.rs:355`) calls `replace_history` (`conversations.rs:369-386`). That runs `DELETE FROM messages WHERE conversation_id = ?` and then `save_message` for each kept row. A persistence failure is only logged (`compaction.rs:430`). `/status` and `/compact` read raw `load_history` (`crates/runtime/src/command_registry.rs:274-310`). As a side effect, the web UI loses earlier messages after compaction (`crates/web-ui/src/api/conversations.rs:420`).
- **Model selection.** One `LlmConfig` per org (`crates/core/src/types/llm.rs:97`). `/model` stores a per-conversation override. `Orchestrator::llm_for` (`mod.rs:480`) builds a provider for a model name with the org's provider kind, endpoint and key. No endpoint lists models.
- **Web turns.** `send_message` (`crates/web-ui/src/api/messages.rs:111`) calls `submit_turn_with_request_id` (`worker.rs:701`, called at `messages.rs:163`), which calls `submit_turn_internal` (`worker.rs:727`, eight positional arguments). It emits `run_started {run_id}` at seq 0 before the turn runs (`messages.rs:185-209`), then drives a ~300-line SSE, persistence and push loop inline. 400 and 403 responses are plain strings (`messages.rs:118-131`), and so is `owned_conversation`'s 404 (`conversations.rs:50-62`). An envelope helper exists: `json_error` (`crates/web-ui/src/errors.rs:25`).
- **Redelivery.** A transient LLM error nacks the `TurnRequest`, and the worker re-runs the whole turn (`worker.rs:427-458`). `prepare_history` saves a new user row on every run, so a redelivered append already duplicates the user message today.
- **In-flight tracking.** Three unrelated `conversation_id → request_id` maps: `ApiState.active_turns` (`crates/web-ui/src/api/mod.rs:152`, read by `/stop` and `/status` at `command_registry.rs:195`, `262`, but never written by `send_message`, so web `/stop` never finds a turn), `ChannelRunner.active_turns` (`crates/runtime/src/channel_runner.rs:64`, written at `427`, `438`) and the CLI's (`crates/interface-cli/src/main.rs:355`). Scheduler turns publish `TurnRequest` straight to the bus (`crates/runtime/src/scheduler/tasks.rs:206-230`) and bypass `submit_turn_internal`. Several services may share one database (`worker.rs:96-102`), and the bus may be NATS.
- **No orchestrator `TurnContext`.** The only `struct TurnContext` is the skill learner's (`crates/runtime/src/skill_learner.rs:24`). `orchestrator-turn-context` is an unimplemented spec. This change does not depend on it.
- **Other history readers.** The title generator (`title_generator.rs:306`), the conversation indexer (`conversation_indexer.rs:88`), `/status` and `/compact` (`command_registry.rs:274-310`).

## Goals / Non-Goals

**Goals:** a message tree with one active path per conversation; LLM context built only from that path; edit, regenerate and regenerate-with-model; a branch picker; existing interfaces unchanged; turns that stay correct under redelivery and cancellation.

**Non-Goals:** cross-provider regenerate, branching from messengers, branch deletion, editing messages with attachments, editing assistant messages, a cross-process in-flight guard.

## Decisions

### D1. Adjacency list plus an active leaf pointer

`messages.parent_id TEXT NULL REFERENCES messages(id)`, with no `ON DELETE` action, and an index on `(conversation_id, parent_id)`. `conversations.active_leaf_id TEXT NULL`, with no FK. A `NULL` parent marks a root. Editing the first message creates a second root, and roots are siblings of each other. The active path is the ancestor chain of `active_leaf_id`, loaded with one recursive CTE.

There is no self-referencing cascade. SQLite counts FK actions against `SQLITE_MAX_TRIGGER_DEPTH` (default 1000), so a cascade along a long chain could make `delete_conversation` fail. It would also add nothing: the `conversation_id` cascade already deletes every row in one statement, and SQLite checks a non-deferred FK at the end of that statement, by which point parents and children are both gone. Branch deletion is a non-goal. A contract test deletes a conversation with a 1,500-row chain.

`messages.run_id TEXT NULL` records the turn run that wrote a row: the request id when present, otherwise the bus message id. Redelivery cleanup (D3a) and the cancelled-reply race guard (D12) both use it.

| Alternative                                                         | Why not                                                                                                                                               |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Copy-on-branch: duplicate the shared prefix into a new conversation | Branches no longer belong to one conversation. Titles, list entries and attachments get duplicated, and the picker would have to cross conversations. |
| `branch_id` on every message plus a `branches` table                | Every regenerate has to rewrite or duplicate descendants to tag them. Two concepts to keep consistent instead of one.                                 |
| An `active_child_id` per node (ChatGPT-style memory of each fork)   | More writes on every switch, and leaf resolution needs a second walk. Kept as a follow-up (see Open Questions).                                       |

The migration backfills existing rows as one chain: `parent_id` is set to the previous row by `(turn, created_at, rowid)` with `LAG()` per conversation, and `active_leaf_id` to each conversation's last row. The backfill updates only `parent_id`, so the `AFTER UPDATE OF content` FTS triggers from `conversation-list-management` PR 5 do not fire. There is no legacy-shape fallback.

### D2. `save_message` appends at the active leaf atomically

`save_message(&msg)` runs in one `BEGIN IMMEDIATE` transaction. It reads `active_leaf_id`, inserts the row with `parent_id = active_leaf_id`, and only when the insert affected a row does it set `active_leaf_id = msg.id` and bump `updated_at`. `BEGIN IMMEDIATE` takes the write lock up front. A deferred `BEGIN` would upgrade from reader to writer and fail with `SQLITE_BUSY` when a messenger turn and a web turn write at the same time. The existing `busy_timeout = 5000` (`lib.rs:283`) then makes the second writer wait.

**Idempotency:** an insert that hits `ON CONFLICT(id) DO NOTHING` changes nothing, the leaf included. So a re-run that re-saves a row with a pre-assigned id cannot move the leaf back and orphan later rows.

`branch_message(&msg, parent_id: Option<Uuid>)` does the same, but under an explicit parent (`None` means a new root).

**Alternative:** thread an explicit "current tip" to every save site. Rejected: it touches about ten call sites for behaviour the store can guarantee by itself, and a forgotten site would silently corrupt the tree.

### D3. Anchored turns

`TurnRequest` (`crates/core/src/bus_messages.rs:60`) gains:

```rust
#[serde(default)] anchor: TurnAnchor,          // Append | Edit { parent_message_id: Option<Uuid> } | Regenerate { user_message_id: Uuid }
#[serde(default)] user_message_id: Option<Uuid>, // the id the run will persist its user row under (Append, Edit)
#[serde(default)] model_override: Option<String>,
```

Messenger, CLI and scheduler producers stay unchanged, because they deserialize to `Append`.

- **Append.** The user row is saved with `save_message` and the turn appends.
- **Edit `U` (parent `P`).** The new user row `U'` is saved with `branch_message(U', P)`, and the turn appends. The leaf is on `U'` from then on. If the turn then fails, the path ends at `U'`, just as a failed send ends at its user row.
- **Regenerate `U`.** No user row is saved. The turn's first persisted row uses `branch_message(row, Some(U))`, and the rest append. If the turn fails before persisting anything, `active_leaf_id` never moved and the old reply is still shown.

**History is the anchor's ancestor chain**, not "the active path up to `U`". Edit builds context from `P`'s chain plus `U'`. Regenerate builds it from `U`'s chain (including `U` and its attachments). So an edit or regenerate of a message that is not on the active path is well defined: it branches from where that message lives, and the new branch becomes active.

**Plumbing without a `TurnContext`.** `prepare_history` (`mod.rs:1535`) already returns the store the turn saves through. It now returns a `TurnStore`, a small wrapper around the `SqliteConversationStore` that implements `ConversationStore`. It carries the turn's `run_id`, which it stamps on every row, and an optional pending branch parent. The first `save_message` through a wrapper with a pending parent becomes `branch_message(row, Some(parent))` and clears the parent. The run loops already pass this value to every save site, and the `dispatch.rs`/`turn_control.rs` helpers take `&dyn ConversationStore`, so no signature beyond `prepare_history`'s return type changes. Submitting goes through a `SubmitTurn` options struct that replaces the positional `submit_turn_*` variants, so no new `too_many_arguments` allow is needed. `SubmitTurn` also absorbs `interactive` from `tool-approval-prompts` PR 6.

**Alternative:** an explicit `TurnContext` flag. Rejected for now: there is no orchestrator `TurnContext`, and extracting one is a refactor of its own. The `TurnStore` wrapper can move into a `TurnContext` when `orchestrator-turn-context` lands.

#### D3a. Redelivery is idempotent

A nacked turn re-runs from the start (`worker.rs:441-458`). Before the first run, the worker fixes the run's identity: `run_id` is the request id (`msg.batch_id`) or else the bus message id, both stable across `nack_delayed`. `user_message_id` is assigned if the producer did not set one. On every run, `prepare_history` then:

1. **Discards the run's partial rows.** It deletes every row with this `run_id` except the user row, in one statement, and resets the leaf to the anchor point (`U` for Append and Regenerate, `U'` for Edit). If the delete fails, for example because another interface appended under a partial row, it logs a warning and only resets the leaf.
2. **Re-saves the user row by its pre-assigned id.** For Append and Edit, a conflicting insert is a no-op that does not move the leaf (D2). For Regenerate, there is no user row to save.
3. **Re-arms the pending branch parent** for Regenerate.

So a re-delivered Regenerate leaves exactly one new reply sibling, a re-delivered Edit leaves one `U'`, and a re-delivered Append no longer duplicates the user message.

### D4. `turn` is the iteration index along a path

`turn` keeps today's meaning: the user row gets `base_turn`, and every row of iteration `i` gets `base_turn + i + 1`. Rows from one iteration share it, and along one path `(turn, tool_name)` is unique. `base_turn` is the number of rows strictly before the turn's user row on its path:

| Anchor          | `base_turn`                                                                                               |
| --------------- | --------------------------------------------------------------------------------------------------------- |
| Append          | length of the active path                                                                                 |
| Edit (parent P) | length of P's ancestor chain, including P (0 when P is `None`)                                            |
| Regenerate U    | length of U's ancestor chain, excluding U, so the reply rows get the same turns as the reply they replace |

It is computed from the raw path, never from the summary-applied history (D5). The detail handler's tool-result join moves to `tool_call_id` and ordinal in `tool-result-renderers` S4, which lands before PR 11 here, so branching does not depend on `(turn, tool_name)` uniqueness. **Alternative:** drop `turn`. Rejected as out of scope: titling eligibility and the indexer's `session://…/turn_N` keys depend on it.

### D5. Summaries keep branches intact, through one history builder

New table `conversation_summaries(id, conversation_id, through_message_id, content, created_at)`, behind a `ConversationSummaryStore` trait with Sqlite and InMemory impls. `maybe_compact` writes a row instead of calling `replace_history`.

One builder, `build_turn_history(conversation_id, leaf) -> TurnHistory { history, row_ids }`, is used by `prepare_history`, `/status` and `/compact`. It loads the path to `leaf`, takes the newest summary whose `through_message_id` is on that path, emits it as the `[Conversation summary — earlier context compacted]` assistant text (`compaction.rs:416`), and follows it with the path rows after `through_message_id`. So `/status` estimates tokens over what the model actually sees, and `/compact` re-summarises from the existing summary rather than the whole raw path. A summary covering messages that a later edit branched away from is not on the new path, so it does not apply.

Mapping the in-memory split index back to a message id uses `row_ids: Vec<Option<Uuid>>`, a shadow of `history` built next to `messages_to_chat_history` (`crates/runtime/src/history.rs:29`) and extended at every in-turn push through one helper. If no id resolves, no summary is persisted and the in-memory compaction still applies, as on today's failure path (`compaction.rs:430`).

**Rollout:** PR 1 makes `replace_history` delete the rows and set `active_leaf_id = NULL` in one transaction before it re-inserts. Without that reset, the first re-insert would take a deleted row as its parent, which is an FK violation, and the CTE would start from a dead leaf and return an empty history. PR 5 then deletes `replace_history` outright.

**Alternatives:** keep `replace_history` (rejected: it destroys every branch); store the summary as a tree node (rejected: it would show in the UI and shift `turn`).

### D6. REST surface

All routes follow `openspec/config.yaml`. Errors use the `{"error": …}` envelope through `json_error` (`crates/web-ui/src/errors.rs:25`). PR 11 adds an envelope-returning variant of `owned_conversation` for the new handlers. Existing handlers keep the plain-text one until a later cleanup.

| Route                                                           | operationId                  | Access / scope                          | Body                                  | Response                                         |
| --------------------------------------------------------------- | ---------------------------- | --------------------------------------- | ------------------------------------- | ------------------------------------------------ |
| `POST /api/conversations/{id}/messages/{message_id}/edit`       | `edit_message`               | `Access::Write` + `conversations:write` | `EditMessageRequest { message }`      | SSE, same frames as `send_message`               |
| `POST /api/conversations/{id}/messages/{message_id}/regenerate` | `regenerate_message`         | `Access::Write` + `conversations:write` | `RegenerateMessageRequest { model? }` | SSE                                              |
| `POST /api/conversations/{id}/switch-branch`                    | `switch_conversation_branch` | `Access::Write` + `conversations:write` | `SwitchBranchRequest { message_id }`  | 200 `ConversationDetail`                         |
| `GET /api/models`                                               | `list_models`                | `Access::Read`                          | —                                     | 200 bare `ModelSummary[]` (`name`, `is_default`) |

Validation: an invalid UUID or empty message returns 400; editing a non-user message returns 400; a `model` not in the list returns 400; a missing conversation or message, or a message in another conversation, returns 404; a member without write access, or a token without `conversations:write`, gets 403; a conversation with a turn in flight returns 409 (D7). An unauthenticated request gets 401 from the existing middleware.

For regenerate, `message_id` can be a user message or any row of a reply. For a reply row, the server walks up to the nearest user ancestor.

For switch-branch, the new leaf is found by descending from `message_id` to the **most recently created child** at each level. Switching does not bump `updated_at`. It does broadcast `ConversationEvent::Upserted`, so other tabs reload.

### D7. One orchestrator-owned turn registry

A `TurnRegistry` on `Orchestrator` replaces `ApiState.active_turns`, `ChannelRunner.active_turns` and the CLI map. `CommandContext.active_turns` becomes a handle to it, which also fixes web `/stop` and `/status`. It maps `conversation_id → { request_ids, exclusive }` and hands out leases:

- `try_begin(conv, request_id, Shared | Exclusive) -> Option<TurnLease>` checks and inserts under one lock, so it is atomic. Two rapid edits cannot both pass. Shared is refused while an exclusive lease is held. Exclusive is refused while any lease is held.
- `TurnLease` releases on `Drop`, so every exit path (success, error, cancel, panic unwinding, a dropped handler future) clears it. The map sits behind a `std::sync::Mutex` that is never held across an `.await`, so `Drop` can release it synchronously. This deliberately departs from the "tokio `RwLock`" default.

**Who acquires:**

- **Web handlers** acquire before submitting and move the lease into the spawned submit task. `send_message` takes Shared and returns 409 only while an exclusive lease is held. Edit and regenerate take Exclusive for the whole turn. Switch-branch takes Exclusive for the duration of the database call.
- **The worker** takes a Shared lease for every turn it claims whose `request_id` is not already registered. That covers the scheduler, which bypasses `submit_turn_internal`, and messenger turns. If the conversation holds an exclusive lease, the worker defers the message with `nack_delayed` instead of running it.

**Limitation (documented):** the registry is per process. With several services sharing a database, or a NATS bus with workers in other processes, a branch operation in one process does not see a turn running in another. D2's transactions keep the tree consistent in that case, but a concurrent append can land on the branch the user just left. A cross-process lease is out of scope.

### D8. Edit, regenerate and send share one runner

`send_message`'s post-`run_started` loop is extracted once, in PR 6, as `stream_turn_to_sse(…, RunOptions)`. `chat-stop-cancels-turn` (voice), `tool-approval-prompts` PR 7 and PR 12, `follow-up-suggestions` PR 3 and `chat-a11y-and-code-blocks` S3 edit the runner, not `send_message`. `RunOptions` collects per-entry-point policy: `push`, `interactive`, `suggestions` and `anchor`. To avoid dead fields, each field is added by the first PR that reads it. `interactive` comes from `tool-approval-prompts` PR 6, `suggestions` from `follow-up-suggestions` PR 3, and `anchor` from PR 12 here.

| Entry point          | `interactive` | `suggestions` | `push`   | `anchor`   |
| -------------------- | ------------- | ------------- | -------- | ---------- |
| `send_message`       | true          | true          | true     | Append     |
| `edit_message`       | true          | true          | true     | Edit       |
| `regenerate_message` | true          | true          | true     | Regenerate |
| voice                | false         | false         | as today | Append     |

`interactive` is copied onto `TurnRequest.interactive`, the single flag that `tool-approval-prompts` and `generative-ui-a2ui` both read.

**No new SSE event types.** The `run_started` payload of text, edit and regenerate runs gains `user_message_id`. This is **the id the run will persist its user row under** (pre-assigned for Append and Edit), or the existing `U` for Regenerate. It is sent before the worker has saved anything, so a run that fails before `prepare_history` may never persist it (D13). It is written at seq 0, so replay carries it. After the terminal frame, the client reloads `GET /api/conversations/{id}` for the authoritative tree. Unknown fields are ignored on both sides. **Alternative:** a `branch_created` event. Rejected: it duplicates data one GET provides and needs replay coverage.

### D9. The server projects branch metadata

Each `MessageSummary` gains `parent_id` (omitted for roots), `model` (assistant rows only) and `branch: BranchInfo { index, count, sibling_ids }`, present only when `count > 1`:

- **User rows** carry their sibling user messages (edit versions) as `branch`.
- **Reply versions** are the children of a user message. A reply's first row is often a hidden thinking or tool-call row, so the server copies that row's `BranchInfo` onto the **last visible assistant row** of the reply (non-empty `content`, or attachments).
- **Fallback:** if the active reply has no visible assistant row (only tool-call or thinking rows, or a cancelled reply with no text), the reply's `BranchInfo` is carried on the user row as `reply_branch`. The client renders that picker under the user bubble, so the user can always switch back.

`sibling_ids` lists first-row ids, which is enough because switch-branch descends from any of them. After `tool-result-renderers` S4, tool rows join by `tool_call_id` and ordinal, not `(turn, tool_name)`.

### D10. Regenerate with a model: a one-turn override and an allowlist

`RegenerateMessageRequest.model` must be `llm.model` or an entry of the new `[llm] available_models: Vec<String>` (default empty). It travels as `TurnRequest.model_override`. In `llm_for`, the turn override wins over the `/model` conversation override, which wins over the org default. The override is not written to `conversation_configs`. The producing model is stored in `messages.model` through the `persist_final_answer(FinalAnswerMeta { traceparent, model, timing })` helper that `message-feedback` PR 3 introduces. This change adds `model` to it, as `chat-a11y-and-code-blocks` S2 adds `timing`, so no PR edits every save site. The model name reaches the helper through `TurnStore`, not through new parameters.

**Alternative:** free-text model names, as `/model` accepts. Rejected for the web UI: a mistyped name fails only after the turn starts, and an allowlist lets admins limit cost.

### D11. Consumers that stay linear

- **Messengers, CLI, scheduler, quick-message:** `Append`, linear `load_history`. If a web user switches branches, the messenger's next turn continues the selected branch. This is intended and documented.
- **Title generator:** reads the path. The title locks after the first success.
- **Conversation indexer:** indexes the path under `session://…/turn_N`. Because a regenerate reuses the replaced reply's turns (D4), it re-indexes under the same keys. Chunks from abandoned deeper branches stay behind (Risks).
- **Event log:** keyed by `run_id`, unaffected.

### D12. Cancellation of branch turns

`chat-stop-cancels-turn` PR 2 adds `Orchestrator::persist_cancelled_reply`, which the runner calls in its `Cancelled` branch after the turn future has been dropped. PR 13 here makes it anchor-aware. The runner passes `{ run_id, anchor }`. If the run has not persisted any row, a Regenerate's partial reply is saved with `branch_message(row, Some(U))`. Otherwise it appends to the run's own leaf. The race guard ("the final answer is already saved") checks only rows with this `run_id`, so it no longer sees the old reply of a regenerate. A cancel that leaves no text and no rows leaves the leaf where it was.

### D13. Client retry

A failed user bubble with a server id (from `run_started.user_message_id`) is retried by **regenerate** whenever replay does not end in `done`: an `agent_error`, a 404 or a 410. If regenerate returns 404, because the run failed before saving the user row, the client falls back to `sendMessage`. A bubble without a server id re-sends as today. A failed edit is retried the same way, by regenerating `U'`.

## Risks / Trade-offs

- **Recursive CTE cost** on long paths: O(depth) over the primary key. A contract test with a 1,000-row chain guards load time, and a 1,500-row chain guards deletion.
- **The compaction id shadow can drift** if a future push skips the helper. Mitigation: one helper for push and save, and a test asserting `history.len() == row_ids.len()` after a tool-call iteration.
- **Per-process registry** (D7). Accepted and documented.
- **Contended files.** The `Message` struct and `conversations.rs` follow this order: PR 1 here, `tool-result-renderers` S3, `message-feedback` PR 3, then PR 10 here. Save sites change only through `persist_final_answer`. `conversation-list-management` PR 8 clears `archived_at` in `branch_message` too, and depends on PR 2 here. `worker.rs` follows `chat-stop-cancels-turn` PR 1, then `tool-approval-prompts` PR 2 and PR 6, then PRs 7–9 here. The client stream loop follows `chat-stop-cancels-turn` PR 4 and PR 5, then `follow-up-suggestions` PR 7, then PR 18 here.
- **Generated artefacts and baselines.** Never hand-merge `openapi.json`, `app/packages/assistant_api` or Playwright PNGs. Rebase, then rerun `make dump-openapi && make generate-flutter-client` and regenerate baselines with the e2e-testing skill, recording the diff.
- **Stale index chunks** from abandoned deeper branches can surface in `memory-search`. Accepted for v1.
- **Playwright baselines move twice:** when compaction stops hiding messages (PR 5) and when the picker and actions land (PR 20).

## Migration Plan

Two forward-only SQLite migrations in `migrations/`, each registered in the `run_migrations` list (`crates/storage/src/lib.rs:297`):

- `message_tree`: `parent_id`, `run_id`, `active_leaf_id`, the index and the backfill.
- `conversation_summaries` (PR 3) and `message_model` (`messages.model`, PR 10).

Numbers come from the cross-change allocation in the stack plan. Without one, use the next free number after `045_skill_executions.sql` at implementation time. Rollback means restoring a backup. Rows that compaction already deleted are not reconstructed.

## Open Questions

1. **Model list source.** Should a curated `[llm] available_models` allowlist (proposed) gate the models users can pick, or should any model name be accepted, as `/model` does? Should regenerate-with-model be limited to space admins?
2. **Which leaf a switch lands on.** The proposal descends to the most recently created child. The alternative is to remember the last-viewed leaf of each fork (`active_child_id`), as ChatGPT does: more writes, but no surprise jumps in nested forks.
3. **Editing messages with attachments.** v1 hides Edit for them. Carrying the attachments forward would need a many-to-many link.
