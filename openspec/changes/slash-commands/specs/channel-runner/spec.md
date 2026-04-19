## MODIFIED Requirements

### Requirement: ChannelRunner drives any ChannelAdapter

The system SHALL provide a `ChannelRunner` struct in `crates/runtime` that accepts an `Arc<dyn ChannelAdapter>`, an `Arc<Orchestrator>`, and an optional `Arc<AudioStore>`, and runs the full message dispatch loop generically, without any platform-specific logic. Before dispatching a text message to the orchestrator, the runner SHALL check if the text is a slash command and route it to the `CommandRegistry` instead.

#### Scenario: Starts the adapter stream

- **WHEN** `ChannelRunner::run()` is called
- **THEN** it calls `adapter.start()` to obtain the inbound message stream before entering the dispatch loop

#### Scenario: Dispatches each inbound message

- **WHEN** a `ChannelMessage` arrives on the stream with text that is NOT a slash command
- **THEN** `ChannelRunner` resolves or creates a conversation UUID, acquires the per-conversation lock, and calls `run_turn_with_tools` on the orchestrator

#### Scenario: Intercepts slash commands before dispatch

- **WHEN** a `ChannelMessage` arrives with text starting with `/` followed by a registered command name
- **THEN** the message is routed to `CommandRegistry::execute()` instead of the orchestrator
- **THEN** the command result's ack text is sent back via `adapter.send()`
- **THEN** a `conversation_events` record is persisted

#### Scenario: Stops cleanly on shutdown signal

- **WHEN** SIGINT or SIGTERM is received
- **THEN** `ChannelRunner` exits its dispatch loop and calls `adapter.stop()`

#### Scenario: Audio attachments from TurnResult are sent

- **WHEN** a turn completes and `turn_result.attachments` contains entries with audio MIME types
- **THEN** each audio attachment is sent via `adapter.send()` as `ChannelContent::FileData` (same as image attachments — existing behavior)

### Requirement: Per-conversation turn serialization

The system SHALL serialize concurrent turns within the same conversation so that at most one `run_turn_with_tools` call is in-flight per conversation at any time.

#### Scenario: Two messages arrive for the same conversation

- **WHEN** two messages arrive simultaneously with the same conversation key
- **THEN** the second turn waits for the first to complete before `run_turn_with_tools` is called

#### Scenario: Two messages arrive for different conversations

- **WHEN** two messages arrive simultaneously with different conversation keys
- **THEN** both turns proceed concurrently without blocking each other

### Requirement: Conversation key to UUID mapping

The system SHALL maintain an LRU cache (capacity 10,000) mapping the adapter's conversation key string to a stable `Uuid`, creating a new UUID on first encounter. The `/new` command SHALL evict the key from this cache.

#### Scenario: Same key returns same UUID

- **WHEN** two messages share the same conversation key
- **THEN** `ChannelRunner` resolves both to the same conversation UUID

#### Scenario: New key creates a new UUID

- **WHEN** a message arrives with a previously unseen conversation key
- **THEN** a new UUID is generated and stored in the cache

#### Scenario: `/new` evicts conversation key

- **WHEN** a user sends `/new`
- **THEN** the conversation key is removed from the LRU cache
- **THEN** the next message from the same context generates a new UUID

### Requirement: Platform tools injected per turn

The system SHALL call `adapter.platform_tools(&msg, conv_id)` before each `run_turn_with_tools` call and pass the returned tools as extensions.

#### Scenario: Adapter returns tools

- **WHEN** `adapter.platform_tools()` returns a non-empty vec
- **THEN** those tools are available to the LLM during the turn

#### Scenario: Adapter returns no tools

- **WHEN** `adapter.platform_tools()` returns an empty vec
- **THEN** `run_turn_with_tools` is called with an empty extensions list

### Requirement: Lifecycle hooks called around each turn

The system SHALL call `adapter.on_turn_start()` before dispatching and either `adapter.on_turn_success()` or `adapter.on_turn_error()` after the turn completes.

#### Scenario: Successful turn

- **WHEN** `run_turn_with_tools` returns `Ok`
- **THEN** `adapter.on_turn_success(user, &result)` is called

#### Scenario: Failed turn

- **WHEN** `run_turn_with_tools` returns `Err`
- **THEN** `adapter.on_turn_error(user, &err)` is called and the error is logged

### Requirement: Response sent back through the adapter

The system SHALL call `adapter.send_in_thread()` (or `send()` if no thread) with the turn result text after a successful turn.

#### Scenario: Turn produces a reply

- **WHEN** `run_turn_with_tools` returns a non-empty answer
- **THEN** the answer is sent via the originating adapter to the originating user
