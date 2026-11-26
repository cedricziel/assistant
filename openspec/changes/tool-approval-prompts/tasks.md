# Tasks — tool-approval-prompts

A stack of PRs. Each one stays under ~500 changed lines (generated
`assistant_api` code, `openapi.json` and golden PNGs excluded), starts with a
failing test, and leaves `main` shippable. PR ids follow the cross-change
stack plan (`openspec/chat-ui-parity-stack.md`, "TAP PR n"). A letter suffix
splits one planned PR into reviewable parts that land in the same wave.

**Shippable at every step.** The server PRs (1–12) land dormant: production
bootstraps do not call `SpaceRuntimes::with_interactions()`, so the callback
stays `None` (deny on `Ask`, as today) and no turn gets `ask-user`. Tests call
the builder explicitly. **PR 20a flips the switch.** It is the last server
PR, and it merges only after the client can render and answer approvals
(PR 16) and questions (PR 17). There is no feature flag and no config option
(design D9). PRs 7b and 7c change behaviour that is already live (the runner
and streaming in the split topology), and each is correct on its own.

**Rollout order is the decision.** The client cards, their wiring and the
receipts all merge before 20a. Do not reorder 20a earlier "to unblock
testing": integration tests call `with_interactions()` themselves.

**Generated client.** Every PR that changes `openapi.json` runs
`make dump-openapi && make generate-flutter-client` and commits both, so the
`api-client-drift` job (`.github/workflows/flutter.yml:60-97`) stays green.

**Shared turn runner.** `message-branching` PR 5 extracts the `send_message`
SSE loop into the shared turn runner (taking `RunOptions { interactive,
suggestions, push, anchor }`). This change edits that runner, never
`send_message` directly (PRs 7b, 7c, 12 and 20a). Hotspot order inside the
runner: `follow-up-suggestions` PR 3 merges before PR 7b and 7c.

**Migrations.** PRs 3, 4 and 12 add space.db migrations. Use the numbers
pre-allocated in the cross-change stack plan and register each in both
migration lists in `crates/storage/src/lib.rs`:
`047_interactions` (PR 3), `048_tool_permission_rules` (PR 4),
`059_push_subscription_user` (PR 12).

**Playwright baselines.** Whichever PR changes a screen updates its baselines
(PRs 16, 17, 18a, 18b, 19) and records the visual diff in the PR body. Never
merge PNGs by hand: after rebasing, regenerate them
(`.claude/skills/e2e-testing`). Add any route mocks those PRs need
(`/api/tool-permission-rules`, `/api/conversations/{id}/interactions`).

**Parallelism at a glance**

| Wave | PRs                  | Notes                                                                          |
| ---- | -------------------- | ------------------------------------------------------------------------------ |
| 1    | 1, 3, 4, 5a, 5b      | All base: main. PR 1 lands with ADR-0018 and ADR-0019.                         |
| 2    | 2, 6a, 6b, 10        | PR 2 and 6b after chat-stop PR 1. Merge 6a before tool-result-renderers S3.    |
| 3    | 7a, 7b, 7c, 13a, 13b | 7b and 7c after follow-up-suggestions PR 3. 13a/13b are client-only (package). |
| 4    | 8a, 8b, 12, 14a, 14b | 8b after follow-up-suggestions PR 2.                                           |
| 5    | 9, 11, 15            | 15 is client-only and can start as soon as 8a, 8b and 10 have merged.          |
| 6    | 16, 18a, 18b         | 16 needs chat-element-library PRs 5, 6, 9, 10 and chat-a11y C2.                |
| 7    | 17, 19               |                                                                                |
| 8    | 20a, then 20b        | 20a needs PRs 9, 11, 12, 16, 17. 20b archives last.                            |

Sibling edges in one place: chat-stop-cancels-turn PR 1 (2, 6b) and PR 4/5
(15, 16); message-branching PR 5 (7b, 7c, 12, 20a) and PR 12 (20a's edit
test); follow-up-suggestions PR 2 (8b) and PR 3 (7b, 7c);
tool-result-renderers S3 (after 6a), S4 and C1 (17); chat-element-library
PRs 1, 3, 4 (13a), 5, 6, 9, 10 (16); chat-a11y-and-code-blocks C2 (16).
Reverse edges that siblings take on this change: generative-ui-a2ui PR 5
takes 6b, its in-turn route takes 7a and 8a, its client PRs take 15 and 16;
message-branching PR 7 and PR 12 take 6b, and its edit/regenerate cancel
path relies on 7c.

---

## PR 1 — docs(adr): ADR-0017 turn interactions and tool approvals

Layer: docs
Depends on: —
Parallel with: everything; land together with ADR-0018 (message-branching) and
ADR-0019 (generative-ui-a2ui)

