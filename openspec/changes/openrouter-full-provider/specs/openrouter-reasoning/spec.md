# Spec Delta

## Purpose

Exposes OpenRouter's unified reasoning controls. The model's reasoning is shown as thinking and kept across tool calls, as OpenRouter requires for continuity.

## ADDED Requirements

### Requirement: Reasoning configuration

`[llm.openrouter.reasoning]` SHALL accept the following, all sent as the request's `reasoning` object:

- `effort`: one of `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, `max`.
- `max_tokens`
- `exclude`
- `enabled`

Setting both `effort` and `max_tokens` SHALL be rejected at load time. When the catalog says the model does not support the configured effort, startup SHALL fail and name the efforts the model does support.

#### Scenario: Effort sent

- **WHEN** `[llm.openrouter.reasoning] effort = "high"` is set for a model that supports it
- **THEN** every chat request includes `"reasoning": {"effort": "high"}`

#### Scenario: Conflicting settings

- **WHEN** both `effort` and `max_tokens` are set
- **THEN** config loading fails with an error naming both keys

### Requirement: Reasoning output

Reasoning text from `reasoning_details` entries of type `reasoning.text` and `reasoning.summary` SHALL be streamed as thinking chunks. In non-streaming calls, it SHALL be returned as the response's thinking. Encrypted entries SHALL NOT be shown. When `exclude = true` is set, no thinking SHALL be emitted.

#### Scenario: Streamed thinking

- **WHEN** a stream delivers `delta.reasoning_details` with text "Checking the file first"
- **THEN** a thinking chunk with that text is emitted before any answer text

#### Scenario: Reasoning budget exhausted

- **WHEN** the response has `finish_reason: "length"` and empty content after reasoning
- **THEN** the call fails with an error saying the token budget was used up by reasoning, rather than returning an empty answer

### Requirement: Reasoning continuity across tool calls

When an assistant turn that requests tool calls carries `reasoning_details`, the system SHALL store those details unchanged with that assistant message, including encrypted entries and signatures. On every later request that includes that message, the system SHALL send the details back in their original order. This SHALL hold after the server restarts, because history is rebuilt from storage. Reasoning stored for one model SHALL NOT be sent when the conversation is continued with a different provider.

#### Scenario: Tool loop keeps reasoning

- **WHEN** the model returns a tool call with `reasoning_details` [A, B], and the tool result is sent back
- **THEN** the assistant message in the follow-up request includes `reasoning_details` [A, B] unchanged

#### Scenario: Survives restart

- **WHEN** the server restarts between a tool call and the next turn
- **THEN** the rebuilt history still carries the stored `reasoning_details` for that assistant message

#### Scenario: Provider switched

- **WHEN** an org changes `[llm] provider` from `openrouter` to `anthropic`
- **THEN** stored OpenRouter reasoning is not sent to Anthropic
