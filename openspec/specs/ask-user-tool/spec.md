# ask-user-tool Specification

## Purpose

The `ask-user` tool, which lets an interactive turn put one to four
clarifying questions to its owner through the interaction channel, and the
`QuestionCard` that renders and answers them.

## Requirements

### Requirement: Interactive turns offer an ask-user tool

The runtime SHALL add an `ask-user` tool to the per-turn tool overlay when the
turn is interactive and an interaction broker is configured. It SHALL NOT be
offered to non-interactive turns. `ask-user` SHALL be on the built-in allow
list, so asking a question never needs approval.

#### Scenario: Web chat sees the tool

- **WHEN** a streaming web chat turn builds its tool specs
- **THEN** `ask-user` SHALL be among them

#### Scenario: Scheduled task does not

- **WHEN** a scheduler turn builds its tool specs
- **THEN** `ask-user` SHALL NOT be among them

### Requirement: ask-user accepts one to four questions

`ask-user` SHALL accept `questions` (1 to 4 items). Each item SHALL have a
`prompt` (non-empty, at most 500 characters), optional `options` (0 to 8 items
of `{label, description?}`, with `label` at most 100 and `description` at most
300 characters), `multiple` (default false) and `free_text` (default false;
forced true when there are no options). Invalid input SHALL return a
`ToolOutput::error` describing the problem and SHALL NOT create an interaction.
The server SHALL assign question and option ids.

#### Scenario: Too many questions

- **WHEN** the model calls `ask-user` with five questions
- **THEN** the tool SHALL return an error AND no interaction SHALL be created

#### Scenario: Prompt too long

- **WHEN** the model calls `ask-user` with a 600-character prompt
- **THEN** the tool SHALL return an error AND no interaction SHALL be created

#### Scenario: Free text only

- **WHEN** a question has no options
- **THEN** the interaction SHALL present it with `free_text = true`

### Requirement: ask-user waits for the owner's answer

`ask-user` SHALL create a `question` interaction and wait for it. On answer, it
SHALL return a JSON observation `{"answers": [...]}` with one object per
question, in request order, carrying `question` (the prompt), `selected` (the
chosen option labels) and `text` (any free text). On `decline`, it SHALL return `The user declined to
answer.`. On expiry, it SHALL return `No answer within <N> s.`. Decline and
expiry SHALL be successful tool outputs, not errors.

#### Scenario: Multi-question answer

- **WHEN** the owner answers two questions, one by option and one by free text
- **THEN** the tool result's `answers` list SHALL contain two objects, each
  naming its question prompt, one with `selected` and one with `text`

#### Scenario: Multiple selection constraint

- **WHEN** an answer selects two options for a question with `multiple = false`
- **THEN** the respond endpoint SHALL return `400`

### Requirement: QuestionCard renders questions inline and becomes a receipt

`assistant_ui` SHALL provide `QuestionCard`, a pure widget taking an
`InteractionRecord` and an `onAnswer` / `onDecline` callback pair. It SHALL
render each question with radio (single), checkbox (multiple) and/or text
input, label the card as coming from the assistant, and enable submit only when
every question has an answer. Once resolved, it SHALL render a read-only receipt
of the answers, or of "Declined", "Expired" or "Cancelled". It SHALL ship
widgetbook stories and goldens for single, multiple, free-text, multi-question,
submitting and each resolved state.

#### Scenario: Rendered on the ask-user tool call

- **WHEN** a `question` interaction arrives with the `tool_call_id` of an
  `ask-user` call
- **THEN** its QuestionCard SHALL render in that tool call's `ToolCallBody`
  footer

#### Scenario: Submit disabled until complete

- **WHEN** one of two questions is unanswered
- **THEN** the submit action SHALL be disabled

#### Scenario: Receipt

- **WHEN** the interaction resolves
- **THEN** the card SHALL show the chosen answers and SHALL NOT accept input
