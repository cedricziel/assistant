# Spec Delta

## Purpose

Stores provider API keys set from the app encrypted at rest. The API can set and remove them but never read them back.

## ADDED Requirements

### Requirement: Sealed named secrets

Secrets set through the API SHALL be sealed with the install's secrets key, using an AAD (the context string bound into each encryption) that binds the org, the scope and the secret name. The name SHALL be one of:

- `llm.api_key`
- `llm.embeddings.api_key`
- `transcription.api_key`
- `tts.api_key`
- `images.api_key`
- `permissions.decision_model.api_key`

Ciphertext SHALL be stored in org.db. Plaintext SHALL never be written to disk, logs, traces or API responses.

#### Scenario: Set the OpenRouter key

- **WHEN** an org admin sets `llm.api_key`
- **THEN** org.db holds only ciphertext, nonce and key ID, and later reads report `{"set": true, "updated_at", "updated_by"}` with no key material

#### Scenario: Ciphertext moved to another org

- **WHEN** a sealed row is copied to another org's ID
- **THEN** opening it fails, because the AAD does not match

### Requirement: Secret precedence

A stored secret SHALL take precedence over the key at the same path in `org.toml`. That file key in turn takes precedence over reused and environment fallbacks (see `openrouter-key-reuse`). Removing the stored secret SHALL restore the file or fallback value.

#### Scenario: Remove a stored key

- **WHEN** an admin removes the stored `tts.api_key` and `org.toml` has none
- **THEN** TTS resolves its key through the `[llm]` reuse rule or the environment variable

### Requirement: Secrets survive backups and key rotation

`assistant backup` SHALL include the sealed rows. When the secrets key changes, rows sealed under an unknown key ID SHALL be reported as unreadable by `doctor` and by the settings read. Such a row SHALL NOT silently fall back to another key.

#### Scenario: Unknown key ID

- **WHEN** a row's key ID does not match the loaded secrets key
- **THEN** the settings read shows the secret as `unreadable`, and the provider build fails with an error naming the secret
