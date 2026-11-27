# Tasks: follow-up-suggestions

This change is a stack of 12 PRs (11 code PRs plus the archive PR). Each one stays under about 500 changed lines (generated `assistant_api` code, `openapi.json` dumps and PNG baselines are not counted) and leaves `main` shippable. Server PRs that need nothing from the client come first.

Every PR starts with a failing test. Confirm it is RED before writing production code. Rust production code stays panic-free (no `unwrap`/`expect`/`panic`). Each client widget PR follows the chat-element-library per-PR checklist: registry entry, widgetbook use-case, `golden`-tagged test, and an `ELEMENTS.md` row update.

Baselines: never merge Playwright PNGs by hand. After rebasing a PR that touches baselines, regenerate them (see `.claude/skills/e2e-testing`) and describe the diff in the PR body.

## Parallelism

```
PR 1 (runtime follow_ups + config) ──┐
PR 2 (run lifecycle + status + ADR) ─┼─ PR 3 (runner suggestion phase) ─┐
message-branching PR 5 (runner) ─────┘                                  │
PR 7a (client SuggestionsEvent) ────────────────────────────────────────┼─ PR 7b (FollowUpListener) ─┐
chat-stop-cancels-turn PR 5 ────────────────────────────────────────────┘                            ├─ PR 7c (chips + e2e + docs)
PR 6 (assistant_ui widgets) ─────────────────────────────────────────────────────────────────────────┤
PR 4 (storage starter_prompts) ── PR 5 (persona PATCH) ── PR 8a (starters editor) ───────────────────┴─ PR 8b (empty-state starters)
```

- Start immediately, in parallel: PR 1, PR 2, PR 4, PR 6 (once chat-element-library PR 1 and PR 3 have merged) and PR 7a.
- PR 3 needs PR 1, PR 2 and message-branching PR 5. PR 5 needs PR 4. PR 3 and PR 5 can run in parallel.
- PR 7b needs PR 3 and PR 7a. PR 8a needs PR 5. PR 7b and PR 8a can run in parallel.
- PR 7c needs PR 6 and PR 7b. PR 8b needs PR 6 and PR 8a. PR 7c and PR 8b can run in parallel.

Wave placement (from `openspec/chat-ui-parity-stack.md`): PR 1, PR 2 and PR 4 are wave 1; PR 5 is wave 2; PR 3 and PR 6 are wave 3; PR 7a lands in wave 3 or earlier, before PR 7b; PR 7b, PR 7c, PR 8a and PR 8b are wave 4; PR 9 is wave 8.

Sibling dependencies and interlocks:

- **Hotspot: runner (`messages.rs`).** PR 3 merges before tool-approval-prompts PR 7 in the same wave. message-branching PR 7 (edit and regenerate) passes `suggestions: true`.
- **Shared turn runner.** message-branching PR 5 extracts `send_message`'s loop once, as `stream_turn_to_sse(…, RunOptions)`. PR 3 here edits the runner, never `send_message`, and adds the `RunOptions.suggestions` field. chat-stop-cancels-turn PR 6 passes `suggestions: false` for voice; message-branching PR 7 passes `true` for edit and regenerate.
- **Turn-status precedence.** PR 2 sets the terminal-row rule. tool-approval-prompts PR 8 depends on PR 2 and adds `awaiting_input` below `completed`: `errored` > `completed` > `awaiting_input` > `running`.
- **Client stream lane.** Order: chat-stop-cancels-turn PR 4, chat-stop-cancels-turn PR 5, PR 7b here, message-branching PR 12. PR 7a adds a `StreamEvent` subclass in the wave the cross-change stack plan assigns, and updates every exhaustive switch over `StreamEvent` that exists when it lands.
- **Widgets.** PR 6 needs chat-element-library PR 1 (package scaffold) and PR 3 (gallery and registry). PR 7c needs chat-element-library PR 10 (`ChatThreadLayout.aboveComposer`). PR 8b needs chat-element-library PR 9 (`ThreadEmptyState.starters`).
- **Migration.** PR 4's migration is `051_persona_starter_prompts`, the number pre-allocated by the cross-change stack plan (046 to 050 belong to sibling changes).
- **Archive order.** chat-stop-cancels-turn, then follow-up-suggestions (PR 9), then tool-approval-prompts (all three modify `turn-status-api`).
- **Decisions.** The product decisions for this change are final and recorded in `openspec/chat-ui-parity-decisions.md` and design.md: suggestions on by default, no chips from history, owner-or-admin editing of starters, no org fallback starters, no voice suggestions, one extra replay request per turn accepted.

