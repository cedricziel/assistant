# Tasks: message-branching

A stack of PRs. Each stays under ~500 changed lines (generated `assistant_api` code and `openapi.json` excluded) and leaves `main` shippable. Every PR starts with a failing test (TDD).

PR ids follow the cross-change stack plan (`openspec/chat-ui-parity-stack.md`). A letter suffix splits one stack slot into PRs that land in the same wave.

**Rules that apply to every PR:**

- **Migrations.** Use the numbers the stack plan allocates: `046_message_tree` (PR 1), `055_conversation_summaries` (PR 3a), `060_message_model` (PR 8a). `message_attachment_links` (PR 2b) has no allocation yet, so the stack plan assigns one before PR 2b starts. Register each migration in the `run_migrations` list in `crates/storage/src/lib.rs:297`.
- **New `Message` fields** carry `#[serde(default)]`.
- **Generated files.** Never hand-merge `openapi.json`, `app/packages/assistant_api` or Playwright PNGs. After a rebase, rerun `make dump-openapi && make generate-flutter-client`, regenerate baselines with the e2e-testing skill, and record the visual diff in the PR body.
- **Panic-free.** Run `cargo clippy -D warnings` with no unwrap, expect or panic in production code.

## Parallelism

- **Wave 1:** PR 0 (ADR, together with ADR-0017 and ADR-0019), PR 1 (tree) and PR 5 (the shared SSE runner).
- **Wave 2:** PR 2, PR 2b and PR 3a run in parallel after PR 1 (PR 3a needs only PR 1). PR 3b follows PR 3a and must merge before `conversation-list-management` PR 5.
- **Wave 3 (after `chat-stop-cancels-turn` PR 1 and `tool-approval-prompts` PR 6):** PR 4a, then PR 4b and PR 4c in parallel (PR 4c also needs PR 2 and PR 2b), then PR 4d (also needs `tool-approval-prompts` PR 7). PR 9 starts any time after `chat-element-library` PR 1 and PR 3.
- **Wave 4:** PR 6 after PR 2, PR 4b, PR 4d, `tool-result-renderers` S4 and `conversation-list-management` PR 1. PR 10 after PR 9 and `chat-element-library` PR 5 and PR 8a.
- **Wave 5:** PR 7a after PR 4c, PR 4d, PR 5, PR 6, `follow-up-suggestions` PR 3 and `tool-approval-prompts` PR 6 and PR 7. PR 7b after PR 7a and `chat-stop-cancels-turn` PR 2. PR 11 after PR 6, PR 9 and `chat-element-library` PR 4.
- **Wave 6:** PR 8a after PR 4a, `message-feedback` PR 3 and `tool-result-renderers` S3. PR 8b after PR 7a and PR 8a. PR 12a after PR 7a and the stream-loop lane. PR 12b after PR 12a. PR 12c after PR 10, PR 11 and PR 12b.
- **Wave 7:** PR 13 after PR 8b and PR 12c.
- **Wave 8:** PR 14 last.

| PR  | Layer  | Depends on (this change)  |
| --- | ------ | ------------------------- |
| 0   | docs   | none                      |
| 1   | server | none                      |
| 2   | server | 1                         |
| 2b  | server | 1                         |
| 3a  | server | 1                         |
| 3b  | server | 3a                        |
| 4a  | server | none                      |
| 4b  | server | 4a                        |
| 4c  | server | 2, 2b, 4a                 |
| 4d  | server | 4b                        |
| 5   | server | none                      |
| 6   | server | 2, 4b, 4d                 |
| 7a  | server | 4c, 4d, 5, 6              |
| 7b  | server | 7a                        |
| 8a  | server | 4a                        |
| 8b  | server | 7a, 8a                    |
| 9   | client | none                      |
| 10  | client | 9                         |
| 11  | client | 6, 9                      |
| 12a | client | 7a                        |
| 12b | client | 12a                       |
| 12c | client | 10, 11, 12b               |
| 13  | client | 8b, 12c                   |
| 14  | both   | 3b, 7b, 13                |


