# Spec Delta

## Purpose

Defines how AI settings from defaults, `org.toml`, org overrides, space overrides and user preferences combine into one effective configuration, and how each value's origin is reported.

## ADDED Requirements

### Requirement: Layer order

The effective AI configuration of a space SHALL be computed by deep-merging these layers, from lowest to highest precedence:

1. built-in defaults
2. `org.toml`
3. org overrides
4. space overrides

Objects SHALL merge key by key. Arrays and scalars SHALL replace the lower value. A key set to `null` in an override SHALL remove the inherited value, so the default applies. User preferences SHALL NOT change the effective configuration. They are applied per turn (see `user-ai-preferences`).

The AI sections are:

- `[llm]`, including provider-specific subsections and `[llm.embeddings]`
- `[transcription]`
- `[tts]`
- `[images]`
- `[permissions.decision_model]`

#### Scenario: Space overrides the model

- **WHEN** `org.toml` sets `model = "anthropic/claude-sonnet-5.5"` and the space override sets `llm.model = "openai/gpt-5.6"`
- **THEN** that space runs `openai/gpt-5.6`, and other spaces run the org model

#### Scenario: Org override replaces a list

- **WHEN** `org.toml` sets `fallback_models = ["a/x"]` and the org override sets `["b/y", "c/z"]`
- **THEN** the effective list is `["b/y", "c/z"]`

### Requirement: Validation of the merged result

Every write SHALL be validated by deserializing the merged configuration with the same types and checks that `org.toml` uses, including cross-field rules. For example, reasoning `effort` together with `max_tokens` is rejected. A write that fails validation SHALL be rejected as a whole. Each error SHALL report the dotted key path and the message, and nothing SHALL be stored.

#### Scenario: Conflicting reasoning settings

- **WHEN** an org override sets `llm.openrouter.reasoning.max_tokens` while `org.toml` sets `effort`
- **THEN** the write is rejected with an error at `llm.openrouter.reasoning`

### Requirement: Space override allowlist and org locks

Space overrides SHALL be limited to these keys:

- `llm.model`
- `llm.openrouter.fallback_models`
- `llm.openrouter.reasoning.*`
- `llm.openrouter.web_search.*`
- `llm.openrouter.image_output`
- `llm.openrouter.audio_input.enabled`
- `llm.openrouter.audio_output.enabled`
- `llm.openrouter.audio_output.voice`
- `tts.voice`, `tts.format`, `tts.speed`
- `transcription.language`
- `images.enabled`, `images.model`

The org SHALL be able to lock any of these keys. A space write that touches a key outside the allowlist, or a locked key, SHALL be rejected with the key path. When a key is locked after a space already overrides it, the space override SHALL stop applying, but it SHALL stay stored and SHALL be reported as overridden by the lock.

#### Scenario: Locked model

- **WHEN** the org locks `llm.model` and a space admin tries to override it
- **THEN** the write is rejected with "llm.model is locked by the organization"

#### Scenario: Key outside the allowlist

- **WHEN** a space write sets `llm.provider`
- **THEN** it is rejected as not overridable at space level

### Requirement: Source reporting

Reading settings at any level SHALL return the effective value of every key, together with its source: `default`, `file`, `org`, `space` or `locked`. For org and space reads, it SHALL also return the value inherited from the layer below.

#### Scenario: Inherited value

- **WHEN** a space has no override for `tts.voice` and the org override sets `"nova"`
- **THEN** the space read shows `tts.voice = "nova"` with source `org`

### Requirement: File changes still apply

On startup and on reload, `org.toml` SHALL be re-read. Values set in the file SHALL apply wherever no higher layer overrides them.

#### Scenario: Operator edits org.toml

- **WHEN** an operator changes `org.toml` and triggers a reload
- **THEN** keys without org or space overrides take the new file values
