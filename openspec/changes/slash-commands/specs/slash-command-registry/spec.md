## ADDED Requirements

### Requirement: Command definitions are typed and centralized

The system SHALL provide a `CommandDef` struct containing: `name` (kebab-case string), `description` (one-line help text), `category` (grouping string), and `args` (a list of argument definitions with name, required flag, and optional completions endpoint). A `CommandRegistry` SHALL hold all built-in command definitions and provide methods to list and look up commands by name.

#### Scenario: List all commands

- **WHEN** a consumer calls `CommandRegistry::list()`
- **THEN** it receives all registered `CommandDef` entries

#### Scenario: Look up a command by name

- **WHEN** a consumer calls `CommandRegistry::get("model")`
- **THEN** it receives the `CommandDef` for `/model`

#### Scenario: Look up unknown command

- **WHEN** a consumer calls `CommandRegistry::get("nonexistent")`
- **THEN** it receives `None`

### Requirement: `/new` starts a fresh conversation

The system SHALL provide a `/new` command that resets the conversation context. In `ChannelRunner`, this evicts the conversation key from the LRU cache so the next message generates a fresh UUID. In the CLI, this resets the conversation ID. Old messages remain in the database but are not loaded for subsequent turns.

#### Scenario: `/new` in a messenger interface

- **WHEN** a user sends `/new` in a Slack/Matrix/etc channel
- **THEN** the conversation key is evicted from the LRU cache
- **THEN** the adapter sends an ack: "New conversation started."
- **THEN** the next message from the same context creates a new conversation UUID

#### Scenario: `/new` clears per-conversation config

- **WHEN** a user sends `/new` after having set `/model claude-opus-4`
- **THEN** the model override is cleared
- **THEN** the next conversation uses the global default model

### Requirement: `/stop` cancels a running turn

The system SHALL provide a `/stop` command that cancels the currently in-flight turn for the conversation. It SHALL locate the active `CancellationToken` for the conversation's current turn and cancel it. The command SHALL execute immediately without acquiring the conversation lock.

#### Scenario: Stop a running turn

- **WHEN** a user sends `/stop` while an orchestrator turn is in progress
- **THEN** the turn's `CancellationToken` is cancelled
- **THEN** the adapter sends an ack: "Stopped."
- **THEN** the orchestrator stops after the current tool execution completes

#### Scenario: Stop when no turn is running

- **WHEN** a user sends `/stop` with no turn in progress
- **THEN** the adapter sends an ack: "Nothing to stop."

### Requirement: `/model` switches the model for a conversation

The system SHALL provide a `/model` command that accepts a model name argument and sets it as a per-conversation override. The override SHALL take effect on the next turn. Without arguments, `/model` SHALL display the current model.

#### Scenario: Switch model

- **WHEN** a user sends `/model claude-opus-4`
- **THEN** the per-conversation model override is set to `claude-opus-4`
- **THEN** the adapter sends an ack: "Model switched to claude-opus-4."
- **THEN** subsequent turns use `claude-opus-4` for LLM calls

#### Scenario: Show current model

- **WHEN** a user sends `/model` with no arguments
- **THEN** the adapter sends the current model name (override if set, else global default)

#### Scenario: Unknown model name

- **WHEN** a user sends `/model nonexistent-model`
- **THEN** the override is set (validation is deferred to the LLM provider at turn time)
- **THEN** the adapter sends an ack: "Model switched to nonexistent-model."

### Requirement: `/compact` triggers context compaction

The system SHALL provide a `/compact` command that invokes the existing compaction engine (`maybe_compact`) with a force flag, bypassing the token-threshold check. The command SHALL acquire the per-conversation lock before executing (waiting for any in-flight turn to finish).

#### Scenario: Compact a conversation with history

- **WHEN** a user sends `/compact` in a conversation with multiple turns
- **THEN** the system waits for any running turn to complete
- **THEN** the compaction engine summarizes older turns via the LLM
- **THEN** the conversation history is replaced with a summary plus recent turns
- **THEN** the adapter sends an ack: "Context compacted."

#### Scenario: Compact with insufficient history

- **WHEN** a user sends `/compact` in a conversation with fewer turns than `keep_recent_turns`
- **THEN** no compaction is performed
- **THEN** the adapter sends an ack: "Nothing to compact."

### Requirement: `/status` shows conversation state

The system SHALL provide a `/status` command that displays read-only introspection data: current model, conversation ID, estimated token count, and interface type.

#### Scenario: Status with model override

- **WHEN** a user sends `/status` after setting `/model claude-opus-4`
- **THEN** the response shows `model: claude-opus-4 (override)` and the conversation UUID

#### Scenario: Status with default model

- **WHEN** a user sends `/status` with no model override
- **THEN** the response shows the global default model name and the conversation UUID

### Requirement: `/help` lists available commands

The system SHALL provide a `/help` command that lists all registered commands with their descriptions.

#### Scenario: Help output

- **WHEN** a user sends `/help`
- **THEN** the adapter sends a formatted list of all commands with descriptions
- **THEN** each entry shows the command name and a one-line description
