# Spec Delta

## Purpose

Records the full token and cost picture of every LLM call (cached input, cache writes, reasoning tokens and cost) in response metadata and telemetry, so spend can be observed per provider and model.

## ADDED Requirements

### Requirement: Extended usage in response metadata

LLM response metadata SHALL carry optional `cached_input_tokens`, `cache_write_input_tokens`, `reasoning_tokens` and `cost_usd` fields alongside the input and output token counts. The input count SHALL include cache reads and writes, and the output count SHALL include reasoning tokens, as the GenAI semantic conventions require. Providers whose API reports them separately (Anthropic) SHALL add them up. Every provider SHALL fill in the fields its API reports and leave the rest unset. OpenRouter reports all four: `prompt_tokens_details.cached_tokens`, `prompt_tokens_details.cache_write_tokens`, `completion_tokens_details.reasoning_tokens` and `usage.cost`. For streamed responses, these values SHALL come from the final usage chunk.

#### Scenario: OpenRouter usage

- **WHEN** a response reports `cached_tokens: 10318`, `reasoning_tokens: 40` and `cost: 0.0021`
- **THEN** the metadata carries `cached_input_tokens = 10318`, `reasoning_tokens = 40` and `cost_usd = 0.0021`

#### Scenario: Anthropic cache tokens

- **WHEN** Anthropic reports `input_tokens: 100`, `cache_read_input_tokens: 900` and `cache_creation_input_tokens: 50`
- **THEN** the metadata carries `input_tokens = 1050`, `cached_input_tokens = 900` and `cache_write_input_tokens = 50`

#### Scenario: Provider without cost reporting

- **WHEN** the Ollama provider answers
- **THEN** `cost_usd` is unset, and no zero value is invented

### Requirement: Usage in telemetry

The LLM call span SHALL record `gen_ai.usage.cache_read.input_tokens`, `gen_ai.usage.cache_creation.input_tokens` and `gen_ai.usage.reasoning.output_tokens` when they are known. It SHALL record cost under `assistant.llm.cost_usd`. The token-usage metric SHALL keep only the `input` and `output` token types the conventions define; cached and reasoning counts are already part of those totals.

#### Scenario: Span attributes

- **WHEN** an OpenRouter call reports cached tokens and cost
- **THEN** its span carries the cache-read attribute and `assistant.llm.cost_usd`
