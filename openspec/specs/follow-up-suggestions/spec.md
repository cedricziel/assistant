# follow-up-suggestions Specification

## Purpose

Bounded, best-effort follow-up prompt suggestions that the web chat offers after a completed turn, delivered as a `suggestions` event on the run replay stream and configured by the org `[suggestions]` section.

## Requirements

### Requirement: Org-level suggestions configuration

`OrgConfig` SHALL have a `[suggestions]` section with `enabled` (bool, default `true`), `max_suggestions` (integer, default `3`, clamped to `1..=5`) and `timeout_secs` (integer, default `10`, clamped to `1..=15`). An `org.toml` without the section SHALL deserialize to the defaults. The upper timeout bound of 15 seconds SHALL be a shared constant that the client's listen window is derived from.

#### Scenario: Absent section yields defaults

- **WHEN** an `org.toml` without `[suggestions]` is loaded
- **THEN** `suggestions.enabled` SHALL be `true`, `max_suggestions` SHALL be `3`, and `timeout_secs` SHALL be `10`

#### Scenario: Out-of-range count is clamped

- **WHEN** `org.toml` sets `max_suggestions = 9`
- **THEN** the effective maximum SHALL be `5`

#### Scenario: Out-of-range timeout is clamped

- **WHEN** `org.toml` sets `timeout_secs = 0`
- **THEN** the effective timeout SHALL be 1 second
- **AND WHEN** `org.toml` sets `timeout_secs = 60`
- **THEN** the effective timeout SHALL be 15 seconds

### Requirement: Follow-up generation is a bounded secondary LLM call

The runtime SHALL provide `follow_ups::generate(last_user_message, answer, max, llm)`. It SHALL issue one non-streamed `LlmProvider::chat` call with no tools, with each input cut to 1 000 Unicode scalar values. It SHALL return at most `max` prompts. A pure `parse_suggestions(raw, max)` SHALL strip list markers, numbering and wrapping quotes, drop empty lines and lines longer than 120 characters, and remove case-insensitive duplicates. A response that is not a final answer SHALL yield an empty list, not an error. The call SHALL be recorded in a GenAI span marked as the follow-up suggestion feature, with its outcome.

#### Scenario: Numbered list is cleaned

- **GIVEN** a scripted LLM answering `1. "Show an example"\n2. Explain step two\n- show an example`
- **WHEN** `generate` runs with `max = 3`
- **THEN** it SHALL return `["Show an example", "Explain step two"]`

#### Scenario: Tool-call response yields nothing

- **GIVEN** a scripted LLM returning a tool-call response
- **WHEN** `generate` runs
- **THEN** it SHALL return an empty list

#### Scenario: Long inputs are truncated

- **GIVEN** a last user message of 3 000 characters
- **WHEN** `generate` runs
- **THEN** the prompt the provider receives SHALL contain only the first 1 000 characters of that message

#### Scenario: Over-long lines are dropped

- **GIVEN** a scripted LLM answering one 121-character line and one short line
- **WHEN** `generate` runs
- **THEN** it SHALL return only the short line

#### Scenario: Count is capped at max

- **GIVEN** a scripted LLM answering five distinct lines
- **WHEN** `generate` runs with `max = 2`
- **THEN** it SHALL return the first two lines

### Requirement: Web turns emit a suggestions event after done

The shared turn runner SHALL run the suggestion phase only when its caller opts in (`RunOptions.suggestions`). After the runner emits a `done` frame that carries a `message_id`, it SHALL generate follow-ups if all of these hold: the caller opted in, `[suggestions].enabled` is true, the turn ended in `done` (not `agent_error` of any reason), and someone is listening (the `done` frame reached the original SSE receiver, or at least one replay subscriber exists). The runner SHALL send the Web Push notification for the turn, for every terminal kind, and then close the original POST stream before the phase starts. Generation SHALL be bounded by the effective timeout and SHALL be aborted when, after a 2 second attach grace, no replay subscriber is connected. If the result is non-empty, the server SHALL persist a `suggestions` row at the next sequence number and broadcast it to replay subscribers. The payload SHALL be `{"message_id": "<uuid>", "suggestions": [{"prompt": "<text>"}]}` and SHALL be documented in OpenAPI. On an empty result, an error, a timeout or an abort, it SHALL emit nothing. The run SHALL finish only after this phase.

#### Scenario: POST stream closes at done

- **GIVEN** suggestions are enabled
- **WHEN** a client streams a message to completion
- **THEN** the POST stream SHALL end with `done` and close without waiting for the suggestion phase