- [ ] 1.1 Write `docs/adr/adr-0017-turn-interactions.md`. Cover the generic
      interaction channel and the kind registry (D1); the topology, with the
      shared table, store-polled waiters, the run event relay and the single
      event-log writer (D2); the async typed callback, the explicit
      interactivity flag and removal of `AutoDenyConfirmation` (D3); where
      saved rules sit in the pipeline, their scope and admin management
      (D4, D4a); and the dormant-until-client-ready rollout with no feature
      flag (D9). Cross-link `adr-0023-tool-permission-decision-model.md` by
      filename and
      `adr-0013-per-space-runtimes.md`. Note that `generative-ui-a2ui`
      (ADR-0019) adds the `surface_action` kind.
- [ ] 1.2 Add "Interaction" to `docs/glossary.md`.
- [ ] 1.3 Fix the main specs `openspec/specs/notification-settings/spec.md`
      and `openspec/specs/pwa-push-notifications/spec.md`. Both start with a
      delta header (`## ADDED Requirements`) instead of `## Requirements`, so
      archive would refuse this change's deltas to them. (Pre-existing; fixed
      here, and `openspec validate --strict` of this change reports them until
      this PR lands.)

## PR 2 — fix(runtime): heartbeat bus claims while a turn runs

Layer: server
Depends on: chat-stop-cancels-turn PR 1 (same `worker.rs` region)
Parallel with: 1, 3–5, 6a, 10

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

## PR 3 — feat(storage): interactions store

Layer: server
Depends on: —
Parallel with: 1, 2, 4, 5a, 5b

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
- [ ] 3.2 Migration `047_interactions.sql` (space.db). Columns as in
      design.md D2, plus an index on (`conversation_id`, `created_at`) and one
      on (`run_id`, `status`). Register it in both lists in
      `crates/storage/src/lib.rs`.
- [ ] 3.3 Implement both stores and re-export the in-memory one through
      `assistant_test_support::prelude`. GREEN.

## PR 4 — feat(storage): tool permission rules store

Layer: server
Depends on: —
Parallel with: 1–3, 5a, 5b

- [ ] 4.1 RED: contract tests for `ToolPermissionRuleStore` (both impls).
      Cover: `upsert` is idempotent on (`user_id`, `effect`, `rule`);
      `list_for_user` returns only that user's rows; `list_for_space` returns
      every user's rows, oldest first; `get(id)` returns the row;
      `delete(id, Some(user_id))` returns `false` for another user's row;
      `delete(id, None)` deletes any row (the admin path).
- [ ] 4.2 Migration `048_tool_permission_rules.sql` (space.db), registered in
      both lists. Implement both stores. GREEN.

## PR 5a — feat(core): suggest permission rules for approvals

Layer: server
Depends on: —
Parallel with: 1–4, 5b, 6a

- [ ] 5a.1 RED: `crates/core/src/permission/suggest.rs` unit tests for every
      scenario of "Suggested rules are derived deterministically": `git
status --short` → `bash(git status*)`; `npm -s run build` →
      `bash(npm -s run*)`; a pipe returns `None`; `rm -rf build` returns
      `None`; `python3 -c …`, `node -e …`, `sh -c …`, `bash -lc …`, `env X=1
cmd` and `xargs rm` return `None`; the web-fetch host; the bare-tool
      fallback for file tools and MCP tools; and `*` escaping. Every name in
      the shell, interpreter, wrapper and destructive lists of design D4 has
      a case, so "never always-allow shells, interpreters or destructive
      commands" cannot regress.
- [ ] 5a.2 Implement `suggest_rule` (pure, no I/O). GREEN.

## PR 5b — feat(core): saved-rule stages in the permission pipeline

Layer: server
Depends on: —
Parallel with: 1–4, 5a, 6a

- [ ] 5b.1 RED: `static_rules.rs` tests that `evaluate_deny`,
      `evaluate_ask` and `evaluate_allow` each return what the matching stage
      of today's `evaluate` returns, and that `evaluate` still composes them
      in the same order.
- [ ] 5b.2 Split `StaticRules::evaluate` (`static_rules.rs:56-96`) into those
      three stage functions; `evaluate` calls them in order. GREEN with no
      behaviour change.
- [ ] 5b.3 RED: `policy.rs` tests with an in-memory `UserRuleSource`. Cover:
      user deny beats static allow; configured deny beats user allow; a
      configured ask beats user allow; user allow satisfies
      `requires_confirmation`; user allow skips the decision model; the
      decider is `user-rules`; a `Resolution` from a configured ask rule
      reports `offers_always = false`; and the user stages apply the same way
      to a non-interactive turn (a saved allow lets a scheduler-style context
      through; a saved deny blocks it; no match still ends in `Ask`).
- [ ] 5b.4 Add the `UserRuleSource` trait, `UserRules`, and
      `PermissionPolicy::with_user_rules`, interleaving the user stages as in
      design.md D4. Expose `Resolution::offers_always`. GREEN.
- [ ] 5b.5 RED, then GREEN: add `interaction_timeout_secs` (default 600, range
      `30..=3600`) to `PermissionsConfig`; an out-of-range value fails the
      load with an error naming the key.

## PR 6a — refactor(runtime)!: async typed ConfirmationCallback, no AutoDenyConfirmation

Layer: server
Depends on: —
Parallel with: 1–5b, 2, 10. Merge before tool-result-renderers S3, which edits
the same `dispatch.rs` denied path.