---

## PR 1 — feat(runtime): follow-up suggestion generator and [suggestions] config

Layer: server
Depends on: none
Parallel with: PR 2, PR 4, PR 6, PR 7a

- [x] 1.0 Fix the main spec `openspec/specs/conversation-event-log/spec.md`, which starts with a delta header (`## ADDED Requirements`, line 1). Replace that header with `## Purpose` plus `## Requirements` so that `openspec archive` can apply this change's MODIFIED delta. Run `openspec validate --specs --strict` and confirm it is clean for that spec.
- [x] 1.1 Test first in `crates/core/src/types/agent.rs` tests: an `OrgConfig` without `[suggestions]` gives `enabled = true`, `max_suggestions = 3`, `timeout_secs = 10`; an explicit block overrides them; `effective_max()` clamps 9 to 5 and 0 to 1; `effective_timeout()` clamps 0 to 1 s and 60 to 15 s. Confirm RED.
- [x] 1.2 Add `SuggestionsConfig` (with `Default`, `effective_max()`, `effective_timeout()` and `pub const SUGGESTIONS_MAX_TIMEOUT_SECS: u64 = 15`) to `crates/core/src/types/features.rs` next to `TitlingConfig` (`:197`), and `#[serde(default)] pub suggestions: SuggestionsConfig` to `OrgConfig` (`agent.rs:45-62`). Confirm GREEN.
- [x] 1.3 Test first in `crates/runtime/src/follow_ups.rs` (`#[cfg(test)]`): `parse_suggestions` table tests (numbering, `-`/`*` bullets, curly and straight quotes, blank lines, a 121-char line dropped and a 120-char line kept, case-insensitive dedupe, `max` cap). `generate` with `ScriptedLlmProvider`: a final answer is parsed; a tool-call or thinking-only response yields `[]`; inputs over 1 000 chars (counted as `char`s, with a multi-byte fixture) are truncated in the prompt the provider sees. Confirm RED.
- [x] 1.4 Implement `follow_ups::{generate, parse_suggestions}`, modelled on `title_generator::generate_title` (`title_generator.rs:50-85`): no tools, one call, a system prompt asking for next _user_ prompts in the user's language, one per line. Wrap the call in a GenAI span (`gen_ai.operation.name`, `gen_ai.request.model`, token usage when reported, `assistant.feature = "follow_up_suggestions"`, `outcome`), per the `opentelemetry-genai-spec` skill. Test the span attributes with the existing tracing test layer if the crate has one; otherwise assert the `outcome` value returned alongside the prompts. Export from `crates/runtime/src/lib.rs`. Confirm GREEN.
- [x] 1.5 `make lint && make format`, then `cargo test -p assistant-core -p assistant-runtime`.

## PR 2 — fix(web-ui): run replay and turn status outlive the done frame

Layer: server
Depends on: none
Parallel with: PR 1, PR 4, PR 6, PR 7a

