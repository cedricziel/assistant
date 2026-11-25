# Tasks — tool-approval-prompts

A stack of PRs. Each one stays under ~500 changed lines, starts with a failing
test, and leaves `main` shippable.

**Shippable at every step.** The server PRs (1–18) land dormant: production
bootstraps do not call `SpaceRuntimes::with_interactions()`, so the callback
stays `None` (deny on `Ask`, as today) and no turn gets `ask-user`. Tests call
the builder explicitly. **PR 28 flips the switch**, and it merges only after
the client can render and answer approvals (PR 24) and questions (PR 25).
PRs 9 and 10 change behaviour that is already live (the runner and streaming
in the split topology), and each is correct on its own.

**Generated client.** Every PR that changes `openapi.json` runs
`make dump-openapi && make generate-flutter-client` and commits both, so the
`api-client-drift` job (`.github/workflows/flutter.yml:60-97`) stays green.

**Shared turn runner.** `message-branching` PR 5 extracts the `send_message`
SSE loop into the shared turn runner (taking `RunOptions { interactive,
suggestions, push, anchor }`). This change edits that runner, never
`send_message` directly (PRs 9, 10, 12 and 18).

**Migrations.** PRs 3, 4 and 18 add space.db migrations (`interactions`,
`tool_permission_rules`, `push_subscription_user`). Use the numbers
pre-allocated in the cross-change stack plan and register each in both
migration lists in `crates/storage/src/lib.rs`.

**Playwright baselines.** Whichever PR changes a screen updates its baselines
(PRs 24, 25, 26, 27) and records the visual diff in the PR body. Never merge
PNGs by hand: after rebasing, regenerate them (`.claude/skills/e2e-testing`).
Add any route mocks those PRs need (`/api/tool-permission-rules`,
`/api/conversations/{id}/interactions`).

**Parallelism at a glance**

| Can run in parallel                | Notes                                                                   |
| ---------------------------------- | ----------------------------------------------------------------------- |
| PR 1, 3, 4, 5, 9                   | Wave 1. PR 1 lands with ADR-0018 and ADR-0019. PR 9 needs MB PR 5 only. |
| PR 2, 6, 7                         | Wave 2. PR 2 after chat-stop PR 1. PR 7 before tool-result-renderers S3 |
| PR 8 → PR 10                       | PR 8 after PR 7 and chat-stop PR 1; PR 10 after PR 8 and PR 9           |
| PR 11 with PR 10, 16               | Broker needs PRs 3, 6, 7                                                |
| PR 13, 16, 18 with each other      | After PR 11 / 12                                                        |
| PR 19, 21, 22 with every server PR | Client package work; needs chat-element-library PRs 1, 3, 4 only        |
| PR 25, 26, 27 with each other      | After PR 24 / 23                                                        |

Sibling edges in one place: chat-stop-cancels-turn PR 1 (2, 8) and PR 4/5a
(23, 24); message-branching PR 5 (9, 10, 12, 18); follow-up-suggestions PR 2
(14); tool-result-renderers S4 and C1 (25); chat-element-library PRs 1, 3, 4
(19), 5, 6, 9, 10 (24); chat-a11y-and-code-blocks C2 (24). Reverse edges that
siblings take on this change: generative-ui-a2ui PR 5 → PR 8, PR 8 → PRs 11
and 13, PR 12 → PRs 23 and 24; message-branching PR 7 → PR 8.

---

## PR 1 — `docs(adr): ADR-0017 turn interactions and tool approvals`

Layer: docs · Depends on: — · Parallel with: everything; land together with
ADR-0018 (message-branching) and ADR-0019 (generative-ui-a2ui)

- [ ] 1.1 Write `docs/adr/adr-0017-turn-interactions.md`. Cover the generic
      interaction channel and the kind registry (D1); the topology, with the
      shared table, store-polled waiters, the run event relay and the single
      event-log writer (D2); the async typed callback, the explicit
      interactivity flag and removal of `AutoDenyConfirmation` (D3); where
      saved rules sit in the pipeline (D4); and the dormant-until-enabled
      rollout (D9). Cross-link `adr-0011-tool-permission-decision-model.md`
      by filename (there are two ADR-0011 files) and
      `adr-0013-per-space-runtimes.md`. Note that `generative-ui-a2ui`
      (ADR-0019) adds the `surface_action` kind.
