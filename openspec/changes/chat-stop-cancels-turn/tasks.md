# Tasks — chat-stop-cancels-turn

A stack of nine PRs. Each stays under ~500 changed lines (generated `assistant_api` and `openapi.json` output not counted) and leaves `main` shippable.
One additive migration (`message_finish_reason`, PR 2a), no new SSE event type, no ADR (this change uses the existing cancel endpoint and terminal frame; the column is additive).

```
server:  PR 1 ──► PR 1b ──► PR 2 ──► PR 3
                   ▲         ▲ ▲       ▲
   message-branching PR 5 ───┴─┼───────┘   (shared turn runner, wave 1)
   message-branching PR 1 ──► PR 2a ┘      (messages-table lane, finish_reason)

client:  chat-element-library PR 2 ──► PR 4 ──► PR 5a ──► PR 5b ──► PR 5c
                                         PR 2a ──────────────────────┘
```

**Parallelism**

- **Start immediately:** PR 1 (runtime only, wave 1) and, once `chat-element-library` PR 2 has merged, PR 4. The two tracks are independent of each other.
- **PR 2a** (storage column and API field) needs only `message-branching` PR 1 and can run in parallel with PR 1 and PR 1b.
- **PR 1b** needs PR 1 and `message-branching` PR 5. PR 2 follows PR 1b and PR 2a, and PR 3 follows PR 2, because all three edit the runner's terminal branch.
- **Client:** PR 5a follows PR 4, and PR 5b follows PR 5a. PR 5c follows PR 5b and PR 2a (it reads the regenerated `finish_reason` field). The client is correct against today's server, which already emits `agent_error {reason: "cancelled", partial_content}`. It gains `message_id`, `user_message_persisted`, the "_Stopped._" placeholder and voice cancel as the server PRs land.
- When a sibling refers to "chat-stop PR 5", it means PR 5b, the top of the stream-loop split, which includes PR 5a. PR 5c touches only history mapping, docs and archive, so siblings do not depend on it.

**Ordering edges with siblings**

- **Messages-table lane:** PR 2a follows `message-branching` PR 1, which opens the `Message`-struct lane, and rebases on `tool-result-renderers` S3 and `message-feedback` PR 3 when they have merged first. The stack plan assigns the migration number; the file is named `message_finish_reason`. `message-feedback` ships after `message-branching` D5 and may read `finish_reason`.
- PR 1 lands in wave 1. In wave 2, `tool-approval-prompts` PR 2 and PR 6 follow it. In wave 3, `message-branching` PR 4 follows, and its `SubmitTurn` struct absorbs the `claimed` flag added here.
- PR 1b, PR 2 and PR 3 edit `stream_turn_to_sse` from `message-branching` PR 5, never `send_message`.
- `message-branching` PR 7 depends on PR 2 here. It makes `persist_cancelled_reply` anchor-aware (design D1), and keeps writing `finish_reason` and the "_Stopped._" placeholder.
- PR 4 stacks on `chat-element-library` PR 2, which deletes `tool_call_chip.dart` and `timeline_section.dart`, so PR 4 adds no switch arms to them. `chat-element-library` PR 4 (moves `MessageStatus` / `ToolCallStatus`) comes after PR 4 here and carries `stopped` and `cancelled` along.
- `chat-element-library` PR 9 (progress card) and PR 11 (Composer, with an `isStopping` input) come after PR 5a.
- Client stream loop: PR 4 here, then PR 5b, then `follow-up-suggestions` PR 7, then `message-branching` PR 12. `chat-a11y-and-code-blocks` C2 comes after PR 4.
- Archive order for `turn-status-api`: this change (after PR 5c), then `follow-up-suggestions`, then `tool-approval-prompts`.

TDD is mandatory: every PR starts with a failing test, confirmed red before implementation. Run `make lint` and `make format` before each commit. Rust production code stays panic-free.

---

## PR 1 — `fix(runtime): honour cancel for turns not yet claimed by a worker`

- **Layer:** server
- **Depends on:** none (wave 1)
- **Parallel with:** PR 2a, PR 4, PR 5a, PR 5b, `message-branching` PR 5