- [x] 2.1 Test first in `crates/web-ui/src/api/turns.rs` tests: seed `run_started`, `done`, `suggestions`; status returns `state: "completed"` and `last_event_kind: "suggestions"`. Seed `agent_error` followed by another row; status returns `errored`. Confirm RED, because today the derivation from the last row says `running`.
- [x] 2.2 Change the derivation (`turns.rs:177-181`) to check for terminal rows: `agent_error` present gives errored, `done` present gives completed, otherwise running. Leave a doc comment stating the combined precedence (`errored` > `completed` > `awaiting_input` > `running`) that tool-approval-prompts PR 8 extends. Confirm GREEN.
- [x] 2.3 Test first in `messages.rs` tests: (a) a completed run seeded with `done` then `suggestions` replays both and closes; (b) an active run (broadcaster started) with `done` persisted keeps tailing, delivers a later broadcast `suggestions` event, and closes on `finish_run`; (c) an active run whose replay client drops its body releases its broadcast receiver (`receiver_count()` returns to 0) without another event being broadcast. Confirm (b) and (c) RED.
- [x] 2.4 In `stream_run_events`, delete the `already_complete` short-circuit (declared `messages.rs:698`, used `:716`) and the `is_terminal` early return (`:733-737`). After replay, close when `!is_active`; otherwise tail until `RecvError::Closed`, `select!`ing on `sse_tx.closed()` so a departed client ends the tail. Confirm GREEN, and check that the existing replay tests (`:1505`, `:1572`) and `stream_run_events_returns_404_for_pruned_run` (`:1807`) still pass.
- [x] 2.5 Update the `stream_run_events` doc comment (`:599-605`): it closes when the run finishes, not at `done`/`error`, and a pruned run returns `404` (drop the stale `410` line and the `410` response in the utoipa attribute, `:618`). Run `make dump-openapi` and commit `openapi.json`. Run `make generate-flutter-client` and commit the regenerated `app/packages/assistant_api/` if it changed.
- [x] 2.6 Docs: update the state-machine section of `docs/operations/turn-status-endpoint.md` (`:63-67`) to the terminal-row rule, note that `last_event_kind` can be `suggestions` after `done`, and note that `done` is no longer always the last frame of a run (the POST stream still closes at `done`).
- [x] 2.7 ADR: add `docs/adr/adr-00NN-run-bounds-replay-stream.md` ("The run, not `done`, bounds the replay stream"). Take the number from the cross-change stack plan (adr-0017..0019 are reserved for siblings and adr-0020 is already claimed). Cover the POST-vs-replay split, the terminal-row status rule and its precedence, and the consumers (follow-up-suggestions, tool-approval-prompts, generative-ui-a2ui).
- [x] 2.8 `make lint && make format && cargo test -p assistant-web-ui`.

## PR 3 — feat(web-ui): suggestion phase on the shared turn runner

Layer: server
Depends on: PR 1, PR 2, message-branching PR 5
Parallel with: PR 5, PR 6, PR 7a