- [ ] 1.2 Add "Interaction" to `docs/glossary.md`.
- [ ] 1.3 Fix the main specs `openspec/specs/notification-settings/spec.md`
      and `openspec/specs/pwa-push-notifications/spec.md`. Both start with a
      delta header (`## ADDED Requirements`) instead of `## Requirements`, so
      archive would refuse this change's deltas to them.

## PR 2 — `fix(runtime): heartbeat bus claims while a turn runs`

Layer: server · Depends on: chat-stop-cancels-turn PR 1 (same `worker.rs`
region) · Parallel with: 1, 3–7

- [ ] 2.1 RED: storage contract test for `MessageBus::touch(message_id)`.
      After `touch`, `reap_stale(5 min)` with a clock advanced 4 min past the
      touch does not reset the message. Run it against `SqliteMessageBus`.
- [ ] 2.2 Add `touch` to the `MessageBus` trait (`crates/core/src/bus.rs`).
      Implement it for SQLite (`UPDATE bus_messages SET claimed_at = ? WHERE
id = ? AND status = 'claimed'`), for the scoped bus, and for NATS
      (`AckKind::Progress`, `crates/bus-nats/src/lib.rs`). GREEN.
- [ ] 2.3 RED: worker test with a scripted tool that sleeps past a
      shortened heartbeat interval. Assert `touch` was called on the claimed
      message id. GREEN: spawn a 60 s heartbeat beside the turn future in
      `worker.rs` and abort it when the turn finishes.
- [ ] 2.4 `make lint && make format && make test`.

## PR 3 — `feat(storage): interactions store`

Layer: server · Depends on: — · Parallel with: 1, 2, 4–7

- [ ] 3.1 RED: contract tests (`crates/storage/tests/contract/`) for
      `InteractionStore`, run against both `InMemoryInteractionStore` and
      `SqliteInteractionStore`. Cover: `create` then `get`;
      `resolve_if_pending` succeeds once and reports `AlreadyResolved(row)` the
      second time; `list_for_conversation(status filter)` in creation order;
      `touch_waiter`; `expire_dead(older_than)` expires only `pending` rows
      with a stale `waiter_seen_at`; `cancel_pending_for_run` returns the
      affected ids; `list_unemitted_for_run` returns every row of the run
      with `resolution_emitted = false`, including resolved ones; and
      `mark_emitted`.
- [ ] 3.2 Migration `NNN_interactions.sql` (space.db). Columns as in
      design.md D2, plus an index on (`conversation_id`, `created_at`) and one
      on (`run_id`, `status`).
- [ ] 3.3 Implement both stores and re-export the in-memory one through
      `assistant_test_support::prelude`. GREEN.

## PR 4 — `feat(storage): tool permission rules store`

Layer: server · Depends on: — · Parallel with: 1–3, 5–7

- [ ] 4.1 RED: contract tests for `ToolPermissionRuleStore` (both impls).
      Cover: `upsert` is idempotent on (`user_id`, `effect`, `rule`);
      `list_for_user` returns only that user's rows; `delete(id, user_id)`
      returns `false` for another user's row.
- [ ] 4.2 Migration `NNN_tool_permission_rules.sql` (space.db). Implement
      both stores. GREEN.

## PR 5 — `feat(core): suggest permission rules for approvals`

Layer: server · Depends on: — · Parallel with: 1–4, 6, 7

- [ ] 5.1 RED: `crates/core/src/permission/suggest.rs` unit tests for every
      scenario of "Suggested rules are derived deterministically": `git
status --short` → `bash(git status*)`; `npm -s run build` →
      `bash(npm -s run*)`; a pipe returns `None`; `rm -rf build` returns
      `None`; `python3 -c …`, `node -e …`, `sh -c …`, `bash -lc …`, `env X=1
cmd` and `xargs rm` return `None`; the web-fetch host; the bare-tool
      fallback for file tools and MCP tools; and `*` escaping.
- [ ] 5.2 Implement `suggest_rule` (pure, no I/O). GREEN.

## PR 6 — `feat(core): saved-rule stages in the permission pipeline`

Layer: server · Depends on: — · Parallel with: 1–5, 7

- [ ] 6.1 RED: `static_rules.rs` tests that `evaluate_deny`,
      `evaluate_ask` and `evaluate_allow` each return what the matching stage
      of today's `evaluate` returns, and that `evaluate` still composes them
      in the same order.
- [ ] 6.2 Split `StaticRules::evaluate` (`static_rules.rs:56-96`) into those
      three stage functions; `evaluate` calls them in order. GREEN with no
      behaviour change.
