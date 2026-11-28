# Tasks: chat-a11y-and-code-blocks

This change is a stack of 19 PRs. Each stays under about 500 changed lines (the generated `app/packages/assistant_api` diff, `openapi.json` and golden PNGs do not count) and leaves `main` shippable. Every PR starts with a failing test (TDD): write it, run it, confirm RED, then implement.

**Rules that apply to every PR:**

- **Migrations.** `053_message_timings.sql` is the number the cross-change stack plan (`openspec/chat-ui-parity-stack.md`) pre-allocates to this change. Register it in both migration lists in the `run_migrations` list in `crates/storage/src/lib.rs` (the `run_migrations` list, `:273`, ending at `:472`).
- **Decisions.** The product decisions in `openspec/chat-ui-parity-decisions.md` for this change are final and are encoded in design.md Decisions 4, 6 and 9: announce "{Persona} replied: {200-character excerpt}", including after a reconnect replay; compare the chat pane width for the list breakpoint; show timing only behind the "Show response timing" preference, off by default.
- **Generated files.** Never hand-merge `openapi.json`, `app/packages/assistant_api` or Playwright PNGs. After a rebase, rerun `make dump-openapi && make generate-flutter-client`, regenerate baselines with the e2e-testing skill, and record the visual diff in the PR body.
- **Panic-free.** No `unwrap`, `expect` or `panic` in production Rust.
- **Client element PRs** follow chat-element-library's per-element checklist: a `kAssistantUiElements` entry, a widgetbook use-case, a `golden`-tagged test in light and dark, an `ELEMENTS.md` row update, and `flutter analyze --fatal-infos` and `flutter test` green in both the app and the package.
- **Sibling PR numbers** follow chat-element-library's "Sibling unblock map" and each sibling's own tasks.md. Seam names are given next to the numbers.

## Parallelism

```
Server:  S1 (store) ──────┐
         S2a (timed script) ┴─ S2b (TurnResult/bus) ── S2c (measure) ── S3 (done/history) ─┐
Client:  C1 (breakpoints) ───────────────────────────────────────────────────────────────────┤
         C2a (announcer policy) ─┬─ C2b (lifecycle wiring) ─────────────────────────────────┤
                                 └──────────────┐                                            │
         C3 (highlighter) ── C4 (CodeBlock) ────┴─ C5 (fences) ─────────────────────────────┤
         C6 (bubble/timeline semantics) ────────────────────────────────────────────────────┤
         C7 (labels/targets) ── C8 (focus order) ───────────────────────────────────────────┤
         C9 (reduced motion) ───────────────────────────────────────────────────────────────┤
         C10 (timing element) ─┐                                                             │
         C10b (timing setting) ┴─ C11 (timing wiring, needs S3) ───────────────────────────┴─ C12 ─ C13
```

- **Start immediately, in parallel:** S1, S2a, C2a and C10b. None needs a sibling.
- **After S1 and S2a:** S2b, then S2c (S2c also needs message-feedback PR 3, `persist_final_answer`).
- **S3** needs S2c, message-branching's shared runner extraction PR (`stream_turn_to_sse`; PR 5 in the stack plan), chat-stop-cancels-turn PR 3 (voice on the runner), and merges after message-branching's edit/regenerate-on-the-runner PR (PR 7 in the stack plan), per the runner hotspot rule.
- **C1** after chat-element-library PR 5a (`ThreadDensityScope`).
- **C2b** after C2a and chat-stop-cancels-turn PR 4 and PR 5b (stopped reconciliation, `stopTurn`, `_untilDetached`).
- **C3** after chat-element-library PR 1. **C4** after C3 and chat-element-library PR 3. C4 **unblocks tool-result-renderers C7**.
- **C5** after C2a, C4 and chat-element-library PR 7a (`ChatMarkdownScope`) and PR 8b.
- **C6** after chat-element-library PR 5b and PR 8b, and chat-stop-cancels-turn PR 4 (`MessageStatus.stopped`, moved by chat-element-library PR 4).
- **C7** after chat-element-library PR 9, PR 11a and PR 11b. **C8** after C7 (and so after chat-element-library PR 11a, which takes `isStopping`).
- **C9** after chat-element-library PR 8b (`TypingIndicator`) and PR 10 (`ThreadViewport`).
- **C10** after chat-element-library PR 3 and PR 4, in parallel with all server PRs. **C10b** needs only the existing settings screen and `shared_preferences`.
- **C11** after S3, C10, C10b and chat-element-library PR 8a (`MessageActions.trailing` list, timing first).
- **C12** (guideline suite and docs) after everything above and chat-stop-cancels-turn PR 5c (reloaded "Stopped"). **C13** (archive) after chat-element-library is archived.
- **Run in parallel with each other once their dependencies have merged:** S1/S2a/C2a/C10b; C1, C3–C10; and every client PR with every server PR except C11.

## Stack placement

The cross-change plan (`openspec/chat-ui-parity-stack.md`) schedules this change's PRs in waves. Wave 1: S1. Wave 2: C2 (C2a, C2b after STOP PR 4), C3. Wave 3: C4, C10 (and C10b alongside it). Wave 4: S2 (S2a, S2b, S2c via the `FinalAnswerMeta` helper), C1, C7, C9. Wave 5: S3 (after message-branching PR 7), C8. Wave 6: C11. Wave 7: C5, C6. Wave 8: C12 (after chat-element-library is archived), then C13.

## Sibling coordination

