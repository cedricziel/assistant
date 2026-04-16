## ADDED Requirements

### Requirement: Client parses transcript SSE event

The Flutter client SHALL recognise `event: transcript` frames emitted by `POST /api/conversations/{id}/voice` and surface them as a typed `TranscriptEvent` in the `StreamEvent` hierarchy.

#### Scenario: Transcript event is parsed

- **WHEN** the SSE parser receives a frame with `event: transcript` and `data: {"role":"user","content":"<text>"}`
- **THEN** it yields a `TranscriptEvent` whose `transcript` field equals `<text>`

#### Scenario: Unknown event types are silently ignored

- **WHEN** the SSE parser receives a frame with an unrecognised event type
- **THEN** no `StreamEvent` is yielded and parsing continues normally

### Requirement: Voice message user bubble shows spoken text

After a voice recording is uploaded, the user message bubble SHALL display the transcribed spoken text received via `TranscriptEvent`, not a placeholder or the assistant's reply.

#### Scenario: Transcript replaces placeholder

- **WHEN** the client receives a `TranscriptEvent` during a voice stream
- **THEN** the user message bubble content is updated to the transcript text

#### Scenario: User bubble is not overwritten by DoneEvent

- **WHEN** the client receives a `DoneEvent` at the end of a voice stream
- **THEN** the user message bubble content is NOT changed (it already shows the transcript)
- **AND** the assistant message is finalised with `DoneEvent.content`

#### Scenario: Placeholder shown before transcript arrives

- **WHEN** a voice recording has been uploaded but no `TranscriptEvent` has yet been received
- **THEN** the user message bubble displays the initial `🎤 Voice message` placeholder
