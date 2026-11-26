# ADR 0018: Conversations Are a Message Tree With One Active Path

**Status**: Accepted
**Date**: 2026-11-26
**Builds on**: [ADR-0009](adr-0009-testability-architecture.md), [ADR-0011](adr-0011-message-bus-delivery-tracking-and-nack-delay.md), [ADR-0013](adr-0013-per-space-runtimes.md)
**Related**: [ADR-0023](adr-0023-tool-permission-decision-model.md) (tool approvals), which D7a below interacts with

## Context

A conversation is a flat, append-only list. Users cannot fix a sent message, get a second answer, or try another model on the same question. The storage and runtime make that hard to change in place:

- **No link between messages.** `messages` has no parent column. `load_history` orders by `turn ASC, created_at ASC`. `PRAGMA foreign_keys=ON` is set at startup and `conversation_id` cascades on delete.
- **`turn` is an iteration index, not a depth.** `prepare_history` sets `base_turn` to the number of prior rows and saves the user row with that turn. Every row of one loop iteration (thinking, tool calls, tool results, final text) gets `base_turn + iteration + 1`. It is not unique, and the web detail handler joins tool results to tool calls on `(turn, tool_name)`.
- **Save sites are scattered.** The user message, thinking rows, tool-call rows, tool results and final text are saved from the orchestrator loops and from the `dispatch.rs` and `turn_control.rs` helpers. `save_message` is two statements without a transaction: an `INSERT ... ON CONFLICT(id) DO NOTHING`, then an `UPDATE` of `conversations.updated_at` that runs even when the insert did nothing.
- **Compaction is destructive.** `persist_compacted` calls `replace_history`, which runs `DELETE FROM messages WHERE conversation_id = ?` and re-inserts the kept rows. A persistence failure is only logged. The web UI loses the earlier messages as a side effect, `/status` and `/compact` read the raw `load_history`, and no message id survives compaction.
- **Redelivery duplicates rows.** A transient LLM error nacks the `TurnRequest` and the worker re-runs the whole turn. `prepare_history` saves a new user row with a fresh id on every run, so a redelivered turn already duplicates the user message.
- **In-flight tracking is split and partly dead.** Three unrelated `conversation_id -> request_id` maps exist: `ApiState.active_turns`, `ChannelRunner.active_turns` and one in the CLI. `/stop` and `/status` read the first through `CommandContext`, but the web `send_message` handler never writes it, so web `/stop` never finds a turn. The CLI map is created and passed on but never written. The scheduler publishes `TurnRequest` straight to the bus. Several services can share one database, and the bus can be NATS.
- **Attachments belong to one message.** `message_attachments.message_id` is a single column, and the bytes are stored under a path derived from the attachment's own id. A second message cannot reference the same attachment.

## Decision

**Store messages as a tree in an adjacency list, keep one active leaf per conversation, and make every history reader follow the path to that leaf.** Branch operations work through the store, one turn registry and anchored turns, so the existing save sites do not change.

### D1. Adjacency list plus an active leaf pointer

- `messages.parent_id TEXT NULL REFERENCES messages(id)`, with no `ON DELETE` action, and an index on `(conversation_id, parent_id)`. A `NULL` parent marks a root. Editing the first message creates a second root, and roots are siblings of each other.
- `conversations.active_leaf_id TEXT NULL`, with no foreign key. The active path is the ancestor chain of the active leaf, loaded with one recursive CTE, root first.
- `messages.run_id TEXT NULL` records the turn run that wrote a row: the request id when present, otherwise the bus message id. Redelivery cleanup (D3a) uses it, and so does the guard that stops a cancelled turn from clobbering a finished reply (D3, cancellation).
- **No self-referencing cascade.** SQLite counts foreign-key actions against `SQLITE_MAX_TRIGGER_DEPTH` (1000 by default), so a cascade along a long chain could make `delete_conversation` fail. It would also add nothing: the `conversation_id` cascade already deletes every row of the conversation in one statement, and SQLite checks a non-deferred foreign key at the end of the statement, when parents and children are both gone. Deleting a single branch is a non-goal. A contract test deletes a conversation with a 1,500-row chain.
- **Backfill.** The migration sets `parent_id` to the previous row by `(turn, created_at, rowid)` with `LAG()` per conversation and `active_leaf_id` to each conversation's last row. Existing conversations become one chain. There is no fallback for the old flat shape.

| Alternative                                                         | Why not                                                                                                                                               |
| ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Copy-on-branch: duplicate the shared prefix into a new conversation | Branches no longer belong to one conversation. Titles, list entries and attachments get duplicated, and the picker would have to cross conversations. |
| `branch_id` on every message plus a `branches` table                | Every regenerate has to rewrite or duplicate descendants to tag them. Two concepts to keep consistent instead of one.                                 |
| An `active_child_id` per node (remember the last-viewed fork child) | More writes on every switch, and leaf resolution needs a second walk. Kept as a follow-up.                                                            |