| Sibling                      | Interaction                                                                                                                                                                                                                                                                                                                               |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| chat-element-library         | This change amends its allowlist and `ChatMarkdownScope` (MODIFIED delta copied verbatim from its current text). chat-element-library archives **first**; C13 re-diffs the delta before archiving.                                                                                                                                        |
| tool-result-renderers        | Its C7 depends on C4 (`CodeBlock`). It must not add a highlighter of its own.                                                                                                                                                                                                                                                             |
| message-branching            | The shared-runner PR (PR 5 in the stack plan) extracts the SSE loop once as `stream_turn_to_sse(…, RunOptions)`. S3 edits that runner, never `send_message`, and merges after the edit/regenerate-on-the-runner PR (PR 7). Its `model` PR (`messages.model`) and this change's S2c both extend message-feedback's `FinalAnswerMeta`. When branching auto-cancels a pending approval on edit, regenerate or branch switch, the announcer needs no new event: `TurnStarted` resets the awaiting-input flag (C2a).                                                                                                           |
| message-feedback             | PR 3 introduces `persist_final_answer(…, FinalAnswerMeta)`. S2c adds `timing` to `FinalAnswerMeta` instead of editing each save site. Its FeedbackButtons follow the timing indicator in `MessageActions.trailing` (`buildMessageActions` composes timing, then feedback); with the timing preference off the timing slot is empty and feedback still renders. It ships after message-branching's non-destructive compaction, which does not affect `message_timings` (no foreign key to `messages`).                                                                |
| chat-stop-cancels-turn       | Order: its PR 3 (voice on the runner) lands before S3, so S3 has one `done` build site. Its PR 4 (`MessageStatus.stopped`, `AgentErrorEvent`) and PR 5a/5b (`stopTurn`, `isStopping`, `_untilDetached`) land before C2b (`TurnStopped`), C6 (", stopped") and C8 (the "Stopping…" state). Its PR 2a (persisted `finish_reason`) and PR 5c ("Stopped" after reload) make the ", stopped" label survive a reload; C12 asserts that.                                                 |
| tool-approval-prompts        | Its "Approval needed" announcement goes through this change's `ChatAnnouncer` by emitting `TurnAwaitingInput` / `TurnInputResolved`; its PR 16 (approval in chat, which lands after the client cards because approvals are not feature-flagged) depends on C2b. "Still working" is suppressed while awaiting input. Its settings section PR (18a) and C10b both edit `settings_screen.dart` and the `settings.png` baseline: either order works, the later PR rebases and regenerates the baseline.                                                                                                                        |
| follow-up-suggestions        | Client stream lane order is chat-stop PR 4 → PR 5a/5b → follow-up-suggestions PR 7 → message-branching PR 18 (stream edit and regenerate). C2b hooks `_finishTurn`, `_failTurn` and the stopped reconciliation, not the loop, and rebases onto whatever part of the lane has landed. Its conversation-event-log delta modifies a different requirement than this change's. |
| conversation-list-management | Owns any keyboard or semantic alternative to swipe-to-delete. C12's guideline suite pumps the list, so it must pass after that change too. `get_conversation` preloads (S3 timing) follow the attachments-map pattern and rebase freely.                                                                                                  |
| all `StreamEvent` editors    | C11 adds a field (`DoneEvent.timing`), not a subclass, so it breaks no exhaustive switch. It lands in the wave the stack plan assigns to it and updates every `DoneEvent` constructor.                                                                                                                                                    |

---

## PR S1 — `feat(storage): add message timing store`

Layer: server
Depends on: none
Parallel with: S2a, C2a and every client PR

- [x] S1.0 Fix the malformed main spec `openspec/specs/chat-auto-scroll/spec.md`, which starts with a delta header (`## ADDED Requirements`, line 1): replace it with `## Purpose` plus `## Requirements`, so that archiving this change's MODIFIED delta works. Do the same for `openspec/specs/conversation-event-log/spec.md` unless follow-up-suggestions PR 2 (its task 1.0) has already done it. Run `openspec validate --specs --strict` and confirm both are clean.
- [x] S1.1 RED: a `MessageTimingStore` contract suite in `crates/storage` run against both implementations:
  - `put` then `get_for_conversation` round-trips all fields, with `None` fields preserved;
  - an empty conversation returns an empty map;
  - `put` twice for the same message overwrites;
  - deleting the conversation removes its timings (SQLite);
  - `replace_history` keeping message M leaves M's timing readable (SQLite, `conversations.rs:369-386`).

  Confirm RED.

- [x] S1.2 Add `TurnTiming { time_to_first_token_ms: Option<u64>, total_ms: u64, output_tokens: Option<u64>, decode_ms: Option<u64> }` (`Serialize`, `Deserialize`, `Clone`, `PartialEq`, `Debug`) to `assistant_core`, next to `LlmResponseMeta` (`crates/core/src/llm/types.rs:91`).
- [x] S1.3 Add migration `migrations/053_message_timings.sql`: `message_timings(message_id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE, time_to_first_token_ms INTEGER, total_ms INTEGER NOT NULL, output_tokens INTEGER, decode_ms INTEGER)` plus an index on `conversation_id`. No foreign key to `messages` (design Decision 7). Register it in both migration lists in `crates/storage/src/lib.rs`.
- [x] S1.4 Implement `MessageTimingStore` (`#[async_trait]`, `put(conversation_id, message_id, &TurnTiming)`, `get_for_conversation`), `SqliteMessageTimingStore` and `InMemoryMessageTimingStore` (ADR-0009 pair, `tokio::sync::RwLock`). Expose `StorageLayer::message_timing_store()` and re-export through `assistant_test_support::prelude`. Confirm GREEN.
- [x] S1.5 `make lint && make format && cargo test -p assistant-storage` green.

## PR S2a — `test(llm-provider): timed streaming scripts for ScriptedLlmProvider`

Layer: server
Depends on: none
Parallel with: S1, C2a and every client PR