#### Scenario: Suggestions arrive on the run tail

- **GIVEN** suggestions are enabled and the LLM produces an answer and then two follow-ups
- **WHEN** the client, after `done` at seq N, opens the run's replay stream with `since=N+1`
- **THEN** it SHALL receive `suggestions` with two items whose `message_id` equals `done.message_id`
- **AND** the replay stream SHALL then close

#### Scenario: Disabled org emits no suggestions

- **GIVEN** `[suggestions] enabled = false`
- **WHEN** a turn completes
- **THEN** no `suggestions` row SHALL be persisted or broadcast AND no extra LLM call SHALL be made

#### Scenario: Cancelled turn emits no suggestions

- **WHEN** a turn ends in `agent_error` with `reason: "cancelled"`
- **THEN** no `suggestions` row SHALL be persisted AND no extra LLM call SHALL be made

#### Scenario: Errored turn emits no suggestions

- **WHEN** a turn ends in `agent_error` with `reason: "error"`
- **THEN** no `suggestions` row SHALL be persisted AND no extra LLM call SHALL be made

#### Scenario: Done without a message id emits no suggestions

- **WHEN** a turn ends in a `done` frame that has no `message_id`
- **THEN** no extra LLM call SHALL be made

#### Scenario: Timeout emits nothing and still finishes the run

- **GIVEN** an LLM that does not answer the suggestion call within the effective timeout
- **WHEN** the turn completes
- **THEN** no `suggestions` row SHALL be persisted AND the run SHALL finish within the effective timeout after `done`

#### Scenario: Nobody listening skips the call

- **GIVEN** the client disconnected before `done` and no replay subscriber exists
- **WHEN** the turn completes
- **THEN** the server SHALL NOT make the suggestion LLM call

#### Scenario: Departed client aborts the call

- **GIVEN** the client received `done` but opened no replay subscription, or closed it
- **WHEN** the 2 second attach grace has passed and no replay subscriber is connected
- **THEN** the server SHALL abort the suggestion call, persist nothing, and finish the run

#### Scenario: Push is not delayed by the phase

- **GIVEN** push notifications are configured and the suggestion call takes 5 seconds
- **WHEN** a turn completes
- **THEN** the push notification SHALL be sent before the suggestion call starts

### Requirement: Only opted-in web turns produce suggestions

The suggestion phase SHALL run only for runner callers that pass `RunOptions.suggestions = true`: the web text send, edit and regenerate handlers. Voice-message turns (`suggestions = false`), quick-message turns and messenger, CLI, MCP, scheduler and A2A turns SHALL NOT generate or emit suggestions.

#### Scenario: Messenger turn has no suggestion call

- **WHEN** a Slack turn completes through the channel runner
- **THEN** the LLM SHALL receive no suggestion request for that turn

#### Scenario: Voice turn has no suggestion call

- **GIVEN** suggestions are enabled
- **WHEN** a voice message turn completes on the shared runner
- **THEN** no `suggestions` row SHALL be persisted AND no extra LLM call SHALL be made

### Requirement: Suggestions survive replay

A persisted `suggestions` row SHALL be returned by `GET /api/conversations/{id}/runs/{run_id}/events/stream` like any other event, in sequence order and subject to the event TTL.

#### Scenario: Reconnect after done receives suggestions

- **GIVEN** a completed run whose log holds `done` at seq 40 and `suggestions` at seq 41
- **WHEN** a client replays with `since=40`
- **THEN** it SHALL receive `done` and then `suggestions`, and the stream SHALL close

#### Scenario: Suggestions ready before the client subscribes

- **GIVEN** a run that persisted `suggestions` at seq 41 and finished before the client subscribed
- **WHEN** the client replays with `since=41`
- **THEN** it SHALL receive `suggestions`, and the stream SHALL close

### Requirement: Client parses suggestions and ignores unknown events

The Flutter SSE parser SHALL turn the `suggestions` event into a `SuggestionsEvent(messageId, prompts)` in the sealed `StreamEvent` hierarchy, carrying the SSE `id` as `sequenceId`. A malformed payload SHALL be dropped without error. Event types the client does not recognize SHALL continue to be ignored.

#### Scenario: Parse a suggestions frame

- **WHEN** the parser receives `event: suggestions` with two prompts and `id: 41`
- **THEN** it SHALL yield one `SuggestionsEvent` with two prompts and `sequenceId == 41`

#### Scenario: Unknown event is skipped

- **WHEN** the parser receives `event: future_thing`
- **THEN** it SHALL yield nothing for that frame and continue with later frames

