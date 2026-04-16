## ADDED Requirements

### Requirement: conversation_key method on ChannelAdapter

The `ChannelAdapter` trait SHALL provide a `conversation_key(&self, msg: &ChannelMessage) -> String` method with a default implementation of `"{sender.platform_id}:{thread_id ?? platform_message_id}"`. Adapters MAY override this to encode platform-specific threading logic.

#### Scenario: Default key groups by sender and thread

- **WHEN** two messages share the same `sender.platform_id` and `thread_id`
- **THEN** `conversation_key()` returns the same string for both

#### Scenario: Matrix adapter uses room_id only

- **WHEN** `MatrixAdapter::conversation_key()` is called
- **THEN** it returns only the `sender.platform_id` (room ID), ignoring `thread_id`

#### Scenario: Slack adapter encodes channel and thread_ts

- **WHEN** `SlackAdapter::conversation_key()` is called with a message that has a `thread_id`
- **THEN** it returns `"{channel_id}:{thread_ts}"`

### Requirement: platform_tools method on ChannelAdapter

The `ChannelAdapter` trait SHALL provide a `fn platform_tools(&self, msg: &ChannelMessage, conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>>` method with a default that returns an empty vec. Adapters MAY override this to return platform-specific tool handlers scoped to the current message context.

#### Scenario: Default returns empty vec

- **WHEN** an adapter does not override `platform_tools()`
- **THEN** no additional tools are added to the turn

#### Scenario: Slack adapter returns Slack-specific tools

- **WHEN** `SlackAdapter::platform_tools()` is called
- **THEN** it returns tools such as `slack-post`, `slack-react`, and `slack-reply` scoped to the inbound message's channel and thread

### Requirement: on_turn_start hook on ChannelAdapter

The `ChannelAdapter` trait SHALL provide an `async fn on_turn_start(&self, user: &ChannelUser) -> Result<()>` method with a default no-op. Adapters MAY override this to send a typing indicator or add a processing reaction.

#### Scenario: Default is a no-op

- **WHEN** an adapter does not override `on_turn_start()`
- **THEN** no side effects occur and `Ok(())` is returned

#### Scenario: Slack adapter adds eyes reaction

- **WHEN** `SlackAdapter::on_turn_start()` is called
- **THEN** it posts a 👀 reaction to the inbound message

### Requirement: on_turn_success hook on ChannelAdapter

The `ChannelAdapter` trait SHALL provide an `async fn on_turn_success(&self, user: &ChannelUser, result: &TurnResult) -> Result<()>` method with a default no-op. Adapters MAY override this to post a completion reaction or confirmation.

#### Scenario: Default is a no-op

- **WHEN** an adapter does not override `on_turn_success()`
- **THEN** no side effects occur and `Ok(())` is returned

#### Scenario: Slack adapter adds check-mark reaction

- **WHEN** `SlackAdapter::on_turn_success()` is called
- **THEN** it posts a ✅ reaction to the inbound message

### Requirement: on_turn_error hook on ChannelAdapter

The `ChannelAdapter` trait SHALL provide an `async fn on_turn_error(&self, user: &ChannelUser, err: &anyhow::Error) -> Result<()>` method with a default no-op. Adapters MAY override this to post an error notification to the user.

#### Scenario: Default is a no-op

- **WHEN** an adapter does not override `on_turn_error()`
- **THEN** no side effects occur and `Ok(())` is returned

#### Scenario: Slack adapter posts error message

- **WHEN** `SlackAdapter::on_turn_error()` is called
- **THEN** it sends a message to the channel indicating the turn failed
