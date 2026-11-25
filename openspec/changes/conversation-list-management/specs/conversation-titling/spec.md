## MODIFIED Requirements

### Requirement: Titles are never retroactively changed

The title-generator worker SHALL NOT overwrite a title once `title_locked = 1`, regardless of how the title was set (auto-generated, manually renamed, or explicit at create). Manual renames through `PATCH /api/conversations/{id}` always apply. The title-generator worker SHALL write its title through `ConversationStore::set_generated_title`, which updates the row only while `title_locked = 0` in the same SQL statement, so that a manual rename committed while the LLM call is in flight is never overwritten. `set_generated_title` SHALL NOT change `updated_at`.

#### Scenario: Worker does not retitle after manual rename

- **WHEN** a user renames a conversation via `PATCH` (setting `title_locked = 1`)
- **AND** a subsequent `turn.completed` is published for that conversation
- **THEN** the worker observes `title_locked = 1` and does not call the LLM
- **AND** the user's chosen title persists

#### Scenario: Worker does not retitle a previously auto-titled conversation

- **WHEN** the worker has already auto-titled a conversation in an earlier turn
- **AND** a later `turn.completed` arrives for the same conversation
- **THEN** the worker observes `title_locked = 1` and exits without an LLM call

#### Scenario: Rename during the LLM call wins

- **GIVEN** the worker has read `title_locked = 0` and is awaiting the LLM
- **WHEN** the user renames the conversation to "Trip plan" before the LLM returns
- **AND** the worker then calls `set_generated_title` with the generated title
- **THEN** `set_generated_title` returns `false` and the stored title remains "Trip plan"
- **AND** no `ConversationUpserted` event is emitted by the worker's write
- **AND** the worker acks the bus message

#### Scenario: Generated title does not reorder the list

- **WHEN** `set_generated_title` writes a title
- **THEN** the conversation's `updated_at` is unchanged

## ADDED Requirements

### Requirement: Clients create conversations untitled

First-party clients SHALL create conversations without a `title` field unless the user typed a title, so the title-generator worker can title them. The Flutter app's "New Chat" button and the new-chat keyboard shortcut SHALL send `POST /api/conversations` with an empty body.

#### Scenario: New Chat button leaves the conversation unlocked

- **WHEN** the user presses "New Chat" in the conversation list
- **THEN** the app sends `POST /api/conversations` with no `title`
- **AND** the created row has `title = NULL` and `title_locked = 0`
- **AND** the list shows the conversation as "Untitled" until the worker titles it

#### Scenario: Keyboard shortcut leaves the conversation unlocked

- **WHEN** the user presses Cmd+N (Apple) or Ctrl+N (other platforms)
- **THEN** the app sends `POST /api/conversations` with no `title`
