# Design: tool-approval-prompts

## Context

`tool-permission-decision-model` built static rules → decision model → human
fallback. The fallback is `Outcome::Ask`, which the runtime resolves in
`Orchestrator::authorize_tool_call` (`crates/runtime/src/orchestrator/dispatch.rs:370-399`):
only when `ctx.interactive` and a `ConfirmationCallback` is set does a human
decide, and the callback is synchronous. This change builds the missing half:
a way for a running turn to wait for a browser, and for the browser to answer.

The hard constraint is the deployment topology. In the packaged install the
turn does **not** run in the process that serves HTTP:

- `assistant webui serve` spawns no turn workers (`crates/web-ui/src/lib.rs:399-423`,
  `BinaryRole::WebUi` has an empty worker plan). Operators run
  `assistant orchestrator run --no-repl` beside it (`docs/web-ui.md:25-45`,
  `packaging/systemd/user/assistant-orchestrator.service`,
  `assistant-web-ui.service`).
- That orchestrator process picks `AutoDenyConfirmation` for `--no-repl`
  (`crates/interface-cli/src/main.rs:144-151`). It is the process that runs
  web turns.
- `send_message` registers its event sink in the web-ui process's own
  `token_sinks` map (`messages.rs:151`, `orchestrator/mod.rs:221`,
  `worker.rs:38-44`). The worker in the other process looks the sink up in
  _its_ map (`worker.rs:188`) and finds nothing. So today no
  `OrchestratorEvent` crosses the process boundary at all. Only the turn
  result does, over the bus (`TURN_RESULT`, `worker.rs:795-830`).

An in-process broker with oneshot waiters would therefore never see an
answer, and the `interaction_requested` frame would never reach the SSE
stream. D2 deals with this directly.

### Verified state of the code