- [ ] 6.3 RED: `policy.rs` tests with an in-memory `UserRuleSource`. Cover:
      user deny beats static allow; configured deny beats user allow; a
      configured ask beats user allow; user allow satisfies
      `requires_confirmation`; user allow skips the decision model; the
      decider is `user-rules`; and a `Resolution` from a configured ask rule
      reports `offers_always = false`.
- [ ] 6.4 Add the `UserRuleSource` trait, `UserRules`, and
      `PermissionPolicy::with_user_rules`, interleaving the user stages as in
      design.md D4. Expose `Resolution::offers_always`. GREEN.
- [ ] 6.5 RED, then GREEN: add `interaction_timeout_secs` (default 600, range
      `30..=3600`) to `PermissionsConfig`; an out-of-range value fails the
      load with an error naming the key.

## PR 7 — `refactor(runtime)!: async typed ConfirmationCallback, no AutoDenyConfirmation`

Layer: server · Depends on: — · Parallel with: 1–6. Merge before
tool-result-renderers S3, which edits the same `dispatch.rs` denied path.

- [ ] 7.1 RED: update `crates/runtime/src/orchestrator/tests/permission.rs`
      to use an async scripted callback that returns each `ConfirmationAnswer`.
      Assert that `AllowOnce`/`AllowAlways` run the tool and that the other
      answers produce the matching observation (`User denied …`, `…expired…`).
- [ ] 7.2 Change `ConfirmationCallback` to the async `ConfirmationRequest`
      → `ConfirmationAnswer` shape (design.md D3). Thread `event_sink` and
      `tool_call_id` into `authorize_tool_call` (`dispatch.rs:370`). Make
      `SpaceRuntime` take `Option<Arc<dyn ConfirmationCallback>>`. GREEN.
- [ ] 7.3 Delete `AutoDenyConfirmation` (`bootstrap.rs:20-38`) and its tests.
      Callers in `web-ui` (`lib.rs:329`, `space_tasks.rs:157`,
      `test_helpers.rs:199`), `interface-cli` (`main.rs:144-151`),
      `space_runtime.rs` and `space_supervisor.rs` tests pass `None` or the
      CLI callback. No shim.
- [ ] 7.4 Port `CliConfirmation` to async using `spawn_blocking`, offering
      once answers only. Test it with an injected reader.

## PR 8 — `feat(runtime): explicit interactive submit option and run id on turns`

Layer: server · Depends on: PR 7, chat-stop-cancels-turn PR 1 (same
`worker.rs` / `bus_messages.rs` region) · Parallel with: 9, 11. Taken as a
dependency by message-branching PR 4 (its `SubmitTurn` struct absorbs
`SubmitOptions`) and PR 7, and by generative-ui-a2ui PR 5.

- [ ] 8.1 RED: orchestrator tests. A `Web` turn submitted with default
      `SubmitOptions` has `ctx.interactive == false`; one submitted with
      `interactive: true` has `true` and `ctx.run_id == Some(request_id)`.
- [ ] 8.2 RED: `interface-cli` tests that a REPL turn gets
      `ctx.interactive == true`, and that the one-shot CLI command
      (`main.rs:196-201`) and a `--no-repl` process do not.
- [ ] 8.3 Add `SubmitOptions { interactive, relay_events }` to
      `submit_turn` / `submit_turn_with_request_id`, `interactive` to
      `TurnRequest` (`bus_messages.rs:60`), and `run_id` to
      `ExecutionContext`. Derive `interactive` from the request flag, not
      from `Interface` (`mod.rs:1233`). The REPL (`main.rs:504-511`) passes
      `interactive: true`. GREEN.
- [ ] 8.4 Update the 33 files with `ExecutionContext { … }` literals. If the
      diff passes ~500 lines, land the `run_id` field with a
      `ExecutionContext::default()`-based helper first as PR 8a.
- [ ] 8.5 `send_message` does not set `interactive` yet. The shared runner
      sets it in PR 28, so web stays deny-on-`Ask` until then.

## PR 9 — `fix(web-ui): keep persisting run events after the SSE client leaves`

Layer: server · Depends on: message-branching PR 5 (shared turn runner) ·
Parallel with: 1–8

- [ ] 9.1 RED: runner test. Drop the SSE receiver after `run_started`; the
      scripted turn emits 100 events and finishes. The persisted log holds all
      100 events and `done`, and the turn was never blocked on a full sink.
