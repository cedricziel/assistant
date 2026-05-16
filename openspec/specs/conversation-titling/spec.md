## ADDED Requirements

### Requirement: Conversations start with NULL title regardless of interface

The system SHALL create every conversation with `title = NULL` at the storage layer, independent of the originating interface (web, Slack, Mattermost, Matrix, Nextcloud, Signal, CLI, MCP, scheduler, A2A). Read endpoints SHALL coerce `NULL` titles to the display string `"Untitled"`.

#### Scenario: New conversation from web API has NULL title

- **WHEN** a client calls `POST /api/conversations` without a `title` in the request body
- **THEN** the persisted `conversations` row has `title = NULL` and `title_locked = 0`
- **AND** the response includes `"title": "Untitled"` in the summary projection

#### Scenario: New conversation from messenger interface has NULL title

- **WHEN** a Slack/Matrix/Mattermost/Nextcloud/Signal interface dispatches a first turn to the orchestrator
- **THEN** the conversation created by `prepare_history` has `title = NULL` and `title_locked = 0`

#### Scenario: New conversation from CLI has NULL title

- **WHEN** the CLI starts a session and calls `submit_turn` with a fresh `conversation_id`
- **THEN** the persisted conversation has `title = NULL` and `title_locked = 0`

### Requirement: Conversations carry a `title_locked` flag

The system SHALL store a `title_locked` boolean on every conversation row. The flag MUST be `1` whenever a title was written — by the title-generator worker, by explicit user rename, or by an explicit non-NULL title in `POST /api/conversations`. The flag MUST be `0` for conversations whose `title` is `NULL`.

#### Scenario: Manual rename locks the title

- **WHEN** a client calls `PATCH /api/conversations/{id}` with a new title
- **THEN** the row has `title_locked = 1` after the update completes

#### Scenario: Explicit title at creation locks the title

- **WHEN** a client calls `POST /api/conversations` with a non-empty `title` in the request body
- **THEN** the row has `title_locked = 1` after creation

#### Scenario: Migration locks all pre-existing titled rows

- **WHEN** the schema migration introducing `title_locked` runs
- **THEN** every existing row with `title IS NOT NULL` has `title_locked = 1`
- **AND** every existing row with `title IS NULL` has `title_locked = 0`

### Requirement: A background worker consumes `turn.result` and produces titles

The system SHALL run a background worker that claims messages from the `turn.result` topic on the `MessageBus` and, for eligible conversations, calls an LLM to produce a short title (≤ 60 characters) and writes it via `ConversationStore::update_title`.

#### Scenario: Worker titles an unlocked conversation that meets the turn threshold

- **WHEN** a `turn.result` envelope is published with `turn >= min_turns` (default `2`)
- **AND** the conversation referenced by `conversation_id` has `title_locked = 0`
- **THEN** the worker calls the LLM with a summarisation prompt
- **AND** writes the LLM output via `update_title`
- **AND** the row's `title_locked` becomes `1`
- **AND** a `ConversationUpserted` event is broadcast carrying the new title

#### Scenario: Worker titles after the first turn when the user message is long

- **WHEN** a `turn.result` envelope is published with `turn == 1`
- **AND** the first user message length exceeds `long_first_message_chars` (default `200`)
- **AND** the conversation has `title_locked = 0`
- **THEN** the worker generates and persists a title for that conversation

#### Scenario: Worker skips locked conversations

- **WHEN** a `turn.result` envelope is published for a conversation with `title_locked = 1`
- **THEN** the worker MUST NOT call the LLM
- **AND** MUST NOT overwrite the existing title
- **AND** acks the bus message

#### Scenario: Worker skips conversations below threshold

- **WHEN** a `turn.result` envelope is published with `turn < min_turns`
- **AND** the first user message length is `<= long_first_message_chars`
- **THEN** the worker MUST NOT call the LLM
- **AND** acks the bus message

#### Scenario: Worker is idempotent under bus redelivery

- **WHEN** a `turn.result` envelope is redelivered after the worker already titled the conversation
- **THEN** on second delivery the worker observes `title_locked = 1` and exits without an LLM call
- **AND** acks the redelivered message

