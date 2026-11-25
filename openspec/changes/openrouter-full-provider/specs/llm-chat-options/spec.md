# Spec Delta

## Purpose

Gives callers of any LLM provider per-call control over tool choice, parallel tool calls, response format and output modalities. Each provider declares which of these it supports natively.

## ADDED Requirements

### Requirement: Per-call chat options

A chat call SHALL accept options with the following fields:

- `tool_choice`: `auto` (default), `none`, `required`, or a specific tool name.
- `parallel_tool_calls`: `true` or `false`; when unset, the provider default applies.
- `response_format`: text (default), or a JSON schema with a name and a strict flag.
- `output`: any of text (always on), image and audio. Audio carries a voice.

A call with default options SHALL behave exactly as calls do today.

#### Scenario: Default options

- **WHEN** a caller passes default options
- **THEN** the provider request equals the request made today without options

### Requirement: Declared support

Each provider SHALL report whether it natively supports `tool_choice`, `parallel_tool_calls`, JSON-schema response format, image output and audio output. For OpenRouter, support SHALL come from the model catalog: `supported_parameters` values `tool_choice`, `parallel_tool_calls`, `structured_outputs` and `response_format`, plus the output modalities. For Anthropic, OpenAI, Moonshot and Ollama, support SHALL reflect what their APIs accept.

#### Scenario: OpenRouter model without structured outputs

- **WHEN** the catalog lists neither `structured_outputs` nor `response_format` for the model
- **THEN** the provider reports no native JSON-schema support

### Requirement: Native tool choice

When a provider supports `tool_choice` natively, it SHALL map the option to its wire format:

- OpenAI and OpenRouter: `"auto"`, `"none"`, `"required"`, or `{"type": "function", "function": {"name"}}`.
- Anthropic: `{"type": "auto" | "none" | "any" | "tool", "name"?}`.

`parallel_tool_calls = false` SHALL be sent as `parallel_tool_calls: false` on OpenAI-style APIs and as `disable_parallel_tool_use: true` on Anthropic. When OpenRouter receives `tool_choice` or `response_format`, it SHALL also send `provider.require_parameters: true`, so routing only selects endpoints that honor them.

#### Scenario: Forced tool on Anthropic

- **WHEN** `tool_choice` names `submit_decision` and the provider is Anthropic
- **THEN** the request carries `"tool_choice": {"type": "tool", "name": "submit_decision"}`

#### Scenario: Specific tool on OpenRouter

- **WHEN** `tool_choice` names `submit_decision` and the provider is OpenRouter
- **THEN** the request carries the function-form `tool_choice` and `provider.require_parameters: true`

### Requirement: Tool choice without native support

When a provider lacks native `tool_choice` and the option is `required` or a specific tool, the provider SHALL add an instruction to the system prompt naming the required tool. When the model answers without calling it, the provider SHALL retry once with the instruction repeated as the last user message. If the model still does not call it, the call SHALL fail with an error naming the tool. `none` SHALL be honored by sending no tools.

#### Scenario: Ollama ignores the forced tool

- **WHEN** an Ollama model answers in text although `tool_choice = required`
- **THEN** the provider retries once and then fails with an error naming the required tool

### Requirement: Parallel tool calls setting

`[llm] parallel_tool_calls` SHALL accept `true` or `false`; when it is unset, the provider default applies. The orchestrator SHALL pass the setting as `parallel_tool_calls` on every agent-loop call.

#### Scenario: Sequential tool use

- **WHEN** `[llm] parallel_tool_calls = false` is set
- **THEN** every agent-loop request disables parallel tool calls