| Fact                                                                                        | Evidence                                                                                                                      |
| ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Callback is sync, returns `bool`                                                            | `crates/runtime/src/orchestrator/mod.rs:50-54`                                                                                |
| One callback per `SpaceRuntime`, shared by every interface in that runtime                  | `crates/runtime/src/space_runtime.rs:83,98,267`                                                                               |
| Web wires auto-deny                                                                         | `crates/web-ui/src/lib.rs:329`; `crates/web-ui/src/space_tasks.rs:157`; tests `crates/web-ui/src/api/test_helpers.rs:199`     |
| The process that runs web turns wires auto-deny too                                         | `crates/interface-cli/src/main.rs:144-151`                                                                                    |
| CLI prompts on stdin                                                                        | `crates/interface-cli/src/cli_helpers.rs:12-27`                                                                               |
| CLI REPL and one-shot CLI both submit with `Interface::Cli`                                 | `crates/interface-cli/src/main.rs:196-201,504-511`                                                                            |
| Only CLI turns are interactive                                                              | `orchestrator/mod.rs:1233`                                                                                                    |
| Subagents are non-interactive                                                               | `orchestrator/subagent.rs:451`                                                                                                |
| Web uses `Interface::Web` for streaming chat, quick-message and voice alike                 | `crates/web-ui/src/api/messages.rs:168,869,1041`                                                                              |
| `Ask` text returned to the LLM                                                              | `crates/core/src/permission/mod.rs:128-142`                                                                                   |
| `StaticRules::evaluate` is one function: deny → confine → ask → allow                       | `crates/core/src/permission/static_rules.rs:56-96`; then `policy.rs:48-77`                                                    |
| Allow-mode refuses to vouch across shell metacharacters                                     | `static_rules.rs:20-22` (`SHELL_META`)                                                                                        |
| Built-in deny covers only exact `rm -rf ~` style forms                                      | `static_rules.rs:228-256`                                                                                                     |
| Built-in allow: file tools inside `<workspace>/**`; outside is denied by confinement        | `static_rules.rs:62-72,285-307`; roots include extra roots (`path.rs:128-140`)                                                |
| Rules are `tool(pattern)` with `*`, `\*`                                                    | `crates/core/src/permission/rule.rs:1-60`                                                                                     |
| The policy audit event fires before the callback                                            | `crates/core/src/permission/policy.rs:36-46`                                                                                  |
| `[permissions]` lives in `OrgConfig` (org.toml)                                             | `crates/core/src/types/agent.rs:65-67`, `types/permissions.rs:14-33`                                                          |
| Turn identity (user, org, space) reaches `ExecutionContext`; the run id does not            | `orchestrator/worker.rs:213-217`, `types/conversation.rs:80-103`                                                              |
| `ExecutionContext { … }` literals                                                           | 33 files (`grep -rln 'ExecutionContext {' crates`)                                                                            |
| run id = orchestrator `request_id` = bus `batch_id`                                         | `messages.rs:154-160`, `worker.rs:199-210,736-745`                                                                            |
| Turn future is dropped on cancel (`tokio::select!`)                                         | `worker.rs:220-225`                                                                                                           |
| Persona `turn_timeout_secs` overrides the submit timeout                                    | `space_runtime.rs:252-276`                                                                                                    |
| SSE seq is owned by the per-request loop in `send_message`; frames come from `SseProjector` | `messages.rs:216-345`, `crates/runtime/src/projection/mod.rs:48-56`                                                           |
| That loop `break`s on client disconnect and stops draining `event_rx` (capacity 64)         | `messages.rs:146,296-298,359-361`                                                                                             |
| Projectors are total, with no `_` arm                                                       | `projection/mod.rs:19-27`; `CliProjector` at `:223`                                                                           |
| Replay sends persisted rows verbatim                                                        | `messages.rs:704-712`                                                                                                         |
| SSE keep-alive every 15 s (axum default)                                                    | `crates/web-ui/src/api/mod.rs:71-74`                                                                                          |
| Turn routes use `{conversation_id}/turns/{turn_id}`                                         | `crates/web-ui/src/api/turns.rs:80,202`                                                                                       |
| `owned_conversation` returns plain-text 404/500 bodies                                      | `crates/web-ui/src/api/conversations.rs:50-62`                                                                                |
| `ToolCallSummary` has no `tool_call_id`                                                     | `conversations.rs:87-98` (added by `tool-result-renderers` S4)                                                                |
| Conversations are per-user; non-owners get 404                                              | `crates/web-ui/src/api/conversations.rs:40-62`                                                                                |
| Per-turn tool overlay exists                                                                | `orchestrator/mod.rs:343-368` (`with_turn_tool_source`, `turn_overlay`)                                                       |
| **Bus claims are reaped after 5 min**, and NATS `ack_wait` is 5 min                         | `crates/runtime/src/scheduler/reap.rs:15-16,30`; `crates/storage/src/message_bus.rs:236-252`; `crates/bus-nats/src/lib.rs:69` |
| Orphan sweep skips runs with an active bus message; NATS always reports one                 | `reap.rs:56-80`; `crates/bus-nats/src/lib.rs:565-568`                                                                         |
| Push goes to every subscription in the space; no user column                                | `crates/web-ui/src/push.rs:154-200`; `migrations/031_push_subscriptions.sql`                                                  |
| Service worker always shows the notification and ignores `tag`                              | `app/web/sw.js:20-45`                                                                                                         |
| Local notifications only when the app is not `resumed`                                      | `app/lib/features/notifications/agent_event_listener.dart:52-62`                                                              |
| Client ignores unknown SSE events                                                           | `app/lib/api/api_client.dart:599-790` (no `else` branch)                                                                      |
| Client treats any non-`running` turn state as finished and recovers                         | `app/lib/features/chat/chat_provider.dart:932-940`                                                                            |
| Client stall label after 30 s of silence                                                    | `openspec/specs/chat-stream-progress-ux/spec.md` "Stall indicator"                                                            |

## Decisions

### D1. One generic channel: `interaction_requested` / respond / `interaction_resolved`

An _interaction_ is a typed question that a running turn puts to the turn's
owner. It has an id, a `kind`, a kind-specific request, and at most one
response. Kinds in this change: `tool_approval` and `question`.
`generative-ui-a2ui` adds `surface_action` through the kind registry below.

Wire (SSE `event:` names, snake_case, internally tagged by `kind`):