Switch-branch moves the active leaf to the most recently created child at each fork below the chosen message. The newest child is the version the user most likely produced last, so the result is predictable and needs no extra state. Remembering a leaf per fork would change only which child the descent picks.

### D2. `save_message` appends at the active leaf in one transaction

`save_message` runs in one `BEGIN IMMEDIATE` transaction. It reads `active_leaf_id`, inserts the row with `parent_id = active_leaf_id`, and only when the insert affected a row does it set `active_leaf_id` to the new row and bump `updated_at`. `BEGIN IMMEDIATE` takes the write lock up front. A deferred `BEGIN` would upgrade from reader to writer and fail with `SQLITE_BUSY` when a messenger turn and a web turn write at the same time. The existing `busy_timeout = 5000` then makes the second writer wait.

An insert that hits `ON CONFLICT(id) DO NOTHING` changes nothing, the leaf included. A re-run that re-saves a row under a pre-assigned id therefore cannot move the leaf back and orphan later rows.

`branch_message(&msg, parent_id: Option<Uuid>)` does the same under an explicit parent. `None` means a new root.

**Alternative: thread an explicit "current tip" to every save site.** Rejected. It touches about ten call sites for behavior the store can guarantee by itself, and a forgotten site would corrupt the tree without any error.

`load_history` and `last_messages` return the active path only. Messengers, the CLI, the scheduler and the quick-message path keep appending through `save_message` and reading `load_history`, so they stay linear. If a web user switches branches in a conversation a messenger also uses, the messenger's next turn continues the selected branch.

### D3. Anchored turns

`TurnRequest` gains three serde-default fields:

- `anchor: TurnAnchor`, one of `Append`, `Edit { parent_message_id: Option<Uuid> }` or `Regenerate { user_message_id: Uuid }`.
- `user_message_id: Option<Uuid>`, the id the run persists its user row under (Append and Edit).
- `model_override: Option<String>`, which applies to this turn only.

Messenger, CLI and scheduler producers keep their behavior, because requests without the new fields deserialize to `Append`. Producers that build `TurnRequest` as a struct literal, such as the scheduler, only need the new fields spelled out.

- **Append.** The user row is saved with `save_message` and the turn appends.
- **Edit `U` (parent `P`).** The new user row `U'` is saved with `branch_message(U', P)`. `U`'s attachments are linked to `U'` (D14), and the turn appends. If the turn then fails, the path ends at `U'`, just as a failed send ends at its user row.
- **Regenerate `U`.** No user row is saved. The turn's first persisted row uses `branch_message(row, Some(U))` and the rest append. If the turn fails before it persists anything, `active_leaf_id` never moved and the old reply stays active.

**History is the anchor's ancestor chain**, not "the active path up to `U`". Edit builds context from `P`'s chain plus `U'` and its carried-forward attachments. Regenerate builds it from `U`'s chain, including `U` and its attachments. Editing or regenerating a message that is not on the active path is therefore well defined: the new branch grows from where that message lives and becomes active.

`turn` keeps its meaning: the user row gets `base_turn` and every row of iteration `i` gets `base_turn + i + 1`. `base_turn` is the number of rows strictly before the turn's user row on its path, computed from the raw path and never from the summary-applied history (D5):

| Anchor          | `base_turn`                                                                                               |
| --------------- | --------------------------------------------------------------------------------------------------------- |
| Append          | length of the active path                                                                                 |
| Edit (parent P) | length of P's ancestor chain, including P (0 when P is `None`)                                            |
| Regenerate U    | length of U's ancestor chain, excluding U, so the reply rows get the same turns as the reply they replace |

Dropping `turn` is rejected as out of scope. Titling eligibility and the indexer's `session://.../turn_N` keys depend on it.

**Cancellation is anchor-aware.** When a branch turn is stopped, the cancelled reply is saved under the same anchor. A Regenerate that persisted nothing yet saves its partial reply with `branch_message(row, Some(U))`, and any other run appends to its own leaf. The guard that skips saving when "the final answer is already saved" looks only at rows with this run's `run_id`, so it never mistakes the old reply of a regenerate for the new one. A turn cancelled before a worker claimed it persists nothing and leaves the leaf where it was.

**Plumbing without an orchestrator `TurnContext`.** `prepare_history` already returns the store the turn saves through. It will return a `TurnStore`, a thin wrapper around `SqliteConversationStore` that implements `ConversationStore`. The wrapper carries the turn's `run_id`, which it stamps on every row, and an optional pending branch parent. The first `save_message` through a wrapper with a pending parent becomes `branch_message(row, Some(parent))` and clears the parent. The run loops already hand this store to every save site, and the `dispatch.rs` and `turn_control.rs` helpers take `&dyn ConversationStore`, so no signature changes beyond `prepare_history`'s return type. The alternative, an explicit `TurnContext` flag, is deferred: the only `TurnContext` in the repository belongs to the skill learner, and extracting an orchestrator one is its own refactor. The wrapper can move into one later.

