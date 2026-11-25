# Spec Delta

## Purpose

Defines how the assistant talks to OpenRouter's Chat Completions API. OpenRouter-specific request fields, response fields and errors must survive the round trip intact.

## ADDED Requirements

### Requirement: OpenRouter-native request and response handling

The OpenRouter chat provider SHALL build requests and parse responses as OpenRouter JSON. Fields that a generic OpenAI client would drop SHALL be preserved: `reasoning`, `reasoning_details`, `plugins`, `provider`, `models`, `session_id`, `user`, `annotations`, `usage.cost` and `prompt_tokens_details`. Streaming SHALL ignore SSE comment lines, including `: OPENROUTER PROCESSING`, and SHALL end at `[DONE]`.

#### Scenario: Keep-alive comments during streaming

- **WHEN** the SSE stream contains `: OPENROUTER PROCESSING` lines between data events
- **THEN** they produce no chunks and no error, and the final response is unchanged

#### Scenario: Served model is reported

- **WHEN** the response `model` differs from the requested model
- **THEN** the response metadata reports the served model

### Requirement: OpenRouter error handling

Non-2xx responses SHALL be parsed from `{"error": {"code", "message", "metadata"}}`, and the returned error SHALL include the status, the message and, when present, `metadata.error_type`. Status codes 408, 429, 502, 503, 524 and 529 SHALL be retried within the existing bounded retry policy. When the response has a `Retry-After` header, the retry SHALL wait at least that long. Status codes 400, 401, 402, 403, 404 and 413 SHALL NOT be retried. For 403 moderation errors, the error SHALL include `metadata.reasons`. A stream chunk with a top-level `error` or `finish_reason: "error"` SHALL fail the call with that error, even though the HTTP status was 200. A 200 non-streaming body with `error` and no `choices` SHALL also fail.

#### Scenario: Rate limit with Retry-After

- **WHEN** OpenRouter answers 429 with `Retry-After: 2`, then 200
- **THEN** the provider waits at least 2 seconds, retries once and returns the 200 result

#### Scenario: Insufficient credits

- **WHEN** OpenRouter answers 402
- **THEN** the call fails at once, and the error says the OpenRouter account lacks credits

#### Scenario: Mid-stream failure

- **WHEN** a stream chunk carries `error: {code: 502, message: "Provider disconnected"}` with `finish_reason: "error"`
- **THEN** the call fails with that message, and no partial answer is returned as final

### Requirement: Attribution headers

When `[llm.openrouter] referer` is set, the provider SHALL send `HTTP-Referer`. When `title` is set, it SHALL send `X-OpenRouter-Title`. When `categories` is set (at most two), it SHALL send `X-OpenRouter-Categories`. Config with more than two categories SHALL be rejected at load time.

#### Scenario: Title header

- **WHEN** `[llm.openrouter] title = "Assistant"` is set
- **THEN** every chat request carries `X-OpenRouter-Title: Assistant`

### Requirement: Provider routing and model fallbacks

`[llm.openrouter.provider]` SHALL accept `order`, `only`, `ignore`, `allow_fallbacks`, `require_parameters`, `data_collection`, `zdr`, `sort`, `quantizations` and `max_price`, and SHALL send them as the request's `provider` object. `[llm.openrouter] fallback_models` SHALL be sent as `models`, with the primary model first. Unknown keys under `[llm.openrouter.provider]` SHALL be rejected at load time.

#### Scenario: Data collection denied

- **WHEN** `[llm.openrouter.provider] data_collection = "deny"` and `zdr = true` are set
- **THEN** every chat request includes `"provider": {"data_collection": "deny", "zdr": true}`

#### Scenario: Fallback models

- **WHEN** `model = "anthropic/claude-sonnet-5.5"` and `fallback_models = ["openai/gpt-5.6"]` are set
- **THEN** requests include `"models": ["anthropic/claude-sonnet-5.5", "openai/gpt-5.6"]`

### Requirement: Conversation and user identity

When a chat call runs on behalf of a conversation, the request SHALL include `session_id` set to the conversation ID and `user` set to a stable, non-reversible hash of the org-scoped user ID. Raw user identifiers and email addresses SHALL NOT be sent. Calls made outside a conversation, such as background jobs, SHALL omit both fields.

#### Scenario: Turn in a conversation

- **WHEN** user U sends a message in conversation C
- **THEN** every chat request in that turn carries `session_id = C` and the same `user` hash, and that hash is not U's ID or email