- [x] 3.0 Test-helper seam (no behaviour change): in `crates/web-ui/src/api/test_helpers.rs`, set `config.suggestions.enabled = false` in `state_over_in` (`:180-181`, next to `memory.enabled = false`), so existing `send_message` tests keep one `/api/chat` call and no new frame. Add `state_over_with_config(llm_url, spaces, workers, config: OrgConfig)`, which `state_over_in` delegates to. Add `mount_llm_replies(server, &[&str])`, which mounts ordered wiremock responses with `up_to_n_times(1)` (answer first, then the follow-up response). Run the existing web-ui tests and the e2e flow to confirm they are unchanged.
- [x] 3.1 Test first in the runner's tests (`messages.rs`, or the module message-branching PR 5 created), using `state_over_with_config` with suggestions enabled and `mount_llm_replies`: the POST SSE body ends with `done` and closes; a replay subscription opened after `done` with `since = done.seq + 1` receives `suggestions` with matching `message_id` and then closes; the event store holds a `suggestions` row at `done.seq + 1`. Confirm RED.
- [x] 3.2 Test first, and confirm RED before implementing each case: `enabled = false` makes one `/api/chat` call only (assert wiremock's received-request count); a cancelled turn and an errored turn make no suggestion call; a `done` without `message_id` makes no call; a provider that hangs past the timeout (`timeout_secs = 1`, wiremock `set_delay(3s)`) persists nothing and the run finishes within about 1 s of `done`; a receiver dropped before `done` with no subscriber makes no call; a client that receives `done` but opens no replay subscription has its call aborted after the 2 s attach grace (wiremock delay longer than the grace, run finishes early, nothing persisted); Web Push fires before the suggestion call starts (record call order in a fake push dispatcher).
- [x] 3.3 Add `suggestions: bool` to `RunOptions`; `send_message` passes `true`. Implement the post-terminal phase in `stream_turn_to_sse` per design.md D4: send the terminal frame and remember whether it reached the client; dispatch Web Push for every terminal kind; drop `sse_tx`; gate on `options.suggestions`, `cfg.enabled`, a `done` terminal with a `message_id`, and a listener; race `tokio::time::timeout(cfg.effective_timeout(), follow_ups::generate(…))` against the listener watchdog (2 s grace, 500 ms poll of `broadcast_tx.receiver_count()`) in `tokio::select!`; persist and broadcast a non-empty result; then `finish_run`. Read `SuggestionsConfig` from `SpaceRuntime.config` and the LLM from `SpaceRuntime.llm`; clone the user `content` before `submit_turn` takes it. Errors only log. Confirm GREEN.
- [x] 3.4 Test first (regression guard on the flag): with suggestions enabled, a runner call with `RunOptions { suggestions: false, .. }` (what `send_voice_message` passes after chat-stop-cancels-turn PR 6) and a `quick_message` turn make no suggestion call. If chat-stop-cancels-turn PR 6 has not landed, assert on `send_voice_message` directly. Confirm GREEN without code changes; if RED, fix the caller.
- [x] 3.5 OpenAPI: add `SseSuggestionsEvent { message_id: Uuid, suggestions: Vec<SseSuggestion> }` and `SseSuggestion { prompt: String }` next to the existing `Sse*Event` schemas (`messages.rs:509-590`) and register them in the OpenAPI components. List the `suggestions` frame in the replay endpoint's description. Run `make dump-openapi && make generate-flutter-client` and commit `openapi.json` and the regenerated `app/packages/assistant_api/`.
- [x] 3.6 Docs: add a `[suggestions]` section to `docs/operations/configuration.md` (keys, defaults and clamps, cost per turn, the Ollama contention note, which turns opt in, and that voice does not) and add `[suggestions]` to the `org.toml` row of its file table (`:17`). State that the feature is on by default and that `enabled = false` turns it off.
- [x] 3.7 `make lint && make format && make test`, plus `make lint-flutter` (generated client compiles).

## PR 4 — feat(storage): persona starter_prompts column

Layer: server
Depends on: none
Parallel with: PR 1, PR 2, PR 6, PR 7a

- [x] 4.1 Test first in `crates/storage/src/personas.rs` tests, run against both `SqlitePersonaStore` (`StorageLayer::new_in_memory()`) and `InMemoryPersonaStore`: a new Persona has `[]`; `set_starter_prompts` round-trips in order; a raw `UPDATE personas SET starter_prompts = 'not json'` reads back as `[]` without an error. Confirm RED.
- [x] 4.2 Add migration `migrations/051_persona_starter_prompts.sql` (`ALTER TABLE personas ADD COLUMN starter_prompts TEXT NOT NULL DEFAULT '[]'`); 051 is the number pre-allocated by the cross-change stack plan. Register it in `run_migrations` (`crates/storage/src/lib.rs:297`) and in any other migration list in that file that the stack plan requires. Leave `seed_database_through_016` (`:562`) unchanged; it is a fixed test fixture.
- [x] 4.3 Add `starter_prompts: Vec<String>` to `PersonaRecord` (`personas.rs:21`), add it to every `SELECT` column list (`:104`, `:116`, `:135`, `:234`) and to `row_to_record` (`:509`, tolerant parse with `warn!`), and add `set_starter_prompts` to the trait (`:42`) and both implementations. Confirm GREEN.
- [x] 4.4 `make lint && make format && cargo test -p assistant-storage`.

## PR 5 — feat(web-ui): PATCH /api/personas/{id} with starter_prompts

Layer: server
Depends on: PR 4
Parallel with: PR 3, PR 6, PR 7a

- [x] 5.1 Test first in `personas.rs` handler tests: `GET` includes `starter_prompts: []`; `PATCH` with trimmed prompts returns 200 and the stored values; 5 prompts returns 400 `{"error"}` and stores nothing; a whitespace-only prompt or one of 201 chars after trimming returns 400; a prompt of 200 non-ASCII chars returns 200; an unknown id returns 404; read-only space access returns 403 and stores nothing; `{}` returns 200 and leaves the prompts unchanged. Permission cases: the owner of a personal Persona gets 200 and `can_edit_starter_prompts: true`; a space admin editing an org-owned Persona gets 200; a plain member editing an org-owned Persona gets 403, stores nothing, and sees `can_edit_starter_prompts: false` on `GET`. Confirm RED.
- [x] 5.2 Add `starter_prompts` and `can_edit_starter_prompts` to `PersonaDetail` (`personas.rs:125`; the flag is computed from the caller's role and the Persona's `owner_user_id`, one helper shared with the handler), add `UpdatePersonaRequest`, add the `update_persona` handler (`operation_id = "update_persona"`, documented with the 200/400/401/403/404 responses, with the owner-or-admin rule (design.md D10) after the `Access::Write` and `get_accessible` checks, lengths counted with `chars().count()` after `trim()`), and route `.patch(update_persona)` on `/personas/{id}` (`:199`). Register the handler and schema in the OpenAPI doc. Confirm GREEN.
- [x] 5.3 `make dump-openapi && make generate-flutter-client`. Commit `openapi.json` and the regenerated `app/packages/assistant_api/`, which is never hand-edited.
- [x] 5.4 `make lint && make format && make test`, plus `make lint-flutter` (generated client compiles).

## PR 6 — feat(app): SuggestionChips and StarterPrompts elements

Layer: client
Depends on: chat-element-library PR 1, PR 3
Parallel with: PR 1 to PR 5, PR 7a

- [x] 6.1 Test first in `packages/assistant_ui/test/suggestion_chips_test.dart`: one chip per prompt, keyed `suggestion_chip_<i>`; a tap fires `onSelected` with the prompt; `enabled: false` blocks taps; an empty list renders `SizedBox.shrink`; the semantics container is labelled "Suggested follow-ups"; each chip has button semantics and meets the tap-target guideline (`meetsGuideline(androidTapTargetGuideline)`). Confirm RED.
- [x] 6.2 Implement `SuggestionChips` (`lib/src/suggestions/suggestion_chips.dart`, using `flutter` and `material_ui` only). Export it from the barrel and add it to `kAssistantUiElements`. Confirm GREEN.
- [x] 6.3 Test first in `test/starter_prompts_test.dart`: keys `starter_prompt_<i>`; a tap emits the prompt; 1 column at 400 dp and 2 columns at 720 dp (asserted by card x-offsets); an empty list renders nothing. Confirm RED, implement `StarterPrompts`, export and register it, confirm GREEN.
- [x] 6.4 Widgetbook use-cases (chips: 1, 3, long prompt, disabled; starters: narrow, wide) and `golden` tests, light and dark. Update the `ELEMENTS.md` row 9 ("Follow-up suggestions") to point at these elements.
- [x] 6.5 `make lint-flutter && make test-flutter`.

## PR 7a — feat(app): parse the suggestions stream event

Layer: client
Depends on: none (lands in the event-class wave the stack plan assigns)
Parallel with: PR 1 to PR 6

- [x] 7a.1 Test first in `app/test/unit/api/sse_parser_test.dart`: `event: suggestions` with `id: 41` yields `SuggestionsEvent(messageId, prompts, sequenceId: 41)`; malformed JSON yields nothing; an unknown `event: future_thing` yields nothing and later frames still parse. Confirm RED.
- [x] 7a.2 Add `SuggestionsEvent` to `app/lib/api/models/stream_event.dart` and a branch to `parseSseByteStream` (`api_client.dart`, next to the `done` branch at `:656`). Update every exhaustive `switch` over `StreamEvent` (in `chat_provider.dart`, the stream reducers and any sibling code merged by then) so that `SuggestionsEvent` is ignored there. Confirm GREEN.
- [x] 7a.3 `make lint-flutter && make test-flutter`.

## PR 7b — feat(app): listen for follow-up suggestions after done

Layer: client
Depends on: PR 3, PR 7a, chat-stop-cancels-turn PR 5 (client stream lane order)
Parallel with: PR 8a

- [x] 7b.1 Test first in `app/test/unit/chat/follow_up_listener_test.dart` and `chat_provider_suggestions_test.dart`, with a fake `ApiClient`: (a) `done` sets `isSending = false` and starts a replay subscription with `since = done.sequenceId + 1`; a later `suggestions` frame on it sets `ChatState.suggestions`; (b) with a queued message, the second send starts before the delayed `suggestions` arrives, those suggestions are dropped, and the listener's subscription was cancelled; (c) after (b), `_session.runId` and `_session.lastSeq` still belong to run 2 once run 1's late suggestions arrive; (d) `sendMessage`, `loadConversation` and `clearConversation` clear suggestions and cancel the listener; (e) a replay that never sends `suggestions` is cancelled after `kSuggestionsListenWindow` (20 s, `fake_async`); (f) the `_replayRun` path also starts the listener at `done`; (g) a `done` without `messageId` starts no listener; (h) `loadConversation` of a conversation with a finished run starts no listener and leaves `suggestions` empty (no chips from history). Confirm RED.
- [x] 7b.2 Add `suggestions` (with `copyWith`/clear) to `ChatState` (`chat_models.dart:328`). Add `FollowUpListener` in `features/chat/follow_up_listener.dart` per design.md D8: it takes captured `conversationId`, `runId`, `since`, `messageId`, owns its `CancelToken`, and never touches `ChatStreamSession`. Wrap its source in chat-stop-cancels-turn's `_untilDetached`. Start it from `_streamMessage` (`chat_provider.dart:692`) and `_replayRun` (`:477`) at `DoneEvent`, without changing how those loops return. Cancel it at the start of `_streamMessage`, in `loadConversation`, `clearConversation`, conversation switches and `dispose`. Set `kSuggestionsListenWindow = Duration(seconds: 15 + 5)`, with a comment tying it to `SUGGESTIONS_MAX_TIMEOUT_SECS`. Confirm GREEN.
- [x] 7b.3 `make lint-flutter && make test-flutter`.

## PR 7c — feat(app): render follow-up chips above the composer

Layer: client
Depends on: PR 6, PR 7b, chat-element-library PR 10
Parallel with: PR 8b

- [x] 7c.1 Test first in `app/test/widget/features/chat/chat_screen_suggestions_test.dart`: with suggestions in state and `isSending` false, `suggestion_chip_0` sits above the composer; tapping it calls `sendMessage` with the prompt; while sending, no chips render. Confirm RED.
- [x] 7c.2 Wire `SuggestionChips` into `ChatThreadLayout.aboveComposer` through a small `follow_up_chips.dart` adapter, instead of growing `chat_screen.dart`. Confirm GREEN.
- [ ] 7c.3 Playwright: the e2e server runs the real binary with no LLM mock (`crates/web-ui/e2e/playwright.config.ts:83-85`), so add `page.route` fixtures: `POST .../messages` answers with an SSE body ending in `done` (with `message_id` and `id`), and `GET .../runs/{run}/events/stream` answers with a `suggestions` frame. Add a new `chat-follow-up-suggestions.png` baseline. Regenerate baselines after rebasing and describe the diff in the PR body.
- [x] 7c.4 Docs: in `docs/web-ui.md`, add a "Suggestions and starter prompts" section covering the follow-up chips.
- [x] 7c.5 `make lint-flutter && make test-flutter`, then `make lint && make format && make test`.

## PR 8a — feat(app): persona starter prompts editor

Layer: client
Depends on: PR 5
Parallel with: PR 7b, PR 7c

- [x] 8a.1 Test first in `app/test/unit/personas/persona_starters_controller_test.dart`: the editor's state comes from `personaDetailProvider(id)` (loading, success, error) without a second `getPersona` call; `save(list)` calls `updatePersona` and then invalidates `personaDetailProvider(id)` and `activePersonaStarterPromptsProvider` (assert both rebuild); `canEditStarterPrompts == false` exposes the list read-only and `save` is not offered; client-side validation rejects a 5th prompt, blank prompts and prompts over 200 chars (counted as runes). Confirm RED.
- [x] 8a.2 Implement `persona_starters_controller.dart` in `features/personas/`, built on `personaDetailProvider`. Declare `activePersonaStarterPromptsProvider` here (derived from `personasProvider`'s active Persona and `personaDetailProvider`) so the invalidation is real; PR 8b consumes it. Confirm GREEN.
- [x] 8a.3 Test first in `app/test/widget/features/personas/persona_starters_editor_test.dart`: add, edit, reorder and remove prompts; the add affordance is disabled at 4; save calls the controller; the detail screen shows a "Starter prompts" section with a count that opens the editor, and shows the new list after a save; with `can_edit_starter_prompts == false` the section lists the prompts without an edit action. Confirm RED.
- [x] 8a.4 Add `persona_starters_editor_screen.dart` (adaptive widgets) and a `persona_starters_section.dart` tile on `persona_detail_screen.dart`, plus a go_router route. Confirm GREEN.
- [ ] 8a.5 Playwright: update `persona-detail.png` (`visual-regression.spec.ts:541`); the new section appears even for a Persona with no starters (the e2e user is an admin, so the editor is offered). Regenerate after rebasing and describe the diff in the PR body.
- [x] 8a.6 Docs: extend `docs/web-ui.md` with the starter prompts editor, its limits (4 prompts, 200 characters), who may edit (owner of a personal Persona, space admin for an org-owned one) and that there are no org-level fallback starters.
- [x] 8a.7 `make lint-flutter && make test-flutter`.

## PR 8b — feat(app): show Persona starter prompts in the empty chat

Layer: client
Depends on: PR 6, PR 8a, chat-element-library PR 9
Parallel with: PR 7c

- [x] 8b.1 Test first in `app/test/widget/features/chat/chat_empty_state_starters_test.dart`: with an active Persona that has starters, `starter_prompt_0` renders in the empty state and a tap calls `sendMessage`; with none, or while loading or after an error, only the default copy renders. Confirm RED.
- [x] 8b.2 Add a small `chat_empty_starters.dart` adapter that reads `activePersonaStarterPromptsProvider` and passes `StarterPrompts` into `ThreadEmptyState.starters`; `ChatScreen` only swaps in the adapter. Confirm GREEN.
- [ ] 8b.3 Playwright: add a fixture Persona with starters (via `page.route` on the Persona endpoints, or by `PATCH`ing the seeded Persona in the test setup) and a new `empty-chat-starters.png` baseline. Regenerate after rebasing and describe the diff in the PR body.
- [x] 8b.4 `make lint-flutter && make test-flutter`, then `make lint && make format && make test`.

## PR 9 — chore(openspec): archive follow-up-suggestions

Layer: docs
Depends on: PR 1, PR 2, PR 3, PR 4, PR 5, PR 6, PR 7a, PR 7b, PR 7c, PR 8a, PR 8b, and the chat-stop-cancels-turn archive
Parallel with: none (wave 8; archive order is chat-stop-cancels-turn, follow-up-suggestions, tool-approval-prompts)

- [ ] 9.1 Confirm every PR of this change has merged and chat-stop-cancels-turn has been archived. Run `openspec validate follow-up-suggestions --strict` and `openspec validate --specs --strict`; both are clean (task 1.0 fixed the main `conversation-event-log` spec).
- [x] 9.2 Run `openspec archive follow-up-suggestions`. Commit the resulting spec updates. tool-approval-prompts archives after this change.