### Requirement: Worker failures must not block other consumers

The system SHALL retry transient LLM failures using the bus's bounded redelivery mechanism. On permanent failure (exceeding `MAX_TURN_REDELIVERIES`), the worker SHALL leave the conversation untitled. Other `turn.result` consumers (webhooks, workflows) MUST NOT be affected by titling failures.

#### Scenario: Transient LLM error triggers redelivery with backoff

- **WHEN** the LLM provider returns a transient error (timeout, 5xx)
- **THEN** the worker `nack_delayed`s the bus message with exponential backoff
- **AND** the conversation's `title` and `title_locked` remain unchanged

#### Scenario: Permanent failure leaves conversation untitled

- **WHEN** the LLM provider has returned errors exceeding `MAX_TURN_REDELIVERIES` for the same `turn.result` message
- **THEN** the bus marks the message permanently `Failed`
- **AND** the conversation's `title_locked` remains `0`
- **AND** the conversation remains displayable as `"Untitled"`

#### Scenario: Webhook dispatch is unaffected by title-worker failure

- **WHEN** the title-generator worker fails to process a `turn.result`
- **THEN** the separate webhook-dispatch consumer of `turn.result` still receives and processes the same envelope independently

### Requirement: Titles are never retroactively changed

The system SHALL NOT overwrite a title once `title_locked = 1`, regardless of how the title was set (auto-generated, manually renamed, or explicit at create).

#### Scenario: Worker does not retitle after manual rename

- **WHEN** a user renames a conversation via `PATCH` (setting `title_locked = 1`)
- **AND** a subsequent `turn.result` is published for that conversation
- **THEN** the worker observes `title_locked = 1` and does not call the LLM
- **AND** the user's chosen title persists

#### Scenario: Worker does not retitle a previously auto-titled conversation

- **WHEN** the worker has already auto-titled a conversation in an earlier turn
- **AND** a later `turn.result` arrives for the same conversation
- **THEN** the worker observes `title_locked = 1` and exits without an LLM call

### Requirement: Titling is configurable per-org

The system SHALL accept a `[titling]` block in `orgs/{slug}/org.toml` with the keys `enabled` (bool, default `true`), `min_turns` (integer, default `2`), and `long_first_message_chars` (integer, default `200`). When `[titling]` is absent, defaults SHALL apply. The worker SHALL use the conversation's primary LLM provider for the title call.

#### Scenario: Disabling titling stops the worker for that org

- **WHEN** `[titling].enabled = false` in an org's `org.toml`
- **THEN** the worker MUST NOT call the LLM for any conversation belonging to that org
- **AND** still acks `turn.result` envelopes for that org

#### Scenario: Missing `[titling]` block applies defaults

- **WHEN** an org's `org.toml` has no `[titling]` section
- **THEN** the worker behaves as if `enabled = true`, `min_turns = 2`, `long_first_message_chars = 200`

### Requirement: Auto-titling removes legacy truncation behaviour

The system SHALL NOT perform the 57-character truncation of the first user message previously implemented in `POST /api/conversations/{id}/messages` and `POST /quick-message`. The system SHALL NOT apply the `"New Chat"` default title in `POST /api/conversations` when the caller omits a title.

#### Scenario: `send_message` no longer auto-truncates titles

- **WHEN** a client sends the first message to a conversation via `POST /api/conversations/{id}/messages`
- **THEN** the persisted `title` remains `NULL` (it is no longer set to the truncated message)
- **AND** the conversation's `title_locked` remains `0`

#### Scenario: `quick-message` no longer auto-truncates titles

- **WHEN** a client calls `POST /quick-message`
- **THEN** the newly created conversation has `title = NULL` and `title_locked = 0`
- **AND** the response includes the conversation ID as before

#### Scenario: `create_conversation` without explicit title leaves NULL

- **WHEN** a client calls `POST /api/conversations` with `{}` or omits `title`
- **THEN** the persisted row has `title = NULL` and `title_locked = 0`
- **AND** the response `title` field is `"Untitled"` (NULL-coerced for display)
