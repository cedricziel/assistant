## MODIFIED Requirements

### Requirement: ChannelRunner drives any ChannelAdapter

The system SHALL provide a `ChannelRunner` struct in `crates/runtime` that accepts an `Arc<dyn ChannelAdapter>`, an `Arc<Orchestrator>`, and an optional `Arc<AudioStore>`, and runs the full message dispatch loop generically, without any platform-specific logic.

#### Scenario: Starts the adapter stream

- **WHEN** `ChannelRunner::run()` is called
- **THEN** it calls `adapter.start()` to obtain the inbound message stream before entering the dispatch loop

#### Scenario: Dispatches each inbound message

- **WHEN** a `ChannelMessage` arrives on the stream
- **THEN** `ChannelRunner` resolves or creates a conversation UUID, acquires the per-conversation lock, and calls `run_turn_with_tools` on the orchestrator

#### Scenario: Stops cleanly on shutdown signal

- **WHEN** SIGINT or SIGTERM is received
- **THEN** `ChannelRunner` exits its dispatch loop and calls `adapter.stop()`

#### Scenario: Audio attachments from TurnResult are sent

- **WHEN** a turn completes and `turn_result.attachments` contains entries with audio MIME types
- **THEN** each audio attachment is sent via `adapter.send()` as `ChannelContent::FileData` (same as image attachments — existing behavior)