- [ ] 9.2 In the runner, replace `if sse_tx.send(..).is_err() { break; }`
      (`messages.rs:296-298,359-361` before extraction) with a
      `client_gone` flag that only stops forwarding. Move end-of-run work
      (thinking flush, terminal frame, `finish_run`) into one exit path
      that every branch reaches. GREEN.

## PR 10 — `feat(runtime): relay run events across processes over the bus`

Layer: server · Depends on: PR 8, PR 9 · Parallel with: 11, 16

- [ ] 10.1 RED: worker test. A turn submitted with `relay_events: true` and no
      local sink publishes its events on `turn.event` with
      `batch_id = run_id`, `Token`/`Thinking` coalesced to at most one message
      per 200 ms; with a local sink, nothing is published.
- [ ] 10.2 Implement the relay sink in `worker.rs` (fallback when
      `token_sinks` has no entry, `worker.rs:188`). Serialize
      `OrchestratorEvent` with a stable serde form. GREEN.
- [ ] 10.3 RED: runner test with web-ui and worker in **separate runtimes**
      that share a `space.db` and a bus. The SSE stream receives tokens, a
      tool result and `done`, and ends when the turn ends.
- [ ] 10.4 The runner sets `relay_events: true`, claims `turn.event` with a
      `batch_id` filter, merges those events into its channel, and
      `select!`s over the channel and the turn result so it ends with the
      turn. GREEN.
- [ ] 10.5 Update `docs/web-ui.md` (operator note): streaming now works with
      the orchestrator in a separate process.

## PR 11 — `feat(runtime): interaction broker and kind registry`

Layer: server · Depends on: PR 3, PR 6 (timeout config), PR 7 · Parallel
with: 8–10, 16

- [ ] 11.1 RED: `crates/runtime/src/interaction/` tests using
      `InMemoryInteractionStore` and a paused Tokio clock. Cover: `request`
      persists `pending` and emits `InteractionRequested` on the sink; a
      same-process `respond` resumes the waiter at once; a resolution written
      straight to the store (another process) resumes it within 2 s; the
      waiter refreshes `waiter_seen_at` every 15 s; expiry emits `{expired}`;
      `expires_at` is clamped to the turn deadline; a non-owner `respond`
      returns `NotFound`; a duplicate same answer returns `Ok(same)`; a
      different answer returns `Conflict`; a stale waiter makes `respond`
      return `Gone` and expire the row.
- [ ] 11.2 RED: `InteractionKinds` tests. Validation, observation, push
      copy, progress label and `renders_card` for `tool_approval` and
      `question`; an unregistered kind fails validation.
- [ ] 11.3 Implement `InteractionBroker` (D2) with a drop guard on waiters,
      the `InteractionKindHandler` trait and registry (D1), and
      `arguments` truncation at 16 KiB (D10). Add the broker to
      `SpaceRuntime` behind `SpaceRuntimes::with_interactions()`. GREEN.

## PR 12 — `feat(runtime): interaction stream events and end-of-run cleanup`

Layer: server · Depends on: PR 10, PR 11, message-branching PR 5 · Parallel
with: 13, 16

- [ ] 12.1 RED: `projection/mod.rs` tests that `SseProjector` maps both new
      `OrchestratorEvent` variants to `interaction_requested` /
      `interaction_resolved` with the exact JSON in design.md D1, and that
      `CliProjector` renders a one-line prompt notice. GREEN. Handle the new
      variants explicitly in every other exhaustive `match` over
      `OrchestratorEvent` (A2A projection, channel adapters).
- [ ] 12.2 RED: runner tests. (a) Cancel with one pending interaction: the
      log ends `interaction_resolved{cancelled}` then
      `agent_error{reason: cancelled}`. (b) The respond compare-and-set wins,
      then the turn is cancelled before the waiter emits: the log carries
      `interaction_resolved{resolved}` before the terminal frame. (c) Persisting
      an `interaction_resolved` frame sets `resolution_emitted`.
- [ ] 12.3 GREEN: the runner marks rows emitted as it persists their frames,
      and on every exit path calls `broker.abandon_run(run_id)` and writes one
      `interaction_resolved` per returned row before the terminal frame.
- [ ] 12.4 RED, then GREEN: `scheduler/reap.rs` tests. Dead pending rows
      become `expired` even when the run has an active bus message (and with
      a bus whose `has_active_message` always returns true). When the sweep
      appends a synthetic terminal, it first appends
      `interaction_resolved{expired}` for each unemitted row.

## PR 13 — `feat(web-ui): respond to turn interactions`

Layer: server · Depends on: PR 11 · Parallel with: 12, 16, 18. Taken as a
dependency by generative-ui-a2ui PR 8.

