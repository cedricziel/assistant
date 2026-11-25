# Tasks: message-branching

A stack of PRs. Each stays under ~500 changed lines (generated `assistant_api` code and `openapi.json` excluded) and leaves `main` shippable. Every PR starts with a failing test (TDD).

**Rules that apply to every PR:**

- **Migrations.** Use the numbers the cross-change stack plan allocates to this change. Without an allocation, use the next free number after `045_skill_executions.sql` at implementation time. Register each migration in the `run_migrations` list in `crates/storage/src/lib.rs:297`.
- **New `Message` fields** carry `#[serde(default)]`.
- **Generated files.** Never hand-merge `openapi.json`, `app/packages/assistant_api` or Playwright PNGs. After a rebase, rerun `make dump-openapi && make generate-flutter-client`, regenerate baselines with the e2e-testing skill, and record the visual diff in the PR body.
- **Panic-free.** Run `cargo clippy -D warnings` with no unwrap, expect or panic in production code.

## Parallelism

- **Start immediately (wave 1):** PR 0 (ADR, together with ADR-0017 and ADR-0019), PR 1 (tree), PR 3 (summary store) and PR 6 (runner extraction).
- **After PR 1:** PR 2 runs in parallel with PR 4. PR 4 also needs PR 3. PR 5 follows PR 4.
- **Wave 3 (after `chat-stop-cancels-turn` PR 1 and `tool-approval-prompts` PR 6):** PR 7, then PR 8 and PR 9 in parallel (PR 9 also needs PR 2). PR 10 needs PR 7 and `message-feedback` PR 3.
- **Web:** PR 11 after PR 2, PR 8, `tool-result-renderers` S4 and `conversation-list-management` PR 1. PR 12 after PR 6, PR 8, PR 9 and PR 11. PR 13 after PR 12 and `chat-stop-cancels-turn` PR 2. PR 14 after PR 10 and PR 12.
- **Client:** PR 15 can start once `chat-element-library` PR 1 and PR 3 have landed, in parallel with all server PRs. PR 16 after PR 15 and `chat-element-library` PR 8a. PR 17 after PR 11, PR 15 and `chat-element-library` PR 4. PR 18 after PR 12 and the stream-loop lane. PR 19 after PR 18. PR 20 after PR 16, PR 17 and PR 19. PR 21 after PR 14 and PR 20. PR 22 last.

```
PR0
PR1 ─┬─ PR2 ──────────────┬──────────────┐
     └─ PR4 ─ PR5         │              │
PR3 ───┘                  │              │
(chat-stop 1, tool-appr 6) ─ PR7 ─┬─ PR8 ─┼─ PR11 ─┐
                                  ├─ PR9 ─┘        ├─ PR12 ─┬─ PR13
                                  └─ PR10 ──────── │ ───────┴─ PR14
PR6 ───────────────────────────────────────────────┘
PR15 ─┬─ PR16 ───────────────┐
      └─ PR17 (+PR11) ───────┼─ PR20 ─ PR21 ─ PR22
PR12 ─ PR18 ─ PR19 ──────────┘
```

---

## PR 0: `docs(adr): ADR-0018 conversation message tree`

Layer: server (docs only) · Depends on: none · Parallel with: everything. Land in wave 1 together with ADR-0017 (`tool-approval-prompts` PR 1) and ADR-0019 (`generative-ui-a2ui` PR 1).

- [ ] 0.1 Write `docs/adr/adr-0018-conversation-message-tree.md` covering design.md D1 (adjacency list, no self-cascade, `run_id`), D2/D3/D3a (append-at-leaf, anchored turns, redelivery), D5 (non-destructive compaction), D7 (turn registry and its per-process limitation), with the rejected alternatives. Cite `adr-0011-tool-permission-decision-model.md` by filename where relevant.
- [ ] 0.2 Add "Branch", "Active path" and "Active leaf" to `docs/glossary.md`.

## PR 1: `feat(storage): persist messages as a tree with an active leaf`

Layer: server · Depends on: none · Parallel with: PR 0, PR 3, PR 6. **Lane:** first in the `Message`-struct lane (then `tool-result-renderers` S3, `message-feedback` PR 3, PR 10 here).

