# ADR 0017: A Running Turn Asks Its Owner Through Interactions

**Status**: Accepted
**Date**: 2026-11-25
**Builds on**: [ADR-0023](adr-0023-tool-permission-decision-model.md) (tool permissions), [ADR-0013](adr-0013-per-space-runtimes.md)

## Context

[ADR-0023](adr-0023-tool-permission-decision-model.md) ends in a human: a tool
call that no rule and no decision model settles resolves to `Outcome::Ask`.
`Orchestrator::authorize_tool_call` (`crates/runtime/src/orchestrator/dispatch.rs`)
hands that to a `ConfirmationCallback` only when `ctx.interactive` is set and a
callback exists. Nothing in the web chat can answer it:

- The callback is synchronous and returns a `bool`
  (`crates/runtime/src/orchestrator/mod.rs`). It cannot wait for a browser
  without blocking a Tokio worker, and it cannot express "always".
- Every web wiring uses `AutoDenyConfirmation`, so an `Ask` is denied.
- `ExecutionContext.interactive` is derived from the interface
  (`matches!(interface, Interface::Cli)`), so only CLI turns can prompt at all.
- The turn and the HTTP server are usually different processes (below).

### The turn does not run where HTTP is served

In the packaged install the turn runs in a different process from the one that
serves the browser:

- `assistant webui serve` spawns no turn workers (`BinaryRole::WebUi` has an
  empty worker plan, asserted in `crates/web-ui/src/lib.rs`). Operators run
  `assistant orchestrator run --no-repl` beside it
  (`docs/web-ui.md`, `packaging/systemd/user/`). That process owns the
  `Web`-filtered worker.
- The orchestrator process is started with `AutoDenyConfirmation` because of
  `--no-repl` (`crates/interface-cli/src/wiring.rs`,
  `uses_auto_deny_confirmation`, used in `crates/interface-cli/src/main.rs`).
- `send_message` registers its event sink in the web-ui process
  (`Orchestrator::register_token_sink`, the `token_sinks` map). The worker in
  the other process looks the sink up in its own map
  (`crates/runtime/src/orchestrator/worker.rs`) and finds nothing. Today no
  `OrchestratorEvent` crosses the process boundary. Only the turn result does,
  over the bus (`TURN_RESULT`).

An in-process broker with `oneshot` waiters would never see an answer, and an
`interaction_requested` frame would never reach the SSE stream. The design
below has to work with the turn and the server in separate processes, and
equally when they share one (tests, the REPL, single-binary dev setups).

## Decision

**A running turn asks its owner through a generic, persisted interaction. The
state lives in `space.db`, events cross processes on the message bus, and the
feature stays dormant until the client can answer.**

### One generic channel (D1)

An _interaction_ is a typed question that a running turn puts to the turn's
owner: an id, a `kind`, a kind-specific request, and at most one response.

- **Kinds.** This change registers `tool_approval` and `question` (the
  `ask-user` tool). `generative-ui-a2ui`
  ([ADR-0019](adr-0019-generative-ui-a2ui.md)) adds `surface_action` through the
  same registry.
- **Wire.** SSE events `interaction_requested` and `interaction_resolved`,
  snake_case, internally tagged by `kind`. A resolution carries an `outcome` of
  `resolved`, `cancelled` or `expired`, the `response` only when resolved, and
  the saved `rule_id` when a rule was saved.
- **Respond.** One route, shaped like the existing turn routes:
  `POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`.
  The body is a closed enum tagged by `kind`: a tool decision, a list of
  answers, or `decline` (any kind; for an approval it means reject once). A
  sibling extends the enum with a variant. A repeated identical answer returns
  `200` with the stored resolution, a different answer after resolution returns
  `409`, an expired or cancelled one returns `410`.
- **Kind registry.** `InteractionKindHandler` in
  `crates/runtime/src/interaction/kinds.rs` validates a response, words the
  observation returned to the LLM, supplies push copy and the progress label,
  and says whether the kind renders a card. The respond route and the push path
  go through the registry, with no per-kind `match`.

### Topology: the shared table, store-polled waiters and the event relay (D2)

The turn runs wherever a turn worker claims it. Both processes open the same
`space.db` files under `~/.assistant/` and share the message bus. The design
assumes nothing else is shared.