- [ ] 13.1 RED: handler tests in `crates/web-ui/src/api/interactions.rs`
      covering every scenario of "Owners answer through one respond endpoint"
      and "Responses are idempotent and first-writer-wins": 200, 400 (wrong
      kind, unknown option, two selections for `multiple = false`, 4001-char
      text), 403 (viewer and missing scope), 404 (unknown conversation,
      another user, wrong run, unknown id), 409, 410 (expired, cancelled, no
      live waiter) and 413. Assert the `{"error": …}` body shape for every
      error.
- [ ] 13.2 Make `owned_conversation` (`conversations.rs:50-62`) return
      enveloped `404`/`500` bodies for all callers, with a test for one
      existing caller.
- [ ] 13.3 Implement `POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`
      (`respond_to_interaction`), validating through `InteractionKinds`, with
      a 64 KiB body limit on the route. Add `utoipa` docs for
      `RespondToInteractionRequest`, `InteractionDetail` and the SSE event
      schemas (`SseInteractionRequestedEvent`,
      `SseInteractionResolvedEvent`). GREEN.
- [ ] 13.4 RED, then GREEN: a tampered body carrying a `rule` field is
      rejected with `400` (`deny_unknown_fields`).
- [ ] 13.5 `make dump-openapi && make generate-flutter-client`; commit both.

## PR 14 — `feat(web-ui): list interactions and report awaiting_input`

Layer: both · Depends on: PR 12, PR 13, follow-up-suggestions PR 2
(presence-based turn status) · Parallel with: 15, 16

- [ ] 14.1 RED: handler tests for `GET
/api/conversations/{conversation_id}/interactions` (bare array of
      `InteractionDetail`, `?status=pending`, only the caller's
      conversation) and for `ConversationDetail.interactions`
      (`conversations.rs:77`), in creation order.
- [ ] 14.2 Implement both (`list_conversation_interactions`). GREEN.
- [ ] 14.3 RED: `turns.rs` tests for the rule in the turn-status-api delta:
      `awaiting_input` while any `interaction_requested` has no matching
      `interaction_resolved`, even when a later `status` row is the last;
      `running` once resolved; `errored`/`completed` still win. GREEN:
      add `TurnState::AwaitingInput` and extend `last_event_kind`.
- [ ] 14.4 `make dump-openapi && make generate-flutter-client`.
- [ ] 14.5 RED, then GREEN: client test that the stalled-stream probe treats
      `TurnState.awaitingInput` like `running` (`chat_provider.dart:932`).

## PR 15 — `feat(runtime): approve tool calls through interactions, with saved rules`

Layer: server · Depends on: PR 4, PR 5, PR 6, PR 12, PR 13 · Parallel with:
14, 16

- [ ] 15.1 RED: integration test with the HTTP routes and the turn worker in
      **separate runtimes** sharing `space.db` and the bus
      (`ScriptedLlmProvider`, `with_interactions()` on both). A web turn
      submitted with `interactive: true` calls `bash` with no allow rule; the
      SSE stream carries `interaction_requested{tool_approval}`; posting
      `allow_once` makes the tool run and `done` arrive. Variants:
      `reject_once` (the LLM sees the denial), expiry, and `allow_always`.
      For `allow_always`, the rule is saved, the resolution carries
      `rule_id`, and a second turn runs without a prompt.
- [ ] 15.2 RED: the "Answer after closing the tab" scenario. Disconnect the
      SSE client while the approval is pending, answer through the respond
      route, replay with `since=0`, and expect the resolution, the tool result
      and `done`.
- [ ] 15.3 Implement `InteractiveConfirmation` (broker plus rules store): the
      options come from `Resolution::offers_always` and `suggest_rule`, and
      the "always" answers upsert the rule from the stored interaction. Add
      a `UserRuleSource` adapter over `ToolPermissionRuleStore`, attach it in
      `SpaceRuntime`, and route `Interface::Cli` turns to the CLI callback in
      a REPL process. Emit the `tool approval answered` audit event. GREEN.
- [ ] 15.4 Update `docs/tool-permissions.md`: approvals, the four options,
      when "always" is withheld, saved rules, evaluation order,
      `interaction_timeout_secs`, and that messenger interfaces still deny on
      `Ask`.

## PR 16 — `feat(web-ui): list and revoke saved tool permission rules`

Layer: server · Depends on: PR 4 · Parallel with: 5–15, 17, 18

