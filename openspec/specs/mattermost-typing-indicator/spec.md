## ADDED Requirements

### Requirement: MattermostClient exposes a send_typing method

The `MattermostClient` struct SHALL expose an async `send_typing(channel_id: &str) -> Result<()>` method using `POST /api/v4/users/me/typing`.

#### Scenario: send_typing posts to the correct endpoint

- **WHEN** `send_typing(channel_id)` is called
- **THEN** a POST request is sent to `/api/v4/users/me/typing` with body `{ "channel_id": "<channel_id>" }`

#### Scenario: send_typing returns Err on non-200

- **WHEN** the server responds with a non-200 status
- **THEN** `send_typing` returns an `Err`

### Requirement: MattermostClient exposes reaction add and remove methods

The `MattermostClient` struct SHALL expose `add_reaction(user_id: &str, post_id: &str, emoji_name: &str) -> Result<()>` and `remove_reaction(user_id: &str, post_id: &str, emoji_name: &str) -> Result<()>` methods.

#### Scenario: add_reaction posts to reactions endpoint

- **WHEN** `add_reaction(user_id, post_id, "hourglass_flowing_sand")` is called
- **THEN** a POST request is sent to `/api/v4/reactions` with the correct JSON body

#### Scenario: remove_reaction calls DELETE endpoint

- **WHEN** `remove_reaction(user_id, post_id, "hourglass_flowing_sand")` is called
- **THEN** a DELETE request is sent to `/api/v4/users/{userId}/posts/{postId}/reactions/{emojiName}`

### Requirement: Mattermost adapter adds hourglass reaction on message receipt

In `on_message_received`, the Mattermost adapter SHALL add a `hourglass_flowing_sand` reaction to the inbound post.

#### Scenario: Hourglass reaction added on receipt

- **WHEN** a Mattermost message with a known post ID is received
- **THEN** `add_reaction` is called with `"hourglass_flowing_sand"` and the post ID from the message metadata

#### Scenario: Reaction failure is silently ignored

- **WHEN** the server rejects the reaction
- **THEN** the error is logged at `debug!` level and the message proceeds

### Requirement: Mattermost adapter removes hourglass and sends typing on turn start

In `on_turn_start`, the Mattermost adapter SHALL remove the `hourglass_flowing_sand` reaction and call `send_typing(channel_id)`.

#### Scenario: Hourglass removed before typing sent

- **WHEN** `on_turn_start` is called
- **THEN** `remove_reaction` is called with `"hourglass_flowing_sand"`, then `send_typing(channel_id)` is called

#### Scenario: Failures do not fail the turn

- **WHEN** either remove or typing call fails
- **THEN** the error is logged at `debug!` level and `on_turn_start` returns `Ok(())`

### Requirement: Mattermost typing signal is fire-and-forget

The Mattermost server auto-expires typing indicators server-side. The adapter SHALL NOT send an explicit clear on turn end.

#### Scenario: No explicit clear on turn success or error

- **WHEN** `on_turn_success` or `on_turn_error` is called
- **THEN** no additional typing API call is made
