## ADDED Requirements

### Requirement: Orchestrator measures turn timing

For every turn run through `run_turn_core` that ends in a persisted final answer, the orchestrator SHALL produce a `TurnTiming`, using its injected `Clock` (`now_instant`). The fields are:

| Field                    | Definition                                                                                                                                                                                                           |
| ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `total_ms`               | from turn start to just before the final answer is persisted                                                                                                                                                         |
| `time_to_first_token_ms` | from turn start to the first visible answer token (`StreamChunk::Text`); `None` when the turn has no token sink                                                                                                      |
| `output_tokens`          | sum of `LlmResponseMeta.output_tokens` over every LLM call in the turn; `None` if any call reports `None`                                                                                                            |
| `decode_ms`              | sum over LLM calls of: call end minus that call's first streamed chunk of any kind, or, for a call that produced no chunk (non-streaming, or tool calls only), the whole call; `None` when `output_tokens` is `None` |

The timing SHALL be returned on the orchestrator's `TurnResult.timing` and SHALL be carried across the message bus on `bus_messages::TurnResult.timing`, so that a turn submitted through `submit_turn_with_request_id` returns it. Turns that are cancelled, fail, or end with an empty answer SHALL produce no timing. Thinking chunks SHALL NOT count as the first token. The extension-tool path (`run_turn_with_tools_impl`) SHALL NOT produce timing, because its final answers have no message id.

#### Scenario: Streaming turn timing on a fake clock

- **GIVEN** a `FakeClock` and a timed scripted streaming LLM that emits its first text chunk after 800 ms, returns after 3,000 ms, and reports 120 output tokens
- **WHEN** the turn completes through `run_turn_streaming`
- **THEN** `TurnResult.timing` SHALL be `{time_to_first_token_ms: 800, total_ms: 3000, output_tokens: 120, decode_ms: 2200}`

#### Scenario: Non-streaming turn has no first-token time

- **GIVEN** the same script run through `run_turn` (no token sink)
- **WHEN** the turn completes
- **THEN** `time_to_first_token_ms` SHALL be `None` AND `decode_ms` SHALL be the whole call duration

#### Scenario: Tool round-trips are excluded from decode time

- **GIVEN** a turn with two streaming LLM calls separated by a 5,000 ms tool call
- **WHEN** timing is computed
- **THEN** `decode_ms` SHALL exclude the tool time AND `total_ms` SHALL include it

#### Scenario: Tool-call-only streaming call counts its whole duration

- **GIVEN** a first streaming call that emits no chunk, returns tool calls after 1,000 ms and reports 30 output tokens, and a second call that emits text 200 ms after it starts, returns 1,200 ms after it starts and reports 100 output tokens
- **WHEN** timing is computed
- **THEN** `output_tokens` SHALL be 130 AND `decode_ms` SHALL be 2000 (1,000 for the first call plus 1,000 for the second)

#### Scenario: Missing usage yields no token figures

- **GIVEN** a provider that reports `output_tokens: None` for one of the turn's calls
- **WHEN** timing is computed
- **THEN** `output_tokens` and `decode_ms` SHALL be `None` AND `total_ms` SHALL be present

#### Scenario: Thinking does not count as first token

- **GIVEN** a stream that emits thinking chunks at 200 ms and its first text chunk at 1,500 ms
- **WHEN** timing is computed
- **THEN** `time_to_first_token_ms` SHALL be 1500

#### Scenario: Timing crosses the bus

- **GIVEN** an orchestrator with a running worker and a timed scripted provider
- **WHEN** a turn is submitted through `submit_turn_with_request_id`
- **THEN** the returned `TurnResult.timing` SHALL equal the timing the worker computed

#### Scenario: Cancelled turn has no timing

- **WHEN** a turn is cancelled through `Orchestrator::cancel_turn`
- **THEN** no timing SHALL be persisted or returned

### Requirement: Timing is persisted per assistant message

`space.db` SHALL gain a `message_timings` table: `message_id` as primary key, `conversation_id` referencing `conversations(id)` with `ON DELETE CASCADE` and indexed, nullable `time_to_first_token_ms`, `total_ms NOT NULL`, nullable `output_tokens`, nullable `decode_ms`. It SHALL NOT reference `messages(id)`, so that rewriting a conversation's messages during context compaction does not delete timings. A `MessageTimingStore` trait SHALL provide `put(conversation_id, message_id, &TurnTiming)` and `get_for_conversation(conversation_id) -> HashMap<Uuid, TurnTiming>`, with `InMemoryMessageTimingStore` and `SqliteMessageTimingStore` implementations sharing one contract test suite, exposed from `StorageLayer`. The final answer SHALL be saved and its timing put through the shared `persist_final_answer` helper. A failed `put` SHALL be logged with `warn!` and SHALL NOT fail the turn.

#### Scenario: Round trip

- **WHEN** a timing is put for a message and then fetched with `get_for_conversation`
- **THEN** the same values SHALL be returned, with absent fields as `None`

#### Scenario: Deleting the conversation removes timings

- **GIVEN** a stored timing for a message
- **WHEN** its conversation is deleted
- **THEN** the timing row SHALL be gone

#### Scenario: Compaction keeps timing of kept messages