- [ ] 6a.1 RED: update `crates/runtime/src/orchestrator/tests/permission.rs`
      to use an async scripted callback that returns each `ConfirmationAnswer`.
      Assert that `AllowOnce`/`AllowAlways` run the tool and that the other
      answers produce the matching observation (`User denied …`, `…expired…`).
- [ ] 6a.2 Change `ConfirmationCallback` to the async `ConfirmationRequest`
      → `ConfirmationAnswer` shape (design.md D3). Thread `event_sink` and
      `tool_call_id` into `authorize_tool_call` (`dispatch.rs:370`). Make
      `SpaceRuntime` take `Option<Arc<dyn ConfirmationCallback>>`. GREEN.
- [ ] 6a.3 Delete `AutoDenyConfirmation` (`bootstrap.rs:20-38`) and its tests.
      Callers in `web-ui` (`lib.rs:329`, `space_tasks.rs:157`,
      `test_helpers.rs:199`), `interface-cli` (`main.rs:144-151`),
      `space_runtime.rs` and `space_supervisor.rs` tests pass `None` or the
      CLI callback. No shim.
- [ ] 6a.4 Port `CliConfirmation` to async using `spawn_blocking`, offering
      once answers only. Test it with an injected reader.

## PR 6b — feat(runtime): explicit interactive submit option and run id on turns

Layer: server
Depends on: PR 6a, chat-stop-cancels-turn PR 1 (same `worker.rs` /
`bus_messages.rs` region)
Parallel with: 7a, 10. Taken as a dependency by message-branching PR 4 and
PR 7 (its `SubmitTurn` struct absorbs `SubmitOptions`) and PR 12, and by
generative-ui-a2ui PR 5.

- [ ] 6b.1 RED: orchestrator tests. A `Web` turn submitted with default
      `SubmitOptions` has `ctx.interactive == false`; one submitted with
      `interactive: true` has `true` and `ctx.run_id == Some(request_id)`.
- [ ] 6b.2 RED: `interface-cli` tests that a REPL turn gets
      `ctx.interactive == true`, and that the one-shot CLI command
      (`main.rs:196-201`) and a `--no-repl` process do not.
- [ ] 6b.3 Add `SubmitOptions { interactive }` to `submit_turn` /
      `submit_turn_with_request_id`, `interactive` to `TurnRequest`
      (`bus_messages.rs:60`, `#[serde(default)]`), and `run_id` to
      `ExecutionContext`. Derive `interactive` from the request flag, not
      from `Interface` (`mod.rs:1233`). The REPL (`main.rs:504-511`) passes
      `interactive: true`. GREEN.
- [ ] 6b.4 Update the 33 files with `ExecutionContext { … }` literals. If the
      diff passes ~500 lines, land the `run_id` field with an
      `ExecutionContext::default()`-based helper first as PR 6c.
- [ ] 6b.5 `send_message` does not set `interactive` yet. The shared runner
      sets it in PR 20a, so web stays deny-on-`Ask` until then.

## PR 7a — feat(runtime): interaction broker and kind registry

Layer: server
Depends on: PR 3, PR 5b (timeout config), PR 6a
Parallel with: 6b, 7b, 10, 13a. Taken as a dependency by
generative-ui-a2ui (`InteractionKindHandler` registry).

- [ ] 7a.1 RED: `crates/runtime/src/interaction/` tests using
      `InMemoryInteractionStore` and a paused Tokio clock. Cover: `request`
      persists `pending` and emits `InteractionRequested` on the sink; a
      same-process `respond` resumes the waiter at once; a resolution written
      straight to the store (another process) resumes it within 2 s; the
      waiter refreshes `waiter_seen_at` every 15 s; expiry emits `{expired}`;
      the default expiry is 600 s and `expires_at` is clamped to the turn
      deadline; a non-owner `respond` returns `NotFound`; a duplicate same
      answer returns `Ok(same)`; a different answer returns `Conflict`; a
      stale waiter makes `respond` return `Gone` and expire the row.
- [ ] 7a.2 RED: `InteractionKinds` tests. Validation, observation, push
      copy, progress label and `renders_card` for `tool_approval` and
      `question`; an unregistered kind fails validation.
- [ ] 7a.3 Implement `InteractionBroker` (D2) with a drop guard on waiters,
      the `InteractionKindHandler` trait and registry (D1), and
      `arguments` truncation at 16 KiB (D10). Add the broker to
      `SpaceRuntime` behind `SpaceRuntimes::with_interactions()`. GREEN.

## PR 7b — feat(runtime): relay run events across processes over the bus

Layer: server
Depends on: PR 6b, message-branching PR 5 (shared turn runner),
follow-up-suggestions PR 3 (runner merge order)
Parallel with: 7a, 10, 13a

- [ ] 7b.1 RED: worker test. A turn submitted with `relay_events: true` and no
      local sink publishes its events on `turn.event` with
      `batch_id = run_id`, `Token`/`Thinking` coalesced to at most one message
      per 200 ms; with a local sink, nothing is published.