- [ ] 1.1 RED: add contract tests in `crates/storage/tests/contract/conversation_store.rs`, run against both impls: (a) sequential `save_message` calls form a chain and the active leaf is the last row; (b) `load_history` follows the active path; (c) an empty conversation returns `[]`; (d) a 1,000-row chain loads in order; (e) re-saving an existing id inserts nothing and leaves the leaf in place; (f) deleting a conversation with a 1,500-row chain succeeds. Confirm RED.
- [ ] 1.2 RED: a contract test for `replace_history` followed by `load_history`: the history is non-empty and equals the re-inserted rows, chained. Confirm RED (FK violation or empty history).
- [ ] 1.3 RED: a migration test that seeds a three-row conversation under the pre-migration schema, applies migrations, and asserts the backfilled `parent_id` chain and `active_leaf_id`.
- [ ] 1.4 Add migration `NNN_message_tree.sql`: `messages.parent_id TEXT NULL REFERENCES messages(id)` (no `ON DELETE` action), `messages.run_id TEXT NULL`, index `(conversation_id, parent_id)`, index `(conversation_id, run_id)`, `conversations.active_leaf_id TEXT NULL`, and the `LAG()` backfill over `(turn, created_at, rowid)`. Register it in `crates/storage/src/lib.rs:297`.
- [ ] 1.5 Add `parent_id: Option<Uuid>` and `run_id: Option<Uuid>` with `#[serde(default)]` to `Message` (`crates/core/src/types/conversation.rs:33`), both `None` in `Message::new`.
- [ ] 1.6 `SqliteConversationStore::save_message` (`crates/storage/src/conversations.rs:413-450`): in one `BEGIN IMMEDIATE` transaction, read the leaf, insert with that parent, and move the leaf and `updated_at` only when the insert affected a row. Do the same in `InMemoryConversationStore`.
- [ ] 1.7 `load_history` (`conversations.rs:453-465`): a recursive CTE from `active_leaf_id`, root first. `last_messages` returns the tail of the path. Do the same in-memory.
- [ ] 1.8 `replace_history` (`conversations.rs:369-386`): run the `DELETE` and `UPDATE conversations SET active_leaf_id = NULL` in one transaction before the re-insert loop. Confirm 1.1–1.3 GREEN. Run `cargo test -p assistant-storage -p assistant-runtime`.

## PR 2: `feat(storage): branch insert and navigation on ConversationStore`

Layer: server · Depends on: PR 1 · Parallel with: PR 3, PR 4, PR 6. **Unblocks:** `conversation-list-management` PR 8, which clears `archived_at` in `branch_message` for user rows too.

- [ ] 2.1 RED: contract tests for `branch_message(msg, Some(parent))` (sibling created, becomes the leaf, old rows kept); `branch_message(msg, None)` (second root); a conflicting `branch_message` leaves the leaf in place; `list_siblings(message_id)` in creation order, with roots as siblings; `ancestors(message_id)` root first; `switch_to(message_id)` descends through the most recent child, returns the new leaf and leaves `updated_at` unchanged; `set_leaf(message_id)` sets the leaf exactly; a foreign-conversation message returns `None`. Confirm RED.
- [ ] 2.2 Add `branch_message`, `list_siblings`, `children_of`, `ancestors`, `switch_to`, `set_leaf` and `delete_run_rows(conversation_id, run_id, keep: Option<Uuid>)` to the `ConversationStore` trait. Implement them in Sqlite (`BEGIN IMMEDIATE`) and InMemory. Emit `ConversationEvent::Upserted` on switch. GREEN.

## PR 3: `feat(storage): conversation summaries store`

Layer: server · Depends on: none · Parallel with: PR 0, PR 1, PR 2, PR 6

- [ ] 3.1 RED: contract tests for `ConversationSummaryStore`: insert, `newest_on_path(conversation_id, path_ids)` picks the newest summary whose `through_message_id` is in the path, none for a diverged path, and summaries are deleted with their conversation. Confirm RED.
- [ ] 3.2 Add migration `NNN_conversation_summaries.sql` (`id, conversation_id REFERENCES conversations ON DELETE CASCADE, through_message_id, content, created_at`) and register it. Add the trait, `SqliteConversationSummaryStore`, `InMemoryConversationSummaryStore`, a `StorageLayer` accessor and the prelude re-export. GREEN.

## PR 4: `refactor(runtime): summary-applied history builder with row-id shadow`

