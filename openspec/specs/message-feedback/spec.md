# message-feedback Specification

## Purpose

Storage, REST contract and OTel emission for per-user ratings of assistant replies.

## Requirements

### Requirement: Feedback is stored per user per assistant message

The system SHALL store at most one feedback record per (message, user) pair in the space database's `message_feedback` table. Each record SHALL hold a `rating` of `positive` or `negative`, an optional `comment`, and `created_at` and `updated_at` timestamps. Setting feedback SHALL be a single atomic upsert that never changes `created_at` of an existing record. Clearing SHALL report whether a record was removed. Deleting a message SHALL delete its feedback. The storage SHALL be reached through a `MessageFeedbackStore` trait with SQLite and in-memory implementations that pass the same contract tests, except that referential checks (cascade, unknown message id) apply to the SQLite implementation only.

#### Scenario: Setting feedback twice replaces it

- **GIVEN** a user has rated an assistant message `positive`
- **WHEN** the same user rates it `negative` with comment "wrong date"
- **THEN** exactly one record SHALL exist for that message and user, with rating `negative` and comment "wrong date"
- **AND** its `created_at` SHALL be unchanged while `updated_at` SHALL advance

#### Scenario: Concurrent sets keep the first creation time

- **GIVEN** no feedback exists for a message and user
- **WHEN** two sets for that pair run concurrently
- **THEN** both sets SHALL succeed, exactly one record SHALL exist, and its `created_at` SHALL be the one written by whichever set committed first

#### Scenario: Clearing feedback removes the record

- **GIVEN** a feedback record for a message and user
- **WHEN** the feedback is cleared
- **THEN** no record SHALL exist and the clear SHALL report that a record was removed
- **AND** clearing again SHALL succeed and report that nothing was removed

#### Scenario: Feedback is listed for one user only

- **GIVEN** users A and B have each rated messages in the same conversation
- **WHEN** feedback is listed for that conversation and user A
- **THEN** only user A's records SHALL be returned

#### Scenario: Message deletion cascades

- **GIVEN** a feedback record on a message in the SQLite store
- **WHEN** the message's conversation is deleted
- **THEN** the feedback record SHALL no longer exist

#### Scenario: Unknown message id is rejected by SQLite

- **GIVEN** the SQLite store
- **WHEN** feedback is set for a message id that does not exist
- **THEN** the store SHALL return an error and store nothing

### Requirement: Feedback REST endpoints

The server SHALL expose `PUT /api/conversations/{id}/messages/{message_id}/feedback` (operationId `set_message_feedback`), accepting `SetMessageFeedbackRequest { rating, comment? }` and returning 200 with `MessageFeedbackResponse { message_id, rating, comment?, created_at, updated_at }`. `PUT` SHALL fully replace the caller's feedback, so an absent `comment` clears any stored comment. The server SHALL expose `DELETE` on the same path (operationId `delete_message_feedback`), returning 204 whether or not feedback existed. Both SHALL require that the caller owns the conversation, has a space role with write access, and either holds the `conversations:write` scope or is an org admin. The target SHALL be a final-answer assistant message: role `assistant`, non-empty content, no tool calls, and not a thinking entry. A `comment` SHALL be trimmed, an empty result SHALL mean no comment, and its length SHALL be limited to 2000 Unicode scalar values. Every error response, including malformed path ids, malformed or mistyped bodies and ownership failures, SHALL use the `{"error": "..."}` envelope. Both operations SHALL be documented in `openapi.json`, including the `X-Space-Id` header.

#### Scenario: Rate an assistant message

- **GIVEN** a conversation owned by the caller containing a final-answer assistant message
- **WHEN** the caller sends `PUT .../feedback` with `{"rating": "positive"}`
- **THEN** the response SHALL be 200 with `rating` `positive` and no `comment` field

#### Scenario: Whitespace comment is stored as none

- **WHEN** the caller sends a `comment` of only whitespace
- **THEN** the feedback SHALL be stored without a comment

#### Scenario: Over-long comment is rejected

- **WHEN** the caller sends a `comment` of 2001 Unicode scalar values after trimming
- **THEN** the response SHALL be 400 with the error envelope and nothing SHALL be stored

#### Scenario: Comment limit counts scalar values

- **WHEN** the caller sends a `comment` of 2000 Unicode scalar values that includes multi-scalar emoji
- **THEN** the response SHALL be 200

#### Scenario: Only final-answer assistant messages accept feedback

- **WHEN** the caller sends `PUT .../feedback` for a user message, a thinking entry, or an assistant row that carries tool calls with empty content
- **THEN** the response SHALL be 400 with the error envelope

#### Scenario: Unknown rating is rejected

- **WHEN** the caller sends `{"rating": "meh"}`
- **THEN** the response SHALL be 400 with the error envelope

#### Scenario: Missing content type is rejected

- **WHEN** the caller sends a JSON body without a `Content-Type: application/json` header
- **THEN** the response SHALL be 400 with the error envelope

#### Scenario: Malformed path id is rejected

- **WHEN** `{id}` or `{message_id}` is not a UUID
- **THEN** the response SHALL be 400 with the error envelope

#### Scenario: Foreign or unknown targets are not found

- **WHEN** the conversation belongs to another user, or the message does not belong to the conversation
- **THEN** the response SHALL be 404 with the error envelope and nothing SHALL be stored