- **The `interactions` table is the source of truth** for authorization,
  idempotency, liveness, the pending list and receipts after reload. It sits
  behind an `InteractionStore` trait with a `Sqlite*` and an `InMemory*`
  implementation ([ADR-0009](adr-0009-testability-architecture.md)). Resolving
  is a compare-and-set (`UPDATE … WHERE id = ? AND status = 'pending'`), so two
  racing responders cannot both win.
- **The turn process waits on the store, with a local fast path.**
  `InteractionBroker::request()` inserts the row, emits
  `OrchestratorEvent::InteractionRequested` on the turn's event sink, then waits
  on the first of a local `oneshot` (same process as the respond route), a
  store poll once a second (cross-process), and `expires_at`. It refreshes
  `waiter_seen_at` every 15 s while it waits.
- **The HTTP process answers through the store.** The respond route does the
  compare-and-set and fires a local `oneshot` if it holds one. A `pending` row
  whose `waiter_seen_at` is older than 45 s has no live waiter, so the route
  sets it to `expired` and answers `410`.
- **A run event relay carries events across the boundary.** `send_message` sets
  the `SubmitTurn` option `relay_events: true`. When the worker finds no local sink
  for such a turn it publishes each `OrchestratorEvent` on bus topic
  `turn.event` with `batch_id = run_id` (the run id is the orchestrator
  `request_id` and the bus `batch_id`). `Token` and `Thinking` events are
  coalesced to one message per 200 ms. The shared turn runner claims
  `turn.event` filtered by `batch_id` and feeds the events into the same
  channel as a local sink. It selects over that channel and the turn result, so
  it ends with the turn. This also fixes streaming in the split topology in
  general. The relay ships in this change and not as a separate dependency,
  because without it the interaction events never reach the browser in the
  packaged install.
- **One writer for the event log.** The runner in the HTTP process owns SSE
  sequence numbers. The turn emits request and resolution events through its
  sink, so replay order is right. When the runner persists an
  `interaction_resolved` frame it sets `resolution_emitted = true` on the row.
- **The runner keeps draining after the client disconnects.** It stops
  forwarding to the SSE sender but keeps persisting and broadcasting. Today it
  `break`s out of the loop and stops draining a 64-slot channel, which would
  block the turn after 64 events and lose every event after the interaction for
  reload, push-tap and replay.
- **End of run.** On every exit path the runner calls
  `broker.abandon_run(run_id)`. It marks the run's `pending` rows `cancelled`
  and returns every row of the run with `resolution_emitted = false`, including
  one whose compare-and-set won just before the turn was cancelled. The runner
  writes one `interaction_resolved` per returned row before the terminal
  `done` or `agent_error` frame. Cancel from Stop, edit or regenerate takes this
  path. It stays per-process (`cancel_turn` reaches only same-process workers),
  so in the split topology the interaction ends when the turn actually ends.
- **Restart.** `reap_stale_and_recover` (`crates/runtime/src/scheduler/reap.rs`)
  expires dead rows in every sweep, whether or not the run still has an active
  bus message. The sweep skips runs with an active message, and NATS always
  reports one (`has_active_message`), so this cannot hang off the orphan path.
  When the sweep appends a synthetic terminal for an orphaned run it first
  appends `interaction_resolved{expired}` for each row not yet emitted. Where no
  runner is alive and no terminal is written (NATS), the log may lack the
  resolution. The list and the conversation detail still report `expired`, and
  the client reconciles from them. The guarantee is exactly one resolution event
  for runs whose runner is live, and at most one otherwise.
- **Callback wiring.** The interaction-backed callback is built in the process
  that runs turns: the `interface-cli` orchestrator bootstrap.
  `SpaceRuntimes::with_interactions()` makes each space runtime build its broker
  and callback over its own stores. The web-ui process calls the same builder,
  because the respond route needs the store and the fast path. It never
  consults the callback.

The table is not derived from the event log (`conversation_events`): those rows
expire after 24 hours, receipts must outlive that, and authorization would mean
scanning JSON.

### An async, typed callback and an explicit interactivity flag (D3)

- `ConfirmationCallback::confirm` becomes `async` and takes a
  `ConfirmationRequest` (tool, params, context, `tool_call_id`, reason, the
  `ApprovalOptions` that may be offered, the `suggested_rule`, the event sink).
  It returns a `ConfirmationAnswer`: `AllowOnce`, `AllowAlways`, `RejectOnce`,
  `RejectAlways`, `Expired` or `Cancelled`.