#### D3a. Redelivery is idempotent

A nacked turn re-runs from the start. Before the first run the worker fixes the run's identity. `run_id` is the request id (`batch_id`) or else the bus message id, both stable across `nack_delayed`, and `user_message_id` is assigned if the producer did not set one. On every run `prepare_history` then:

1. **Discards the run's partial rows.** It deletes every row with this `run_id` except the user row, in one statement, and resets the leaf to the anchor point (`U` for Append and Regenerate, `U'` for Edit). If the delete fails, for example because another interface appended under a partial row, it logs a warning and only resets the leaf.
2. **Re-saves the user row under its pre-assigned id.** For Append and Edit a conflicting insert is a no-op that does not move the leaf (D2). For Edit the attachment links are re-inserted with `INSERT OR IGNORE` (D14). Regenerate has no user row to save.
3. **Re-arms the pending branch parent** for Regenerate.

A re-delivered Regenerate leaves exactly one new reply sibling, a re-delivered Edit leaves one `U'`, and a re-delivered Append no longer duplicates the user message.

### D5. Compaction is non-destructive

A new table `conversation_summaries(id, conversation_id, through_message_id, content, created_at)` sits behind a `ConversationSummaryStore` trait with Sqlite and InMemory implementations, following the trait pair pattern in [ADR-0009](adr-0009-testability-architecture.md). `maybe_compact` writes a summary row instead of calling `replace_history`, and `replace_history` is removed.

One builder, `build_turn_history(conversation_id, leaf) -> TurnHistory { history, row_ids }`, serves `prepare_history`, `/status` and `/compact`. It loads the path to `leaf`, takes the newest summary whose `through_message_id` is on that path, emits it as the existing `[Conversation summary — earlier context compacted]` assistant text, and follows it with the path rows after `through_message_id`. `/status` therefore estimates tokens over what the model sees, and `/compact` re-summarizes from the existing summary rather than the whole raw path. A summary that covers messages a later edit branched away from is not on the new path, so it does not apply.

The in-memory split index maps back to a message id through `row_ids: Vec<Option<Uuid>>`, a shadow of `history` built next to `messages_to_chat_history` and extended at every in-turn push through one helper. If no id resolves, no summary is persisted and the in-memory compaction still applies, as on today's failure path.

**Rollout.** The tree migration lands before the summary table. In between, `replace_history` deletes the rows and sets `active_leaf_id = NULL` in one transaction before it re-inserts. Without the reset, the first re-insert would take a deleted row as its parent, which is a foreign-key violation, and the CTE would start from a dead leaf and return an empty history. The summary PR then deletes `replace_history`.

**Alternatives.** Keeping `replace_history` is rejected because it destroys every branch. Storing the summary as a tree node is rejected because it would show in the UI and shift `turn`.

**Consequence.** Once compaction stops deleting rows, a message id stays valid for the life of its conversation. Features that key data by message id, such as feedback votes, can rely on that.

### D7. One orchestrator-owned turn registry

A `TurnRegistry` on `Orchestrator` replaces the three `active_turns` maps. `CommandContext.active_turns` becomes a handle to it, which also repairs web `/stop` and `/status`. It maps `conversation_id` to `{ request_ids, exclusive }` and hands out leases:

- `try_begin(conversation_id, request_id, Shared | Exclusive) -> Option<TurnLease>` checks and inserts under one lock, so two rapid edits cannot both pass. Shared is refused while an exclusive lease is held. Exclusive is refused while any lease is held.
- `TurnLease` releases on `Drop`, so every exit path (success, error, cancel, panic unwinding, a dropped handler future) clears it. The map sits behind a `std::sync::Mutex` that is never held across an `.await`, so `Drop` can release it synchronously. This departs on purpose from the workspace default of a tokio `RwLock`.

**Who acquires.** Web handlers acquire before submitting and move the lease into the spawned submit task. `send_message` takes Shared and answers 409 only while an exclusive lease is held. Edit and regenerate take Exclusive for the whole turn, and switch-branch takes Exclusive for the duration of its database call. The worker takes a Shared lease for every turn it claims whose `request_id` is not already registered. That covers the scheduler, which bypasses the web submit path, and messenger turns. If the conversation holds an exclusive lease, the worker defers the message with `nack_delayed` instead of running it.

**A plain send during an edit or regenerate gets 409, and a messenger turn is deferred.** The exclusive lease protects the anchor of a running branch turn. A send that appended in the middle would land its user row on a leaf that is about to move. The web client holds the draft and can retry. A messenger user cannot see a 409, so a delayed nack is the only kind answer. The wait is one turn long. If deferrals run past `MAX_TURN_REDELIVERIES`, the turn fails through the existing exhaustion path.