Layer: server · Depends on: PR 1, PR 3 · Parallel with: PR 2, PR 6

- [ ] 4.1 RED: unit tests for `build_turn_history`: with no summary it equals `messages_to_chat_history` over the path; with a summary on the path it starts with the summary text followed by the rows after `through_message_id`; `row_ids.len() == history.len()`.
- [ ] 4.2 RED: a test that the row-id shadow stays the same length as `history` across a tool-call iteration (pushes go through one helper).
- [ ] 4.3 RED: a `/status` test asserting the token estimate is taken over the summary-applied history, and a `/compact` test asserting its input is that history (`command_registry.rs:274-310`).
- [ ] 4.4 Implement `build_turn_history` next to `messages_to_chat_history` (`crates/runtime/src/history.rs:29`). Use it in `prepare_history` (`orchestrator/mod.rs:1535`), `/status` and `/compact`. Keep `base_turn` computed from the raw path length. Thread `row_ids` through the compaction call sites (`mod.rs:815`, `913`, `1207`). Compaction still persists through `replace_history`. GREEN.

## PR 5: `refactor(runtime): non-destructive context compaction`

Layer: server · Depends on: PR 4 · Parallel with: PR 2, PR 6–PR 10

- [ ] 5.1 RED: change `maybe_compact_persists_to_conversation_store` (`crates/runtime/src/compaction.rs:788`) to assert that every original message is still stored after compaction and that a summary row exists with `through_message_id` = the last summarised message.
- [ ] 5.2 RED: an orchestrator test: the next turn's LLM request starts with the summary, followed by the post-summary path. A branch created before `through_message_id` gets no summary. An unresolvable split skips persistence.
- [ ] 5.3 Replace `persist_compacted` (`compaction.rs:355`) with a summary insert keyed by `row_ids[split - 1]`.
- [ ] 5.4 Delete `ConversationStore::replace_history` from the trait and both impls, plus its contract tests (`crates/storage/tests/contract/conversation_store.rs:164` and the PR 1.2 test). No shim. GREEN.
- [ ] 5.5 Run the Playwright suite. If a compacted conversation is in a baseline, regenerate it and record the diff: earlier messages are visible again.

## PR 6: `refactor(web-ui): extract the streamed turn runner from send_message`

Layer: server · Depends on: none · Parallel with: PR 0–PR 5. **The only extraction of this loop.** `chat-stop-cancels-turn` PR 2 and PR 3 (voice), `tool-approval-prompts` PR 7 and PR 12, `follow-up-suggestions` PR 3 and `chat-a11y-and-code-blocks` S3 edit `stream_turn_to_sse`, not `send_message`, and list this PR in their Depends on.

- [ ] 6.1 RED (characterisation): pin the current `send_message` frame sequence in `crates/web-ui/src/api/messages.rs` tests: `run_started` at seq 0, thinking batching every 20 tokens, `done` with `message_id`, `agent_error{reason: cancelled}`, and the push dispatch. Add only the ones not already covered.
- [ ] 6.2 Move the post-`run_started` SSE, persistence, push and terminal logic (`messages.rs:186-486`) into `stream_turn_to_sse(ctx, state, run_id, conv_id, opts: RunOptions, turn: oneshot::Receiver<Result<TurnResult>>) -> Response`. `RunOptions` starts with `push: bool`. Later PRs add `interactive` (`tool-approval-prompts` PR 6), `suggestions` (`follow-up-suggestions` PR 3) and `anchor` (PR 12 here), each in the first PR that reads it. `send_message` becomes validation, submit and one call. No behaviour change. GREEN.

## PR 7: `refactor(runtime): SubmitTurn options struct`

Layer: server · Depends on: `chat-stop-cancels-turn` PR 1, `tool-approval-prompts` PR 6 (wave 3) · Parallel with: PR 2–PR 6

- [ ] 7.1 RED: a test that `TurnRequest` deserialised from JSON without the new fields gets `anchor = Append`, `user_message_id = None` and `model_override = None`, and a test that `SubmitTurn::new(prompt, conv, interface)` defaults match today's `submit_turn` behaviour.
- [ ] 7.2 Add `TurnAnchor` and the `anchor`, `user_message_id` and `model_override` fields with `#[serde(default)]` to `TurnRequest` (`crates/core/src/bus_messages.rs:60`). They are carried but not yet acted on.
- [ ] 7.3 Replace the positional `submit_turn_internal` (`crates/runtime/src/orchestrator/worker.rs:727`) and its `submit_turn_*` variants (including `submit_turn_with_request_id`, `worker.rs:701`) with `Orchestrator::submit(SubmitTurn)`. `SubmitTurn` absorbs `interactive` from `tool-approval-prompts` PR 6. Update the callers (web-ui, channel runner, CLI, tests). Remove the `too_many_arguments` allows. GREEN.

