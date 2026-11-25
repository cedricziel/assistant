## ADDED Requirements

### Requirement: Ask outcomes on interactive web turns become approval prompts

When the permission pipeline yields `Ask` on an interactive web turn, the
runtime SHALL create a `tool_approval` interaction. The interaction SHALL carry
`tool_name`, the call's `arguments`, the pipeline `reason`, `tool_call_id` when
the provider supplies one, the offered `options`, and `suggested_rule` when an
"always" option is offered. The tool SHALL NOT run until the answer is
`allow_once` or `allow_always`.

#### Scenario: Allow once

- **WHEN** the owner answers `allow_once`
- **THEN** the tool SHALL run AND no rule SHALL be saved

#### Scenario: Reject once

- **WHEN** the owner answers `reject_once`
- **THEN** the tool SHALL NOT run AND the LLM SHALL receive the observation
  `User denied execution of '<tool>'.`

#### Scenario: Expired approval

- **WHEN** the approval expires
- **THEN** the tool SHALL NOT run AND the LLM SHALL receive an observation
  stating that the approval request expired without an answer

#### Scenario: Static deny never prompts

- **WHEN** a static deny rule or workspace confinement denies the call
- **THEN** no interaction SHALL be created

### Requirement: ConfirmationCallback is asynchronous and typed

`ConfirmationCallback::confirm` SHALL be an `async` method taking a
`ConfirmationRequest` (tool name, params, execution context, tool call id,
reason, offered options, suggested rule, event sink). It SHALL return a
`ConfirmationAnswer` (`AllowOnce`, `AllowAlways`, `RejectOnce`, `RejectAlways`,
`Expired`, `Cancelled`). `AutoDenyConfirmation` SHALL be removed. A runtime
without a callback SHALL deny `Ask` outcomes.

#### Scenario: CLI keeps prompting

- **WHEN** a CLI REPL turn yields `Ask`
- **THEN** the CLI callback SHALL prompt on stdin without blocking the async
  runtime AND SHALL offer only once answers

#### Scenario: The turn process has the interactive callback

- **WHEN** the orchestrator process that runs web turns builds its space
  runtimes
- **THEN** they SHALL use the interaction-backed callback for interactive
  non-CLI turns, not auto-deny

#### Scenario: Each answer is audited

- **WHEN** the owner answers an approval
- **THEN** the runtime SHALL log a `tool approval answered` event naming the
  user, the tool, the interaction id, the decision and any saved `rule_id`

### Requirement: Always answers save a per-space per-user rule

`allow_always` and `reject_always` SHALL save the interaction's
`suggested_rule` as an `allow` or `deny` rule in the space's
`tool_permission_rules` store, keyed to the answering user. The rule text SHALL
come from the stored interaction, never from the request body. Saving the same
(user, effect, rule) twice SHALL keep one row.

#### Scenario: Allow always

- **WHEN** the owner answers `allow_always` to `bash` with
  `suggested_rule = "bash(git status*)"`
- **THEN** an allow rule `bash(git status*)` SHALL be saved for that user and
  space, the tool SHALL run, and `interaction_resolved` SHALL carry the new
  `rule_id`

#### Scenario: Saved allow skips the prompt next time

- **WHEN** the same user's next turn in the same space calls `bash` with
  `git status --short`
- **THEN** the pipeline SHALL allow it with `decider = "user-rules"` AND no
  interaction SHALL be created

#### Scenario: Rules are per user and per space

- **WHEN** another user, or the same user in a different space, makes the same
  call
- **THEN** the saved rule SHALL NOT apply

#### Scenario: Reject always

- **WHEN** the owner answers `reject_always`
- **THEN** a deny rule SHALL be saved AND the next matching call by that user
  in that space SHALL be denied without a prompt

### Requirement: Saved rules never outrank operator configuration

Saved rules SHALL be evaluated in this order inside the pipeline: static deny
and confinement, then user deny, then configured ask, then static allow, then
user allow (allow-mode matching), then `requires_confirmation`, then the
decision model. A saved allow SHALL NOT override a built-in or configured deny,
or a configured ask rule.

#### Scenario: Configured deny beats saved allow

- **WHEN** a user has saved `bash(git *)` and `org.toml` denies `bash(git push*)`
- **THEN** `git push origin main` SHALL be denied

#### Scenario: Configured ask is not bypassable

- **WHEN** the `Ask` comes from a configured `ask` rule
- **THEN** the approval SHALL NOT offer `allow_always`

#### Scenario: Saved allow satisfies requires_confirmation

- **WHEN** a tool has `requires_confirmation() == true` and a saved allow rule
  matches
- **THEN** the call SHALL be allowed without a prompt

### Requirement: Suggested rules are derived deterministically

`suggest_rule(tool, params)` SHALL be a pure function. For `bash`, it SHALL
return nothing (withholding both always options) when the command contains a
shell metacharacter, when the first token is a shell, an interpreter or a
command wrapper (`sh`, `bash`, `zsh`, `fish`, `dash`, `python*`, `node`, `deno`,
`bun`, `ruby`, `perl`, `php`, `lua`, `env`, `xargs`, `eval`, `exec`, `nohup`,
`time`, `watch`), or when the first token is destructive (`rm`, `mv`, `chmod`,
`chown`, `dd`, `truncate`, `shred`). Otherwise it SHALL return
`bash(<first token>[ <every leading flag token>][ <first non-flag token>]*)`.
For `web-fetch`, it SHALL return `web-fetch(<scheme>://<host>/*)`. For every
other tool, it SHALL return the bare tool name. Literal `*` in a subject SHALL
be escaped.