- **GIVEN** a stored timing for assistant message M
- **WHEN** `replace_history` rewrites the conversation keeping M
- **THEN** `get_for_conversation` SHALL still return M's timing

#### Scenario: Storage failure does not fail the turn

- **GIVEN** a `MessageTimingStore` whose `put` returns an error
- **WHEN** a turn completes
- **THEN** the `done` result SHALL still be returned with its content

### Requirement: Timing is delivered on the done frame and in history

The `done` SSE frame of every web turn run through the shared turn runner (`POST /api/conversations/{id}/messages`, and the voice-message endpoint once it uses the runner) SHALL be built from a typed `SseDoneEvent` (`role`, `content`, optional `message_id`, optional `attachment_ids`, optional `timing`) and SHALL include `timing` when the turn produced one. `timing` SHALL be a `MessageTimingSummary` object with `total_ms` and, when present, `time_to_first_token_ms`, `output_tokens` and `decode_ms`, with absent fields omitted. Because the terminal frame is persisted to the conversation event log, replay through `GET /api/conversations/{id}/runs/{run}/events/stream` SHALL return the same `timing`. No new SSE event type SHALL be introduced.

`MessageSummary` in `GET /api/conversations/{id}` SHALL carry an optional `timing` (`MessageTimingSummary`) on the assistant message the timing belongs to, omitted when absent. `openapi.json` SHALL contain the `SseDoneEvent` and `MessageTimingSummary` schemas, and the generated Dart client SHALL be regenerated.

#### Scenario: Live done frame carries timing

- **GIVEN** a scripted turn that completes
- **WHEN** the client reads the SSE stream
- **THEN** the `done` data SHALL contain `timing.total_ms` as an integer AND absent timing fields SHALL be omitted, not `null`

#### Scenario: Replay carries timing

- **GIVEN** a completed run
- **WHEN** it is replayed with `since=0`
- **THEN** the replayed `done` payload SHALL contain the same `timing` object

#### Scenario: History carries timing

- **WHEN** the conversation is fetched after the turn
- **THEN** the final assistant `MessageSummary` SHALL include `timing` AND earlier messages SHALL omit it

#### Scenario: OpenAPI is in sync

- **WHEN** `make dump-openapi` runs
- **THEN** the committed `openapi.json` SHALL be unchanged AND SHALL contain the `SseDoneEvent` and `MessageTimingSummary` schemas

### Requirement: Client shows timing in message meta

The client SHALL parse `timing` from `DoneEvent` and from history `MessageSummary` into a package `MessageTiming` model on `ChatMessage.timing`, ignoring unknown fields. For finished assistant messages with timing, the app SHALL place a `MessageTimingIndicator` first in the `MessageActions` `trailing` list.

The indicator SHALL be a focusable button showing a small speed glyph, with a hit area of at least 24×24 logical pixels. Its tooltip SHALL appear on mouse hover, on keyboard focus, and on activation (tap, Enter or Space), without platform branching. The tooltip text SHALL be `formatMessageTiming(timing)`, for example "First token 0.8 s · Total 12.4 s · 42 tok/s". The semantics label SHALL be `describeMessageTiming(timing)`, for example "Response timing: first token 0.8 seconds, total 12.4 seconds, 42 tokens per second".

- Tokens per second SHALL be `output_tokens / (decode_ms / 1000)`, rounded to an integer, and shown only when both are present and `decode_ms ≥ 250`.
- Durations under 10 s SHALL show one decimal, durations from 10 s to under 60 s whole seconds, and longer durations minutes and seconds.
- Missing parts SHALL be omitted.

`MessageTimingIndicator` SHALL be registered in `kAssistantUiElements` with a widgetbook use-case and goldens.

#### Scenario: Formatter

- **WHEN** `formatMessageTiming` formats `{ttft: 800, total: 12400, output_tokens: 420, decode: 10000}`
- **THEN** it SHALL return "First token 0.8 s · Total 12.4 s · 42 tok/s"
- **WHEN** it formats `{total: 65000}`
- **THEN** it SHALL return "Total 1 m 5 s"

#### Scenario: Indicator appears after done

- **GIVEN** a live turn whose `done` frame carries timing
- **WHEN** the reply finishes
- **THEN** a `MessageTimingIndicator` SHALL appear in that message's actions

#### Scenario: No timing, no indicator

- **WHEN** an assistant message without timing renders, or a streaming message renders
- **THEN** no `MessageTimingIndicator` SHALL render

#### Scenario: Tap reveals details on touch

- **GIVEN** the indicator on a touch-sized test surface
- **WHEN** it is tapped
- **THEN** the tooltip text SHALL be visible

#### Scenario: Keyboard focus reveals details

- **GIVEN** the indicator inside a focus traversal group with a preceding focusable control
- **WHEN** Tab moves focus onto the indicator
- **THEN** the tooltip text SHALL be visible without any pointer event

#### Scenario: Screen reader gets the details without hover

- **WHEN** the indicator's semantics are inspected
- **THEN** its label SHALL be the spelled-out timing sentence starting with "Response timing:"

#### Scenario: Reload keeps timing

- **GIVEN** a conversation whose last reply has stored timing
- **WHEN** the conversation is reopened
- **THEN** the indicator SHALL be present on that reply