## PR 8: `feat(runtime): one orchestrator-owned turn registry`

Layer: server · Depends on: PR 7 · Parallel with: PR 9, PR 10

- [ ] 8.1 RED: `TurnRegistry` unit tests: concurrent `try_begin(Exclusive)` admits exactly one; Shared is refused while Exclusive is held; Exclusive is refused while Shared is held; dropping a lease releases it, including on a dropped future and an error path.
- [ ] 8.2 RED: an orchestrator test that a turn published straight to the bus (as the scheduler does, `scheduler/tasks.rs:206`) is registered while the worker runs it. A worker that claims a turn while an exclusive lease is held defers it with `nack_delayed`.
- [ ] 8.3 RED: a web `/stop` test: a running web turn is found and cancelled (today `ApiState.active_turns` is never written).
- [ ] 8.4 Implement `TurnRegistry` on `Orchestrator` (`std::sync::Mutex`, never held across `.await`, release on `Drop`). Acquire in `Orchestrator::submit` when the caller passed no lease, and in the worker for unregistered claims. Remove `ApiState.active_turns` (`crates/web-ui/src/api/mod.rs:152`), `ChannelRunner.active_turns` (`channel_runner.rs:64`, `427`, `438`) and the CLI map (`interface-cli/src/main.rs:355`). Point `CommandContext` at the registry. Document the per-process limitation in the rustdoc. GREEN.

## PR 9: `feat(runtime): edit and regenerate turn anchors with idempotent redelivery`

Layer: server · Depends on: PR 2, PR 7 · Parallel with: PR 8, PR 10

- [ ] 9.1 RED: worker tests with `ScriptedLlmProvider`: (a) `Edit{parent}` persists `U'` under `parent` with the pre-assigned `user_message_id`, and the reply chains after it; (b) `Regenerate{U}` persists no user row, sends U's ancestor chain including U's attachments, and makes the first reply row a child of U; (c) a regenerate whose provider errors before output leaves the leaf unchanged; (d) edit and regenerate of a message on an inactive branch use that message's ancestor chain; (e) `base_turn` per anchor matches design D4.
- [ ] 9.2 RED: redelivery tests that nack once with a transient error after a row was persisted, then redeliver: Regenerate ends with exactly two children under U; Edit ends with one `U'`; Append ends with one user row. Every row carries the run's `run_id`.
- [ ] 9.3 Add `TurnStore` (wraps the Sqlite store, stamps `run_id`, holds an optional pending branch parent) and return it from `prepare_history` (`mod.rs:1535`). Handle the anchors there: discard the run's partial rows with `delete_run_rows`, reset the leaf with `set_leaf`, re-save the user row by its fixed id, and arm the pending parent. In the worker, fix `run_id` and `user_message_id` before the first run. GREEN.

## PR 10: `feat(runtime): per-turn model override and recorded model`

Layer: server · Depends on: PR 7, `message-feedback` PR 3 (`persist_final_answer`), `tool-result-renderers` S3 (lane order) · Parallel with: PR 8, PR 9

- [ ] 10.1 RED: parse `[llm] available_models` from `org.toml` (`crates/core/src/types/llm.rs:97`), defaulting to empty.
- [ ] 10.2 RED: an `llm_for` (`mod.rs:480`) precedence test: the turn's `model_override` beats the `/model` conversation override, which beats the default, and the turn override is not written to `conversation_configs`.
- [ ] 10.3 RED: assistant rows saved by a turn carry the provider's `model_name()` in `Message.model`.
- [ ] 10.4 Add migration `NNN_message_model.sql` (`messages.model TEXT NULL`) and register it. Add `model: Option<String>` with `#[serde(default)]` to `Message`. Add `model` to `FinalAnswerMeta` and set it in `persist_final_answer`. The model name reaches it through `TurnStore`, with no new parameters at the `dispatch.rs`/`turn_control.rs` save sites. GREEN.

