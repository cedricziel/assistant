## ADDED Requirements

### Requirement: Every tool call passes a permission pipeline before execution

The orchestrator SHALL evaluate a permission pipeline for every tool call
dispatched through the global executor path and the subagent path, before the
tool runs. The pipeline SHALL apply, in order: static rules, the decision model,
and the fallback (human confirmation or denial). A call that does not obtain an
`Allow` outcome MUST NOT execute.

#### Scenario: Denied call never executes

- **WHEN** the pipeline returns `Deny` for a tool call
- **THEN** the tool SHALL NOT run AND the LLM SHALL receive an error observation
  containing the denial reason

#### Scenario: Subagent calls are gated

- **WHEN** a subagent requests a tool call
- **THEN** the same pipeline SHALL run with `interactive = false`

### Requirement: Static rules decide first and deny always wins

Static rules SHALL be evaluated in the order deny, ask, allow. A matching deny
rule (configured or built-in hard-deny) SHALL yield `Deny` regardless of any
allow rule and SHALL NOT be sent to the decision model.

#### Scenario: Deny beats allow

- **WHEN** config contains `allow = ["bash(git *)"]` and a built-in hard-deny
  matches `bash(git push --force * && rm -rf /)`
- **THEN** the outcome SHALL be `Deny`

#### Scenario: Hard-denied call skips the model

- **WHEN** a static deny matches and a decision model is configured
- **THEN** the decision model SHALL NOT be called

#### Scenario: Allow rule short-circuits

- **WHEN** an allow rule matches and no deny or ask rule matches
- **THEN** the outcome SHALL be `Allow` and the decision model SHALL NOT be
  called

### Requirement: Rule grammar

A rule SHALL be `tool-name` or `tool-name(pattern)`. `*` and `**` in a pattern
SHALL match any run of characters, including `/`. The pattern SHALL be matched
against the tool's primary argument (`bash`→`command`, `file-read` /
`file-write` / `file-edit` / `file-glob`→`path`, `web-fetch`→`url`,
`web-search`→`query`); for other tools it SHALL be matched against the compact
JSON of the parameters. The token `<workspace>` SHALL expand to the persona
workspace and configured extra roots. File-path subjects SHALL be canonicalized
(symlinks resolved) before matching.

#### Scenario: Argument pattern matches

- **WHEN** the rule `bash(git status*)` is evaluated for `bash` with
  `command = "git status --short"`
- **THEN** it SHALL match

#### Scenario: Symlink escape is matched by real location

- **WHEN** `file-write(<workspace>/**)` is evaluated for a path that is a
  symlink inside the workspace pointing outside it
- **THEN** the rule SHALL NOT match

#### Scenario: Malformed rule is rejected at load

- **WHEN** config contains a rule that does not parse
- **THEN** configuration loading SHALL fail with an error naming the rule

### Requirement: Decision model approves only at or above the threshold

Calls left undecided by static rules SHALL be sent to the configured decision
model with the tool name, arguments and the turn's task text. The call SHALL be
approved only when every asked question's score is at or above
`permissions.decision_model.threshold` (default `0.9`). Any other result — low
score, transport error, non-2xx status, timeout, malformed or out-of-range
answer — SHALL yield `Ask` (never `Allow`). The configured model id SHALL be
pinned; ids that are `latest` aliases SHALL be rejected at configuration load.

#### Scenario: High-confidence approval

- **WHEN** the model returns scores of 0.95 and 0.97 and the threshold is 0.9
- **THEN** the outcome SHALL be `Allow` with the minimum score recorded as the
  confidence

#### Scenario: Below threshold

- **WHEN** any score is below the threshold
- **THEN** the outcome SHALL be `Ask`

#### Scenario: Fail closed

- **WHEN** the request times out, returns an error status, or returns a body
  that does not parse
- **THEN** the outcome SHALL be `Ask`

#### Scenario: Unpinned model rejected

- **WHEN** `model = "typesafe/jev-latest"`
- **THEN** configuration loading SHALL fail

#### Scenario: Model input excludes tool output

- **WHEN** the decision request is built
- **THEN** it SHALL contain only the tool name, the arguments and the user's
  task text, and SHALL NOT contain prior tool results or fetched content

### Requirement: Undecided calls fall back to a human or are denied

When the pipeline yields `Ask`, an interactive turn with a confirmation
callback SHALL defer to the callback. A non-interactive turn (or an interactive
turn without a callback) SHALL be denied, and the denial reason SHALL be
returned to the LLM as an error tool result.

#### Scenario: Non-interactive turn denies

- **WHEN** a scheduled turn calls `bash` with a command no rule decides and no
  decision model is configured
- **THEN** the tool SHALL NOT run AND the LLM SHALL receive an error observation
  stating that approval was required but unavailable

#### Scenario: Interactive turn asks the human

- **WHEN** an interactive turn's call yields `Ask` and the callback returns true
- **THEN** the tool SHALL run

#### Scenario: Tools requiring confirmation are not implicitly allowed

- **WHEN** a tool returns `requires_confirmation() == true`, no static rule
  matches, and the turn is non-interactive
- **THEN** the call SHALL be denied

### Requirement: Every decision is recorded

Each pipeline evaluation SHALL emit a `tracing` event on target
`assistant::permission` with fields `tool`, `decider`, `rule` (rule text or
model id), `confidence` (when applicable) and `outcome`. Tool arguments and
task text SHALL NOT be included in the event.

#### Scenario: Decision event emitted

- **WHEN** any tool call is evaluated
- **THEN** exactly one decision event SHALL be emitted with the final outcome

### Requirement: Permissions are configured under `[permissions]`

The runtime config SHALL expose `[permissions]` with `deny`, `ask`, `allow`
(rule lists), `workspace_roots`, and `[permissions.decision_model]`. With no
`[permissions]` section the built-in defaults SHALL apply and permissive
behaviour SHALL NOT be restored.

#### Scenario: Defaults deny the undecided

- **WHEN** no `[permissions]` section exists and a non-interactive turn calls a
  tool with no built-in allow
- **THEN** the call SHALL be denied