**Limitation: the registry is per process.** With several services on one database, or a NATS bus with workers in other processes, a branch operation in one process does not see a turn running in another. The transactions in D2 keep the tree consistent, but a concurrent append can land on the branch the user just left. A cross-process lease is out of scope.

#### D7a. Branching while a turn waits for the user

**A branch operation cancels a turn that is only waiting for the user.** Edit, regenerate and switch-branch acquire through `Orchestrator::acquire_exclusive_for_branch(conversation_id, request_id)`, not a bare `try_begin(Exclusive)`:

1. It tries `try_begin(Exclusive)`. On success the operation proceeds.
2. On refusal it asks the interactions store whether every in-flight run of the conversation has a pending interaction: an approval, a question or an A2UI surface wait. If any run is still working, it returns 409 and cancels nothing.
3. Otherwise it cancels those runs through the path `/stop` uses. The run's end-of-run cleanup resolves the interactions as `cancelled`, so the gated tool call never runs, and the cancelled-reply persistence records the stop on the branch being left.
4. It waits for the leases to be released, up to 10 seconds, and tries `try_begin(Exclusive)` once more. A timeout returns 409.

A turn blocked on the user cannot make progress, and the only way to unblock it is to answer a prompt on a branch the user is leaving. Returning 409 would trap the user between a card they no longer want and an edit they cannot submit. A turn that is still working is not cancelled: that stays a 409 and the user can press Stop. The approval gate itself, with its confirmation callback and `interactive` flag, is the one described in [adr-0023-tool-permission-decision-model.md](adr-0023-tool-permission-decision-model.md). The interactions store and the run cleanup that step 2 and step 3 rely on come with the turn-interactions ADR and are not in the code yet.

**Alternatives.** Denying the approval and letting the turn continue is rejected, because the model would run on, possibly acting on a branch that is no longer shown. Blocking until the approval times out is rejected, because that takes ten minutes.

### D14. Edit carries attachments forward through a link table

Dropping attachments on edit would silently change what the model sees: a question about a screenshot becomes a question about nothing. Hiding Edit for such messages would leave the user no way to fix a typo. So an edit carries them forward.

`message_attachments.message_id` is a single foreign key and the bytes are keyed by the attachment's own id, so copying the row would point at a file that does not exist. A new table `message_attachment_links(message_id REFERENCES messages(id) ON DELETE CASCADE, attachment_id REFERENCES message_attachments(id) ON DELETE CASCADE, PRIMARY KEY (message_id, attachment_id))` records the extra references.

- `AttachmentStore::list_for_message` returns the rows whose `message_id` matches plus the rows reached through the link table.
- `AttachmentStore::carry_forward(from_message_id, to_message_id)` inserts one link per attachment of `from` with `INSERT OR IGNORE`, so a redelivered edit leaves one link each.
- Nothing is copied on disk. Both versions share the bytes.

Attachments cannot be added or removed during an edit. The edit request body stays text only.

**Alternatives.** Copying the files and rows into the new message is rejected: it duplicates bytes on every edit and needs new ids for each copy. Replacing `message_attachments.message_id` with a join table outright is rejected: it is a larger migration that touches every attachment reader for no gain.

## Consequences

- **Four migrations**, in this order: message tree, attachment links, conversation summaries, message model. They are forward-only. Rollback means restoring a backup, and rows that compaction already deleted are not reconstructed.
- **Every reader of history follows the path.** The title generator, the conversation indexer, `/status` and `/compact` read the active path. The title locks after the first success. The indexer writes under `session://.../turn_N`. A regenerate reuses the replaced reply's turn numbers, so it re-indexes under the same keys, but chunks from abandoned deeper branches stay behind and can surface in `memory-search`. This is accepted for the first release.
- **Cost of the CTE.** Loading a path is O(depth) over the primary key. Contract tests load a 1,000-row chain and delete a 1,500-row chain.
- **Messenger interop.** A messenger turn continues whichever branch is active. Edit and regenerate reply on the web only, and nothing is posted back to the messenger thread.
- **The `Message` struct and `conversations.rs` are contended files.** Several changes add columns or helpers there, so the order of those PRs matters and is fixed in the execution stack.
- **Compaction no longer hides messages in the web UI.** After the summary PR lands, earlier messages stay visible, so Playwright baselines move.
- **Deferred messenger turns** can run into `MAX_TURN_REDELIVERIES` if branch turns follow each other for a long time. That is unlikely at human pace, and the failure is the existing one.
- **Non-goals.** Switching the provider kind or endpoint on regenerate, branching from messenger interfaces, deleting, pruning or comparing branches, editing attachments or assistant messages, and a cross-process in-flight guard.