```jsonc
// interaction_requested
{ "interaction_id": "uuid", "kind": "tool_approval",
  "created_at": "RFC3339", "expires_at": "RFC3339",
  "tool_call_id": "toolu_…",            // omitted when the provider has none
  "tool_name": "bash", "arguments": {…}, "arguments_truncated": false,
  "reason": "no rule allows 'bash' …",
  "options": ["allow_once","allow_always","reject_once","reject_always"],
  "suggested_rule": "bash(git status*)" } // omitted when allow/reject_always not offered

// interaction_requested, kind = question
{ "interaction_id": "uuid", "kind": "question", "created_at": "…", "expires_at": "…",
  "tool_call_id": "…",
  "questions": [ { "id": "q1", "prompt": "Which branch?", "options": [
      { "id": "o1", "label": "main", "description": "…" } ],
      "multiple": false, "free_text": true } ] }

// interaction_resolved
{ "interaction_id": "uuid", "kind": "tool_approval",
  "outcome": "resolved" | "cancelled" | "expired",
  "resolved_at": "RFC3339",
  "response": { "decision": "allow_always" },  // only when outcome = resolved
  "rule_id": "uuid" }                           // only when a rule was saved
```

Respond route, aligned with the existing turn routes (`turns.rs:80,202`):
`POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`.
Body `RespondToInteractionRequest`, tagged by `kind`, `deny_unknown_fields`:

```jsonc
{ "kind": "tool_approval", "decision": "reject_once" }
{ "kind": "question", "answers": [ { "question_id": "q1",
    "selected_option_ids": ["o1"], "text": "…" } ] }
{ "kind": "decline" }   // any kind: "I won't answer"; approval ⇒ reject_once
```

The enum is closed in code, and siblings extend it by adding a variant through
a MODIFIED delta of the turn-interactions requirement "Owners answer through
one respond endpoint" (`generative-ui-a2ui` adds
`{ "kind": "surface_action", "action": …, "data_model"?: … }`).

Response: `200 InteractionDetail` (request + resolution). Errors use the
`{"error": …}` envelope:

- `400`: the answer does not fit the kind or the questions, or a text field is
  too long (D10).
- `403`: below `Access::Write`, or missing `conversations:write`.
- `404`: conversation, run or interaction unknown, or not the caller's.
- `409`: already resolved with a _different_ answer.
- `410`: expired or cancelled, or no live waiter (D2).
- `413`: body over 64 KiB.

A repeated identical answer returns `200` with the stored resolution. That is
the idempotency rule.

**Kind registry.** `crates/runtime/src/interaction/kinds.rs` defines

```rust
pub trait InteractionKindHandler: Send + Sync {
    fn kind(&self) -> &'static str;
    fn validate_response(&self, request: &Value, response: &Value) -> Result<(), String>;
    fn observation(&self, request: &Value, outcome: &InteractionOutcome) -> String;
    fn push_copy(&self, request: &Value) -> PushCopy;      // title + body
    fn progress_label(&self) -> &'static str;               // "Waiting for your approval"
    fn renders_card(&self) -> bool;                         // false for surface_action
}
```

and an `InteractionKinds` registry held by the broker. The respond route
validates through it (no per-kind `match` in the route), push copy comes from
it, and `renders_card` is part of `InteractionDetail` so the client can tell
a card kind from a kind that renders elsewhere. This change registers
`tool_approval` and `question`. `generative-ui-a2ui` registers
`surface_action` (`renders_card = false`, label "Waiting for your input"),
which shares `interaction_timeout_secs`.

_Alternatives._ (a) A WebSocket per turn. Rejected: SSE plus POST already
carries everything, replay comes for free, and EventSource clients keep
working. (b) One endpoint per kind (`/approve`, `/answer`). Rejected:
`generative-ui-a2ui` would need yet another route and its own authz copy.
(c) Flat payload with no `kind` tag. Rejected: the client could not pick a card
or ignore kinds it does not know.

### D2. Topology: the turn and the HTTP server are different processes

The turn runs wherever a turn worker claims it: in the packaged install that
is `assistant orchestrator run --no-repl`; in tests, the REPL and single-binary
dev setups it is the same process. The design works for both, with no
assumption about which process holds what in memory. Shared state is
`space.db` (both processes open the same files under `~/.assistant/`) and
the message bus.