#### Scenario: Viewer cannot rate or clear

- **GIVEN** a caller with the viewer role in the space
- **WHEN** they send `PUT .../feedback` or `DELETE .../feedback`
- **THEN** the response SHALL be 403

#### Scenario: Missing write scope is forbidden

- **GIVEN** a non-admin caller whose token lacks the `conversations:write` scope
- **WHEN** they send `PUT .../feedback`
- **THEN** the response SHALL be 403

#### Scenario: Org admin bypasses the scope check

- **GIVEN** an org admin who owns the conversation and whose token lacks the `conversations:write` scope
- **WHEN** they send `PUT .../feedback`
- **THEN** the response SHALL be 200

#### Scenario: Clear feedback

- **GIVEN** the caller has rated a message
- **WHEN** they send `DELETE .../feedback` twice
- **THEN** both responses SHALL be 204 and the feedback SHALL be gone

### Requirement: Conversation history carries the caller's feedback

`GET /api/conversations/{id}` SHALL include, on each `MessageSummary` the caller has rated, a `feedback` object `MessageFeedbackSummary { rating, comment?, updated_at }`. The field SHALL be omitted for messages the caller has not rated. The projection SHALL load feedback with a single query per request.

#### Scenario: Existing vote appears in history

- **GIVEN** the caller rated an assistant message `negative` with comment "too long"
- **WHEN** they load the conversation
- **THEN** that message's summary SHALL include `feedback` with rating `negative` and comment "too long"
- **AND** other messages SHALL have no `feedback` field

### Requirement: Assistant replies record their LLM span context

When the orchestrator persists the final-answer assistant message of a turn, it SHALL store on the message the W3C `traceparent` of the `chat {model}` span that produced the answer, in the nullable column `messages.traceparent`. The value SHALL be formatted from the span's own context, independent of the globally installed propagator. When the span context is invalid, for example with telemetry disabled, the column SHALL be `NULL`. The traceparent SHALL NOT be exposed in the REST API.

#### Scenario: Traceparent captured with telemetry on

- **GIVEN** a tracer provider is installed
- **WHEN** a streaming turn ends with a final answer
- **THEN** the persisted assistant message's `traceparent` SHALL name the trace id and span id of that iteration's `chat {model}` span

#### Scenario: No traceparent without telemetry

- **GIVEN** no tracer provider is installed
- **WHEN** a turn ends with a final answer
- **THEN** the persisted assistant message's `traceparent` SHALL be `NULL`

### Requirement: Feedback is emitted as a GenAI evaluation event

After every successful set, and after every clear that removed a record, the server SHALL emit an OpenTelemetry log event named `gen_ai.evaluation.result` with these attributes:

- `gen_ai.evaluation.name` = `user_feedback`;
- `gen_ai.evaluation.score.label` = `positive`, `negative` or `cleared`;
- `gen_ai.evaluation.score.value` = 1.0 for `positive` and -1.0 for `negative`, and absent for `cleared`;
- `gen_ai.conversation.id`;
- `gen_ai.agent.id`;
- `assistant.message.id`;
- `assistant.feedback.has_comment`.

When the message has a stored traceparent, the event SHALL carry that trace id and span id as its trace context. The comment text SHALL NOT be exported in any telemetry signal, under any configuration, including `[observability] trace_content`: the event SHALL NOT carry `gen_ai.evaluation.explanation` or any attribute holding comment text, and the comment SHALL stay in the space database. For each emitted event, the server SHALL increment the counter `assistant.feedback.count` with attributes `rating` (`positive`, `negative` or `cleared`) and `agent_id`; the counter counts feedback events, not current votes. Telemetry failures SHALL NOT fail the request.

#### Scenario: Event is parented to the evaluated span

- **GIVEN** an assistant message with a stored traceparent
- **WHEN** the caller rates it `negative`
- **THEN** one `gen_ai.evaluation.result` log record SHALL be exported with that trace id and span id, `score.label` `negative` and `score.value` -1.0

#### Scenario: Comment text is never exported

- **GIVEN** `trace_content` is either enabled or disabled
- **WHEN** the caller rates `positive` or `negative` with the comment "secret-token-123"
- **THEN** the event SHALL have `assistant.feedback.has_comment` true
- **AND** it SHALL have no `gen_ai.evaluation.explanation` and no attribute value containing "secret-token-123"

#### Scenario: Vote without a comment

- **WHEN** the caller rates without a comment
- **THEN** the event SHALL have `assistant.feedback.has_comment` false

#### Scenario: Clearing a vote emits a cleared event without a score value

- **GIVEN** the caller has rated a message
- **WHEN** the caller deletes their feedback
- **THEN** an event with `score.label` `cleared` and no `score.value` SHALL be emitted
- **AND** `assistant.feedback.count` SHALL be incremented with `rating` `cleared`

#### Scenario: Clearing nothing emits nothing

- **GIVEN** the caller has not rated a message
- **WHEN** the caller deletes their feedback
- **THEN** no event SHALL be emitted and the counter SHALL NOT change

#### Scenario: Message without traceparent still emits

- **GIVEN** an assistant message whose `traceparent` is `NULL`
- **WHEN** the caller rates it
- **THEN** the event SHALL be emitted without trace context, still carrying `gen_ai.conversation.id` and `assistant.message.id`