### 1. Cancel before claim

- [ ] 1.1 Failing test in `crates/runtime/src/orchestrator/tests/bus_worker.rs`: publish a turn via `submit_turn_with_request_id` with no worker running, call `cancel_turn(request_id)`, and assert that `submit_turn` returns an error containing both `TURN_CANCELLED_MARKER` and `TURN_CANCELLED_BEFORE_START_MARKER`. Then start a worker and assert that the `ScriptedLlmProvider` was **never** called and the bus message is acked. Confirm RED: today the worker runs the turn (`worker.rs:199-210`).
- [ ] 1.2 Failing test (stale request vs newer turn): cancel turn A in conversation C before claim, register a new token sink for turn B in C and publish B, then start one worker. It claims A first and skips it, then runs B, and B's sink receives B's tokens. Confirm RED against a naive skip that removes the sink.
- [ ] 1.3 Add a `claimed` flag to the `turn_cancellations` entries and a bounded `cancelled_requests` tombstone (FIFO `VecDeque<Uuid>` + `HashSet<Uuid>`, capacity 256) on `Orchestrator`. `cancel_turn` cancels the token and, for an unclaimed entry, adds the tombstone under the same write lock. Add `TURN_CANCELLED_BEFORE_START_MARKER` beside `TURN_CANCELLED_MARKER`, and have `submit_turn`'s cancel branch (`worker.rs:840-853`) include it when the removed entry was unclaimed.
- [ ] 1.4 In the worker claim path (`worker.rs:186-210`), **before** removing `extension_registrations` and `token_sinks`: under one `.write().await`, skip a tombstoned request (ack, log `submit_state = "cancelled_before_claim"`, `continue`, touching neither map) or mark the entry `claimed`. This replaces the `try_read()` lookup. Confirm 1.1 and 1.2 GREEN.
- [ ] 1.5 Failing test: a cancel that arrives while another task holds the `turn_cancellations` read lock still cancels the claimed turn (today `try_read` silently runs it uncancellable). GREEN.
- [ ] 1.6 Failing tests for `Orchestrator::release_token_sink(conversation_id, &Sender)`: it removes the sink when `same_channel` matches and leaves a different sender in place. Implement. GREEN.
- [ ] 1.7 Unit test the tombstone bound: the 257th insert evicts the first.
- [ ] 1.8 `cargo clippy -p assistant-runtime -- -D warnings`, `cargo test -p assistant-runtime`.

---

## PR 1b — `fix(web-ui): end the stream at once when a turn is cancelled before it starts`

- **Layer:** server
- **Depends on:** PR 1, `message-branching` PR 5 (`stream_turn_to_sse`)
- **Parallel with:** PR 2a, PR 4, PR 5a, PR 5b

### 2. Cancel checks the run's conversation

- [ ] 2.1 Failing handler test in `crates/web-ui/src/api/turns.rs` tests: start a run in conversation B, then call `POST /api/conversations/A/turns/{runB}/cancel` as the owner of A. Assert `200` with `state: "unknown"`, and that B's token was **not** fired (`is_turn_in_flight` stays true, and B's stream later ends with `done`). Confirm RED (`turns.rs:236` cancels before any check).
- [ ] 2.2 Load the run's event-log rows (as `turn_status` does, `turns.rs:130-160`) and call `orchestrator.cancel_turn` only when the first row's `conversation_id` matches. GREEN.

### 3. Runner: race the turn result against the event stream