## PR 11: `feat(web-ui): branch metadata and switch-branch endpoint`

Layer: server · Depends on: PR 2, PR 8, `tool-result-renderers` S4 (tool_call_id join), `conversation-list-management` PR 1 (summary projection) · Parallel with: PR 9, PR 10

- [ ] 11.1 RED: handler tests for `GET /api/conversations/{id}`: `parent_id` is omitted for roots; `branch` appears on edited user messages; reply `branch` is projected onto the last visible assistant row when the first rows are tool-call or thinking rows; `reply_branch` sits on the user row when the active reply has no visible assistant row; `branch` is omitted without siblings.
- [ ] 11.2 RED: tests for `POST /api/conversations/{id}/switch-branch`: 200 with the new path; 404 for a foreign message; 400 for an invalid UUID; 403 without `conversations:write`; 403 for a read-only member; 409 while a turn is in flight; `updated_at` unchanged. Every error has an `{"error": ...}` body.
- [ ] 11.3 Add an envelope-returning `owned_conversation_json` beside `owned_conversation` (`conversations.rs:50-62`), built on `json_error` (`crates/web-ui/src/errors.rs:25`), for the new handlers.
- [ ] 11.4 Add `BranchInfo` and `parent_id`/`branch`/`reply_branch` to `MessageSummary` (`conversations.rs:105`) and the projection in `get_conversation` (`conversations.rs:401`). Add `SwitchBranchRequest` and `switch_conversation_branch`, which takes an exclusive lease. Register the route in `crates/web-ui/src/api/mod.rs` next to line 257 and document it with utoipa (`X-Space-Id`, `ErrorBody`).
- [ ] 11.5 Run `make dump-openapi && make generate-flutter-client` and commit both. GREEN.

## PR 12: `feat(web-ui): edit and regenerate message endpoints`

Layer: server · Depends on: PR 6, PR 8, PR 9, PR 11, `tool-approval-prompts` PR 6 (`TurnRequest.interactive`) · Parallel with: PR 10, PR 15–PR 17

- [ ] 12.1 RED: handler tests for `POST .../messages/{message_id}/edit`: SSE starts with `run_started{run_id, user_message_id}` and ends with `done`; resulting path; 400 for an assistant message; 400 for a blank message; 404 for an unknown or foreign message; 403 without `conversations:write`; 403 for a read-only member. All errors use the envelope.
- [ ] 12.2 RED: handler tests for `POST .../messages/{message_id}/regenerate`, with a user id and with a reply-row id that resolves to the user ancestor: one new reply and no new user row; 403 without `conversations:write`.
- [ ] 12.3 RED: concurrent edits: exactly one 200 and one 409. Edit and regenerate return 409 while a send is running.
- [ ] 12.4 RED: `send_message`'s `run_started` payload includes `user_message_id`, the persisted user row has that id, and replaying from `since=0` returns it.
- [ ] 12.5 RED: edit and regenerate turns are submitted with `interactive = true` and `suggestions = true`.
- [ ] 12.6 Add `anchor` to `RunOptions`. Implement `edit_message` and `regenerate_message` on top of `stream_turn_to_sse` and `Orchestrator::submit`, acquiring the exclusive lease before submitting. `send_message` acquires a shared lease and pre-assigns `user_message_id`. Document everything with utoipa. Update `tool-approval-prompts`' "Only streaming web chat turns are interactive" coverage with edit and regenerate tests here.
- [ ] 12.7 Run `make dump-openapi && make generate-flutter-client` and commit both. GREEN.

## PR 13: `fix(web-ui): anchor-aware cancelled reply for branch turns`

Layer: server · Depends on: PR 12, `chat-stop-cancels-turn` PR 2 (`persist_cancelled_reply`) · Parallel with: PR 14

- [ ] 13.1 RED: runtime tests: a cancelled regenerate with no persisted row saves the partial as a child of U; a cancelled regenerate after a tool-call row appends after it, with nothing under the old reply; a cancelled edit appends under `U'`; the race guard ignores rows without this `run_id` (the old reply).
- [ ] 13.2 Change `persist_cancelled_reply` to take `{ run_id, anchor }` from the runner. Use `branch_message(row, Some(U))` when the run has persisted nothing, and limit the race guard to the run's rows. GREEN.

