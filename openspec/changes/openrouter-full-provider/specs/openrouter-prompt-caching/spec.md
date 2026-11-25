# Spec Delta

## Purpose

Lowers cost and latency on OpenRouter by caching the stable prefix of each request (system prompt and tool definitions) on upstreams that support caching.

## ADDED Requirements

### Requirement: Cache breakpoints

`[llm.openrouter.cache]` SHALL accept `enabled` (default `true`) and `ttl` (`5m` or `1h`, default `5m`). When caching is enabled and the model's author requires explicit breakpoints (Anthropic, Google, Qwen), the system prompt text part SHALL carry `cache_control: {"type": "ephemeral"}`, plus `ttl` when it is `1h`. For Anthropic models, the last user message part SHALL also carry a breakpoint. For other models, no `cache_control` SHALL be sent; those upstreams cache automatically.

#### Scenario: Anthropic model

- **WHEN** the model is `anthropic/claude-sonnet-5.5` and caching is enabled
- **THEN** the system prompt part and the last user part carry `cache_control: {"type": "ephemeral"}`

#### Scenario: OpenAI model

- **WHEN** the model is `openai/gpt-5.6`
- **THEN** no `cache_control` appears in the request

#### Scenario: Caching disabled

- **WHEN** `[llm.openrouter.cache] enabled = false` is set
- **THEN** no request carries `cache_control`

### Requirement: Sticky routing for cache hits

Requests in the same conversation SHALL carry the same `session_id`, so OpenRouter routes them to the same upstream and the cache can hit (see `openrouter-chat`).

#### Scenario: Consecutive turns

- **WHEN** two turns happen in one conversation
- **THEN** both requests carry the same `session_id`