- [ ] 3.1 Failing handler test: occupy the only worker with a blocking scripted turn in conversation X. Send a turn in conversation C and cancel it at once. Assert that the C stream ends within a short timeout with `agent_error` whose JSON has `reason: "cancelled"` and `user_message_persisted: false`, and that `GET /api/conversations/C` has no user row for it. Confirm RED: today the loop waits for `event_rx` to close (`messages.rs:224-391`).
- [ ] 3.2 Failing test: a normal turn whose result arrives while token frames are still buffered streams every token before `done` (guards the `biased` drain).
- [ ] 3.3 In `stream_turn_to_sse`, `select!` (biased) on `event_rx` and `turn_result_rx`. On `Ok`, drain `event_rx` to close. On a cancelled `Err`, `try_recv` what is buffered, call `release_token_sink` with the runner's own sender clone, and emit the terminal frame. Add `user_message_persisted: false` when the before-start marker is present. GREEN.
- [ ] 3.4 Update `docs/operations/turn-status-endpoint.md`: cancel before claim, `user_message_persisted`, and the conversation check.
- [ ] 3.5 Update the `cancel_turn` `#[utoipa::path]` description (`turns.rs:200-214`), run `make dump-openapi`, and commit `openapi.json`. Run `make generate-flutter-client` only if the generated client changes.
- [ ] 3.6 `cargo clippy -p assistant-web-ui -- -D warnings`, `cargo test -p assistant-web-ui`.

---

## PR 2a — `feat(storage): persist a finish_reason on messages`

- **Layer:** server
- **Depends on:** `message-branching` PR 1 (opens the messages-table lane)
- **Parallel with:** PR 1, PR 1b, PR 4, PR 5a, PR 5b

### 3a. Column, model and API field

- [ ] 3a.1 Failing contract tests in `crates/storage/tests/contract/conversation_store.rs`, run against both stores: a message saved with `finish_reason: Some(MessageFinishReason::Cancelled)` comes back from `load_history` with it, and a message saved without one comes back with `None`. Confirm RED.
- [ ] 3a.2 Failing migration test: seed a two-row conversation under the pre-migration schema, apply migrations, and assert both rows read `finish_reason = NULL` and nothing else changed.
- [ ] 3a.3 Add `migrations/NNN_message_finish_reason.sql` (`ALTER TABLE messages ADD COLUMN finish_reason TEXT`) and register it in both migration lists in `crates/storage/src/lib.rs`. The number comes from the stack plan; only the descriptive name is fixed.
- [ ] 3a.4 Add `MessageFinishReason` (`Cancelled`, stored as `"cancelled"`; an unknown stored value reads as `None` with a `warn!`) and `finish_reason: Option<MessageFinishReason>` with `#[serde(default)]` on `Message` (`crates/core/src/types/conversation.rs`), `None` in `Message::new`. Write and read it in `SqliteConversationStore` (`save_message`, `load_history`, `last_messages`, `replace_history`) and `InMemoryConversationStore`. Confirm 3a.1 and 3a.2 GREEN.
- [ ] 3a.5 Failing handler test in `crates/web-ui/src/api/conversations.rs` tests: `GET /api/conversations/{id}` returns `finish_reason: "cancelled"` on a stored cancelled assistant message and omits the field on every other message. Add `finish_reason: Option<String>` (`skip_serializing_if = "Option::is_none"`) to `MessageSummary`. RED to GREEN.
- [ ] 3a.6 Run `make dump-openapi`, commit `openapi.json`, then `make generate-flutter-client` and commit the regenerated `app/packages/assistant_api/` (never hand-edit it).
- [ ] 3a.7 `cargo clippy -p assistant-core -p assistant-storage -p assistant-web-ui -- -D warnings`, `cargo test -p assistant-core -p assistant-storage -p assistant-web-ui`.

---

## PR 2 — `feat(web-ui): persist the partial reply of a cancelled turn`

- **Layer:** server
- **Depends on:** PR 1b (same terminal branch), PR 2a (`finish_reason`), `message-branching` PR 5
- **Parallel with:** PR 4, PR 5a, PR 5b

### 4. Runtime: `persist_cancelled_reply`

- [ ] 4.1 Failing tests in `crates/runtime/src/orchestrator/tests/history.rs` with `InMemoryConversationStore` and an in-memory bus:
  - (a) given `[user]`, `persist_cancelled_reply(conv, "Partial", "web")` saves one assistant row with `turn` = history length and `finish_reason = Cancelled`, and returns its id;
  - (b) empty text saves the placeholder row "_Stopped._" with `finish_reason = Cancelled` and returns its id;
  - (b2) whitespace-only text behaves like (b);
  - (c) given `[user, assistant("Full")]`, it saves nothing, returns the existing id and leaves that row's `finish_reason` unset (race guard);
  - (d) given `[user, assistant-toolcall, tool]`, it persists;
  - (e) given `[user, assistant-toolcall(empty content)]`, it persists;
  - (f) given `[user_1, assistant("Old"), user_2]`, it persists (the guard looks only after the last user row);
  - (g) title announcement: a saved row publishes one `turn.completed` whose `TurnResult` carries the conversation id, the turn number, the persisted text and `message_id`, and its `had_errors` is false. No announcement for (c), for a store that fails on save, or when the last user row's content is empty or whitespace.

  Confirm RED.