## PR 14: `feat(web-ui): list models and regenerate with a model`

Layer: server · Depends on: PR 10, PR 12 · Parallel with: PR 13

- [ ] 14.1 RED: `GET /api/models` returns the default first with `is_default: true`, then the listed models, as a bare array; 401 without credentials; 200 for a read-only member.
- [ ] 14.2 RED: regenerate with a listed model runs on it, and `MessageSummary.model` reports it; an unlisted model returns 400 with the envelope and starts no turn; the next normal send uses the default.
- [ ] 14.3 Add `ModelSummary` and `list_models`, validate `RegenerateMessageRequest.model` and pass it as `model_override`, and expose `MessageSummary.model`. Document `available_models` in `docs/operations/configuration.md`.
- [ ] 14.4 Run `make dump-openapi && make generate-flutter-client` and commit both. GREEN.

## PR 15: `feat(app): BranchPicker and MessageEditor elements`

Layer: client (`app/packages/assistant_ui` only) · Depends on: `chat-element-library` PR 1 (package), PR 3 (widgetbook and `kAssistantUiElements`) · Parallel with: all server PRs

- [ ] 15.1 RED: `app/packages/assistant_ui/test/branch_picker_test.dart`: the "2/3" label, controls disabled at the ends, both disabled when `busy`, and the semantics label "Version 2 of 3".
- [ ] 15.2 Implement `BranchPicker` as a pure widget. Add goldens for first, middle, last and busy, in light and dark.
- [ ] 15.3 RED: `message_editor_test.dart`: Send disabled for blank or unchanged text, Escape fires `onCancel`, and Send fires `onSubmit` with the trimmed text.
- [ ] 15.4 Implement `MessageEditor` and its goldens.
- [ ] 15.5 Register both in `kAssistantUiElements` and add widgetbook use-cases with `index`/`count`/`busy` knobs. Update the `ELEMENTS.md` row "Message branches" (already "Adopt via message-branching") to name `BranchPicker` and `MessageEditor`. The package allowlist is unchanged.

## PR 16: `feat(app): RegenerateMenuButton and branch entries in buildMessageActions`

Layer: client · Depends on: PR 15, `chat-element-library` PR 8a (`MessageActionButton`, `MessageActions` with `leading` and list `trailing`, `buildMessageActions`) · Parallel with: PR 17, PR 18

- [ ] 16.1 RED: `regenerate_menu_button_test.dart`: hidden with fewer than two models; selecting an entry fires `onSelected(name)`; the default is marked.
- [ ] 16.2 Implement `RegenerateMenuButton` in `assistant_ui`, with goldens and a widgetbook use-case. `MessageActions` itself is not changed.
- [ ] 16.3 RED: unit tests for `buildMessageActions` (`app/lib/features/chat/message_actions_builder.dart`): Edit only for persisted user messages without attachments while not sending; Regenerate only for persisted assistant messages while not sending; `leading` is a `BranchPicker` when `branch` is present; the regenerate menu is included only when `onRegenerateWith` and two or more models are supplied.
- [ ] 16.4 Add the entries to `buildMessageActions` behind optional callbacks (`onEdit`, `onRegenerate`, `onRegenerateWith`, `onSwitchBranch`), which stay unwired until PR 20. GREEN.

## PR 17: `feat(app): branch-aware history and switch-branch wiring`

Layer: client · Depends on: PR 11, PR 15, `chat-element-library` PR 4 (`ChatMessage` in `assistant_ui`) · Parallel with: PR 12–PR 16

- [ ] 17.1 RED: `chat_history_mapping_test.dart`: `chatMessagesFromHistory` (`app/lib/features/chat/chat_history_mapping.dart:32`) carries `parentId`, `branch` and `replyBranch` onto `ChatMessage`; tool-only rows are still skipped.
- [ ] 17.2 Add `parentId`, `BranchInfo? branch` and `BranchInfo? replyBranch` to `ChatMessage` in `assistant_ui` models, and map them.
- [ ] 17.3 RED: `ChatNotifier.switchBranch(messageId)` tests: loading sets the picker busy; success replaces the messages with the returned detail; a 409 or network error leaves the messages unchanged and sets an error.
- [ ] 17.4 Implement `switchBranch` through the generated `assistant_api` client, in its own file rather than growing `chat_provider.dart`. GREEN.