- [ ] 7b.2 Add `relay_events` to `SubmitOptions`. Implement the relay sink in
      `worker.rs` (fallback when `token_sinks` has no entry, `worker.rs:188`).
      Serialize `OrchestratorEvent` with a stable serde form. GREEN.
- [ ] 7b.3 RED: runner test with web-ui and worker in **separate runtimes**
      that share a `space.db` and a bus. The SSE stream receives tokens, a
      tool result and `done`, and ends when the turn ends.
- [ ] 7b.4 The runner sets `relay_events: true`, claims `turn.event` with a
      `batch_id` filter, merges those events into its channel, and
      `select!`s over the channel and the turn result so it ends with the
      turn. GREEN.
- [ ] 7b.5 Update `docs/web-ui.md` (operator note): streaming now works with
      the orchestrator in a separate process.

## PR 7c — feat(runtime): interaction stream events and end-of-run cleanup

Layer: server
Depends on: PR 7a, PR 7b, message-branching PR 5, follow-up-suggestions PR 3
Parallel with: 8a, 10, 13a, 13b

- [ ] 7c.1 RED: runner test. Drop the SSE receiver after `run_started`; the
      scripted turn emits 100 events and finishes. The persisted log holds all
      100 events and `done`, and the turn was never blocked on a full sink.
      Replace `if sse_tx.send(..).is_err() { break; }` (`messages.rs:296-298,
359-361` before extraction) with a `client_gone` flag that only stops
      forwarding, and move end-of-run work (thinking flush, terminal frame,
      `finish_run`) into one exit path that every branch reaches. GREEN.
      (generative-ui-a2ui also wants this flag in its persist-after-disconnect
      task; whichever PR lands first owns it and the other drops the task.)
- [ ] 7c.2 RED: `projection/mod.rs` tests that `SseProjector` maps both new
      `OrchestratorEvent` variants to `interaction_requested` /
      `interaction_resolved` with the exact JSON in design.md D1, and that
      `CliProjector` renders a one-line prompt notice. GREEN. Handle the new
      variants explicitly in every other exhaustive `match` over
      `OrchestratorEvent` (A2A projection, channel adapters).
- [ ] 7c.3 RED: runner tests. (a) Cancel with one pending interaction: the
      log ends `interaction_resolved{cancelled}` then
      `agent_error{reason: cancelled}`. (b) The respond compare-and-set wins,
      then the turn is cancelled before the waiter emits: the log carries
      `interaction_resolved{resolved}` before the terminal frame. (c) Persisting
      an `interaction_resolved` frame sets `resolution_emitted`. (d) A turn
      cancelled through the same path that message-branching's edit and
      regenerate use ends its pending interaction `cancelled`, so a branch
      never leaves an approval waiting.
- [ ] 7c.4 GREEN: the runner marks rows emitted as it persists their frames,
      and on every exit path calls `broker.abandon_run(run_id)` and writes one
      `interaction_resolved` per returned row before the terminal frame.
- [ ] 7c.5 RED, then GREEN: `scheduler/reap.rs` tests. Dead pending rows
      become `expired` even when the run has an active bus message (and with
      a bus whose `has_active_message` always returns true). When the sweep
      appends a synthetic terminal, it first appends
      `interaction_resolved{expired}` for each unemitted row.

## PR 8a — feat(web-ui): respond to turn interactions

Layer: server
Depends on: PR 7a
Parallel with: 7c, 8b, 10, 12, 13a, 13b. Taken as a dependency by
generative-ui-a2ui PR 8 (extensible `RespondToInteractionRequest`).

- [ ] 8a.1 RED: handler tests in `crates/web-ui/src/api/interactions.rs`
      covering every scenario of "Owners answer through one respond endpoint"
      and "Responses are idempotent and first-writer-wins": 200, 400 (wrong
      kind, unknown option, two selections for `multiple = false`, 4001-char
      text), 403 (viewer and missing scope), 404 (unknown conversation,
      another user, wrong run, unknown id), 409, 410 (expired, cancelled, no
      live waiter) and 413. Assert the `{"error": …}` body shape for every
      error.
- [ ] 8a.2 Make `owned_conversation` (`conversations.rs:50-62`) return
      enveloped `404`/`500` bodies for all callers, with a test for one
      existing caller.
- [ ] 8a.3 Implement `POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`
      (`respond_to_interaction`), validating through `InteractionKinds`, with
      a 64 KiB body limit on the route. Add `utoipa` docs for
      `RespondToInteractionRequest`, `InteractionDetail` and the SSE event
      schemas (`SseInteractionRequestedEvent`,
      `SseInteractionResolvedEvent`). GREEN.
- [ ] 8a.4 RED, then GREEN: a tampered body carrying a `rule` field is
      rejected with `400` (`deny_unknown_fields`).
- [ ] 8a.5 `make dump-openapi && make generate-flutter-client`; commit both.

## PR 8b — feat(web-ui): list interactions and report awaiting_input