- [ ] 4.2 Implement `Orchestrator::persist_cancelled_reply(&self, conversation_id, text, interface) -> anyhow::Result<Option<Uuid>>` beside `persist_error_recovery` (`history.rs:208`), with a `CANCELLED_REPLY_PLACEHOLDER = "_Stopped._"` constant. The guard counts only assistant rows with non-empty content and no `tool_calls_json`. When it saves a row and the turn's user message has text, it publishes `turn.completed` through the same helper as `announce_completed_turn` (`worker.rs:598`), and nothing on `turn.result` or webhooks (design D14). If `message-feedback` PR 3 has landed, save through its `persist_final_answer` helper. GREEN.
- [ ] 4.3 Failing end-to-end test (runtime, `ScriptedLlmProvider` plus the title generator's `run_one`): stop the first turn of an untitled conversation after the user message is saved. The conversation gets a title, subject to the generator's eligibility settings (use a config where the first turn qualifies). RED to GREEN.

### 5. Runner: wire it into the cancelled terminal

- [ ] 5.1 Failing handler test: drive a turn with a scripted provider that streams tokens and then blocks, cancel it, and assert that the terminal `agent_error` has `reason: "cancelled"`, the streamed `partial_content`, and a `message_id` equal to the id of a new assistant row in `GET /api/conversations/{id}`. Second case: cancel before any token gives `partial_content: "_Stopped._"` and a `message_id` equal to a new "_Stopped._" assistant row with `finish_reason: "cancelled"`. Third case: a turn cancelled before a worker claims it (`user_message_persisted: false`) gives no `message_id` and no assistant row. Confirm RED.
- [ ] 5.2 Failing test: with a conversation store that fails on save, the cancelled frame is still emitted with `partial_content` (the streamed text, empty when none) and without `message_id`.
- [ ] 5.3 In the runner's cancelled branch, skip persistence when the before-start marker is present. Otherwise call `persist_cancelled_reply(conv_id, &reply_text, "web")`, set `partial_content` to the text it persisted (the placeholder when nothing streamed), add `message_id` when it returns `Some`, and log persistence errors with `warn!`. GREEN.
- [ ] 5.4 Assert that the persisted terminal event row also carries `message_id` (replay parity).
- [ ] 5.5 Update `docs/operations/turn-status-endpoint.md:105-109`: a cancelled turn now persists its partial as an assistant message (or the "_Stopped._" placeholder) with `finish_reason: "cancelled"`, reports `message_id`, and still gets an auto-title. Update the `cancel_turn` OpenAPI description, run `make dump-openapi`, and commit `openapi.json`.
- [ ] 5.6 `cargo clippy -p assistant-web-ui -p assistant-runtime -- -D warnings`, `cargo test -p assistant-web-ui -p assistant-runtime`.

---

## PR 3 — `feat(web-ui): run voice turns on the shared turn runner`

- **Layer:** server
- **Depends on:** PR 2, `message-branching` PR 5
- **Parallel with:** PR 4, PR 5a, PR 5b

### 6. Voice joins the run lifecycle

- [ ] 6.1 Failing tests next to `send_voice_message_done_event_contains_message_id` (`messages.rs:1369`):
  - (a) the response has an `X-Run-Id` header, and the SSE body starts with `run_started` (id 0) and then `transcript`;
  - (b) frames are persisted under that run id, and `GET .../runs/{run}/events/stream?since=0` replays them, `transcript` included;
  - (c) cancelling the run ends the stream with `agent_error {reason: "cancelled"}`;
  - (d) an orchestrator error ends with `agent_error {reason: "error"}`, not `done`;
  - (e) the turn is submitted with `interactive: false` and produces no suggestions.

  Confirm RED.

- [ ] 6.2 Switch `send_voice_message` to `submit_turn_with_request_id(run_id, …)`. Emit and persist `run_started`, then `transcript` (persisted with a seq), and hand off to `stream_turn_to_sse` with `RunOptions { interactive: false, suggestions: false, push: true, anchor: None }`. Set `X-Run-Id`. Delete the old voice SSE loop (`messages.rs:1055-1110`). GREEN.
- [ ] 6.3 Update `docs/operations/turn-status-endpoint.md` (voice turns have a run id and can be cancelled and replayed). Update the `send_voice_message` OpenAPI description, run `make dump-openapi`, commit `openapi.json`, and run `make generate-flutter-client` if the generated client changes.
- [ ] 6.4 `cargo clippy -p assistant-web-ui -- -D warnings`, `cargo test -p assistant-web-ui`.

---

## PR 4 — `fix(app): reconcile cancelled turns from the agent_error frame`

- **Layer:** client
- **Depends on:** `chat-element-library` PR 2. The server side needs nothing new, because today's server already emits the cancelled frame.
- **Parallel with:** PR 1, PR 1b, PR 2a, PR 2, PR 3

### 7. Structured `AgentErrorEvent`

- [ ] 7.1 Failing tests in `app/test/unit/api/client_test.dart`:
  - `event:agent_error` with `data:{"reason":"cancelled","message":"m","partial_content":"p","message_id":"M","user_message_persisted":false}` parses to `AgentErrorEvent(reason: cancelled, message: "m", partialContent: "p", messageId: "M", userMessagePersisted: false)`;
  - a missing `user_message_persisted` gives `true`;
  - `reason:"error"` gives `error`, and an unknown reason gives `error`;
  - a non-JSON body gives `error`, with the body as `message`.

  Update the existing raw-text tests (`client_test.dart:171-177`, `:280`, `:594-599`). Confirm RED.

- [ ] 7.2 Implement `AgentErrorReason` and the new fields (`api/models/stream_event.dart:332-336`), and the parsing (`api/api_client.dart:778-779`). GREEN.

### 8. Statuses

- [ ] 8.1 Add `MessageStatus.stopped` and `ToolCallStatus.cancelled` (`chat_models.dart`), and the switch arms in `streaming_timeline_entry.dart:300` (muted block icon) and `chat_history_mapping.dart:7-15` (`"cancelled"`). Widget test in `test/widget/features/chat/streaming_timeline_entry_test.dart`: a `cancelled` tool call renders the cancelled icon with a "Cancelled" semantic label. RED → GREEN.
- [ ] 8.2 Failing widget test for `MetaActionRow`: an assistant message with status `stopped` shows a `stopped_label` ("Stopped") and the normal Copy action. RED → GREEN in `message_meta_actions.dart`.

### 9. One reconciliation path for all three streams

- [ ] 9.1 Failing unit tests in `test/unit/chat/chat_turn_transitions_test.dart` for `stoppedTurnState(from, {userMsgId, partialContent, messageId, userMessagePersisted})`:
  - the placeholder becomes a `stopped` assistant message, with id = `messageId`, or a synthetic id when it is absent;
  - the user message becomes `ok`, or `failed` when `userMessagePersisted` is false;
  - `pending` tool calls become `cancelled`, and active timeline entries complete;
  - content "_Stopped._" with a `messageId` gives a `stopped` message with that content;
  - empty content and no `messageId` removes the placeholder;
  - `pendingQueue` is preserved.

  Confirm RED, then implement. GREEN.

- [ ] 9.2 Failing provider tests in `test/unit/chat/chat_provider_test.dart`, in a new group "agent_error reconciliation" run against the text, voice and replay paths (the parity-group pattern at `chat_provider_test.dart:3038`):
  - cancelled gives a stopped message with the partial;
  - cancelled with `partial_content: "_Stopped._"` and a `message_id` gives that stopped message;
  - cancelled with `user_message_persisted: false` gives a failed user message and no error;
  - error gives a failed user message and surfaces the error;
  - a cancelled frame during replay makes `_replayRun` return `true` with `replayCallCount == 1`.

  Confirm RED: today the text path falls through to "stream ended without done" (`chat_provider.dart:807-835`), and replay marks the message failed (`chat_provider.dart:525-531`).

- [ ] 9.3 Add `_onAgentError` in `ChatNotifier` and call it from `_streamMessage`, `_streamVoiceMessage` and `_replayRun`. GREEN.
- [ ] 9.4 `make lint-flutter`, `make test-flutter`.

---

## PR 5a — `feat(app): Stop cancels the server turn`

- **Layer:** client
- **Depends on:** PR 4
- **Parallel with:** PR 1, PR 1b, PR 2a, PR 2, PR 3

### 10. `stopTurn()` core

- [ ] 10.1 Failing provider tests in a new group "stopTurn", using `_FakeApiClient` with a recorded `cancelTurn` override and fake async:
  - Stop with a run id sends exactly one cancel and keeps listening;
  - a second Stop is ignored;
  - `isStopping` is true until the terminal frame;
  - `done` after Stop finalizes as `ok`;
  - `stopTurn()` with nothing in flight sends nothing;
  - Stop before `RunStartedEvent` sends the cancel when it arrives, with that id;
  - Stop before a run id followed by an `ErrorEvent` leaves the user message `failed` and `error == null`.

  Confirm RED.

- [ ] 10.2 Implement `stopTurn()`, `ChatState.isStopping`, `ChatStreamSession.stopRequested` and the private `_requestServerCancel()`. **Delete** `cancelStream()` and the public `requestCancelTurn()` (`chat_provider.dart:274-287`, `:348-356`). Port the test `cancelStream during retry backoff stops retrying` (`chat_provider_test.dart:1567`) to `stopTurn`, and assert that the server cancel is sent and no further replay starts. GREEN.

### 11. Wiring and UI

- [ ] 11.1 Failing widget tests in `test/widget/features/chat/chat_screen_test.dart`:
  - tapping `stop_button` and tapping `stop_action` each call `stopTurn` once (spy notifier, as in `turn_progress_card_test.dart:26-40`);
  - with `isStopping`, `stop_button` is disabled with the semantic label "Stopping…", and `stop_action` is disabled.

  Update `turn_progress_card_test.dart:266-290` so that Skip calls `stopTurn`. Confirm RED.

- [ ] 11.2 Point `chat_screen.dart:541-543` and `:595-596`, and `turn_progress_card.dart:135`, at `stopTurn()`. Add an `isStopping` parameter to `ChatInputRow` (`chat_input_row.dart:281-286`) and to the bubble's meta actions. GREEN.
- [ ] 11.3 `make lint-flutter`, `make test-flutter`.

---

## PR 5b — `fix(app): bound every Stop wait and resume the queue`

- **Layer:** client
- **Depends on:** PR 5a
- **Parallel with:** PR 1, PR 1b, PR 2a, PR 2, PR 3

### 12. Detach, grace and refresh

- [ ] 12.1 Failing tests:
  - (a) the cancel returns `null` twice and then `running`: no error;
  - (b) the cancel returns `null` three times: local detach, stopped partial, the "Couldn't reach the server…" error, and the queue resumes;
  - (c) the cancel returns `unknown` with two queued messages: detach and refresh, both queued messages stay in `pendingQueue`, and a refresh that completes after the next queued turn started does not overwrite it;
  - (d) cancel `running`, silence for `stopGrace`, then a `running` probe: detach with the unconfirmed-stop error;
  - (e) the same, with a `completed` probe: detach and refresh, and the full answer from the server is shown, not a `stopped` message;
  - (f) Stop before a run id, with no `RunStartedEvent` within `stopGrace`: local detach, user message `ok`, the "Couldn't confirm the stop…" error, and the queue resumes.

  Confirm RED.

- [ ] 12.2 Implement the retry loop (reusing `ChatStreamSession.retryDelay`), `ChatStreamSession.stopGrace = Duration(seconds: 10)` started in `stopTurn()`, the `detach` completer, and an `_untilDetached` wrapper on the sources in `_streamMessage`, `_streamVoiceMessage` and `_replayRun`. Extract `_refreshFromServer(conversationId)` from `_recoverStalledStream` (`chat_provider.dart:968-995`). GREEN.
- [ ] 12.3 Remove `_session.cancelled`. `_recoverStalledStream` completes `detach`, and the readers at `chat_provider.dart:509`, `:652`, `:766`, `:865` and `:919-941` use `detach.isCompleted`. Port the existing stall-recovery tests (group 12, including "stalled recovery does not clobber a concurrently-started stream") and keep them green.
- [ ] 12.4 Failing test: during a stopped turn with two queued messages and a silent server, the first queued message is sent right after the cancelled frame, with no further event needed. Then a regression test showing that detach also unblocks the drain. RED → GREEN.

### 13. Voice run id

- [ ] 13.1 Failing test in `client_test.dart`: `sendVoiceMessage` yields `RunStartedEvent` from the `X-Run-Id` response header before any body frame, and yields none when the header is absent. Implement in `api_client.dart:357-397`, as `streamMessages` does at `:294-299`. GREEN.
- [ ] 13.2 Failing tests (stale run id): after a completed text turn `T`, a voice turn starts and Stop is tapped before its run id arrives, and no cancel for `T` is sent. Then the voice stream's `RunStartedEvent("V")` leads Stop to cancel `V`. Make `_streamVoiceMessage` call `_session.beginTurn()` and capture `RunStartedEvent` (`chat_provider.dart:595`). RED → GREEN.

---

## PR 5c — `feat(app): show Stopped after reload`

- **Layer:** client
- **Depends on:** PR 5b, PR 2a (regenerated `finish_reason` field), PR 4 (`MessageStatus.stopped`)
- **Parallel with:** PR 1, PR 1b, PR 2, PR 3

### 14. History mapping

- [ ] 14.1 Failing tests in `test/unit/chat/chat_history_mapping_test.dart`: a `MessageSummary` with `finishReason: "cancelled"` maps to `MessageStatus.stopped`, one without maps to `ok`, and an unknown reason maps to `ok`. Failing widget test: after `loadConversation` on history that holds a cancelled reply, the message shows `stopped_label`. Confirm RED.
- [ ] 14.2 Map `finishReason` in `chat_history_mapping.dart` (history load and the stall-recovery refresh `_refreshFromServer`). GREEN.
- [ ] 14.3 Failing test: a live stopped reply and the same reply after a reload have equal status and content, including the "_Stopped._" placeholder case. RED to GREEN.
- [ ] 14.4 `make lint-flutter`, `make test-flutter`.

### 15. Docs, verification and archive

- [ ] 15.1 Update `docs/development/chat-stream-probe.md:74-80`: Skip calls `stopTurn()`, and the doc describes the Stopping state, `stopGrace`, local detach and the refresh. Remove every mention of `requestCancelTurn()`.
- [ ] 15.2 Run the Playwright suite (`crates/web-ui/e2e`, `npx playwright test`). Baselines are expected to stay unchanged, because none of the new states appear on captured screens. If a baseline does move, update it with `npm run test:update` and document the visual diff in the PR.
- [ ] 15.3 Update the `openspec/changes/chat-stop-cancels-turn/specs` deltas if the implementation changed any wording.
- [ ] 15.4 Manual check against a local server (`make build-web`, then run the CLI server). Stop mid-answer: a "Stopped" partial appears, reload shows the partial with the same "Stopped" label, and the next queued message sends at once. Stop before any text: a "_Stopped._" reply appears and survives reload, and a new conversation gets a title. Stop a voice turn. Stop while all workers are busy: the stream ends at once, and the user message is offered for Retry.
- [ ] 15.5 Final phase: `make lint && make format && make test`, then `make lint-flutter && make test-flutter`.
- [ ] 15.6 After every PR in this change has merged, archive it (`openspec archive chat-stop-cancels-turn`, after PR 5c, the top of the stack) **before** `follow-up-suggestions` and `tool-approval-prompts`, which rebase their `turn-status-api` deltas on this one.