**Interactions table** (space.db): `id`, `conversation_id`, `run_id`,
`user_id`, `kind`, `tool_call_id?`, `request` JSON, `status` (`pending |
resolved | cancelled | expired`), `response` JSON?, `rule_id?`, `created_at`,
`expires_at`, `resolved_at?`, `waiter_seen_at`, `resolution_emitted` (bool).
`InteractionStore` trait with `Sqlite*` / `InMemory*` pair (ADR-0009). The
table is the source of truth for authz, idempotency, liveness, the pending
list, and receipts after reload. Resolution is a compare-and-set (`UPDATE …
WHERE id = ? AND status = 'pending'`), so two racing responders cannot both
win.

**Waiting (turn process).** `InteractionBroker::request()` inserts the row,
emits `OrchestratorEvent::InteractionRequested` on the turn's event sink, and
then waits on whichever comes first:

1. a local `oneshot` (fast path, same process as the respond route);
2. a store poll every 1 s that sees the row leave `pending` (cross-process);
3. `expires_at` (`tokio::time::sleep_until`);

while refreshing `waiter_seen_at` every 15 s. On wake it emits
`InteractionResolved` on the event sink with the stored outcome. A drop guard
removes the oneshot.

**Answering (HTTP process).** The respond route does the compare-and-set on the
row and fires the local oneshot if this process holds one. A `pending` row
whose `waiter_seen_at` is older than 45 s has no live waiter: the route
compare-and-sets it to `expired` and answers `410`.

**Events across the boundary.** The run event relay (PR 10) carries every
`OrchestratorEvent` from the turn process to the HTTP process. `send_message`
sets `SubmitOptions { relay_events: true }`. When the worker finds no local
sink for such a turn (`worker.rs:188`), it publishes each event on bus topic
`turn.event` with `batch_id = run_id`. `Token` and `Thinking` events are
coalesced into at most one message per 200 ms. The shared turn runner
(`message-branching` PR 5) claims `turn.event` filtered by `batch_id` and
feeds the events into the same channel as the local sink. It selects over
that channel and the turn result, so it ends when the turn ends, not only
when the channel closes. This fixes streaming in the split topology in
general. Today nothing streams there (see Context).

**One writer for the event log.** SSE sequence numbers stay owned by the
runner in the HTTP process. The turn emits request and resolution events
through its sink, so replay order is correct. When the runner persists an
`interaction_resolved` frame it sets `resolution_emitted = true` on the row.

**Disconnect.** The runner keeps draining, persisting and broadcasting after
the SSE client goes away; it only stops forwarding to `sse_tx`
(`messages.rs:296-298,359-361` today `break` out of the loop). Otherwise the
turn's sink fills after 64 events and the turn blocks, and the reload,
push-tap and replay paths lose every event after the interaction.

**End of run.** On every exit path the runner calls
`broker.abandon_run(run_id)`. It marks the run's `pending` rows `cancelled`
and returns every row of the run with `resolution_emitted = false`. That
includes a row whose compare-and-set won just before the turn was cancelled.
The runner writes one `interaction_resolved` per returned row, with its stored
outcome, _before_ the terminal `done`/`agent_error` frame.

**Restart.** A `pending` row with a stale `waiter_seen_at` is dead. The respond
route expires it lazily (above). `reap_stale_and_recover` (`scheduler/reap.rs`)
expires dead rows in every sweep, whether or not the run still has an active
bus message (`reap.rs:56-80` skips such runs, and NATS always reports one,
`bus-nats/src/lib.rs:565-568`). When the sweep does append a synthetic terminal
for an orphaned run, it first appends `interaction_resolved{expired}` for each
row with `resolution_emitted = false`. Where no runner is alive and no
synthetic terminal is written (NATS), the event log may lack the resolution.
The list and `ConversationDetail` still report the row as `expired`, and the
client reconciles from them. So the event-log guarantee is "exactly one
resolution event for runs whose runner is live, at most one otherwise".

**Callback wiring.** The interaction-backed callback is built in the process
that runs turns: the `interface-cli` orchestrator bootstrap
(`main.rs:144-151`). `SpaceRuntimes::with_interactions()` makes each space
runtime build its broker and callback over its own stores. The web-ui process
calls the same builder, because its respond route needs the store and the
local fast path. It never consults the callback.

