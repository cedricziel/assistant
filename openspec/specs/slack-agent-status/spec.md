## ADDED Requirements

### Requirement: Slack adapter calls assistant.threads.setStatus on turn start

When the Slack adapter begins processing a message in an assistant thread context, it SHALL call `assistant.threads.setStatus` to display an animated loading status to the user, in addition to the existing 👀 emoji reaction.

#### Scenario: setStatus called alongside 👀 reaction

- **WHEN** `on_turn_start` is called for a Slack turn
- **THEN** `assistant.threads.setStatus` is called with a non-empty `status` string and at least one entry in `loading_messages`
- **AND** the existing `:eyes:` reaction is still added to the message

#### Scenario: setStatus uses informative rotating messages

- **WHEN** `assistant.threads.setStatus` is called
- **THEN** the request body includes `status` and `loading_messages` with values like `"Thinking..."`, `"Working on it..."`, `"Searching knowledge base..."` etc.

#### Scenario: setStatus failure does not fail the turn

- **WHEN** the Slack API returns an error for the `setStatus` call
- **THEN** the error is logged at `debug!` level and `on_turn_start` returns `Ok(())`

#### Scenario: setStatus auto-clears on reply

- **WHEN** the Slack adapter sends the final reply message
- **THEN** Slack automatically clears the status (no explicit clear call required from the adapter)

### Requirement: SlackClient exposes a set_agent_status method

The `SlackClient` struct SHALL expose an async `set_agent_status(channel_id: &str, thread_ts: &str, status: &str, loading_messages: &[&str]) -> Result<()>` method that wraps the `assistant.threads.setStatus` API endpoint.

#### Scenario: set_agent_status posts to correct endpoint

- **WHEN** `set_agent_status` is called
- **THEN** a POST request is sent to `https://slack.com/api/assistant.threads.setStatus` with JSON body containing `channel_id`, `thread_ts`, `status`, and `loading_messages`

#### Scenario: set_agent_status returns Ok on success

- **WHEN** the Slack API responds with `{ "ok": true }`
- **THEN** `set_agent_status` returns `Ok(())`

#### Scenario: set_agent_status returns Err on API error

- **WHEN** the Slack API responds with `{ "ok": false, "error": "..." }`
- **THEN** `set_agent_status` returns an `Err` and the adapter logs at `debug!` level without propagating