- [x] S2a.1 RED, in `crates/llm-provider/src/scripted.rs` tests: with `with_timed_streaming_calls(clock, calls)`, a consumer that records `clock.now_instant()` immediately after each `recv()` sees exactly the scripted offsets (`[200 ms, 1500 ms]` from the call start), the call returns after `advance_before_return`, and two calls consume two separate scripts in order (today one chunk queue is drained by every call, `scripted.rs:93-96`). Run on the current-thread runtime. Confirm RED.
- [x] S2a.2 Implement `TimedCall { steps: Vec<(Duration, StreamChunk)>, advance_before_return: Duration }` and `with_timed_streaming_calls(FakeClock, Vec<TimedCall>)`. For each step: advance the clock, `send` the chunk, then wait until the channel is drained (`tx.capacity() == tx.max_capacity()`, `tokio::task::yield_now()` in between) before the next advance. Canned responses (and their `LlmResponseMeta.output_tokens`) still come from `with_canned_responses`. Confirm GREEN.
- [x] S2a.3 `make lint && make format && cargo test -p assistant-llm-provider` green.

## PR S2b — `refactor(runtime): carry TurnTiming on TurnResult and across the bus`

Layer: server
Depends on: S1
Parallel with: S2a, client PRs

- [x] S2b.1 RED:
  - in `crates/core/src/bus_messages.rs` tests: a bus `TurnResult` with `timing: None` serialises without a `timing` key, and a payload without `timing` deserialises (`serde(default)`);
  - in `orchestrator/worker.rs` tests: extract the two conversions the worker does today inline (`worker.rs:295-302` orchestrator → bus, `worker.rs:957-963` bus → orchestrator) into `bus_turn_result(…)` and `turn_result_from_bus(…)`, and assert that a `TurnResult` with a fixed `timing` survives both plus a JSON round-trip. (The end-to-end test through `submit_turn_with_request_id` lands in S2c, once timing is measured.)

  Confirm RED.

- [x] S2b.2 Add `timing: Option<TurnTiming>` to the orchestrator `TurnResult` (`orchestrator/mod.rs:80-99`) and to `bus_messages::TurnResult` (`crates/core/src/bus_messages.rs:107-128`, `#[serde(default, skip_serializing_if = "Option::is_none")]`). Copy it in both extracted conversions. Set `timing: None` at every other constructor: `mod.rs:1097`, `mod.rs:1369`, `turn_control.rs:53`, `turn_control.rs:145`, `worker.rs:489`, `orchestration.rs:153,184`, `interface_trait.rs:211`, `title_generator.rs:680,739`, `crates/web-ui/src/a2a/handlers.rs:942`, `crates/mcp-server/tests/io_loop.rs:153,160`, `crates/mcp-server/tests/dispatch.rs:167`. Confirm GREEN.
- [x] S2b.3 `make lint && make format && cargo test -p assistant-core -p assistant-runtime -p assistant-web-ui -p assistant-mcp-server` green.

## PR S2c — `feat(runtime): measure turn timing in run_turn_core`

Layer: server
Depends on: S1, S2a, S2b, message-feedback PR 3 (`persist_final_answer`, `FinalAnswerMeta`)
Parallel with: client PRs

- [x] S2c.1 RED: orchestrator tests with `FakeClock` (`Orchestrator::with_clock`, `orchestrator/mod.rs:385`) and the S2a timed script, on the current-thread runtime. Cases (the message-timing spec scenarios):
  - `run_turn_streaming`: text at 800 ms, return at 3,000 ms, 120 output tokens → `{ttft 800, total 3000, output 120, decode 2200}`;
  - `run_turn` (no sink) with the same script → `ttft: None`, decode = whole call;
  - thinking at 200 ms and text at 1,500 ms → `ttft` 1500;
  - two streaming calls around a 5,000 ms tool call (the tool handler advances the clock): decode excludes the tool time, total includes it;
  - a tool-call-only streaming call (no chunk, 1,000 ms, 30 tokens) then a text call (chunk at +200 ms, return at +1,200 ms, 100 tokens) → `output 130`, `decode 2000`;
  - one call reporting `output_tokens: None` → `output_tokens` and `decode_ms` `None`;
  - an empty final answer, a cancelled turn and an LLM error → no timing and no row;
  - a failing `MessageTimingStore::put` still returns `Ok(TurnResult)` with content;
  - after a turn, `get_for_conversation` returns the timing under `TurnResult.message_id`;
  - an end-to-end worker test: a turn through `submit_turn_with_request_id` and the bus returns the measured timing.

  Confirm RED.

- [x] S2c.2 In `run_turn_core` (`mod.rs:1148`): `t0 = self.clock.now_instant()` on entry; per call, `call_start`/`call_end` from `self.clock` (leave `llm_start` at `:1251` for OTel). In the forward task (`:1252-1272`), record the first-chunk instant per call and the first `StreamChunk::Text` instant per turn into a `tokio::sync::Mutex<CallMarks>` **immediately after `recv()` returns, before any `.await`**; read it after `forward_handle.await` (`:1271`). Accumulate decode (`call_end − first_chunk`, or `call_end − call_start` for a chunk-less call) and output tokens after each call.
- [x] S2c.3 At the final answer (`mod.rs:1325-1340`), build `TurnTiming` with `total_ms` taken just before the save. Add `timing: Option<TurnTiming>` to `FinalAnswerMeta` and have `persist_final_answer` `put` it through `MessageTimingStore` after the message is saved, logging failures with `warn!`. Return it on `TurnResult.timing`. `run_turn_with_tools_impl` passes `timing: None` (no message id, design Decision 7). Confirm GREEN.
- [x] S2c.4 `make lint && make format && cargo test -p assistant-runtime` green.

## PR S3 — `feat(web-ui): typed done frame with timing, and timing in history`

Layer: server
Depends on: S2c, message-branching's shared runner extraction PR (`stream_turn_to_sse`; PR 5 in the stack plan), chat-stop-cancels-turn PR 3 (voice on the runner); merge after message-branching's edit/regenerate-on-the-runner PR (PR 7 in the stack plan)
Parallel with: client PRs
Unblocks: C11