#### Scenario: Bash subcommand

- **WHEN** the command is `git status --short`
- **THEN** the suggestion SHALL be `bash(git status*)`

#### Scenario: Bash with leading flags

- **WHEN** the command is `npm -s run build`
- **THEN** the suggestion SHALL be `bash(npm -s run*)`

#### Scenario: Bash with a pipe

- **WHEN** the command is `cat a | sh`
- **THEN** there SHALL be no suggestion AND the options SHALL be `allow_once`,
  `reject_once`

#### Scenario: Destructive command

- **WHEN** the command is `rm -rf build`
- **THEN** there SHALL be no suggestion AND the options SHALL be `allow_once`,
  `reject_once`

#### Scenario: Interpreter

- **WHEN** the command is `python3 -c "print(1)"`
- **THEN** there SHALL be no suggestion AND the options SHALL be `allow_once`,
  `reject_once`

#### Scenario: Web fetch

- **WHEN** the URL is `https://docs.rs/tokio/latest/tokio/`
- **THEN** the suggestion SHALL be `web-fetch(https://docs.rs/*)`

### Requirement: Users can list and revoke their saved rules

The server SHALL expose `GET /api/tool-permission-rules` (`operationId:
list_tool_permission_rules`), which returns a bare array of
`ToolPermissionRuleSummary` (`id`, `effect`, `rule`, `tool_name`,
`created_at`) for the caller in the selected space. It SHALL also expose
`DELETE /api/tool-permission-rules/{id}` (`operationId:
delete_tool_permission_rule`), which returns `204`. A rule belonging to another
user SHALL return `404`. Both routes SHALL be documented in `openapi.json`.

#### Scenario: Revoke

- **WHEN** the owner deletes a saved allow rule
- **THEN** the response SHALL be `204` AND the next matching call SHALL prompt
  again

#### Scenario: Cannot see others' rules

- **WHEN** a member lists rules
- **THEN** only rules they saved in that space SHALL be returned

### Requirement: Settings show saved approvals

The settings screen SHALL have a "Tool approvals" section that lists the
current space's saved rules, grouped as Allowed and Denied, with each rule's
text and creation date and a revoke action. It SHALL have explicit loading,
error and empty states. The `PermissionRuleTile` row widget SHALL live in
`assistant_ui`.

#### Scenario: Empty state

- **WHEN** the user has no saved rules in the space
- **THEN** the section SHALL say that no approvals are saved and explain that
  "Always allow" on an approval card adds one

#### Scenario: Revoke from settings

- **WHEN** the user activates revoke on a rule and confirms
- **THEN** the client SHALL call `DELETE /api/tool-permission-rules/{id}` AND
  the row SHALL disappear

### Requirement: ApprovalCard renders inline and becomes a receipt

`assistant_ui` SHALL provide `ApprovalCard`, a pure widget taking an
`InteractionRecord` and an `onDecision` callback. While pending, it SHALL show
the tool name, a one-line summary of the primary argument, expandable full
arguments, the reason, the time until expiry, and buttons for the offered
options. For `allow_always` and `reject_always`, it SHALL show the
`suggested_rule` text and note that the rule applies to all of the user's
turns in this space. While a response is in flight, the buttons SHALL be
disabled. Once resolved, it SHALL render as a read-only receipt that states the
outcome. It SHALL ship widgetbook stories and goldens for every state.

#### Scenario: Pending card on its tool call

- **WHEN** `interaction_requested{kind: tool_approval}` arrives with a
  `tool_call_id`
- **THEN** an ApprovalCard SHALL appear in the `ToolCallBody` footer of the
  tool call with that id, AND that entry SHALL be in `EntryState.awaitingInput`

#### Scenario: Pending card without a tool call id

- **WHEN** `interaction_requested{kind: tool_approval}` arrives without a
  `tool_call_id`
- **THEN** an ApprovalCard SHALL appear as an `interaction` timeline entry at
  that point of the turn

#### Scenario: Receipt after reload

- **WHEN** the conversation is reloaded after the approval was answered
- **THEN** the card SHALL render from `ConversationDetail.interactions` as a
  receipt such as "Allowed once" or "Always allowed · bash(git status*)"

#### Scenario: Accessible actions

- **WHEN** a screen reader focuses an action button
- **THEN** its semantics label SHALL name both the action and the tool (e.g.
  "Allow bash once")

#### Scenario: Keyboard

- **WHEN** an ApprovalCard has focus and the user presses Enter
- **THEN** it SHALL fire `allow_once` on every platform
- **AND** Escape SHALL NOT fire any decision

#### Scenario: Truncated arguments

- **WHEN** the request carries `arguments_truncated: true`
- **THEN** the expanded arguments SHALL say that they were shortened