_Alternatives._ (a) Run web turns in the web-ui process. Rejected: it reverses
the deployment model in `docs/web-ui.md`, needs a `Web`-excluding filter on the
orchestrator's workers, and multi-replica web-ui would still split turn and
answer. (b) A pure in-process broker. Rejected: it does not work in the
packaged topology. (c) Relay only interaction events. Rejected: the runner
needs the relay anyway to learn about the run, and a card with no surrounding
tokens or tool results makes no sense. (d) Derive everything from the event
log (`conversation_events`). Rejected: rows expire after 24 h
(`conversation_events.rs:20`), receipts must outlive that, and authz would
mean scanning JSON.

### D3. `ConfirmationCallback` becomes async and typed; `AutoDenyConfirmation` is deleted

```rust
#[async_trait]
pub trait ConfirmationCallback: Send + Sync {
    async fn confirm(&self, req: ConfirmationRequest<'_>) -> ConfirmationAnswer;
}
pub struct ConfirmationRequest<'a> {
    pub tool_name: &'a str, pub params: &'a Value, pub ctx: &'a ExecutionContext,
    pub tool_call_id: Option<&'a str>, pub reason: &'a str,
    pub options: ApprovalOptions,          // which of the four may be offered
    pub suggested_rule: Option<Rule>,
    pub event_sink: Option<&'a mpsc::Sender<OrchestratorEvent>>,
}
pub enum ConfirmationAnswer { AllowOnce, AllowAlways, RejectOnce, RejectAlways, Expired, Cancelled }
```

- **One interactivity flag.** `ExecutionContext.interactive` is no longer
  derived from `Interface` (`mod.rs:1233`). It comes from
  `TurnRequest.interactive` (`crates/core/src/bus_messages.rs:60`), which the
  submitter sets through `SubmitOptions { interactive, relay_events }` on
  `submit_turn` / `submit_turn_with_request_id`. `message-branching` PR 4's
  `SubmitTurn` struct later absorbs these fields. Set to `true` by: the shared
  turn runner for `send_message`, `edit_message` and `regenerate_message`
  (`message-branching` PR 7); the CLI REPL (`main.rs:504-511`). Left `false`
  by: the one-shot CLI command (`main.rs:196-201`), quick-message, voice, A2A,
  scheduler, messenger and subagent turns. `generative-ui-a2ui` gates its
  rich-client tools on the same flag (`ctx.interactive && depth == 0`).
- `ExecutionContext` gains `run_id: Option<Uuid>` (from the bus `batch_id`).
- `InteractiveConfirmation { broker, rules }` is the web implementation.
  `CliConfirmation` moves stdin reading into `spawn_blocking` and offers only
  the `*_once` answers. In a REPL process, a router sends `Interface::Cli`
  turns to `CliConfirmation` and everything else to the broker.
- The space runtime's callback becomes `Option<Arc<dyn ConfirmationCallback>>`.
  `AutoDenyConfirmation` (`bootstrap.rs:20-38`) goes away: a non-interactive
  turn never consults the callback (`dispatch.rs:390`), and `None` denies.
- The "always" answers write the rule _before_ the tool runs (allow) or before
  the denial is returned (reject). The rule text is the `suggested_rule` the
  server computed and stored on the interaction row. The client never sends a
  pattern, so a tampered client cannot widen a rule.
- **Audit.** Each answer emits `info!(target: "assistant::permission",
"tool approval answered", user_id, tool, interaction_id, decision, rule_id)`
  next to the existing policy event (`policy.rs:36-46`), so who decided what,
  and which rule was saved, is in the log.

_Alternative:_ keep `confirm -> bool` and block a thread. Rejected: it stalls a
Tokio worker for minutes, and it cannot express "always".

### D4. Saved rules: one more pipeline stage, never stronger than config

`tool_permission_rules` (space.db): `id`, `user_id`, `effect` (`allow |
deny`), `rule` (grammar text), `created_at`, `source_interaction_id?`, with a
unique key on (`user_id`, `effect`, `rule`). Core gets a `UserRuleSource`
trait (`async fn rules_for(&self, user_id) -> UserRules`). The runtime
implements it over the store, and `PermissionPolicy::with_user_rules` attaches
it.

`StaticRules::evaluate` (`static_rules.rs:56-96`) is one function today. It
splits into `evaluate_deny` (deny + confinement), `evaluate_ask` and
`evaluate_allow`, so the policy can interleave the user stages:

1. static deny (built-in + `org.toml`), workspace confinement
2. **user deny**
3. static ask (config)
4. static allow
5. **user allow** (allow-mode matching, so the `SHELL_META` guard applies)
6. `requires_confirmation` → Ask
7. decision model
8. Ask

A saved allow can never override a built-in or configured deny, or a
configured `ask` (which means "a human, every time"). When the `Ask` came from
a configured ask rule, `allow_always` is not offered. Decisions from these
stages use `decider = "user-rules"`, which feeds the existing audit event.

**Rule suggestion** (`permission::suggest_rule(tool, params) -> Option<Rule>`,
pure, in core). Too broad is the real risk, so the derivation errs narrow:

- `bash`: no suggestion (both "always" options withheld) when
  - the command contains a `SHELL_META` character (allow-mode would never match
    it anyway), or
  - the first token is a shell or interpreter, or a wrapper that runs other
    commands: `sh`, `bash`, `zsh`, `fish`, `dash`, `python*`, `node`, `deno`,
    `bun`, `ruby`, `perl`, `php`, `lua`, `env`, `xargs`, `eval`, `exec`,
    `nohup`, `time`, `watch`, or
  - the first token is destructive: `rm`, `mv`, `chmod`, `chown`, `dd`,
    `truncate`, `shred`.
- Otherwise `bash(<first token>[ <every leading flag token>][ <first non-flag
token>]*)`. `git status --short` → `bash(git status*)`;
  `npm -s run build` → `bash(npm -s run*)`.
- `web-fetch`: `web-fetch(<scheme>://<host>/*)`.
- Every other tool, including MCP tools: the bare tool name.
- Literal `*` in a subject is escaped as `\*`.

There is no file-tool branch. File tools inside the workspace roots are allowed
by the built-in `<workspace>/**` rule, and paths outside are denied by
confinement before `Ask` (`static_rules.rs:62-72,301-304`). A file tool only
reaches `Ask` through a configured ask rule, and then "always" is withheld
anyway. It falls under "every other tool" if that ever changes.

