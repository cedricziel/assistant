## Why

Web chat can never ask a human. `crates/web-ui/src/lib.rs:329` hardwires
`AutoDenyConfirmation`, and web turns are not even interactive:
`ExecutionContext.interactive` is `matches!(interface, Interface::Cli)`
(`crates/runtime/src/orchestrator/mod.rs:1233`). Every call the permission
pipeline from `tool-permission-decision-model` leaves at `Ask` is therefore
denied in the browser (`crates/core/src/permission/mod.rs:128-142`). The model
also has no way to ask the user a clarifying question mid-turn. The
confirmation hook is synchronous (`ConfirmationCallback::confirm -> bool`,
`orchestrator/mod.rs:50-54`), so it cannot wait on a network answer. And in the
packaged install the turn runs in `assistant orchestrator run --no-repl`, not in
the web-ui process (`crates/web-ui/src/lib.rs:399-423`), which also wires
auto-deny (`crates/interface-cli/src/main.rs:144-151`). No turn event crosses
that process boundary today.

## What Changes

- **Turn interaction channel** (owned here, reused by `generative-ui-a2ui`): the
  server emits `interaction_requested` on the run's SSE stream, persisted in the
  event log so replay and reconnect surface it again. The client answers with
  `POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`.
  The server then emits `interaction_resolved` (`resolved | cancelled |
expired`). Responses are idempotent. Only the turn's owner may answer, and
  pending interactions expire after a timeout. Cancelling the turn resolves
  them as `cancelled`. Kinds are registered handlers, so siblings add kinds
  without touching the route.
- **Works across processes.** The waiting turn sees answers through the shared
  `interactions` table, and a run event relay over the bus (`turn.event`)
  carries the turn's events to the web-ui process's SSE stream. The turn
  runner keeps persisting events after the browser disconnects.
- **BREAKING** `ConfirmationCallback` becomes async and takes a rich request,
  returning a typed answer. `AutoDenyConfirmation` is deleted. Interactivity
  becomes an explicit submit option: streaming web chat turns (send, edit,
  regenerate) and CLI REPL turns are interactive. Messenger, scheduler, A2A,
  quick-message, voice, one-shot CLI and subagent turns stay non-interactive
  and keep denying on `Ask`.
- **Approval options** `allow_once`, `allow_always`, `reject_once`,
  `reject_always`. The "always" answers save a per-space, per-user rule in the
  existing `tool(pattern)` grammar. A new pipeline stage applies the rule to every one of that user's
  turns in the space, including scheduled and messenger turns. Users can list
  and revoke their rules over REST and in a settings section, and space admins
  can list and revoke their members' rules. "Always" is never offered for
  shells, interpreters, command wrappers or destructive commands.
- **`ask-user` tool**: one to four questions, each with options and/or free
  text, answered through the same channel. Only interactive turns get it.
- **Client**: `ApprovalCard` and `QuestionCard` in `assistant_ui`, shown inline
  in the timeline, then kept as read-only receipts that survive a reload. The
  turn card reads "Waiting for your approval" with no stall label. An approval
  still pending after 30 s pushes to the turn owner.
- Turn status gains `awaiting_input`. Bus claims get a heartbeat so a long wait
  does not get the turn redelivered.
- Every server PR lands dormant. The stack is ordered so the one server PR that
  switches interactions on lands after the client cards render and answer
  them. There is no feature flag.
- Branching (edit or regenerate) while an approval is pending cancels it.
- ADR-0017.

## Capabilities

### New Capabilities

- `turn-interactions`: the generic pending-interaction channel (events,
  endpoint, persistence, expiry, cancellation, authz).
- `tool-approval-prompts`: human approval of `Ask` calls, saved rules, the
  rules API and settings UI.
- `ask-user-tool`: the clarifying-question tool and `QuestionCard`.

### Modified Capabilities

- `chat-stream-progress-ux`: no stall label while awaiting input.
- `turn-status-api`: `awaiting_input` state, rebased on
  `follow-up-suggestions`' presence-based derivation.
- `notification-settings`: new "Approval requests" category.
- `pwa-push-notifications`: owner-targeted approval push.

## Impact

- Rust: `core`, `runtime`, `storage` (two space.db tables, a nullable
  `push_subscriptions.user_id`, a `turn.event` bus topic; migrations 047,
  048 and 059 from the stack plan), `web-ui`,
  `interface-cli`, `bus-nats`. Regenerated `openapi.json` and `assistant_api`.
- Edits the shared turn runner extracted by `message-branching` PR 5, never
  `send_message` directly.
- Flutter: `assistant_ui` cards and stories, plus `features/chat`,
  `features/settings`, `features/notifications` and `app/web/sw.js`. Cards use
  `chat-element-library`'s `ToolCallBody.footer`, `EntryState.awaitingInput`
  and the parameterised `TurnProgressCard`.
- **Visual change**: new cards, a new settings section (with an admin group),
  and a new turn label.
  **Playwright baselines will move.**
- **User-facing docs: yes.** `docs/tool-permissions.md` (approvals, saved
  rules) and a new `docs/chat/approvals-and-questions.md`.

### Non-goals

- Approvals in Slack, Matrix, Mattermost, Signal or Nextcloud (they keep
  deny-on-`Ask`).
- Approving subagent calls (subagents stay non-interactive).
- Editing a saved rule's pattern, or creating rules by hand in the UI.
- Answering from a different user, or delegating approvals to admins. (Admins
  can revoke members' saved rules, not answer or create them.)
- Cross-process turn cancellation (`cancel_turn` stays in-process; see
  `chat-stop-cancels-turn`).