Layer: server
Depends on: PR 7c, PR 8a, follow-up-suggestions PR 2 (presence-based turn
status)
Parallel with: 12, 14a, 14b

- [ ] 8b.1 RED: handler tests for `GET
/api/conversations/{conversation_id}/interactions` (bare array of
      `InteractionDetail`, `?status=pending`, only the caller's
      conversation) and for `ConversationDetail.interactions`
      (`conversations.rs:77`), in creation order, including interactions of
      runs on inactive branches (the client decides what to show).
- [ ] 8b.2 Implement both (`list_conversation_interactions`). GREEN.
- [ ] 8b.3 RED: `turns.rs` tests for the rule in the turn-status-api delta:
      `awaiting_input` while any `interaction_requested` has no matching
      `interaction_resolved`, even when a later `status` row is the last;
      `running` once resolved; `errored` and `completed` still win. GREEN:
      add `TurnState::AwaitingInput` and extend `last_event_kind`, in the
      precedence order fixed by follow-up-suggestions D7.
- [ ] 8b.4 `make dump-openapi && make generate-flutter-client`.

## PR 9 — feat(runtime): approve tool calls through interactions, with saved rules

Layer: server
Depends on: PR 4, PR 5a, PR 5b, PR 7c, PR 8a
Parallel with: 8b, 10, 11, 12

- [ ] 9.1 RED: integration test with the HTTP routes and the turn worker in
      **separate runtimes** sharing `space.db` and the bus
      (`ScriptedLlmProvider`, `with_interactions()` on both). A web turn
      submitted with `interactive: true` calls `bash` with no allow rule; the
      SSE stream carries `interaction_requested{tool_approval}`; posting
      `allow_once` makes the tool run and `done` arrive. Variants:
      `reject_once` (the LLM sees the denial), expiry, and `allow_always`.
      For `allow_always`, the rule is saved, the resolution carries
      `rule_id`, and a second turn runs without a prompt.
- [ ] 9.2 RED: the "Answer after closing the tab" scenario. Disconnect the
      SSE client while the approval is pending, answer through the respond
      route, replay with `since=0`, and expect the resolution, the tool result
      and `done`.
- [ ] 9.3 RED: scope. After `allow_always` for `bash(git status*)`, a
      scheduler turn and a messenger turn carrying the same user's identity
      in the same space run `git status` with no prompt and no interaction
      (`decider = user-rules`). A `reject_always` rule denies the same turns.
      Another user's turn, and the same user in another space, still prompt
      (interactive) or deny (non-interactive).
- [ ] 9.4 Implement `InteractiveConfirmation` (broker plus rules store): the
      options come from `Resolution::offers_always` and `suggest_rule`, and
      the "always" answers upsert the rule from the stored interaction. Add
      a `UserRuleSource` adapter over `ToolPermissionRuleStore`, attach it in
      `SpaceRuntime` for every interface, and route `Interface::Cli` turns to
      the CLI callback in a REPL process. Emit the `tool approval answered`
      audit event. GREEN.
- [ ] 9.5 Update `docs/tool-permissions.md`: approvals, the four options,
      when "always" is withheld (shells, interpreters, wrappers, destructive
      commands, configured ask rules), saved rules and that they apply to all
      of the user's turns in the space, evaluation order, who can revoke
      them, `interaction_timeout_secs`, and that messenger interfaces still
      deny on `Ask`.

## PR 10 — feat(web-ui): list and revoke saved tool permission rules

Layer: server
Depends on: PR 4
Parallel with: every other server PR. Taken as a dependency by 15 and 18a.