- [ ] 16.1 RED: handler tests for `GET /api/tool-permission-rules` (bare
      array, only the caller's rows, space from `X-Space-Id`) and `DELETE
/api/tool-permission-rules/{id}` (`204`, `404` for another user's id,
      `403` for a viewer), all errors enveloped.
- [ ] 16.2 Implement both routes with `utoipa` docs
      (`ToolPermissionRuleSummary`). GREEN.
- [ ] 16.3 `make dump-openapi && make generate-flutter-client`.

## PR 17 — `feat(runtime): ask-user tool`

Layer: server · Depends on: PR 11, PR 13, PR 15 (`with_interactions()`
wiring) · Parallel with: 16, 18

- [ ] 17.1 RED: `AskUserTool` unit tests for input validation (0 or 5
      questions, empty prompt, 501-char prompt, 9 options, 101-char label) and
      free text being forced when there are no options. Each invalid input
      returns `ToolOutput::error` and creates no interaction.
- [ ] 17.2 RED: the observation for the answered case is
      `{"answers":[{"question", "selected", "text"}]}` in request order;
      decline and expiry return the fixed success texts.
- [ ] 17.3 Implement it and add it to `turn_overlay` (`orchestrator/mod.rs:359`)
      only when `ctx.interactive` and a broker exist. Add `ask-user` to
      `builtin_allow` (`static_rules.rs:285`). GREEN.
- [ ] 17.4 RED, then GREEN: the tool-spec lists of a scheduler turn and of a
      voice turn do not contain `ask-user`.

## PR 18 — `feat(web-ui): push the turn owner when an interaction waits`

Layer: both (server plus `app/web/sw.js`) · Depends on: PR 12,
message-branching PR 5 · Parallel with: 13–17

- [ ] 18.1 RED: storage test showing a subscription upserted with a user id
      is returned by `list_for_user` and not for another user. Then add
      migration `NNN_push_subscription_user.sql` (nullable `user_id`). GREEN.
- [ ] 18.2 RED, then GREEN: `POST /api/push/subscribe` stores the caller's
      user id.
- [ ] 18.3 RED: `wiremock` test of `PushDispatcher::send_to_user`. An
      interaction pending for 30 s (paused clock) sends one push, with the
      kind's push copy, `tag` and `conversationId`, to the owner's endpoint
      only; one resolved at 10 s sends none. GREEN: schedule the delayed push
      from the `interaction_requested` frame in the shared runner, and cancel
      it on resolution.
- [ ] 18.4 `app/web/sw.js` passes `payload.tag` to `showNotification`. Check
      it manually in Chrome (two pushes with one tag show one notification)
      and record the result in the PR body.

---

## PR 19 — `feat(app): interaction models and ApprovalCard`

Layer: client · Depends on: chat-element-library PR 1 (package), PR 3
(widgetbook and `kAssistantUiElements`), PR 4 (models in the package) ·
Parallel with: all server PRs, 21, 22

- [ ] 19.1 RED: `app/packages/assistant_ui/test/approval_card_test.dart`.
      Pending shows the tool, the summary, the reason, a countdown, and only
      the offered buttons. "Always" buttons show `suggested_rule` and the
      scope note. Truncated arguments say so. Tapping fires `onDecision` with
      the right enum. A submitting card has its buttons disabled. Each
      resolved state renders a receipt with no buttons. Semantics labels read
      "Allow bash once" and so on. Enter maps to allow once on every
      platform, and Escape does nothing.
- [ ] 19.2 Add the `InteractionRecord` (with `rendersCard`),
      `InteractionKind` (including `unknown`) and `ApprovalDecision` models in
      `lib/src/models/`, plus `TimelineEntryType.interaction` (the no-id
      fallback only). Implement `ApprovalCard`. GREEN.
- [ ] 19.3 Register `ApprovalCard` in `kAssistantUiElements` with one
      widgetbook use-case and add its `ELEMENTS.md` row.

## PR 20 — `test(app): ApprovalCard goldens, stories and UnknownInteractionCard`

Layer: client · Depends on: PR 19 · Parallel with: 21, 22

- [ ] 20.1 RED: `unknown_interaction_card_test.dart`: inert notice, no
      callbacks.
- [ ] 20.2 Implement `UnknownInteractionCard`. GREEN.
- [ ] 20.3 Golden tests for every `ApprovalCard` state × light/dark, and
      widgetbook use-cases for each state; register the unknown card.

## PR 21 — `feat(app): QuestionCard element`

Layer: client · Depends on: PR 19 (models) · Parallel with: all server PRs,
20, 22

- [ ] 21.1 RED: `question_card_test.dart`. Cover single (radio), multiple
      (checkbox) and free-text questions, and multi-question; submit stays
      disabled until every question is answered; `onAnswer` gets the
      structured answers; `onDecline` fires; and each resolved state is a
      receipt.
- [ ] 21.2 Implement `QuestionCard`. GREEN.
- [ ] 21.3 Goldens, widgetbook use-cases, registry entry and `ELEMENTS.md`
      row.

## PR 22 — `feat(app): PermissionRuleTile element`

Layer: client · Depends on: PR 19 · Parallel with: all server PRs, 20, 21

- [ ] 22.1 RED, then GREEN: `permission_rule_tile_test.dart` checks the
      effect badge, rule text, date, and that the revoke callback fires.
- [ ] 22.2 Goldens, widgetbook use-case, registry entry and `ELEMENTS.md` row.

## PR 23 — `feat(app): interaction stream events and API calls`

Layer: client · Depends on: PR 13, PR 14, PR 16 (generated client),
chat-stop-cancels-turn PR 4 (stream-event lane order) · Parallel with:
19–22. Taken as a dependency by generative-ui-a2ui PR 12.

- [ ] 23.1 RED: `parseSseByteStream` tests showing that
      `interaction_requested` (approval, question, an unknown kind, and
      `renders_card: false`) and `interaction_resolved` parse into new
      `InteractionRequestedEvent` / `InteractionResolvedEvent` subclasses
      with `sequenceId`; that malformed JSON is dropped; and that an unknown
      event name is still ignored.
- [ ] 23.2 Add the subclasses to the sealed `StreamEvent`
      (`app/lib/api/models/stream_event.dart`) and the branches in
      `api_client.dart`. Update every exhaustive `switch` over `StreamEvent`
      in the app. GREEN.
- [ ] 23.3 RED, then GREEN: `respondToInteraction`,
      `listConversationInteractions`, `listToolPermissionRules` and
      `deleteToolPermissionRule` on `ApiClient`, tested against a mocked
      Dio.

## PR 24 — `feat(app): approve tool calls in chat`

Layer: client · Depends on: PR 20, PR 23, chat-element-library PR 5
(`EntryState.awaitingInput`), PR 6 (`ToolCallBody.footer`), PR 9 (pure
`TurnProgressCard`), PR 10 (`ChatThreadLayout.aboveComposer`),
chat-a11y-and-code-blocks C2 (`ChatAnnouncer`), chat-stop-cancels-turn PR 5a
(Stop cancels the server turn) · Parallel with: 25 (partly), 26. Taken as a
dependency by generative-ui-a2ui PR 12.

- [ ] 24.1 RED: `ChatNotifier` unit tests. `InteractionRequestedEvent` with a
      `tool_call_id` attaches the interaction to that tool call entry and
      sets it `awaitingInput`; without an id it adds an `interaction` entry in
      order. The turn status snapshot carries the kind's progress label.
      `respond()` marks it submitting, calls the API, and on `200` waits for
      `InteractionResolvedEvent` to flip it to resolved. `409`/`410` reload
      the record from the pending list. A failed request restores the
      pending state and shows an error.
- [ ] 24.2 Add the client kind registry (`interaction_kinds.dart`):
      `tool_approval` → `ApprovalCard` footer, label "Waiting for your
      approval"; unregistered kinds with `rendersCard` → `UnknownInteractionCard`;
      `rendersCard: false` → no card, label "Waiting for your input".
- [ ] 24.3 Implement the reducers in `chat_timeline_reducers.dart` and
      `respond()` in a new `interaction_actions.dart`; do not grow
      `chat_provider.dart`. GREEN.
- [ ] 24.4 RED, then GREEN: `TurnProgressCard` receives `awaitingLabel`, so it
      reads "Waiting for your approval" and hides the stall label while an
      interaction is pending. `ChatAnnouncer` announces "Approval needed" and
      suppresses its stall announcement.
- [ ] 24.5 RED, then GREEN: a widget test that the chat timeline renders
      `ApprovalCard` in the tool call's footer and that tapping "Allow once"
      reaches `respond()`; and one that Stop while waiting calls the server
      cancel and the card ends "Cancelled".
- [ ] 24.6 Update the Playwright chat baselines (approval card, awaiting
      label) and record the diff.

## PR 25 — `feat(app): ask-user questions, receipts after reload, pending discovery`

Layer: client · Depends on: PR 17, PR 21, PR 24, tool-result-renderers S4
(`ToolCallSummary.tool_call_id`) and C1 (`ToolCallRecord.toolCallId`) ·
Parallel with: 26, 27

- [ ] 25.1 RED: a widget test that a `question` interaction renders
      `QuestionCard` in the `ask-user` tool call's footer and that submit
      calls `respond()` with the answers; the label reads "Waiting for your
      answer".
- [ ] 25.2 RED: a mapping test (`chat_history_mapping.dart`) showing that
      `ConversationDetail.interactions` become receipts on the tool call with
      the matching `tool_call_id`; without an id, they use the same per-turn
      ordinal join as tool-result-renderers, then the end of the turn.
- [ ] 25.3 RED: a provider test where opening a conversation with a pending
      interaction calls the pending list and attaches to the run's replay
      stream. GREEN for 25.1–25.3.
- [ ] 25.4 Update affected Playwright baselines and record the diff.

## PR 26 — `feat(app): tool approvals settings section`

Layer: client · Depends on: PR 16, PR 22, PR 23 · Parallel with: 24, 25, 27

- [ ] 26.1 RED: unit tests for `toolPermissionRulesProvider`
      (AsyncNotifier). Cover loading, success, error, empty, and `revoke`
      optimistic removal with rollback on error.
- [ ] 26.2 RED: a widget test for `ToolApprovalsSection`: Allowed/Denied
      groups, the empty state copy, and that revoke asks for confirmation
      and then calls the provider.
- [ ] 26.3 Implement `app/lib/features/settings/tool_approvals_section.dart`
      (its own file) and add it to `settings_screen.dart`. GREEN.
- [ ] 26.4 Mock `/api/tool-permission-rules` in the e2e API mocks, update the
      `settings.png` baseline
      (`crates/web-ui/e2e/tests/visual-regression.spec.ts:547-556`) and
      record the diff.

## PR 27 — `feat(app): notify on approval requests when backgrounded`

Layer: client · Depends on: PR 24 · Parallel with: 25, 26

- [ ] 27.1 RED: `NotificationPreferences` test showing that
      `notifyApprovalRequests` defaults to true and persists.
- [ ] 27.2 RED: an `AgentEventListener` test where an interaction request
      while not `resumed` shows "Approval needed" (or "Your assistant has a
      question") with `conversationId`; nothing is shown when resumed, when
      the category is off, or on web with an active push subscription.
      GREEN.
- [ ] 27.3 Add the "Approval requests" toggle to the notifications settings
      section, with a widget test. Update the `settings.png` baseline and
      record the diff.

---

## PR 28 — `feat(cli,web-ui): enable tool approvals and questions in web chat`

Layer: server · Depends on: PR 15, PR 17, PR 18, PR 24, PR 25 · **This PR
flips the switch.**

- [ ] 28.1 RED: integration test with the web-ui bootstrap and the
      orchestrator bootstrap in separate runtimes: a web chat turn that hits
      `Ask` produces `interaction_requested` instead of an immediate denial,
      and the shared runner submits send, edit and regenerate turns with
      `interactive: true` and voice turns without.
- [ ] 28.2 Call `SpaceRuntimes::with_interactions()` in the `interface-cli`
      orchestrator bootstrap (`main.rs:144-151`, replacing the auto-deny
      branch) and in `crates/web-ui/src/lib.rs`. The shared runner sets
      `interactive: true` for `send_message`, `edit_message` and
      `regenerate_message`. GREEN.

## PR 29 — `docs(app): approvals and questions guide; ship`

Layer: both · Depends on: PR 26, PR 27, PR 28

- [ ] 29.1 Write `docs/chat/approvals-and-questions.md`: what the cards
      mean, the four options and when "always" is withheld, where saved rules
      apply, revoking, timeouts, push behaviour, and that messenger
      interfaces do not prompt.
- [ ] 29.2 `make lint && make format && make test`;
      `make lint-flutter && make test-flutter`.
- [ ] 29.3 Manual smoke test with the packaged two-service topology: approve
      once, always, reject and let one expire in web chat; stop a turn while
      it waits; reload during a wait; close the tab, answer from a second
      device after the push; answer a multi-question `ask-user`; revoke a rule
      in settings; restart the orchestrator during a wait and confirm the card
      ends "Expired".
- [ ] 29.4 `openspec archive tool-approval-prompts`, after
      chat-stop-cancels-turn and follow-up-suggestions have archived (all
      three modify `turn-status-api`).
