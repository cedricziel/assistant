## MODIFIED Requirements

### Requirement: Owners answer through one respond endpoint

The server SHALL expose
`POST /api/conversations/{conversation_id}/turns/{turn_id}/interactions/{interaction_id}/respond`
(`operationId: respond_to_interaction`) taking a `RespondToInteractionRequest`
tagged by `kind` (`tool_approval`, `question`, `surface_action`, or `decline`),
with unknown fields rejected. The `surface_action` variant SHALL carry `action`
(an A2UI v1.0 action object) and an optional `data_model` object, and SHALL be
accepted only for interactions of kind `surface_action`. It SHALL validate the
answer through the handler registered for the interaction's kind. It SHALL
return `200` with an `InteractionDetail`. It SHALL require `Access::Write` in
the selected space, the `conversations:write` scope, ownership of the
conversation, and that the interaction's recorded `user_id`, `conversation_id`
and `run_id` match the caller and the path. Every error, including conversation
lookup failures, SHALL use the `{"error": …}` envelope.

#### Scenario: Owner answers

- **WHEN** the turn owner posts a valid answer to a pending interaction
- **THEN** the response SHALL be `200` with `status = resolved`
- **AND** the waiting turn SHALL resume with that answer

#### Scenario: Another member cannot answer

- **WHEN** a different member of the same space posts to the endpoint
- **THEN** the response SHALL be `404` with an `{"error": …}` body AND the
  interaction SHALL stay pending

#### Scenario: Viewer is refused

- **WHEN** a caller with only viewer access posts to the endpoint
- **THEN** the response SHALL be `403`

#### Scenario: Wrong run in the path

- **WHEN** the `turn_id` in the path differs from the interaction's run
- **THEN** the response SHALL be `404`

#### Scenario: Answer does not fit the kind

- **WHEN** a `question` answer names a `question_id` or option id that the
  request did not contain, or a `tool_approval` decision is not among the
  offered `options`
- **THEN** the response SHALL be `400`

#### Scenario: Oversized answer

- **WHEN** a free-text answer is longer than 4000 characters
- **THEN** the response SHALL be `400`

#### Scenario: Oversized body

- **WHEN** the request body is larger than 64 KiB
- **THEN** the response SHALL be `413`

#### Scenario: Surface action answer

- **WHEN** the owner posts `{kind: "surface_action", action: {name: "submit_form", …}}`
  to a pending `surface_action` interaction whose surface declares `submit_form`
- **THEN** the response SHALL be `200` with `status = resolved`

#### Scenario: Surface action body for another kind

- **WHEN** the owner posts a `surface_action` body to a `tool_approval` interaction
- **THEN** the response SHALL be `400`
