# Spec Delta

## Purpose

Lets one OpenRouter API key, configured once under `[llm]`, authenticate every OpenRouter-backed feature in an org. `doctor` reports whether that key works.

## ADDED Requirements

### Requirement: Key resolution for OpenRouter audio and embedding providers

For the OpenRouter transcription, TTS and embedding providers, the API key SHALL be resolved in this order:

1. The provider section's own `api_key` (`[transcription]`, `[tts]` or `[llm.embeddings]`).
2. `[llm] api_key`, only when `[llm] provider = "openrouter"`.
3. The `OPENROUTER_API_KEY` environment variable.

When none is set, building the provider SHALL fail with an error naming all three sources.

#### Scenario: Section key wins

- **WHEN** `[tts] api_key = "k1"` is set, and `[llm] provider = "openrouter"` has `api_key = "k2"`
- **THEN** TTS authenticates with `k1`

#### Scenario: Chat key is reused

- **WHEN** `[transcription] provider = "openrouter"` has no key, and `[llm] provider = "openrouter"` has `api_key = "k2"`
- **THEN** transcription authenticates with `k2`

#### Scenario: Non-OpenRouter chat key is never reused

- **WHEN** `[llm] provider = "anthropic"` has a key, `[tts] provider = "openrouter"` has none, and `OPENROUTER_API_KEY` is unset
- **THEN** building TTS fails, and the Anthropic key is never sent to OpenRouter

### Requirement: Key resolution for the decision model

`[permissions.decision_model]` SHALL accept an optional `api_key`. The key SHALL be resolved in this order:

1. That `api_key`.
2. `[llm] api_key`, when `[llm] provider = "openrouter"`.
3. The variable named by `api_key_env`.

When no key resolves, the decision model SHALL be left out of the policy, an error SHALL be logged, and undecided calls SHALL yield `Ask`.

#### Scenario: Reuses the chat key

- **WHEN** the decision model is enabled without `api_key`, `[llm] provider = "openrouter"` has `api_key = "k2"`, and the env var is unset
- **THEN** decision requests authenticate with `k2`

#### Scenario: No key

- **WHEN** no source resolves
- **THEN** the policy is built without the decision model, and undecided calls yield `Ask`

### Requirement: Key health in doctor

`assistant doctor` SHALL list each OpenRouter-backed feature (chat, transcription, TTS, embeddings, decision model) with the source its key came from: section, `[llm]` or env. It SHALL never print key material. For each distinct key, it SHALL call `GET {base_url}/key` once and report one of three outcomes:

- valid: with `limit_remaining`, or "no limit" when there is none, and whether it is a free-tier key.
- invalid: on 401.
- unreachable: on any other failure.

#### Scenario: Shared key

- **WHEN** chat, TTS and transcription all resolve to the same `[llm]` key
- **THEN** doctor shows `[llm]` as the source for each, and makes one `/key` request

#### Scenario: Revoked key

- **WHEN** `/key` answers 401
- **THEN** doctor marks every feature using that key as failing, with the message "OpenRouter key invalid or revoked"