(`message-feedback` ships after PR 3b. `tool-approval-prompts`' enabling PR for web turns lands after its client cards and is independent of this stack.)

---

## PR 0 — `docs(adr): ADR-0018 conversation message tree`

Layer: server (docs only)
Depends on: none
Parallel with: everything. Land in wave 1 together with ADR-0017 (`tool-approval-prompts` PR 1) and ADR-0019 (`generative-ui-a2ui` PR 1).

- [ ] 0.1 Write `docs/adr/adr-0018-conversation-message-tree.md` covering design.md D1 (adjacency list, no self-cascade, `run_id`), D2/D3/D3a (append-at-leaf, anchored turns, redelivery), D5 (non-destructive compaction), D7/D7a (turn registry, its per-process limitation, auto-cancel of a pending approval) and D14 (attachment links), with the rejected alternatives. Cite `adr-0011-tool-permission-decision-model.md` by filename where relevant.
- [ ] 0.2 Add "Branch", "Active path" and "Active leaf" to `docs/glossary.md`.

## PR 1 — `feat(storage): persist messages as a tree with an active leaf`

Layer: server
Depends on: none
Parallel with: PR 0, PR 5. **Lane:** first in the `Message`-struct lane (then `tool-result-renderers` S3, `message-feedback` PR 3, `chat-stop-cancels-turn`'s `finish_reason` column, PR 8a).

- [ ] 1.1 RED: add contract tests in `crates/storage/tests/contract/conversation_store.rs`, run against both impls: (a) sequential `save_message` calls form a chain and the active leaf is the last row; (b) `load_history` follows the active path; (c) an empty conversation returns `[]`; (d) a 1,000-row chain loads in order; (e) re-saving an existing id inserts nothing and leaves the leaf in place; (f) deleting a conversation with a 1,500-row chain succeeds. Confirm RED.
- [ ] 1.2 RED: a contract test for `replace_history` followed by `load_history`: the history is non-empty and equals the re-inserted rows, chained. Confirm RED (FK violation or empty history).
- [ ] 1.3 RED: a migration test that seeds a three-row conversation under the pre-migration schema, applies migrations, and asserts the backfilled `parent_id` chain and `active_leaf_id`.
- [ ] 1.4 Add migration `046_message_tree.sql`: `messages.parent_id TEXT NULL REFERENCES messages(id)` (no `ON DELETE` action), `messages.run_id TEXT NULL`, index `(conversation_id, parent_id)`, index `(conversation_id, run_id)`, `conversations.active_leaf_id TEXT NULL`, and the `LAG()` backfill over `(turn, created_at, rowid)`. Register it in `crates/storage/src/lib.rs:297`.
- [ ] 1.5 Add `parent_id: Option<Uuid>` and `run_id: Option<Uuid>` with `#[serde(default)]` to `Message` (`crates/core/src/types/conversation.rs:33`), both `None` in `Message::new`.
- [ ] 1.6 `SqliteConversationStore::save_message` (`crates/storage/src/conversations.rs:413-450`): in one `BEGIN IMMEDIATE` transaction, read the leaf, insert with that parent, and move the leaf and `updated_at` only when the insert affected a row. Do the same in `InMemoryConversationStore`.
- [ ] 1.7 `load_history` (`conversations.rs:453-465`): a recursive CTE from `active_leaf_id`, root first. `last_messages` returns the tail of the path. Do the same in-memory.
- [ ] 1.8 `replace_history` (`conversations.rs:369-386`): run the `DELETE` and `UPDATE conversations SET active_leaf_id = NULL` in one transaction before the re-insert loop. Confirm 1.1–1.3 GREEN. Run `cargo test -p assistant-storage -p assistant-runtime`.

## PR 2 — `feat(storage): branch insert and navigation on ConversationStore`

Layer: server
Depends on: PR 1
Parallel with: PR 2b, PR 3a, PR 5. **Unblocks:** `conversation-list-management` PR 8, which clears `archived_at` in `branch_message` for user rows too.

- [ ] 2.1 RED: contract tests for `branch_message(msg, Some(parent))` (sibling created, becomes the leaf, old rows kept); `branch_message(msg, None)` (second root); a conflicting `branch_message` leaves the leaf in place; `list_siblings(message_id)` in creation order, with roots as siblings; `ancestors(message_id)` root first; `switch_to(message_id)` descends through the most recent child, returns the new leaf and leaves `updated_at` unchanged; `set_leaf(message_id)` sets the leaf exactly; a foreign-conversation message returns `None`. Confirm RED.
- [ ] 2.2 Add `branch_message`, `list_siblings`, `children_of`, `ancestors`, `switch_to`, `set_leaf` and `delete_run_rows(conversation_id, run_id, keep: Option<Uuid>)` to the `ConversationStore` trait. Implement them in Sqlite (`BEGIN IMMEDIATE`) and InMemory. Emit `ConversationEvent::Upserted` on switch. GREEN.

## PR 2b — `feat(storage): share attachments across message versions`

Layer: server
Depends on: PR 1
Parallel with: PR 2, PR 3a, PR 5. Carries the decision "edit carries attachments forward" (design D14).

- [ ] 2b.1 RED: contract tests for `AttachmentStore`, against both impls: `carry_forward(from, to)` makes `list_for_message(to)` return the attachments of `from` and leaves `list_for_message(from)` unchanged; calling it twice leaves each attachment once; an attachment linked directly and one reached through a link both appear, in creation order; `get_meta` and `load_bytes` of a carried attachment still work; deleting the conversation removes the links. Confirm RED.
- [ ] 2b.2 Add the `message_attachment_links` migration (composite primary key `(message_id, attachment_id)`, `ON DELETE CASCADE` on both foreign keys, index on `attachment_id`) and register it. Add `carry_forward` to the trait, and extend `list_for_message` (`crates/storage/src/attachments.rs:106`) with the link table, in Sqlite and InMemory. GREEN.

## PR 3a — `feat(runtime): conversation summaries store and summary-applied history builder`

Layer: server
Depends on: PR 1
Parallel with: PR 2, PR 2b, PR 5

- [ ] 3a.1 RED: contract tests for `ConversationSummaryStore`: insert, `newest_on_path(conversation_id, path_ids)` picks the newest summary whose `through_message_id` is in the path, none for a diverged path, and summaries are deleted with their conversation. Confirm RED.
- [ ] 3a.2 Add migration `055_conversation_summaries.sql` (`id, conversation_id REFERENCES conversations ON DELETE CASCADE, through_message_id, content, created_at`) and register it. Add the trait, `SqliteConversationSummaryStore`, `InMemoryConversationSummaryStore`, a `StorageLayer` accessor and the prelude re-export. GREEN.
- [ ] 3a.3 RED: unit tests for `build_turn_history`: with no summary it equals `messages_to_chat_history` over the path; with a summary on the path it starts with the summary text followed by the rows after `through_message_id`; `row_ids.len() == history.len()`.
- [ ] 3a.4 RED: a test that the row-id shadow stays the same length as `history` across a tool-call iteration (pushes go through one helper).
- [ ] 3a.5 RED: a `/status` test asserting the token estimate is taken over the summary-applied history, and a `/compact` test asserting its input is that history (`command_registry.rs:274-310`).
- [ ] 3a.6 Implement `build_turn_history` next to `messages_to_chat_history` (`crates/runtime/src/history.rs:29`). Use it in `prepare_history` (`orchestrator/mod.rs:1535`), `/status` and `/compact`. Keep `base_turn` computed from the raw path length. Thread `row_ids` through the compaction call sites (`mod.rs:815`, `913`, `1207`). Compaction still persists through `replace_history`. GREEN.

## PR 3b — `refactor(runtime): non-destructive context compaction`

Layer: server
Depends on: PR 3a
Parallel with: PR 2, PR 2b, PR 5. **Merge before `conversation-list-management` PR 5** (it drops its `replace_history` assertion once `replace_history` is gone). **`message-feedback` ships (its PR 7) after this PR**, so votes keyed by message id are never orphaned by compaction (design D5).

- [ ] 3b.1 RED: change `maybe_compact_persists_to_conversation_store` (`crates/runtime/src/compaction.rs:788`) to assert that every original message is still stored after compaction and that a summary row exists with `through_message_id` = the last summarised message.
- [ ] 3b.2 RED: an orchestrator test: the next turn's LLM request starts with the summary, followed by the post-summary path. A branch created before `through_message_id` gets no summary. An unresolvable split skips persistence.
- [ ] 3b.3 Replace `persist_compacted` (`compaction.rs:355`) with a summary insert keyed by `row_ids[split - 1]`.
- [ ] 3b.4 Delete `ConversationStore::replace_history` from the trait and both impls, plus its contract tests (`crates/storage/tests/contract/conversation_store.rs:164` and the PR 1.2 test). No shim. GREEN.
- [ ] 3b.5 Run the Playwright suite. If a compacted conversation is in a baseline, regenerate it and record the diff: earlier messages are visible again.

## PR 4a — `refactor(runtime): SubmitTurn options struct`

Layer: server
Depends on: `chat-stop-cancels-turn` PR 1, `tool-approval-prompts` PR 6 (wave 3)
Parallel with: PR 2, PR 2b, PR 3a, PR 3b, PR 5. Unblocks PR 4b, PR 4c and PR 8a.

- [ ] 4a.1 RED: a test that `TurnRequest` deserialised from JSON without the new fields gets `anchor = Append`, `user_message_id = None` and `model_override = None`, and a test that `SubmitTurn::new(prompt, conv, interface)` defaults match today's `submit_turn` behaviour.
- [ ] 4a.2 Add `TurnAnchor` and the `anchor`, `user_message_id` and `model_override` fields with `#[serde(default)]` to `TurnRequest` (`crates/core/src/bus_messages.rs:60`). They are carried but not yet acted on.
- [ ] 4a.3 Replace the positional `submit_turn_internal` (`crates/runtime/src/orchestrator/worker.rs:727`) and its `submit_turn_*` variants (including `submit_turn_with_request_id`, `worker.rs:701`) with `Orchestrator::submit(SubmitTurn)`. `SubmitTurn` absorbs `interactive` from `tool-approval-prompts` PR 6. Update the callers (web-ui, channel runner, CLI, tests). Remove the `too_many_arguments` allows. GREEN.

## PR 4b — `feat(runtime): one orchestrator-owned turn registry`

Layer: server
Depends on: PR 4a
Parallel with: PR 4c, PR 8a. Provides `Orchestrator::conversation_turn_in_flight(conversation_id)`, which `generative-ui-a2ui` PR 11 reads.

- [ ] 4b.1 RED: `TurnRegistry` unit tests: concurrent `try_begin(Exclusive)` admits exactly one; Shared is refused while Exclusive is held; Exclusive is refused while Shared is held; dropping a lease releases it, including on a dropped future and an error path; `conversation_turn_in_flight` is true exactly while a lease is held.
- [ ] 4b.2 RED: an orchestrator test that a turn published straight to the bus (as the scheduler does, `scheduler/tasks.rs:206`) is registered while the worker runs it. A worker that claims a turn while an exclusive lease is held defers it with `nack_delayed`, and the deferred turn runs once the lease is released.
- [ ] 4b.3 RED: a web `/stop` test: a running web turn is found and cancelled (today `ApiState.active_turns` is never written).
- [ ] 4b.4 Implement `TurnRegistry` on `Orchestrator` (`std::sync::Mutex`, never held across `.await`, release on `Drop`). Acquire in `Orchestrator::submit` when the caller passed no lease, and in the worker for unregistered claims. Remove `ApiState.active_turns` (`crates/web-ui/src/api/mod.rs:152`), `ChannelRunner.active_turns` (`channel_runner.rs:64`, `427`, `438`) and the CLI map (`interface-cli/src/main.rs:355`). Point `CommandContext` at the registry. Document the per-process limitation in the rustdoc. GREEN.

## PR 4c — `feat(runtime): edit and regenerate turn anchors with idempotent redelivery`

Layer: server
Depends on: PR 2, PR 2b, PR 4a
Parallel with: PR 4b, PR 8a

- [ ] 4c.1 RED: worker tests with `ScriptedLlmProvider`: (a) `Edit{parent}` persists `U'` under `parent` with the pre-assigned `user_message_id`, and the reply chains after it; (b) `Regenerate{U}` persists no user row, sends U's ancestor chain including U's attachments, and makes the first reply row a child of U; (c) a regenerate whose provider errors before output leaves the leaf unchanged; (d) edit and regenerate of a message on an inactive branch use that message's ancestor chain; (e) `base_turn` per anchor matches design D4; (f) an edit of a message with attachments links them to `U'` and the turn's request includes them.
- [ ] 4c.2 RED: redelivery tests that nack once with a transient error after a row was persisted, then redeliver: Regenerate ends with exactly two children under U; Edit ends with one `U'` and one link per attachment; Append ends with one user row. Every row carries the run's `run_id`.
- [ ] 4c.3 Add `TurnStore` (wraps the Sqlite store, stamps `run_id`, holds an optional pending branch parent) and return it from `prepare_history` (`mod.rs:1535`). Handle the anchors there: discard the run's partial rows with `delete_run_rows`, reset the leaf with `set_leaf`, re-save the user row by its fixed id, call `carry_forward(U, U')` for Edit, and arm the pending parent. In the worker, fix `run_id` and `user_message_id` before the first run. GREEN.

## PR 4d — `feat(runtime): cancel a turn that is waiting for the user before branching`

Layer: server
Depends on: PR 4b, `tool-approval-prompts` PR 3 (interactions store) and PR 7 (broker, `abandon_run`), `chat-stop-cancels-turn` PR 1 (cancel path)
Parallel with: PR 4c. Carries the decision "branching while an approval is pending auto-cancels it" (design D7a).

- [ ] 4d.1 RED: unit tests for `Orchestrator::acquire_exclusive_for_branch(conversation_id, request_id)` with an in-memory interactions store and a fake cancel path: no lease held, so it returns an exclusive lease at once; a shared lease whose run has a pending approval, so it cancels that run and returns an exclusive lease after release; two runs, only one with a pending interaction, so it returns `Conflict` and cancels nothing; a run whose lease is not released within the ceiling returns `Conflict` (use a paused clock); a pending question and a pending surface wait count the same as an approval.
- [ ] 4d.2 RED: an integration test with a real broker: a `ScriptedLlmProvider` turn blocks on a pending `bash` approval; calling the helper resolves the interaction as `cancelled`, the `bash` handler never runs, and the old branch ends with the stopped reply row.
- [ ] 4d.3 Implement `acquire_exclusive_for_branch` as in design D7a. The wait ceiling is 10 seconds, the same constant `chat-stop-cancels-turn` uses for "Stopping…". Cancel through the path `/stop` uses. GREEN.

## PR 5 — `refactor(web-ui): extract the streamed turn runner from send_message`

Layer: server
Depends on: none
Parallel with: PR 0, PR 1, PR 2, PR 2b, PR 3a. **The only extraction of this loop.** `chat-stop-cancels-turn` PR 2 and PR 3 (voice), `tool-approval-prompts` PR 7 and PR 12, `follow-up-suggestions` PR 3 and `chat-a11y-and-code-blocks` S3 edit `stream_turn_to_sse`, not `send_message`, and list this PR in their Depends on. It replaces `chat-stop-cancels-turn`'s `drive_run`.

- [ ] 5.1 RED (characterisation): pin the current `send_message` frame sequence in `crates/web-ui/src/api/messages.rs` tests: `run_started` at seq 0, thinking batching every 20 tokens, `done` with `message_id`, `agent_error{reason: cancelled}`, and the push dispatch. Add only the ones not already covered.
- [ ] 5.2 Move the post-`run_started` SSE, persistence, push and terminal logic (`messages.rs:186-486`) into `stream_turn_to_sse(ctx, state, run_id, conv_id, opts: RunOptions, turn: oneshot::Receiver<Result<TurnResult>>) -> Response`. `RunOptions` starts with `push: bool`. Later PRs add `interactive` (`tool-approval-prompts` PR 6), `suggestions` (`follow-up-suggestions` PR 3) and `anchor` (PR 7a here), each in the first PR that reads it. `send_message` becomes validation, submit and one call. No behaviour change. GREEN.

## PR 6 — `feat(web-ui): branch metadata and switch-branch endpoint`

Layer: server
Depends on: PR 2, PR 4b, PR 4d, `tool-result-renderers` S4 (tool_call_id join), `conversation-list-management` PR 1 (summary projection)
Parallel with: PR 10, PR 11 (client PR 11 starts once this lands)

- [ ] 6.1 RED: handler tests for `GET /api/conversations/{id}`: `parent_id` is omitted for roots; `branch` appears on edited user messages; reply `branch` is projected onto the last visible assistant row when the first rows are tool-call or thinking rows; `reply_branch` sits on the user row when the active reply has no visible assistant row; `branch` is omitted without siblings.
- [ ] 6.2 RED: tests for `POST /api/conversations/{id}/switch-branch`: 200 with the new path; 404 for a foreign message; 400 for an invalid UUID; 403 without `conversations:write`; 403 for a read-only member; 409 while a working turn is in flight; `updated_at` unchanged; the descent lands on the most recently created child at each fork; while a turn waits for an approval, the approval resolves as `cancelled` and the switch returns 200. Every error has an `{"error": ...}` body.
- [ ] 6.3 Add an envelope-returning `owned_conversation_json` beside `owned_conversation` (`conversations.rs:50-62`), built on `json_error` (`crates/web-ui/src/errors.rs:25`), for the new handlers.
- [ ] 6.4 Add `BranchInfo` and `parent_id`/`branch`/`reply_branch` to `MessageSummary` (`conversations.rs:105`) and the projection in `get_conversation` (`conversations.rs:401`). Add `SwitchBranchRequest` and `switch_conversation_branch`, which acquires through `acquire_exclusive_for_branch`. Register the route in `crates/web-ui/src/api/mod.rs` next to line 257 and document it with utoipa (`X-Space-Id`, `ErrorBody`).
- [ ] 6.5 Run `make dump-openapi && make generate-flutter-client` and commit both. GREEN.

## PR 7a — `feat(web-ui): edit and regenerate message endpoints`

Layer: server
Depends on: PR 4c, PR 4d, PR 5, PR 6, `follow-up-suggestions` PR 3 (`RunOptions.suggestions`), `tool-approval-prompts` PR 6 (`TurnRequest.interactive`) and PR 7 (broker, for the end-to-end test)
Parallel with: PR 8a, PR 9–PR 11. `generative-ui-a2ui` PR 11 depends on the pre-assigned `user_message_id` from this PR.

- [ ] 7a.1 RED: handler tests for `POST .../messages/{message_id}/edit`: SSE starts with `run_started{run_id, user_message_id}` and ends with `done`; resulting path; 400 for an assistant message; 400 for a blank message; 404 for an unknown or foreign message; 403 without `conversations:write`; 403 for a read-only member; an edit of an attachment-bearing message keeps its attachments; an edit in a conversation whose interface is a messenger is accepted and posts nothing to the messenger. All errors use the envelope.
- [ ] 7a.2 RED: handler tests for `POST .../messages/{message_id}/regenerate`, with a user id and with a reply-row id that resolves to the user ancestor: one new reply and no new user row; 403 without `conversations:write`.
- [ ] 7a.3 RED: concurrent edits: exactly one 200 and one 409. Edit and regenerate return 409 while a send is working. A plain send returns 409 while an edit or regenerate runs, and stores no user message.
- [ ] 7a.4 RED: a turn blocked on a pending approval: edit and regenerate each cancel the wait and are accepted, and the gated tool does not run (end to end through the broker).
- [ ] 7a.5 RED: `send_message`'s `run_started` payload includes `user_message_id`, the persisted user row has that id, and replaying from `since=0` returns it.
- [ ] 7a.6 RED: `send_message`, `edit_message` and `regenerate_message` all build their options through `RunOptions::web_turn(anchor)`: with the web-turn `interactive` flag set either way, the three submit the same value, and `suggestions` is true for all three. A test that edit and regenerate are not interactive while `web_turn` leaves the flag off.
- [ ] 7a.7 Add `anchor` to `RunOptions` and the `web_turn` constructor. Implement `edit_message` and `regenerate_message` on top of `stream_turn_to_sse` and `Orchestrator::submit`, acquiring through `acquire_exclusive_for_branch` before submitting. `send_message` acquires a shared lease, returns 409 while an exclusive lease is held, and pre-assigns `user_message_id`. Do not set `interactive` here: `tool-approval-prompts`' enabling PR flips it for all three entry points, after the client cards. Document everything with utoipa. Add edit and regenerate coverage to `tool-approval-prompts`' "only streaming web chat turns are interactive" tests.
- [ ] 7a.8 Run `make dump-openapi && make generate-flutter-client` and commit both. GREEN.

## PR 7b — `fix(web-ui): anchor-aware cancelled reply for branch turns`

Layer: server
Depends on: PR 7a, `chat-stop-cancels-turn` PR 2 (`persist_cancelled_reply`, `finish_reason`)
Parallel with: PR 8a, PR 8b

- [ ] 7b.1 RED: runtime tests: a cancelled regenerate with no persisted row saves the partial as a child of U; a cancelled regenerate with no text saves a "_Stopped._" child of U with the cancelled `finish_reason`; a cancelled regenerate after a tool-call row appends after it, with nothing under the old reply; a cancelled edit appends under `U'`; the race guard ignores rows without this `run_id` (the old reply); a regenerate cancelled before a worker claimed it persists nothing and the old reply stays active.
- [ ] 7b.2 Change `persist_cancelled_reply` to take `{ run_id, anchor }` from the runner. Use `branch_message(row, Some(U))` when the run has persisted nothing, and limit the race guard to the run's rows. GREEN.

## PR 8a — `feat(runtime): per-turn model override and recorded model`

Layer: server
Depends on: PR 4a, `message-feedback` PR 3 (`persist_final_answer`), `tool-result-renderers` S3 (lane order)
Parallel with: PR 7a, PR 7b

- [ ] 8a.1 RED: parse `[llm] available_models` from `org.toml` (`crates/core/src/types/llm.rs:97`), defaulting to empty.
- [ ] 8a.2 RED: an `llm_for` (`mod.rs:480`) precedence test: the turn's `model_override` beats the `/model` conversation override, which beats the default, and the turn override is not written to `conversation_configs`.
- [ ] 8a.3 RED: assistant rows saved by a turn carry the provider's `model_name()` in `Message.model`.
- [ ] 8a.4 Add migration `060_message_model.sql` (`messages.model TEXT NULL`) and register it. Add `model: Option<String>` with `#[serde(default)]` to `Message`. Add `model` to `FinalAnswerMeta` and set it in `persist_final_answer`. The model name reaches it through `TurnStore`, with no new parameters at the `dispatch.rs`/`turn_control.rs` save sites. GREEN.

## PR 8b — `feat(web-ui): list models and regenerate with a model`

Layer: server
Depends on: PR 7a, PR 8a
Parallel with: PR 7b

- [ ] 8b.1 RED: `GET /api/models` returns the default first with `is_default: true`, then the listed models, as a bare array; 401 without credentials; 200 for a read-only member.
- [ ] 8b.2 RED: regenerate with a listed model runs on it, and `MessageSummary.model` reports it; a member with write access who is not a space admin may do so; an unlisted model returns 400 with the envelope and starts no turn; the next normal send uses the default.
- [ ] 8b.3 Add `ModelSummary` and `list_models`, validate `RegenerateMessageRequest.model` and pass it as `model_override`, and expose `MessageSummary.model`. No admin check: the allowlist is the cost control. Document `available_models` in `docs/operations/configuration.md`.
- [ ] 8b.4 Run `make dump-openapi && make generate-flutter-client` and commit both. GREEN.

## PR 9 — `feat(app): BranchPicker and MessageEditor elements`

Layer: client (`app/packages/assistant_ui` only)
Depends on: `chat-element-library` PR 1 (package), PR 3 (widgetbook and `kAssistantUiElements`)
Parallel with: all server PRs

- [ ] 9.1 RED: `app/packages/assistant_ui/test/branch_picker_test.dart`: the "2/3" label, controls disabled at the ends, both disabled when `busy`, and the semantics label "Version 2 of 3".
- [ ] 9.2 Implement `BranchPicker` as a pure widget. Add goldens for first, middle, last and busy, in light and dark.
- [ ] 9.3 RED: `message_editor_test.dart`: Send disabled for blank or unchanged text, Escape fires `onCancel`, Send fires `onSubmit` with the trimmed text, and attachment widgets passed in `attachments` render read-only beneath the field.
- [ ] 9.4 Implement `MessageEditor` (with the optional `attachments` slot) and its goldens.
- [ ] 9.5 Register both in `kAssistantUiElements` and add widgetbook use-cases with `index`/`count`/`busy` knobs and an attachments variant. Update the `ELEMENTS.md` row "Message branches" (already "Adopt via message-branching") to name `BranchPicker` and `MessageEditor`. The package allowlist is unchanged.

## PR 10 — `feat(app): RegenerateMenuButton and branch entries in buildMessageActions`

Layer: client
Depends on: PR 9, `chat-element-library` PR 5 (`awaitingInput`) and PR 8a (`MessageActionButton`, `MessageActions` with `leading` and list `trailing`, `buildMessageActions`)
Parallel with: PR 6, PR 11

- [ ] 10.1 RED: `regenerate_menu_button_test.dart`: hidden with fewer than two models; selecting an entry fires `onSelected(name)`; the default is marked.
- [ ] 10.2 Implement `RegenerateMenuButton` in `assistant_ui`, with goldens and a widgetbook use-case. `MessageActions` itself is not changed.
- [ ] 10.3 RED: unit tests for `buildMessageActions` (`app/lib/features/chat/message_actions_builder.dart`): Edit for persisted user messages, with and without attachments; Regenerate only for persisted assistant messages; both offered while no turn is streaming or the turn is `awaitingInput`, and withheld while a turn works; `leading` is a `BranchPicker` when `branch` is present, with `busy` true only while a turn works; the regenerate menu is included only when `onRegenerateWith` and two or more models are supplied.
- [ ] 10.4 Add the entries to `buildMessageActions` behind optional callbacks (`onEdit`, `onRegenerate`, `onRegenerateWith`, `onSwitchBranch`), which stay unwired until PR 12c. GREEN.

## PR 11 — `feat(app): branch-aware history and switch-branch wiring`

Layer: client
Depends on: PR 6, PR 9, `chat-element-library` PR 4 (`ChatMessage` in `assistant_ui`)
Parallel with: PR 7a, PR 10

- [ ] 11.1 RED: `chat_history_mapping_test.dart`: `chatMessagesFromHistory` (`app/lib/features/chat/chat_history_mapping.dart:32`) carries `parentId`, `branch` and `replyBranch` onto `ChatMessage`; tool-only rows are still skipped.
- [ ] 11.2 Add `parentId`, `BranchInfo? branch` and `BranchInfo? replyBranch` to `ChatMessage` in `assistant_ui` models, and map them.
- [ ] 11.3 RED: `ChatNotifier.switchBranch(messageId)` tests: loading sets the picker busy; success replaces the messages with the returned detail; a 409 or network error leaves the messages unchanged and sets an error.
- [ ] 11.4 Implement `switchBranch` through the generated `assistant_api` client, in its own file rather than growing `chat_provider.dart`. GREEN.

## PR 12a — `feat(app): stream edit and regenerate responses`

Layer: client
Depends on: PR 7a, the stream-loop lane (`chat-stop-cancels-turn` PR 4 and PR 5, then `follow-up-suggestions` PR 7)
Parallel with: PR 8a, PR 8b, PR 10, PR 11

- [ ] 12a.1 RED: `stream_event_test.dart`: `RunStartedEvent` parses the optional `user_message_id`; unknown fields are ignored.
- [ ] 12a.2 RED: `api_client` tests: `streamEdit(conversationId, messageId, text)` and `streamRegenerate(conversationId, messageId, {model})` POST to the new routes, yield `RunStartedEvent` from `X-Run-Id`, and parse SSE like `streamMessages` (`app/lib/api/api_client.dart:265`); regenerate surfaces a 404 as a typed error; `streamMessages` surfaces a 409 as a typed error.
- [ ] 12a.3 Implement both and the `RunStartedEvent` field. Update every exhaustive `switch` over `StreamEvent`. GREEN.

## PR 12b — `feat(app): edit, regenerate and retry in ChatNotifier`

Layer: client
Depends on: PR 12a, `tool-approval-prompts` PR 15 (client interaction events, to know a turn is `awaitingInput`)
Parallel with: PR 10, PR 11

- [ ] 12b.1 RED: `ChatNotifier` tests for `editMessage` (truncates after the message, adds a sending bubble that keeps the original attachments, streams, reloads the detail on `done`, marks the bubble failed on `agent_error`) and `regenerate` (truncates from the reply, streams, reloads; an `agent_error` cancelled before any row reloads and shows the old reply). The optimistic id is replaced by `user_message_id`. Both refuse while a turn works.
- [ ] 12b.2 RED: tests that `editMessage`, `regenerate` and `switchBranch` are accepted while the turn is `awaitingInput`: the old run's cancellation (`interaction_resolved cancelled` and `agent_error` cancelled) ends that stream without an error banner or a failed bubble, and the timeline shows only the new branch.
- [ ] 12b.3 RED: `retryMessage` tests (`chat_provider.dart:448-470`): with a server id, a replay ending in `agent_error`, 404 or 410 calls `regenerate`; a regenerate 404 falls back to `sendMessage`; without a server id, `sendMessage` as before; a failed edit retries by regenerating `U'`, and when that returns 404 it re-issues `edit(U, text)` and never calls `sendMessage`.
- [ ] 12b.4 RED: a send that the server answers with 409 keeps the draft, marks the message failed with Retry and shows the error.
- [ ] 12b.5 RED: a reconnect test: the stream drops during an edit, `attemptReconnect` (`chat_provider.dart:1083`) completes, and the reloaded timeline shows the edited message with `branch.count == 2` and nothing from the old branch.
- [ ] 12b.6 Move the stream consumption shared with `_streamMessage` (`chat_provider.dart:692`) into `chat_stream_session.dart`, keeping the iterator design from `follow-up-suggestions` PR 7 (wrapped by `_untilDetached`, draining at `done` or at stopped reconciliation). Implement the notifier methods and the retry rule. GREEN.

## PR 12c — `feat(app): edit, regenerate and branch picker in the chat timeline`

Layer: client
Depends on: PR 10, PR 11, PR 12b
Parallel with: PR 8b

- [ ] 12c.1 RED: widget tests: tapping Edit swaps the bubble for a `MessageEditor` (with the message's attachments shown read-only) and Send calls `editMessage`; tapping Regenerate calls `regenerate`; a picker renders under a user row with `replyBranch`; pickers are disabled while a turn works and enabled while it is `awaitingInput`; Edit is offered on a message with attachments.
- [ ] 12c.2 Wire the `buildMessageActions` callbacks to the notifier, in the chat adapter's wiring file rather than in `chat_screen.dart`.
- [ ] 12c.3 Regenerate the Playwright baselines for the picker, actions and editor with the e2e-testing skill, and record the diff in the PR body. GREEN.

## PR 13 — `feat(app): regenerate with another model`

Layer: client
Depends on: PR 8b, PR 12c
Parallel with: none (wave 7)

- [ ] 13.1 RED: `modelsProvider` tests: loading, success, error (menu hidden, no crash) and an empty or single entry (menu hidden).
- [ ] 13.2 Implement `modelsProvider` over `GET /api/models` and pass `models` and `onRegenerateWith` into `buildMessageActions`. Any member sees the menu when two or more models are listed.
- [ ] 13.3 RED, then implement: the picker tooltip shows the model name when the versions' `model` values differ. GREEN.

## PR 14 — `docs(web-ui): conversation branching guide and ship`

Layer: both
Depends on: PR 3b, PR 7b, PR 13
Parallel with: none (wave 8)

- [ ] 14.1 Add an "Editing, regenerating and branches" section to `docs/web-ui.md`, covering how messengers continue the active branch (and that edit and regenerate reply on the web only), that edit keeps attachments, that branching while an approval or question is pending cancels it, that a send during an edit or regenerate is rejected while a messenger message is deferred, that compaction no longer hides earlier messages, and that the in-flight guard is per process.
- [ ] 14.2 Run `make lint && make format && make test`.
- [ ] 14.3 Run `make lint-flutter && make test-flutter`.
- [ ] 14.4 Manual smoke test: edit the first message (second root), edit a message with an attachment, regenerate twice, cancel a regenerate mid-stream, edit while an approval card is showing, switch across nested forks, regenerate with another model as a non-admin member, force `/compact` and then edit before the summary point, and send from a messenger after a web branch switch and during a web regenerate.
- [ ] 14.5 Archive with `openspec archive message-branching`.