- `ExecutionContext.interactive` stops being derived from `Interface`. It comes
  from `TurnRequest.interactive`, which the submitter sets through
  the `SubmitTurn` fields `interactive` and `relay_events`. The shared turn runner sets it
  for send, edit and regenerate, and the CLI REPL sets it. The one-shot CLI
  command, quick-message, voice, A2A, scheduler, messenger and Subagent Process
  turns leave it `false`. Quick-message and voice use `Interface::Web` like
  streaming chat, which is why the interface cannot tell them apart.
- `ExecutionContext` gains `run_id: Option<Uuid>`, taken from the bus
  `batch_id`.
- `InteractiveConfirmation { broker, rules }` is the web implementation.
  `CliConfirmation` moves stdin reading into `spawn_blocking` and offers only
  the `*_once` answers. In a REPL process a router sends `Interface::Cli` turns
  to it and everything else to the broker.
- **`AutoDenyConfirmation` is deleted.** The space runtime's callback becomes
  `Option<Arc<dyn ConfirmationCallback>>`. A non-interactive turn never consults
  it, and `None` denies, as the auto-deny did.
- **The server computes the rule.** The "always" answers write the rule before
  the tool runs (allow) or before the denial is returned (reject). The rule text
  is the `suggested_rule` stored on the interaction row. The client never sends
  a pattern, so a tampered client cannot widen a rule.
- **Audit.** Each answer logs `assistant::permission` "tool approval answered"
  with user, tool, interaction id, decision and rule id, next to the existing
  policy event.

### Saved rules are one more stage, never stronger than config (D4, D4a)

A space.db table `tool_permission_rules` holds a user's saved allow and deny
rules in the grammar of ADR-0023. Core gets a `UserRuleSource` trait and
`PermissionPolicy::with_user_rules`. `StaticRules::evaluate`, one function
today, splits so the policy can interleave the user stages:

1. static deny (built-in and `org.toml`), workspace confinement
2. **user deny**
3. static ask
4. static allow
5. **user allow**, in allow-mode matching so the shell-metacharacter guard
   applies
6. `requires_confirmation`, then the decision model, then `Ask`

A saved allow can never override a built-in or configured deny, or a configured
`ask` (which means a human every time). When the `Ask` came from a configured
ask rule, "always" is not offered.

- **Suggestions err narrow.** `permission::suggest_rule` is pure and unit-tested.
  `bash` gets no suggestion when the command has a shell metacharacter, starts
  with a shell, interpreter or command wrapper (`sh`, `bash`, `python*`, `env`,
  `xargs`, `eval`, …), or starts with a destructive command (`rm`, `mv`, `dd`,
  …). Otherwise the rule is the first token, every leading flag, and the first
  non-flag token, so `git status --short` becomes `bash(git status*)`.
  `web-fetch` gets the scheme and host. Every other tool, MCP tools included,
  gets the bare tool name. Shells, interpreters and destructive commands get
  allow-once and reject-once only.
- **Scope.** A saved rule applies to every turn that carries the user's identity
  in that space, whatever the interface. Non-interactive turns never prompt but
  do consult the rules, so a saved allow lets a scheduled `git status` run
  unattended and a saved deny blocks it. An unmatched call is still denied on
  `Ask`. "Always" is therefore sticky across interfaces, and the card, the
  settings section and the docs say so.
- **Admin management.** Space admins and org admins list and revoke every
  member's saved rules in the selected space (`?scope=space`), and each revoke
  is logged. They cannot create a rule, edit a pattern, or answer a member's
  prompt. Because rules reach unattended turns, an admin needs a way to remove
  one that is too wide without impersonating the user, and a leaver's rules
  should not outlive their access.

### The `ask-user` tool is a per-turn overlay (D5)

`AskUserTool` is added to the turn's tool overlay only when `ctx.interactive`
is set and a broker exists. A non-interactive turn never sees the tool. Asking
needs no approval. A decline or an expiry is returned to the model as a normal
`ToolOutput::success` observation, since neither is a tool fault.

### Timeouts keep the turn alive (D6)