- [x] S3.1 RED, in `crates/web-ui/src/api/messages.rs` tests (next to `send_message_done_event_contains_message_id`):
  - the `done` data contains `timing.total_ms` as an integer, and absent fields are omitted, not `null`;
  - the voice-message `done` frame carries `timing` (it uses the same runner since chat-stop-cancels-turn PR 3);
  - `stream_run_events` replay of a completed run returns the same `timing` (extend `stream_run_events_replays_completed_run`);
  - the OpenAPI document contains `SseDoneEvent` and `MessageTimingSummary` (extend the `openapi.rs` schema test).

  In `conversations.rs` tests: `get_conversation` returns `timing` only on the final assistant row. Confirm RED.

- [x] S3.2 Add `MessageTimingSummary` (`utoipa::ToSchema`, `skip_serializing_if = "Option::is_none"` on optional fields) and a typed `SseDoneEvent { role, content, message_id: Option<String>, attachment_ids: Vec<String> (skipped when empty), timing: Option<MessageTimingSummary> }`. Register both in `openapi.rs` components next to `SseTokenEvent` (`openapi.rs:52,546`). In `stream_turn_to_sse`, build the terminal `done` payload from `SseDoneEvent` (replacing the `serde_json::json!` at today's `messages.rs:416-434`), using `TurnResult.timing`.
- [x] S3.3 Add `MessageSummary.timing` (`conversations.rs:105`). Populate it in `get_conversation` (`conversations.rs:401`, projection ~L503) with one `get_for_conversation` call per request, following the attachments-map preload pattern. Confirm GREEN.
- [x] S3.4 Run `make dump-openapi` and `make generate-flutter-client` (see the `openapi-sync` skill), and commit `openapi.json` and the regenerated `app/packages/assistant_api`.
- [x] S3.5 `make lint && make format && make test` green.

## PR C1 — `refactor(app): shared layout breakpoint tokens`

Layer: client
Depends on: chat-element-library PR 5a (`ThreadDensityScope`)
Parallel with: C2–C10b, all server PRs

- [x] C1.1 RED: `app/test/unit/layout_breakpoints_lint_test.dart` scans every Dart file under `app/lib` with the two patterns from the layout-breakpoints spec (`\b(width|maxWidth)\s*[<>]=?\s*([0-9]|_?k[A-Z]\w*)` and `const double _?k\w*Breakpoint`) against a **file-scoped** allowlist of exactly `shared/layout/breakpoints.dart` and `features/workflows/workflow_editor_screen.dart`. A second test runs the matcher on the fixture lines from the spec ("Lint catches the forms in use today"). Confirm RED: it hits `chat_screen.dart:302` and `nav_shell.dart:14,59`.
- [x] C1.2 RED: in the `chat_screen_test.dart` widget tests:
  - at 740 px the list is in the drawer;
  - at 900 px with the expanded sidebar the list is in the drawer and the menu button is shown;
  - at 840 px collapsed the list is in the drawer, at 841 px collapsed it is inline;
  - at 1280 px expanded it is inline;
  - at 900 px, toggling the sidebar from collapsed to expanded and pumping 16 ms frames through the 200 ms animation flips inline → drawer exactly once with no exception.

  In `nav_shell_test.dart`: 767 px gives the bottom bar and 768 px the rail. Confirm RED where behaviour changes.

- [x] C1.3 Add `app/lib/shared/layout/breakpoints.dart` (`abstract final class Breakpoints { static const double wide = 768; }`). `nav_shell.dart` drops `_kNavRailBreakpoint` (`:14`) and reads the token. `ChatScreen` puts a `LayoutBuilder` **above its `Scaffold`** and uses `constraints.maxWidth >= Breakpoints.wide` for the body, `leading` and `drawer` (`chat_screen.dart:302,345-361`).
- [x] C1.4 Name the density thresholds `kTimelineCompactMaxWidth` (400) and `kTimelineExpandedMinWidth` (700) on the package `TimelineDensity`, and feed `ThreadDensityScope` from the thread's own width (after the inline list) instead of `MediaQuery.of(context).size.width` (`chat_screen.dart:510-512`). Confirm GREEN.
- [x] C1.5 Update `.claude/skills/ux-principles/SKILL.md:58-68`: replace the 640/900 table with `Breakpoints.wide` (768) and the pane-width rule, and point to `adaptive-shell` REQ-5.
- [x] C1.6 Run Playwright (`crates/web-ui/e2e`, `.claude/skills/e2e-testing`). Baselines with viewports of 701–1008 px may change. Regenerate, review each diff, and list them in the PR body.

## PR C2a — `feat(app): ChatAnnouncer policy and announcement sink`

Layer: client
Depends on: none
Parallel with: everything
Unblocks: C2b, C5

- [x] C2a.1 RED: `app/test/unit/chat/chat_announcer_test.dart` drives the pure `ChatAnnouncer` with lifecycle events and a fake clock, asserting `(time, text, assertive)` for each case in the chat-accessibility spec:
  - start at 0 and completion at 4 s → "Ada is responding" at 1.5 s and "Ada replied: Hello there" at 5.5 s (markdown stripped);
  - start at 0 and completion at 0.8 s → only the completion, at 1.5 s;
  - start at 0 and failure at 0.5 s → assertive failure at 0.5 s and nothing at 1.5 s;
  - a 1,000-character reply is excerpted to at most 200 characters at a word boundary, plus "…";
  - `TurnStopped` → "Response stopped";
  - `TurnStalled` twice in one turn → one "Still working"; `TurnStalled` between `TurnAwaitingInput` and `TurnInputResolved` → none;
  - `TurnCompleted{source: replay}` is announced like a live one;
  - the completion text is "{Persona} replied: {excerpt}", never the name alone or the full reply;
  - `TurnAwaitingInput` with no `TurnInputResolved`, then `TurnStarted` and `TurnStalled` for the next turn → "Still working" is announced (`TurnStarted` resets the per-turn flags, so an approval auto-cancelled by an edit, regenerate or branch switch cannot suppress it);
  - a stream of 200 token updates produces nothing (tokens are not lifecycle events).

  Confirm RED.

- [x] C2a.2 Implement `app/lib/features/chat/chat_announcer.dart`: the sealed `ChatLifecycleEvent` family (`TurnStarted`, `TurnCompleted{content, source}`, `TurnStopped`, `TurnFailed{message}`, `TurnStalled`, `TurnAwaitingInput{announcement}`, `TurnInputResolved`, each with `at`), the policy with the held-slot throttle (`TurnStarted` resets the stall count and the awaiting-input flag), `say(text, {assertive})` for other features, `markdownToPlainText` (strips emphasis, links keeping their text, fences replaced with "code block", headings and list markers), `ChatAnnouncementSink` (wraps `SemanticsService.sendAnnouncement(View.of(context), …, assertiveness:)`), and `chatAnnouncerProvider` so tests can override the sink. Confirm GREEN.

## PR C2b — `feat(app): turn lifecycle events drive chat announcements`

Layer: client
Depends on: C2a, chat-stop-cancels-turn PR 4 and PR 5b
Parallel with: C3–C10b
Unblocks: tool-approval-prompts PR 16

- [x] C2b.1 RED, in the `chat_provider` tests (fake clock, scripted streams):
  - a live turn emits `TurnStarted` then `TurnCompleted{source: live}`;
  - a turn whose stream drops and is resumed through `_replayRun` emits one `TurnCompleted{source: replay}`;
  - opening a conversation with ten finished messages emits nothing;
  - `stopTurn()` reconciling to `MessageStatus.stopped` emits `TurnStopped`;
  - `_failTurn` emits `TurnFailed` with the message;
  - with no status update for `kTurnStallThreshold`, one `TurnStalled` is emitted.

  Confirm RED.

- [x] C2b.2 Add `ChatNotifier.lifecycle` (broadcast stream) and emit from the placeholder insertion, `_finishTurn` (only for a turn tracked as in flight), `_failTurn`, the stopped reconciliation, and a stall `Timer` re-armed on each status update. Do not touch the stream loop itself (see "Sibling coordination", client stream lane). Confirm GREEN.
- [x] C2b.3 RED, then GREEN: a widget test mocks `SystemChannels.accessibility` with `TestDefaultBinaryMessengerBinding`, runs a scripted turn through `ChatScreen`, advances the fake clock, and asserts one `announce` message carries the completion text. Wire `ChatNotifier.lifecycle` into the announcer in `ChatScreen`. Add a test asserting no node in the streaming bubble has the `isLiveRegion` flag.

## PR C3 — `feat(app): add highlighter and contrast-checked code themes to assistant_ui`

Layer: client
Depends on: chat-element-library PR 1
Parallel with: S*, C1, C2a

- [x] C3.1 RED: update `app/test/unit/assistant_ui_boundary_test.dart` to the amended allowlist `{flutter, flutter_smooth_markdown, material_ui, highlight}`, plus the rules that `package:highlight/` is only imported under `lib/src/highlight/` and that `Clipboard.` / `SemanticsService.` are never referenced in package source. Confirm RED, because `highlight` is not yet a dependency.
- [x] C3.2 Add `highlight: 0.7.0` to `app/packages/assistant_ui/pubspec.yaml`, the version `app/pubspec.lock:647` already resolves. Confirm GREEN for C3.1. Verify `pubspec.lock` gains no new package.
- [x] C3.3 RED: `packages/assistant_ui/test/highlight/highlight_code_test.dart`, with a spy through an injectable `Highlighter` seam:
  - `rust` yields keyword tokens;
  - an unknown language or `null` yields one plain token without throwing, and is not memoised;
  - 20,001 characters skips the highlighter;
  - a repeated `(language, code)` is served from the memo, keyed by full string equality (the memo map is keyed by the `(language, code)` record itself, not by a hash and length, and two different codes of equal length are both highlighted);
  - eviction beyond 64 distinct entries;
  - a finished `dart` fence, then a `rust` fence growing through 100 chunks: the memo holds at most two entries for them and the finished fence is still a hit.

  Confirm RED.

- [x] C3.4 Implement `lib/src/highlight/highlight_code.dart`: `highlightCode(code, language) → List<CodeToken>`, the LRU with the growing-fence in-place replacement rule (design Decision 2), and the size cap. Confirm GREEN.
- [x] C3.5 RED: `test/theme/code_theme_contrast_test.dart` computes the WCAG ratio of every token colour in `CodeTheme.light` / `CodeTheme.dark` against that theme's own `background` and requires at least 4.5. Confirm RED.
- [x] C3.6 Add `lib/src/theme/code_theme.dart` with a `background` per theme and token colours ported from highlight.js `a11y-light` / `a11y-dark` (MIT, with an attribution comment; source also in `flutter_highlight-0.7.0/lib/themes/a11y-*.dart`). Adjust any colour that fails C3.5 and record the change in a comment. Confirm GREEN.

## PR C4 — `feat(app): CodeBlock element`

Layer: client
Depends on: C3, chat-element-library PR 3
Parallel with: C1, C2, C6–C10b
Unblocks: tool-result-renderers C7

- [x] C4.1 RED: `packages/assistant_ui/test/code_block_test.dart`, covering the chat-code-blocks scenarios:
  - the language label, with `text` when absent;
  - the header does not overlap the code;
  - select-all and copy excludes the label;
  - a 300-character line at 360 px scrolls without overflow;
  - `onCopy` receives the exact code, and the label becomes "Copied" and reverts after 2 s (fake async);
  - with no callback there is no button;
  - Tab and Enter activates copy;
  - the copy target is at least 24×24;
  - under `disableAnimations` the "Copied" state appears in the next frame with no fade;
  - the dark theme paints `CodeTheme.dark.background` and uses its token colours.

  Confirm RED.

- [x] C4.2 Implement `CodeBlock` (`lib/src/code/code_block.dart`): header in `SelectionContainer.disabled`, a `MessageActionButton`-style copy control (a local primitive if chat-element-library PR 8a has not landed; switch to `MessageActionButton` in C5), a `Text.rich` body on `CodeTheme.background`. Export it from the barrel. Confirm GREEN.
- [x] C4.3 Add the registry entry, widgetbook use-cases (rust, plain, long line, copied state, light and dark) and goldens. `ELEMENTS.md` has no code-block row: list `CodeBlock` among the seams and primitives with no assistant-ui counterpart (the paragraph after the table), and note on row 14 that the `CodeBlock` highlighter is available.

## PR C5 — `feat(app): render markdown code fences through CodeBlock`

Layer: client
Depends on: C2a, C4, chat-element-library PR 7a (`ChatMarkdownScope`) and PR 8b
Parallel with: C6–C10b

- [x] C5.1 RED: in `packages/assistant_ui/test/chat_markdown_scope_test.dart`, `onCopyCode` defaults to null, and a `CodeBlock` with no `onCopy` uses the scope's callback. In `test/message_body_test.dart`:
  - a `python` fence renders one `CodeBlock`, both streaming and final;
  - a `mermaid` fence renders through the registered builder and not through `CodeBlock`;
  - a finished fence is served from the memo when a new chunk arrives.

  Confirm RED.

- [x] C5.2 Add `onCopyCode` to `ChatMarkdownScope`, with the `CodeBlock` fallback. In `MessageBody` and `ThinkingBody`, register `code_block → CodeBlockMarkdownBuilder` in the registry passed to `SmoothMarkdown` / `StreamMarkdown`, keeping `useEnhancedComponents: true` for blockquote, link and header. Confirm GREEN.
- [x] C5.3 RED, then GREEN: a chat screen widget test mocks `SystemChannels.platform` (`Clipboard.setData`) and `SystemChannels.accessibility`; activating copy on a reply's fence writes the code and announces "Code copied". In `ChatScreen`, install `onCopyCode` using `Clipboard.setData` and `ChatAnnouncer.say`.
- [ ] C5.4 Regenerate the Playwright `chat.png` baseline (the code block gains a header bar). Review the diff and record it in the PR body.

## PR C6 — `feat(app): semantics for message bubbles and timeline headers`

Layer: client
Depends on: chat-element-library PR 5b and PR 8b, chat-stop-cancels-turn PR 4
Parallel with: C4, C5, C7–C10b

- [x] C6.1 RED: `packages/assistant_ui/test/message_bubble_semantics_test.dart`:
  - the container label starts with `senderLabel`;
  - the status labels are "You, sending", "You, failed", "Ada, responding" and "Ada, stopped";
  - a link is a separate node with the link flag;
  - the typing indicator is labelled "typing";
  - `MessageActions` follows the bubble in traversal order.

  Confirm RED.

- [x] C6.2 Add the required `senderLabel` to `MessageBubble`, with the semantics container (`container: true`, no `MergeSemantics`) and the status suffix including `MessageStatus.stopped`. In the app, pass "You" or the active Persona name (`chat_screen.dart:313`). Confirm GREEN.
- [x] C6.3 RED: `test/timeline_entry_shell_semantics_test.dart`:
  - Tab focuses the header and Enter or Space toggles it;
  - `expanded` flips;
  - the label is "web_search, running" / "…, done" / "…, failed" / "…, awaiting input" / "…, stale";
  - a focus highlight is visible (golden).

  Confirm RED.

- [x] C6.4 Replace the shell header `GestureDetector` (lifted from `streaming_timeline_entry.dart:585`) with `FocusableActionDetector` plus `InkWell`. Keep the existing pin and auto-collapse behaviour, and keep every chat-element-library shell test green. Confirm GREEN.

## PR C7 — `fix(app): label icon-only chat controls and size their targets`

Layer: client
Depends on: chat-element-library PR 9, PR 11a and PR 11b
Parallel with: C4–C6, C9, C10, C10b
Unblocks: C8

- [x] C7.1 RED, in `packages/assistant_ui/test/composer/composer_semantics_test.dart` and `attachment_tray_test.dart`, every row of the label table in the chat-accessibility spec that lives in the package:
  - `send_button` "Send message", `stop_button` "Stop response" and "Stopping…" when `isStopping`;
  - `attach_image_button` "Attach image"; the voice button "Send voice message" / "Stop recording";
  - `remove_attachment_0` "Remove {filename}";
  - `meetsGuideline(androidTapTargetGuideline)` passes for a `Composer` with voice and no attachments;
  - the remove control's hit area is at least 24×24, lies inside the 64 px tile, and a tap 1 px inside each edge of it fires the callback.

  In `test/thread_error_banner_test.dart`: the dismiss button is labelled "Dismiss error" and the banner is a live region. In `test/thread_viewport_test.dart`: `scroll_to_bottom_button` is labelled "Scroll to bottom". Confirm RED.

- [x] C7.2 Add the tooltips and labels in `Composer`, `AttachmentTray`, `ThreadErrorBanner` and `ThreadViewport`. Restructure the attachment tile so the remove control sits **inside** its top-right corner (24×24 hit box, 18 px icon) instead of `Positioned(top: -4, right: -4)` with `BoxConstraints()` (`chat_input_row.dart:201-211`). Confirm GREEN.
- [x] C7.3 RED, then GREEN, in the app: the drawer menu button is labelled "Open conversations" (`chat_screen.dart:346-355`) and the image-viewer close button "Close image" (`message_attachments.dart:310-316`). `MessageActionButton` hit areas are at least 24×24.

## PR C8 — `feat(app): composer focus order and keyboard flow`

Layer: client
Depends on: C7 (and so chat-element-library PR 11a with `isStopping`)
Parallel with: C5, C6, C9, C10, C10b

- [x] C8.1 RED: `packages/assistant_ui/test/composer/composer_focus_test.dart`:
  - three Tabs from attach go to voice, then the input, then send;
  - after Enter submits, the input keeps focus;
  - send to stop moves focus to stop;
  - activating stop moves focus to the input **before** the rebuild with `isStopping: true` disables the stop button, and focus stays in the input after `isSending: false`;
  - Escape closes the slash menu with focus and text unchanged;
  - no focus node without a visible control exists (guards against the `KeyboardListener(focusNode: FocusNode())` pattern at `chat_input_row.dart:245-247`, which chat-element-library PR 11a removes).

  Confirm RED.

- [x] C8.2 Use the default `ReadingOrderTraversalPolicy` with a `FocusTraversalGroup` around the composer, with no manual ordering (the `ux-principles` rule). Send and stop hand focus over through a composer-owned `FocusNode`; the stop handler requests input focus before invoking `onStop`. Add an Escape `Shortcuts` entry that calls the slash menu's `onDismiss`. Confirm GREEN.

## PR C9 — `feat(app): honour reduced motion in thread viewport and typing indicator`

Layer: client
Depends on: chat-element-library PR 8b (`TypingIndicator`) and PR 10 (`ThreadViewport`)
Parallel with: C5–C8, C10, C10b

- [x] C9.1 RED: in `packages/assistant_ui/test/thread_viewport_test.dart`:
  - with `disableAnimations`, the scroll-to-bottom button, auto-scroll and conversation open land on `maxScrollExtent` after one `pump()`;
  - without it, the offset after one 16 ms frame lies strictly between start and end.

  In `test/typing_indicator_test.dart`: with `disableAnimations`, the indicator schedules no frames (`tester.binding.hasScheduledFrame` is false after the first pump) and renders identically before and after a 1 s advance. Confirm that the `TimelineEntryShell` test for no `AnimatedSize` under reduced motion still covers the shell. Confirm RED.

- [x] C9.2 `ThreadViewport` uses `jumpTo` when `MediaQuery.disableAnimationsOf(context)` and `animateTo` otherwise (lifted from `chat_screen.dart:77,149`). `TypingIndicator` stops its controller and renders static dots under reduced motion. Confirm GREEN.

## PR C10 — `feat(app): MessageTiming model and MessageTimingIndicator element`

Layer: client
Depends on: chat-element-library PR 3 and PR 4
Parallel with: S1–S3, C1–C9, C10b

- [x] C10.1 RED: `packages/assistant_ui/test/models/message_timing_test.dart`:
  - `MessageTiming.fromJson` ignores unknown keys and tolerates absent optional fields;
  - `tokensPerSecond` is null unless both `outputTokens` and `decode` are present and `decode ≥ 250 ms`;
  - `formatMessageTiming` gives "First token 0.8 s · Total 12.4 s · 42 tok/s", "Total 1 m 5 s", and whole seconds from 10 s to under 60 s;
  - `describeMessageTiming` gives "Response timing: first token 0.8 seconds, total 12.4 seconds, 42 tokens per second".

  Confirm RED.

- [x] C10.2 Add `MessageTiming` and `ChatMessage.timing` (nullable, mutable like its siblings) in the package models, plus the formatters. Confirm GREEN.
- [x] C10.3 RED: `test/message_timing_indicator_test.dart`:
  - a tap shows the tooltip text;
  - a mouse hover shows it;
  - **Tab from a preceding focusable control onto the indicator shows it**, with no pointer event;
  - Enter on the focused indicator shows it;
  - the semantics label is the `describeMessageTiming` sentence and the node is a focusable button;
  - the hit area is at least 24×24.

  Confirm RED. Implement `MessageTimingIndicator` as a focusable `MessageActionButton`-style control with `Icons.speed_outlined`, wrapped in a `Tooltip` with a `GlobalKey<TooltipState>`; call `ensureTooltipVisible()` on focus gain and on activation. No platform branching, no long-press trigger. Confirm GREEN.

- [x] C10.4 Add the registry entry, widgetbook use-cases (full, total only, focused, light and dark) and goldens.

## PR C10b — `feat(app): "Show response timing" preference and settings switch`

Layer: client
Depends on: none (uses the existing settings screen and `shared_preferences`)
Parallel with: every other PR
Unblocks: C11

- [x] C10b.1 RED: `app/test/unit/settings/response_timing_preference_test.dart` with `SharedPreferences.setMockInitialValues`:
  - with no stored value, `showResponseTimingProvider` yields `false`;
  - `set(true)` updates state first, writes `assistant.showResponseTiming`, and a fresh container reads `true`;
  - a `SharedPreferences` that throws on read yields `false` without throwing, and one that throws on write keeps the in-memory state.

  In `app/test/widget/features/settings/settings_screen_test.dart`: the screen shows a "Show response timing" switch row with its subtitle, off by default; tapping it turns the provider on; the switch has the semantics label "Show response timing" and toggles on Space and Enter. Confirm RED.

- [x] C10b.2 Add `app/lib/features/settings/response_timing_preference.dart`: `kShowResponseTimingPrefsKey = 'assistant.showResponseTiming'` and `showResponseTimingProvider`, an `AsyncNotifier<bool>` modelled on `sidebarCollapsedProvider` (`try`/`catch` around every read and write, `debugPrint` on failure). Add the switch row to `settings_screen.dart` using the existing `AdaptiveListSection` pattern. Confirm GREEN.
- [ ] C10b.3 Regenerate the Playwright `settings.png` baseline (one new row), review the diff and record it in the PR body. If tool-approval-prompts PR 18a has landed, rebase and regenerate instead of merging the PNG.
- [ ] C10b.4 `make lint-flutter && make test-flutter` green.

## PR C11 — `feat(app): show per-message timing from done frames and history`

Layer: client
Depends on: S3, C10, C10b, chat-element-library PR 8a (`MessageActions.trailing` list)
Parallel with: C5–C9, C12 prep

- [x] C11.1 RED:
  - in `app/test/unit/api/stream_event_test.dart` and the `api_client.dart` parser tests: a `done` frame with `timing` yields `DoneEvent.timing`, a frame without it yields null, and a malformed `timing` is ignored without dropping the event;
  - in `chat_history_mapping` tests: `MessageSummary.timing` maps to `ChatMessage.timing` on the right row;
  - in the `chat_provider` tests: after a scripted `done` with timing the final assistant message carries it, and replay through `GET …/events/stream?since=N` yields the same;
  - in `message_actions_builder_test.dart`: with `showResponseTiming: true`, `buildMessageActions` puts the timing indicator first in `trailing`, before feedback; with `showResponseTiming: false` (the default) it builds none even when the message has timing, and feedback still renders.

  Confirm RED.

- [x] C11.2 Add `timing` to `DoneEvent` (`app/lib/api/models/stream_event.dart`) and to the `done` parsing in `api_client.dart:656-671`; update every `DoneEvent` constructor. Pass it through `_finishTurn` and `completedTurnState`. Map it in `chat_history_mapping.dart` from the regenerated `MessageSummary`. Confirm GREEN.
- [x] C11.3 RED, then GREEN: a widget test checks that, with the preference on, a finished assistant reply with timing shows `MessageTimingIndicator` in `MessageActions.trailing`, and that a streaming reply or one without timing shows none. With the preference off (the default) no reply shows one while `ChatMessage.timing` is still populated; flipping the provider to on shows the indicator on every finished reply with timing, including already-rendered and history-loaded ones, without reloading the conversation. Add a `showResponseTiming` argument to `buildMessageActions` in `features/chat/message_actions_builder.dart`, and have `ChatScreen` watch `showResponseTimingProvider` and pass it.
- [ ] C11.4 Playwright baselines do not change (the preference is off by default, so no timing glyph renders); confirm with the e2e run and record that in the PR body.

## PR C12 — `test(app): chat accessibility guideline suite and user docs`

Layer: client
Depends on: S3, C1–C11 (including C10b), chat-stop-cancels-turn PR 5c ("Stopped" after reload)
Parallel with: none

- [x] C12.1 RED: add `app/test/widget/features/chat/chat_accessibility_test.dart`. It pumps `ChatScreen` with the fixture conversation in light and dark themes:
  - a user message with an attachment;
  - an assistant reply with a code fence, a link and timing (with `showResponseTimingProvider` overridden to on; a second pump at the default asserts no `MessageTimingIndicator` renders and the guidelines still pass);
  - a tool call;
  - a thinking entry;
  - an error banner;
  - a conversation reloaded from history whose last reply is stopped (persisted `finish_reason`), asserting the bubble label "Ada, stopped".

  A second test in the same file pumps the settings screen and asserts the "Show response timing" switch is labelled and meets `labeledTapTargetGuideline`.

  It asserts `labeledTapTargetGuideline`, `textContrastGuideline`, and that every node with a tap action has a non-empty label. Any failure is a bug to fix here, not an exception to add. GREEN.

- [x] C12.2 User docs (config.yaml: user-facing features are documented in `docs/`). Add to `docs/web-ui.md` a "Response timing" section (that it is off by default and how to turn it on under Settings, "Show response timing", per device; what first token, total and tok/s mean, why tok/s is missing for some providers, that timing is absent on older messages and on Slack/Mattermost) and an "Accessibility" section (screen-reader announcements, that completion is announced as the Persona name plus the first 200 characters of the reply, including after a reconnect, and their throttling, keyboard operation of the composer, timeline entries and code copy, reduced-motion behaviour, the 768 px layout rule).
- [ ] C12.3 Run a manual screen-reader pass: VoiceOver on macOS desktop and on Safari web with semantics enabled, and TalkBack on an Android emulator if available. Send a message and check that the start and completion announcements are each heard once. Stop a reply and check "Response stopped". Copy code and check "Code copied". With the preference on, Tab through the composer and onto a timing indicator. Expand a tool call with the keyboard. Record the results in the PR body.
- [ ] C12.4 `make lint && make format && make test && make lint-flutter && make test-flutter` green.

## PR C13 — `chore(openspec): archive chat-a11y-and-code-blocks`

Layer: none (specs only)
Depends on: C12, chat-element-library archived (its PR 13.8)
Parallel with: none

- [x] C13.1 Re-diff `specs/chat-element-library/spec.md` here against the archived `openspec/specs/chat-element-library/spec.md`. Both MODIFIED requirements must equal the archived text except for the `highlight` allowlist, the `Clipboard`/`SemanticsService` rule and `ChatMarkdownScope.onCopyCode`. Update the delta if the sibling changed.
- [x] C13.2 `openspec validate chat-a11y-and-code-blocks --strict`, then `openspec archive chat-a11y-and-code-blocks`. This applies the `chat-auto-scroll` (reduced motion) and `conversation-event-log` (`done.timing`) deltas; do not hand-edit those main specs.

## Notes

- No ADR: none of the decisions changes architecture. They are a side table, an element, an app-side announcer, a typed SSE schema and a token file.
- API changes: `done.timing` (with a new `SseDoneEvent` schema) and `MessageSummary.timing`. S3 carries `openapi.json`, `make dump-openapi` and `make generate-flutter-client`.
- No user-facing preference API: "Show response timing" is a per-device `SharedPreferences` boolean (C10b), so there is no server or `openapi.json` change for it.
- One package dependency: `highlight` 0.7.0 is added to the `assistant_ui` allowlist. It is already in the lockfile.
- The guideline suite (C12) is deliberately last so it measures the finished state. Each earlier PR carries its own targeted semantics tests.
