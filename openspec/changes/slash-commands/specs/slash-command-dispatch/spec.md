## ADDED Requirements

### Requirement: Slash commands are intercepted before orchestrator dispatch

The system SHALL check every inbound text message for a leading `/` followed by a registered command name. Matching messages SHALL be routed to the `CommandRegistry` instead of the orchestrator. The command text SHALL never be included in the LLM's conversation history.

#### Scenario: Recognized command is intercepted

- **WHEN** a user sends `/new` in any interface
- **THEN** the message is handled by the `CommandRegistry`
- **THEN** the orchestrator's `run_turn_with_tools` is NOT called
- **THEN** no message is stored in the `messages` table

#### Scenario: Unrecognized slash text is treated as a normal message

- **WHEN** a user sends `/new-york pizza` (not a registered command)
- **THEN** the full text is dispatched to the orchestrator as a normal user message

#### Scenario: Command matching is exact on first word

- **WHEN** a user sends `/model claude-opus-4`
- **THEN** the first word `model` is matched against the registry
- **THEN** `claude-opus-4` is passed as the argument

### Requirement: Most commands bypass the conversation lock

The system SHALL execute `/new`, `/stop`, `/model`, `/status`, and `/help` without acquiring the per-conversation turn lock. These commands SHALL execute immediately even while a turn is in progress.

#### Scenario: `/stop` during a running turn

- **WHEN** a turn is in progress and the user sends `/stop`
- **THEN** `/stop` executes immediately without waiting for the turn to finish

#### Scenario: `/status` during a running turn

- **WHEN** a turn is in progress and the user sends `/status`
- **THEN** `/status` returns current state immediately

### Requirement: `/compact` acquires the conversation lock

The system SHALL acquire the per-conversation lock before executing `/compact`, ensuring no turn is in progress when history is mutated.

#### Scenario: `/compact` waits for running turn

- **WHEN** a turn is in progress and the user sends `/compact`
- **THEN** the system sends an immediate ack to the user
- **THEN** compaction executes after the running turn completes

### Requirement: Active turn tracking per conversation

The system SHALL track which `request_id` is currently active for each conversation so that `/stop` can locate the correct `CancellationToken` in the orchestrator's `turn_cancellations` map.

#### Scenario: Turn starts and is tracked

- **WHEN** `ChannelRunner` dispatches a turn for conversation `conv_id`
- **THEN** the turn's `request_id` is recorded in an `active_turns` map

#### Scenario: Turn finishes and tracking is cleared

- **WHEN** the dispatched turn completes (success or failure)
- **THEN** the `request_id` is removed from `active_turns`

#### Scenario: `/stop` uses active turn tracking

- **WHEN** `/stop` is invoked for `conv_id`
- **THEN** the system looks up the active `request_id` for `conv_id`
- **THEN** it cancels the corresponding token in `turn_cancellations`

### Requirement: CLI uses the shared CommandRegistry

The CLI REPL SHALL route slash commands through the same `CommandRegistry` used by `ChannelRunner`, replacing the existing hardcoded string matching in `main.rs`. CLI-only commands (`/quit`, `/exit`, `/skills`, `/review`, `/install`) SHALL be handled locally before checking the shared registry.

#### Scenario: CLI dispatches `/new` through registry

- **WHEN** the user types `/new` in the CLI REPL
- **THEN** it is handled by `CommandRegistry::execute()`
- **THEN** the conversation ID is reset

#### Scenario: CLI retains local commands

- **WHEN** the user types `/quit` in the CLI REPL
- **THEN** it is handled by the CLI's local dispatch (not the registry)
- **THEN** the process exits