An interaction expires after 10 minutes, clamped to the turn's own deadline, and
an unanswered one pushes the owner after 30 seconds. The org can move the
timeout within `30..=3600` s through `[permissions] interaction_timeout_secs`;
the push delay is fixed. A bus claim older than 5 minutes is reaped and
redelivered (`reap.rs`, and the NATS `ack_wait` of 300 s), so a turn that waits
10 minutes would run twice. The worker therefore refreshes the claim of the
message it runs every 60 s through `MessageBus::touch`. That also fixes the
problem for long tool runs, and it ships first.

### Dormant until the client can answer, with no feature flag (D9)

Making a web turn wait on `Ask` before the client can render a card would turn
an instant denial into a 10-minute hang, because the current client drops unknown
SSE events. We reorder the stack instead of gating it:

- Every server PR lands dormant. Production bootstraps do not call
  `SpaceRuntimes::with_interactions()`, so the callback is `None` (deny on
  `Ask`, as today) and the overlay has no `ask-user`. Tests call it explicitly.
- One small server PR enables interactions in the orchestrator and web-ui
  bootstraps and sets `interactive` for web send, edit and regenerate. It merges
  only after the client PRs that render and answer approvals and questions.
- There is no config flag and no runtime toggle, so no dead option survives and
  no operator can reach a state the client cannot handle. Rollback is a revert
  of that one PR.

### Smaller limits (D10)

The request's `arguments` are capped at 16 KiB of serialized JSON, with long
strings cut and `arguments_truncated` set (the tool still runs with the full
arguments, which never leave the turn process). `ask-user` bounds its prompt and
option text. A respond body is capped at 64 KiB and a free-text answer at 4000
characters.

## Alternatives considered

- **A WebSocket per turn.** SSE plus POST already carries everything, replay
  comes for free, and `EventSource` clients keep working.
- **One endpoint per kind (`/approve`, `/answer`).** `generative-ui-a2ui` would
  need a third route and its own copy of the authorization checks.
- **A flat payload with no `kind` tag.** The client could neither pick a card
  nor ignore kinds it does not know.
- **Run web turns in the web-ui process.** It reverses the deployment model in
  `docs/web-ui.md`, needs a `Web`-excluding filter on the orchestrator's
  workers, and a multi-replica web-ui would still split turn and answer.
- **A pure in-process broker.** It does not work in the packaged topology.
- **Relay only interaction events.** The runner needs the relay anyway to learn
  about the run, and a card without the surrounding tokens and tool results
  makes no sense.
- **Keep `confirm -> bool` and block a thread.** It stalls a Tokio worker for
  minutes and cannot express "always".
- **Write saved rules into `org.toml`.** That file is org-wide and
  operator-owned, and the rules are per space and per user.
- **Scope saved rules to interactive web turns.** A user would answer the same
  question in every scheduled run, and a saved deny would go unenforced there.
- **Gate the feature behind a flag.** A flag that is on in production is dead
  weight, and one that is off hides the work. Ordering the PRs gets the same
  safety without it.

## Consequences

- **Load on the SQLite bus.** Every relayed event is a bus row. Token
  coalescing bounds it to a few rows per second per run, and the rows are acked
  and purged like any other. Watch it.
- **Answers take up to about a second to reach a turn in another process.** That
  suits a human-paced prompt.
- **Stop is still per-process.** In the split topology, Stop while waiting ends
  the interaction `cancelled` only once the turn really ends. Cross-process
  cancel is out of scope.
- **"Always" is sticky across interfaces.** A rule saved in chat also covers the
  user's scheduled and messenger turns in that space. Admins can revoke it.
- **Rule suggestion is heuristic.** Too narrow costs another prompt. Too broad is
  the real risk, so dangerous commands never get one, and the card shows the rule
  verbatim.
- **Prompt injection can ask for approval.** The card shows the real arguments
  and the pipeline's reason, never model-written text. `ask-user` questions are
  model-written and are labelled as coming from the assistant.
- **Admins see other members' rule text.** It derives from a command prefix, so
  it carries little personal data, and admins can already read the space's
  conversations.
- **Push needs a user.** `push_subscriptions` gains a nullable `user_id` (no
  backfill), so rows without one get no approval pushes.
- **No migration of data.** Two new `space.db` tables, one nullable column and
  one bus topic. The new tables are inert without the code.
