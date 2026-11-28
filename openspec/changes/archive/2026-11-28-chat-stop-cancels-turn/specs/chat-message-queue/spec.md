## MODIFIED Requirements

### Requirement: Stop cancels only the current in-flight response

When the user taps Stop, the client SHALL cancel the current turn on the server (see `chat-turn-stop`). Queued messages SHALL be preserved. Draining SHALL resume as soon as the stopped turn has reconciled, either from its terminal frame or by local detach, so the next queued message never overlaps the cancelled turn. The queue SHALL NOT stay blocked while the server is silent after a stop.

#### Scenario: Stop does not clear the pending queue

- **WHEN** the user has queued messages and taps Stop
- **THEN** the current turn is cancelled on the server
- **AND** the pending queue is unchanged
- **AND** the first queued message is sent right after the stopped turn reconciles

#### Scenario: Queue resumes even when the server is silent

- **WHEN** the user taps Stop during a long-running tool call that emits no events
- **AND** the server's cancelled `agent_error` frame arrives
- **THEN** the next queued message SHALL be sent without waiting for any further event from the cancelled turn
