## ADDED Requirements

### Requirement: ChannelAdapter exposes an on_message_received hook

The `ChannelAdapter` trait SHALL expose an async `on_message_received(msg: &ChannelMessage)` method with a default no-op implementation, called immediately when a message is received before any lock is acquired.

#### Scenario: Default implementation is a no-op

- **WHEN** `on_message_received` is called on an adapter that does not override it
- **THEN** the method returns `Ok(())` immediately without any side effects

#### Scenario: Hook is called before the per-conversation lock

- **WHEN** a message arrives at `ChannelRunner`
- **THEN** `on_message_received` is called before `tokio::spawn` and before `lock.lock().await`

### Requirement: ChannelRunner calls on_message_received before spawning

`ChannelRunner` SHALL call `adapter.on_message_received(&channel_msg).await` immediately after resolving the conversation ID, before spawning the turn task.

#### Scenario: Hook failure does not drop the message

- **WHEN** `on_message_received` returns an `Err`
- **THEN** the error is logged at `warn!` level and the message is still dispatched normally

### Requirement: Each adapter adds an hourglass reaction on message receipt

Every adapter that supports emoji reactions (Slack, Matrix, Mattermost, Nextcloud) SHALL add an ⏳ hourglass reaction to the inbound message in `on_message_received`.

#### Scenario: Hourglass added immediately on receipt (Slack)

- **WHEN** a Slack message is received
- **THEN** the Slack adapter adds an `:hourglass_flowing_sand:` reaction to the triggering message

#### Scenario: Hourglass added immediately on receipt (Matrix)

- **WHEN** a Matrix message is received
- **THEN** the Matrix adapter sends an `m.reaction` event with `⏳` to the room

#### Scenario: Hourglass added immediately on receipt (Mattermost)

- **WHEN** a Mattermost message is received
- **THEN** the Mattermost adapter adds an `hourglass_flowing_sand` reaction to the post

#### Scenario: Hourglass added immediately on receipt (Nextcloud)

- **WHEN** a Nextcloud Talk message is received
- **THEN** the Nextcloud adapter adds an ⏳ reaction to the message via the Talk reactions API

#### Scenario: Reaction failure does not drop the message

- **WHEN** adding the hourglass reaction fails for any reason
- **THEN** the error is logged at `debug!` level and the message proceeds to dispatch

### Requirement: Each adapter removes the hourglass reaction when processing begins

In `on_turn_start`, adapters SHALL remove the ⏳ hourglass reaction before (or as part of) signalling that processing has started.

#### Scenario: Hourglass removed when turn begins (Slack)

- **WHEN** `on_turn_start` is called for a Slack turn
- **THEN** the `:hourglass_flowing_sand:` reaction is removed from the triggering message before 👀 is added

#### Scenario: Hourglass removed when turn begins (Matrix)

- **WHEN** `on_turn_start` is called for a Matrix turn
- **THEN** the ⏳ reaction event is redacted from the room before the typing indicator is sent

#### Scenario: Hourglass removed when turn begins (Mattermost)

- **WHEN** `on_turn_start` is called for a Mattermost turn
- **THEN** the `hourglass_flowing_sand` reaction is removed from the post

#### Scenario: Hourglass removed when turn begins (Nextcloud)

- **WHEN** `on_turn_start` is called for a Nextcloud turn
- **THEN** the ⏳ reaction is removed from the message via the Talk reactions DELETE endpoint

#### Scenario: Remove failure does not fail the turn

- **WHEN** removing the hourglass reaction fails
- **THEN** the error is logged at `debug!` level and the turn proceeds normally
