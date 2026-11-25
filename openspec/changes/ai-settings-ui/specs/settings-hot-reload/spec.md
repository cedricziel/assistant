# Spec Delta

## Purpose

Makes saved settings take effect without a restart, without breaking running turns, and without ever running a configuration that fails to build. Voice providers become per space.

## ADDED Requirements

### Requirement: Build before commit

Before an org or space settings write is stored, the system SHALL build every provider the write affects from the would-be effective configuration: chat LLM, embeddings, transcription, TTS, image generation and decision model. Builds SHALL include the catalog checks each provider performs, such as TTS voice and speed. When any build fails, the write SHALL be rejected with the provider and error, and nothing SHALL change. A `validate` request SHALL run the same checks without storing.

#### Scenario: Invalid voice

- **WHEN** an org admin sets `tts.voice` to a voice the configured model does not offer
- **THEN** the write is rejected with the TTS error, and the running configuration is unchanged

### Requirement: Atomic swap without interrupting turns

After a successful write, new turns SHALL use the new configuration: after an org write for every space of the org, after a space write for that space. Turns already running SHALL finish on the configuration they started with. Messenger connections and space databases SHALL NOT be restarted by a settings change.

#### Scenario: Save during a turn

- **WHEN** a turn is streaming and the org admin changes the model
- **THEN** the turn finishes on the old model, and the next turn uses the new one

### Requirement: Per-space voice providers

TTS and transcription providers SHALL be resolved from each space's effective configuration. They SHALL be used by:

- on-demand synthesis
- the `voice_response` tool
- voice send
- messenger voice transcription
- `/api/capabilities`

A space whose effective configuration has no `[tts]` or `[transcription]` SHALL have no such provider, even when other orgs or spaces do.

#### Scenario: Two orgs, different voices

- **WHEN** org A configures OpenRouter TTS and org B configures no TTS
- **THEN** spaces in A can synthesize, and spaces in B report `voice_receive = false`

### Requirement: Reload command

`assistant reload` (CLI) and a reload request from an org admin SHALL re-read `org.toml` and apply it through the same build-before-commit path.

#### Scenario: Broken org.toml

- **WHEN** an operator saves an `org.toml` that fails to parse and requests a reload
- **THEN** the reload fails with the parse error, and the running configuration stays in place