## PR 18: `feat(app): stream edit and regenerate responses`

Layer: client · Depends on: PR 12, the stream-loop lane (`chat-stop-cancels-turn` PR 4 and PR 5, then `follow-up-suggestions` PR 7) · Parallel with: PR 16, PR 17

- [ ] 18.1 RED: `stream_event_test.dart`: `RunStartedEvent` parses the optional `user_message_id`; unknown fields are ignored.
- [ ] 18.2 RED: `api_client` tests: `streamEdit(conversationId, messageId, text)` and `streamRegenerate(conversationId, messageId, {model})` POST to the new routes, yield `RunStartedEvent` from `X-Run-Id`, and parse SSE like `streamMessages` (`app/lib/api/api_client.dart:265`); regenerate surfaces a 404 as a typed error.
- [ ] 18.3 Implement both and the `RunStartedEvent` field. Update every exhaustive `switch` over `StreamEvent`. GREEN.

## PR 19: `feat(app): edit, regenerate and retry in ChatNotifier`

Layer: client · Depends on: PR 18 · Parallel with: PR 16, PR 17

- [ ] 19.1 RED: `ChatNotifier` tests for `editMessage` (truncates after the message, adds a sending bubble, streams, reloads the detail on `done`, marks the bubble failed on `agent_error`) and `regenerate` (truncates from the reply, streams, reloads). The optimistic id is replaced by `user_message_id`. Both refuse while `isSending`.
- [ ] 19.2 RED: `retryMessage` tests (`chat_provider.dart:448-470`): with a server id, a replay ending in `agent_error`, 404 or 410 calls `regenerate`; a regenerate 404 falls back to `sendMessage`; without a server id, `sendMessage` as before; a failed edit retries by regenerating `U'`.
- [ ] 19.3 RED: a reconnect test: the stream drops during an edit, `attemptReconnect` (`chat_provider.dart:1083`) completes, and the reloaded timeline shows the edited message with `branch.count == 2` and nothing from the old branch.
- [ ] 19.4 Move the stream consumption shared with `_streamMessage` (`chat_provider.dart:692`) into `chat_stream_session.dart`, keeping the iterator design from `follow-up-suggestions` PR 7 (wrapped by `_untilDetached`, draining at `done` or at stopped reconciliation). Implement the notifier methods and the retry rule. GREEN.

## PR 20: `feat(app): edit, regenerate and branch picker in the chat timeline`

Layer: client · Depends on: PR 16, PR 17, PR 19

- [ ] 20.1 RED: widget tests: tapping Edit swaps the bubble for a `MessageEditor` and Send calls `editMessage`; tapping Regenerate calls `regenerate`; a picker renders under a user row with `replyBranch`; pickers are disabled while sending.
- [ ] 20.2 Wire the `buildMessageActions` callbacks to the notifier, in the chat adapter's wiring file rather than in `chat_screen.dart`.
- [ ] 20.3 Regenerate the Playwright baselines for the picker, actions and editor with the e2e-testing skill, and record the diff in the PR body. GREEN.

## PR 21: `feat(app): regenerate with another model`

Layer: client · Depends on: PR 14, PR 20

- [ ] 21.1 RED: `modelsProvider` tests: loading, success, error (menu hidden, no crash) and an empty or single entry (menu hidden).
- [ ] 21.2 Implement `modelsProvider` over `GET /api/models` and pass `models` and `onRegenerateWith` into `buildMessageActions`.
- [ ] 21.3 RED, then implement: the picker tooltip shows the model name when the versions' `model` values differ. GREEN.

## PR 22: `docs(web-ui): conversation branching guide and ship`

Layer: both · Depends on: PR 5, PR 13, PR 21

- [ ] 22.1 Add an "Editing, regenerating and branches" section to `docs/web-ui.md`, covering how messengers continue the active branch, that compaction no longer hides earlier messages, and that the in-flight guard is per process.
- [ ] 22.2 Run `make lint && make format && make test`.
- [ ] 22.3 Run `make lint-flutter && make test-flutter`.
- [ ] 22.4 Manual smoke test: edit the first message (second root), regenerate twice, cancel a regenerate mid-stream, switch across nested forks, regenerate with another model, force `/compact` and then edit before the summary point, and send from a messenger after a web branch switch.
- [ ] 22.5 Archive with `openspec archive message-branching`.