**Scope.** Rules apply to every turn carrying that user's identity in that
space (`ctx.user_id`, through the space's own store), whatever the interface.
See the open question.

_Alternatives._ (a) Write rules into `org.toml`. Rejected: that file is
org-wide and operator-owned, and the decision was per space and per user.
(b) Let the client edit the pattern. Deferred: it widens the attack surface
and needs pattern-preview UX.

### D5. `ask-user` is a per-turn overlay tool

`AskUserTool` lives in `crates/runtime` and is added to the turn's
`ToolOverlay` (`orchestrator/mod.rs:359`) only when `ctx.interactive` is set
and a broker exists. A non-interactive turn never sees the tool, so it cannot
fail on it. Built-in allow gets `ask-user` (`static_rules.rs:285`), so
asking a question needs no approval.

Params: `questions[1..=4]`, each `{prompt, options[0..=8]{label,
description?}, multiple?, free_text?}`. With zero options, `free_text` is
forced on. Ids are assigned by the server. The result is a JSON observation
listing each question with its answer:
`{"answers":[{"question":"<prompt>", "selected":["<label>"], "text":"…"}]}`.
A decline returns `"The user declined to answer."` and an expiry returns
`"No answer within N s."`, both as `ToolOutput::success`, since neither is a
tool fault.

_Alternative:_ a static tool in `tool-executor`. Rejected: it would need
broker access through `ExecutionContext` (a plain `Debug + Clone` struct), and
it would show up in non-interactive turns.

### D6. Timeouts, and keeping the turn alive

- `[permissions] interaction_timeout_secs` in `org.toml`, default `600`,
  allowed range `30..=3600`. The value is validated at load. It applies to
  every kind, including `surface_action`.
- **Turn deadline wins.** `expires_at` is clamped to the turn's own deadline
  (turn start + the effective submit timeout, which a Persona's
  `turn_timeout_secs` may shorten, `space_runtime.rs:252-276`). The card's
  countdown therefore never promises more time than the turn has. If the
  deadline still passes first, the turn is cancelled and the interaction ends
  `cancelled`, as in any cancellation.
- **Bus claim heartbeat.** A claim older than 5 min is reaped and redelivered
  (`reap.rs:15`, NATS `ack_wait` 300 s). A turn waiting 10 min would then run
  twice. The worker therefore refreshes the claim of the message it is running
  every 60 s: `MessageBus::touch(message_id)`, SQLite `UPDATE claimed_at`, NATS
  `AckKind::Progress`. This also fixes the existing problem for long tool
  runs. It ships as its own PR first.
- The client's 90 s byte watchdog stays satisfied by the 15 s SSE keep-alive.
  The progress card's stall label is suppressed while an interaction is
  pending, and the client treats turn state `awaiting_input` like `running`
  (`chat_provider.dart:932`).

### D7. Discovery, receipts and push

- `GET /api/conversations/{conversation_id}/interactions?status=pending`
  returns a bare array of `InteractionDetail`. Detail, not Summary, because a
  receipt needs the nested request (questions, options), and
  `openspec/config.yaml` keeps Summaries flat. On opening a conversation the
  client checks it, and if anything is pending it attaches to the run through
  the existing replay stream (`…/runs/{run_id}/events/stream?since=0`). This
  is how a push tap on another device finds the card.
- `ConversationDetail` gains `interactions: Vec<InteractionDetail>` (resolved
  and pending), so receipts render after reload. The client places each one on
  the tool call with the same `tool_call_id`. That field reaches
  `ToolCallSummary` and the client `ToolCallRecord` through
  `tool-result-renderers` S4 and C1. Without an id, it falls back to the same
  per-turn ordinal join `tool-result-renderers` uses, then to the end of the
  turn.
- **Push.** `push_subscriptions` gains a nullable `user_id`, which `POST
/api/push/subscribe` fills from the caller. Rows without a user get no
  approval pushes, and there is no backfill. If an interaction is still
  pending 30 s after it was requested, the runner pushes the kind's
  `push_copy` to the **owner's** subscriptions only, with
  `tag = interaction-<id>`: "Approval needed" for approvals, "Your assistant
  has a question" for questions. `app/web/sw.js` passes `payload.tag` to
  `showNotification`, so a repeat replaces rather than stacks. The delay avoids
  buzzing a user who is looking at the screen; the service worker has to show
  every push (`userVisibleOnly`), so suppressing it there is not an option.
- **Local notification.** On native, `AgentEventListener` shows an immediate
  local notification when the app is not `resumed`, under the new "Approval
  requests" category. On web, it skips the local notification when the browser
  has a push subscription, so the user does not get both.

### D8. Client

- `assistant_ui` adds `InteractionRecord` (pure model, including
  `rendersCard`), `ApprovalCard` and `QuestionCard`. Both take constructor
  data plus callbacks, and each ships widgetbook stories for `pending`,
  `submitting`, `resolved(each decision)`, `expired`, `cancelled` and
  `unknown kind`, with goldens. `PermissionRuleTile` (for settings) also
  lives in the package.
- **Seams from `chat-element-library`, no parallel pattern.**
  - A `tool_approval` interaction with a `tool_call_id` renders its
    `ApprovalCard` in the `ToolCallBody.footer` of the matching tool call
    (CEL PR 6). That entry is in `EntryState.awaitingInput` (CEL PR 5) while
    pending.
  - The `ask-user` tool call's `QuestionCard` is the footer of the `ask-user`
    tool call entry, in the same way.
  - `TimelineEntryType.interaction` exists only as the fallback when the
    provider supplies no `tool_call_id`.
  - There is no `TurnEventKind.awaitingInput`. The turn status snapshot
    carries the pending interaction's `progressLabel`, and the pure
    `TurnProgressCard` (CEL PR 9) takes an `awaitingLabel` that replaces the
    activity label and hides the stall label. It sits in
    `ChatThreadLayout.aboveComposer` (CEL PR 10) as today. Cards are not put in
    `aboveComposer`.
- **Client kind registry.** `features/chat/interaction_kinds.dart` maps a kind
  to a footer builder and a progress label. `tool_approval` and `question`
  are registered here; `generative-ui-a2ui` registers `surface_action` with
  no footer (it renders inside its surface) and the label "Waiting for your
  input". A kind that is not registered renders the inert "This request needs
  a newer app" card and is never answered automatically. A kind whose
  `rendersCard` is false gets no card even if unregistered.
- `app/lib/features/chat` adds `InteractionRequestedEvent` /
  `InteractionResolvedEvent` to the sealed `StreamEvent`, parsing in
  `parseSseByteStream`, and `ChatNotifier` reducers.
- **Accessibility.** Enter on a focused `ApprovalCard` means _Allow once_,
  on every platform (no platform branching inside `assistant_ui`). Escape is
  not bound to reject, so a stray keypress cannot deny a call. Semantics
  labels name the tool and the action. The "Approval needed" / "Your
  assistant has a question" announcement goes through `chat-a11y`'s
  `ChatAnnouncer`, which also suppresses its "Still working" stall
  announcement while input is awaited.

### D9. Dormant until the client can answer

Making web turns wait on `Ask` before the client can render a card would turn
an instant denial into a 10-minute hang, because today's client drops unknown
events (`api_client.dart:599-790`). The same goes for offering `ask-user`
before `QuestionCard` exists. Every server PR therefore lands dormant: the
production bootstraps do not call `SpaceRuntimes::with_interactions()`, so the
callback is `None` (deny on `Ask`, as today) and the overlay has no `ask-user`.
Tests call it explicitly. One small server PR, "enable interactions", calls it
in the orchestrator and web-ui bootstraps, and it merges only after the client
PRs for approvals and questions. No config flag, so no dead option survives.

### D10. Size limits

- `arguments` in the interaction request (row, event, SSE, detail) is capped at
  16 KiB of serialized JSON. Longer string values are cut to fit, with a
  `…[truncated]` marker, and `arguments_truncated: true` is set. The tool
  itself still runs with the full arguments, which never leave the turn
  process.
- `ask-user`: `prompt` ≤ 500 chars, option `label` ≤ 100, `description` ≤ 300.
  Violations return `ToolOutput::error` and create no interaction.
- Respond: free-text answer ≤ 4000 chars (else `400`), body ≤ 64 KiB (else
  `413`).

## Risks / Trade-offs

- **Relay load on the SQLite bus.** Every relayed event is a bus row. Token
  coalescing (200 ms) bounds it to a few rows per second per run, and the rows
  are acked and purged like any other bus message. Watch it.
- **Poll latency.** Cross-process, an answer is seen within about 1 s. That is
  fine for a human-paced prompt.
- **Cancel is still in-process.** `cancel_turn` only reaches same-process
  workers (`turns.rs:236`, `chat-stop-cancels-turn` design). In the split
  topology, Stop while waiting resolves the interaction `cancelled` only once
  the turn actually ends. Fixing cross-process cancel is out of scope here.
- **"Always" is sticky across interfaces** (D4 scope). A user who allows
  `bash(git push*)` in chat has also allowed it for their scheduled tasks.
  The UI states this next to the button.
- **Rule suggestion is heuristic.** A suggestion that is too narrow just means
  another prompt later. One that is too broad is the real risk, so shells,
  interpreters and destructive commands never get one, the derivation is pure
  and unit-tested, and the rule is shown verbatim on the card.
- **Prompt injection asking for approval.** The card shows the real arguments
  and the pipeline reason, never model-authored text. Questions from
  `ask-user` are model-authored and are visibly labelled as coming from the
  assistant.

## Migration Plan

Two new space.db tables, one nullable column, and one bus topic. Migrations
use the numbers pre-allocated in the cross-change stack plan, and each is
registered in both migration lists in `crates/storage/src/lib.rs`. No data
backfill. Rollback is a revert; the new tables are inert without the code.

Archive order for the shared `turn-status-api` spec: `chat-stop-cancels-turn`,
then `follow-up-suggestions`, then this change, whose MODIFIED delta is
rebased on follow-up-suggestions' text.

## Open Questions

1. Should saved "always" rules apply to that user's non-interactive turns too
   (scheduled tasks, messenger turns), or only to interactive web turns?
   (Current design: all of the user's turns in the space.)
2. Should space admins be able to list and revoke other members' saved rules?
   (Current design: each user sees only their own.)
3. Is a 10-minute default approval timeout and a 30 s push delay right?
4. The run event relay (D2) also makes token streaming work in the split
   topology, where today nothing streams. Is that welcome in this change, or
   should it be split out as its own change that this one depends on?