### Requirement: Composer unlocks at done and chips arrive afterwards

The chat view model SHALL complete the turn and clear `isSending` when `DoneEvent` arrives, without waiting for suggestions, and the queue SHALL start draining the next message at once. On a `DoneEvent` with a `messageId`, it SHALL open the run's replay stream from `done.sequenceId + 1` on a separate subscription, using the conversation id, run id and message id captured at `done`, and read it for at most the listen window (the server's 15 second maximum budget plus 5 seconds). That listener SHALL NOT read or modify the shared stream session's replay cursor, run id, cancel flag or detach state. It SHALL set `ChatState.suggestions` from a `SuggestionsEvent` only while that event's `messageId` is the last assistant message and no newer send has started. The view model SHALL cancel the listener when a new turn starts, when the conversation is loaded, cleared or switched, and on dispose. It SHALL clear `suggestions` on send, conversation load, conversation clear and conversation switch.

#### Scenario: Queue does not wait for suggestions

- **GIVEN** a queued second message and a run whose `suggestions` arrive 3 s after `done`
- **WHEN** `done` arrives
- **THEN** the second message SHALL start streaming before the `suggestions` frame arrives
- **AND** the late suggestions SHALL be discarded

#### Scenario: Late suggestions do not move the new turn's cursor

- **GIVEN** a queued second send has started streaming run 2
- **WHEN** suggestions for run 1 arrive
- **THEN** the stream session's run id and replay cursor SHALL still belong to run 2

#### Scenario: New send cancels the listener

- **GIVEN** a listener waiting for suggestions for run 1
- **WHEN** the user sends a new message
- **THEN** the listener's subscription SHALL be cancelled

#### Scenario: Suggestions for the last message are shown

- **WHEN** `SuggestionsEvent` arrives for the last assistant message with no send pending
- **THEN** `ChatState.suggestions` SHALL equal its prompts

#### Scenario: Sending clears chips

- **GIVEN** visible suggestions
- **WHEN** the user sends any message
- **THEN** `ChatState.suggestions` SHALL be empty

### Requirement: Suggestions are not restored from history

Suggestions SHALL NOT be stored on messages or returned by the conversation history endpoints. Opening or reloading a conversation from history SHALL show no suggestion chips and SHALL start no suggestion listener. Chips SHALL appear only for a live run and for a run the client reconnects to, because the listener starts when that run's `done` frame is read.

#### Scenario: Reopened conversation shows no chips

- **GIVEN** a conversation whose last turn produced suggestions an hour ago
- **WHEN** the user opens it from the conversation list
- **THEN** `ChatState.suggestions` SHALL be empty AND no replay request SHALL be made for the old run

#### Scenario: History carries no suggestions

- **WHEN** a client fetches `GET /api/conversations/{id}`
- **THEN** the response SHALL contain no suggestions field

#### Scenario: Reconnected run still shows chips

- **GIVEN** the client lost its POST stream and replays the run through `_replayRun`
- **WHEN** the replay reads `done` with a `message_id` and the run later persists `suggestions`
- **THEN** the chips SHALL be shown

### Requirement: SuggestionChips element

`app/packages/assistant_ui` SHALL provide a pure `SuggestionChips(prompts, onSelected, enabled)` that renders one chip per prompt, keyed `suggestion_chip_<index>`, in a wrapping row inside a semantics container labelled "Suggested follow-ups". Each chip SHALL expose button semantics labelled with its prompt and a target of at least 48×48 dp. Tapping a chip SHALL call `onSelected(prompt)` unless `enabled` is false. An empty list SHALL render nothing. The widget SHALL have a widgetbook use-case and a golden test, and SHALL appear in `kAssistantUiElements`.

#### Scenario: Tap emits the prompt

- **WHEN** `suggestion_chip_1` is tapped
- **THEN** `onSelected` SHALL fire with the second prompt

#### Scenario: Disabled chips do not fire

- **GIVEN** `enabled: false`
- **WHEN** a chip is tapped
- **THEN** `onSelected` SHALL NOT fire

### Requirement: Tapping a chip sends it as a user message

The chat screen SHALL render `SuggestionChips` in `ChatThreadLayout.aboveComposer` while `ChatState.suggestions` is non-empty and no turn is sending. Selecting a chip SHALL call `ChatNotifier.sendMessage(prompt)`.

#### Scenario: Chip tap sends

- **GIVEN** chips "Show an example" and "Explain step two" above the composer
- **WHEN** the user taps "Show an example"
- **THEN** a user message "Show an example" SHALL be sent AND the chips SHALL disappear