- [ ] 10.1 RED: handler tests for `GET /api/tool-permission-rules` (bare
      array, only the caller's rows, space from `X-Space-Id`) and `DELETE
/api/tool-permission-rules/{id}` (`204`, `404` for another user's id,
      `403` for a viewer), all errors enveloped.
- [ ] 10.2 RED: admin tests. `GET /api/tool-permission-rules?scope=space`
      returns every member's rows, each with its `user_id`, for a space
      admin and an org admin; it returns `403` for a member and a viewer;
      `scope=own` (the default) is unchanged for admins. `DELETE` of another
      member's rule returns `204` for a space admin and still `404` for a
      member; a rule in another space returns `404` for an admin. An admin
      revoke logs `tool permission rule revoked` with the acting admin, the
      rule owner, the rule and its id.
- [ ] 10.3 Implement both routes with `utoipa` docs
      (`ToolPermissionRuleSummary` with `user_id`, `scope` query enum).
      Admin checks use `Access::Admin` (`space.rs`). GREEN.
- [ ] 10.4 `make dump-openapi && make generate-flutter-client`.

## PR 11 — feat(runtime): ask-user tool

Layer: server
Depends on: PR 7a, PR 8a, PR 9 (`with_interactions()` wiring)
Parallel with: 10, 12, 15

- [ ] 11.1 RED: `AskUserTool` unit tests for input validation (0 or 5
      questions, empty prompt, 501-char prompt, 9 options, 101-char label) and
      free text being forced when there are no options. Each invalid input
      returns `ToolOutput::error` and creates no interaction.
- [ ] 11.2 RED: the observation for the answered case is
      `{"answers":[{"question", "selected", "text"}]}` in request order;
      decline and expiry return the fixed success texts.
- [ ] 11.3 Implement it and add it to `turn_overlay` (`orchestrator/mod.rs:359`)
      only when `ctx.interactive` and a broker exist. Add `ask-user` to
      `builtin_allow` (`static_rules.rs:285`). GREEN.
- [ ] 11.4 RED, then GREEN: the tool-spec lists of a scheduler turn and of a
      voice turn do not contain `ask-user`.

## PR 12 — feat(web-ui): push the turn owner when an interaction waits

Layer: both (server plus `app/web/sw.js`)
Depends on: PR 7c, message-branching PR 5
Parallel with: 8a, 8b, 9–11

- [ ] 12.1 RED: storage test showing a subscription upserted with a user id
      is returned by `list_for_user` and not for another user. Then add
      migration `059_push_subscription_user.sql` (nullable `user_id`),
      registered in both lists. GREEN.
- [ ] 12.2 RED, then GREEN: `POST /api/push/subscribe` stores the caller's
      user id.
- [ ] 12.3 RED: `wiremock` test of `PushDispatcher::send_to_user`. An
      interaction pending for 30 s (paused clock) sends one push, with the
      kind's push copy, `tag` and `conversationId`, to the owner's endpoint
      only; one resolved at 10 s sends none. GREEN: schedule the delayed push
      from the `interaction_requested` frame in the shared runner, and cancel
      it on resolution.
- [ ] 12.4 `app/web/sw.js` passes `payload.tag` to `showNotification`. Check
      it manually in Chrome (two pushes with one tag show one notification)
      and record the result in the PR body.

## PR 13a — feat(app): interaction models and ApprovalCard

Layer: client
Depends on: chat-element-library PR 1 (package), PR 3 (widgetbook and
`kAssistantUiElements`), PR 4 (models in the package)
Parallel with: all server PRs, 13b, 14a, 14b

- [ ] 13a.1 RED: `app/packages/assistant_ui/test/approval_card_test.dart`.
      Pending shows the tool, the summary, the reason, a countdown, and only
      the offered buttons. "Always" buttons show `suggested_rule` and the
      scope note ("applies to all your turns in this space, including
      scheduled tasks and messenger chats"). Truncated arguments say so.
      Tapping fires `onDecision` with the right enum. A submitting card has
      its buttons disabled. Each resolved state renders a receipt with no
      buttons. Semantics labels read "Allow bash once" and so on. Enter maps
      to allow once on every platform, and Escape does nothing.
- [ ] 13a.2 Add the `InteractionRecord` (with `rendersCard`),
      `InteractionKind` (including `unknown`) and `ApprovalDecision` models in
      `lib/src/models/`, plus `TimelineEntryType.interaction` (the no-id
      fallback only). Implement `ApprovalCard`. GREEN.
- [ ] 13a.3 Register `ApprovalCard` in `kAssistantUiElements` with one
      widgetbook use-case and add its `ELEMENTS.md` row.

## PR 13b — test(app): ApprovalCard goldens, stories and UnknownInteractionCard

Layer: client
Depends on: PR 13a
Parallel with: 14a, 14b

- [ ] 13b.1 RED: `unknown_interaction_card_test.dart`: inert notice, no
      callbacks.
- [ ] 13b.2 Implement `UnknownInteractionCard`. GREEN.
- [ ] 13b.3 Golden tests for every `ApprovalCard` state × light/dark, and
      widgetbook use-cases for each state; register the unknown card.

## PR 14a — feat(app): QuestionCard element

Layer: client
Depends on: PR 13a (models)
Parallel with: all server PRs, 13b, 14b

- [ ] 14a.1 RED: `question_card_test.dart`. Cover single (radio), multiple
      (checkbox) and free-text questions, and multi-question; submit stays
      disabled until every question is answered; `onAnswer` gets the
      structured answers; `onDecline` fires; and each resolved state is a
      receipt.
- [ ] 14a.2 Implement `QuestionCard`. GREEN.
- [ ] 14a.3 Goldens, widgetbook use-cases, registry entry and `ELEMENTS.md`
      row.

## PR 14b — feat(app): PermissionRuleTile element

Layer: client
Depends on: PR 13a
Parallel with: all server PRs, 13b, 14a

- [ ] 14b.1 RED, then GREEN: `permission_rule_tile_test.dart` checks the
      effect badge, rule text, date, that the revoke callback fires, and that
      an optional `ownerLabel` (used in the admin view) renders and does not
      change the revoke semantics.
- [ ] 14b.2 Goldens, widgetbook use-case, registry entry and `ELEMENTS.md` row.

## PR 15 — feat(app): interaction stream events and API calls

Layer: client
Depends on: PR 8a, PR 8b, PR 10 (generated client), chat-stop-cancels-turn
PR 4 (stream-event lane order)
Parallel with: 13a–14b. Taken as a dependency by generative-ui-a2ui PR 12.

- [ ] 15.1 RED: `parseSseByteStream` tests showing that
      `interaction_requested` (approval, question, an unknown kind, and
      `renders_card: false`) and `interaction_resolved` parse into new
      `InteractionRequestedEvent` / `InteractionResolvedEvent` subclasses
      with `sequenceId`; that malformed JSON is dropped; and that an unknown
      event name is still ignored.
- [ ] 15.2 Add the subclasses to the sealed `StreamEvent`
      (`app/lib/api/models/stream_event.dart`) and the branches in
      `api_client.dart`. Update every exhaustive `switch` over `StreamEvent`
      in the app. GREEN.
- [ ] 15.3 RED, then GREEN: `respondToInteraction`,
      `listConversationInteractions`, `listToolPermissionRules(scope)` and
      `deleteToolPermissionRule` on `ApiClient`, tested against a mocked
      Dio.
- [ ] 15.4 RED, then GREEN: client test that the stalled-stream probe treats
      `TurnState.awaitingInput` like `running` (`chat_provider.dart:932`).

## PR 16 — feat(app): approve tool calls in chat

Layer: client
Depends on: PR 13b, PR 15, chat-element-library PR 5 (`EntryState.awaitingInput`),
PR 6 (`ToolCallBody.footer`), PR 9 (pure `TurnProgressCard`), PR 10
(`ChatThreadLayout.aboveComposer`), chat-a11y-and-code-blocks C2
(`ChatAnnouncer`), chat-stop-cancels-turn PR 5 (Stop cancels the server turn)
Parallel with: 17 (partly), 18a. Taken as a dependency by
generative-ui-a2ui PR 12. **Must merge before PR 20a.**

- [ ] 16.1 RED: `ChatNotifier` unit tests. `InteractionRequestedEvent` with a
      `tool_call_id` attaches the interaction to that tool call entry and
      sets it `awaitingInput`; without an id it adds an `interaction` entry in
      order. The turn status snapshot carries the kind's progress label.
      `respond()` marks it submitting, calls the API, and on `200` waits for
      `InteractionResolvedEvent` to flip it to resolved. `409`/`410` reload
      the record from the pending list. A failed request restores the
      pending state and shows an error.
- [ ] 16.2 Add the client kind registry (`interaction_kinds.dart`):
      `tool_approval` → `ApprovalCard` footer, label "Waiting for your
      approval"; unregistered kinds with `rendersCard` → `UnknownInteractionCard`;
      `rendersCard: false` → no card, label "Waiting for your input".
- [ ] 16.3 Implement the reducers in `chat_timeline_reducers.dart` and
      `respond()` in a new `interaction_actions.dart`; do not grow
      `chat_provider.dart`. GREEN.
- [ ] 16.4 RED, then GREEN: `TurnProgressCard` receives `awaitingLabel`, so it
      reads "Waiting for your approval" and hides the stall label while an
      interaction is pending. `ChatAnnouncer` announces "Approval needed" and
      suppresses its stall announcement.
- [ ] 16.5 RED, then GREEN: a widget test that the chat timeline renders
      `ApprovalCard` in the tool call's footer and that tapping "Allow once"
      reaches `respond()`; and one that Stop while waiting calls the server
      cancel and the card ends "Cancelled".
- [ ] 16.6 Update the Playwright chat baselines (approval card, awaiting
      label) and record the diff.

## PR 17 — feat(app): ask-user questions, receipts after reload, pending discovery

Layer: client
Depends on: PR 11, PR 14a, PR 16, tool-result-renderers S4
(`ToolCallSummary.tool_call_id`) and C1 (`ToolCallRecord.toolCallId`)
Parallel with: 18a, 18b, 19

- [ ] 17.1 RED: a widget test that a `question` interaction renders
      `QuestionCard` in the `ask-user` tool call's footer and that submit
      calls `respond()` with the answers; the label reads "Waiting for your
      answer".
- [ ] 17.2 RED: a mapping test (`chat_history_mapping.dart`) showing that
      `ConversationDetail.interactions` become receipts on the tool call with
      the matching `tool_call_id`; without an id, they use the same per-turn
      ordinal join as tool-result-renderers, then the end of the turn. An
      interaction whose tool call is not on the active branch's path (the
      conversation was edited or regenerated after it) renders nowhere and
      is not attached to the end of an unrelated turn.
- [ ] 17.3 RED: a provider test where opening a conversation with a pending
      interaction calls the pending list and attaches to the run's replay
      stream. GREEN for 17.1–17.3.
- [ ] 17.4 Update affected Playwright baselines and record the diff.

## PR 18a — feat(app): tool approvals settings section

Layer: client
Depends on: PR 10, PR 14b, PR 15
Parallel with: 16, 17, 18b, 19

- [ ] 18a.1 RED: unit tests for `toolPermissionRulesProvider`
      (AsyncNotifier). Cover loading, success, error, empty, and `revoke`
      optimistic removal with rollback on error.
- [ ] 18a.2 RED: a widget test for `ToolApprovalsSection`: Allowed/Denied
      groups, the empty state copy, a note that saved approvals apply to all
      of the user's turns in the space, and that revoke asks for confirmation
      and then calls the provider.
- [ ] 18a.3 Implement `app/lib/features/settings/tool_approvals_section.dart`
      (its own file) and add it to `settings_screen.dart`. GREEN.
- [ ] 18a.4 Mock `/api/tool-permission-rules` in the e2e API mocks, update the
      `settings.png` baseline
      (`crates/web-ui/e2e/tests/visual-regression.spec.ts:547-556`) and
      record the diff.

## PR 18b — feat(app): admins review and revoke members' saved approvals

Layer: client
Depends on: PR 18a
Parallel with: 16, 17, 19

- [ ] 18b.1 RED: unit tests for `spaceToolPermissionRulesProvider`
      (`scope=space`): loading, success, error, and `revoke` with rollback.
      It is never requested for a caller who is not a space or org admin.
- [ ] 18b.2 RED: a widget test. For an admin, `ToolApprovalsSection` shows a
      "Members' approvals" group below the caller's own rules, with each rule
      labelled by its owner (member name where the member list already known
      to the app provides one, else `user_id`); revoke confirms and calls the
      provider. For a member or viewer the group is absent and no request is
      made.
- [ ] 18b.3 Implement it in the same file or a sibling
      `members_approvals_group.dart`, reusing the existing role source of
      `features/admin`. A `403` hides the group. GREEN.
- [ ] 18b.4 Extend the e2e mocks with an admin variant, update the
      `settings.png` baseline and record the diff.

## PR 19 — feat(app): notify on approval requests when backgrounded

Layer: client
Depends on: PR 16
Parallel with: 17, 18a, 18b

- [ ] 19.1 RED: `NotificationPreferences` test showing that
      `notifyApprovalRequests` defaults to true and persists.
- [ ] 19.2 RED: an `AgentEventListener` test where an interaction request
      while not `resumed` shows "Approval needed" (or "Your assistant has a
      question") with `conversationId`; nothing is shown when resumed, when
      the category is off, or on web with an active push subscription.
      GREEN.
- [ ] 19.3 Add the "Approval requests" toggle to the notifications settings
      section, with a widget test. Update the `settings.png` baseline and
      record the diff.

---

## PR 20a — feat(cli,web-ui): enable tool approvals and questions in web chat

Layer: server
Depends on: PR 9, PR 11, PR 12, PR 16, PR 17. **This PR flips the switch, and
merges last among the feature PRs: no feature flag, no earlier enablement.**
Land it after 18a and 19 so the same release ships revocation and
notifications.
Parallel with: 18b

- [ ] 20a.1 RED: integration test with the web-ui bootstrap and the
      orchestrator bootstrap in separate runtimes: a web chat turn that hits
      `Ask` produces `interaction_requested` instead of an immediate denial,
      and the shared runner submits send, edit and regenerate turns with
      `interactive: true` and voice turns without. If message-branching PR 12
      already submits edit and regenerate with `interactive = true`, this PR
      adds only the assertion for them.
- [ ] 20a.2 RED: with message-branching PR 12 merged, editing a message while
      an approval is pending cancels the waiting turn: the card ends
      "Cancelled", no tool ran, and the new branch's turn starts. If PR 12
      has not merged, skip this test and note it in the PR body for 20b.
- [ ] 20a.3 Call `SpaceRuntimes::with_interactions()` in the `interface-cli`
      orchestrator bootstrap (`main.rs:144-151`, replacing the auto-deny
      branch) and in `crates/web-ui/src/lib.rs`. The shared runner sets
      `interactive: true` for `send_message`, `edit_message` and
      `regenerate_message`. GREEN.

## PR 20b — docs(app): approvals and questions guide; ship

Layer: both
Depends on: PR 18a, 18b, 19, 20a

- [ ] 20b.1 Write `docs/chat/approvals-and-questions.md`: what the cards
      mean, the four options and when "always" is withheld (shells,
      interpreters, wrappers, destructive commands, configured ask rules),
      that saved rules apply to all of your turns in the space including
      scheduled tasks and messenger chats, revoking (yours; space admins can
      also revoke members'), the 10 minute timeout and 30 s push delay,
      branching while an approval waits, and that messenger interfaces do not
      prompt.
- [ ] 20b.2 `make lint && make format && make test`;
      `make lint-flutter && make test-flutter`.
- [ ] 20b.3 Manual smoke test with the packaged two-service topology: approve
      once, always, reject and let one expire in web chat; stop a turn while
      it waits; reload during a wait; close the tab, answer from a second
      device after the push; answer a multi-question `ask-user`; revoke a rule
      in settings, and as an admin revoke another member's rule; run a
      scheduled task that matches a saved allow; edit a message while an
      approval waits; restart the orchestrator during a wait and confirm the
      card ends "Expired".
- [ ] 20b.4 `openspec archive tool-approval-prompts`, after
      chat-stop-cancels-turn and follow-up-suggestions have archived (all
      three modify `turn-status-api`), and after generative-ui-a2ui's delta
      to `turn-interactions` is rebased on this change.
