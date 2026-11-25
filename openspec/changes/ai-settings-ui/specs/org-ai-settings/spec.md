# Spec Delta

## Purpose

Gives org admins an HTTP API to read and change every AI setting of their org for every provider, plus the lookups a settings UI needs: models, voices, key health and voice preview.

## ADDED Requirements

### Requirement: Org AI settings endpoints

The server SHALL expose the following endpoints. All of them SHALL require an org admin of `{org_id}`, or a token with the `org:manage` scope.

| Endpoint                                          | Behavior                                                                                                   |
| ------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `GET /api/orgs/{org_id}/settings/ai`              | Returns the effective values with sources, the org overrides, the locks, the secret status, and a version. |
| `PATCH /api/orgs/{org_id}/settings/ai`            | Takes a JSON merge patch for the overrides and an optional `locks` list. Requires `If-Match: <version>`.   |
| `DELETE /api/orgs/{org_id}/settings/ai/{section}` | Clears the overrides of one section.                                                                       |
| `POST /api/orgs/{org_id}/settings/ai/validate`    | Runs the same checks as `PATCH` without storing.                                                           |
| `POST /api/orgs/{org_id}/settings/reload`         | Re-reads `org.toml` and applies it.                                                                        |

A `PATCH` with a stale version SHALL fail with 409 and the current version. Validation failures SHALL return 422 with the list of errors (path and message). Users who are not org admins SHALL get 403.

#### Scenario: Concurrent edits

- **WHEN** two admins load version 7 and both patch it
- **THEN** the first patch succeeds and creates version 8, and the second gets 409

#### Scenario: Member tries to read

- **WHEN** a space member calls `GET /api/orgs/{org_id}/settings/ai`
- **THEN** the server returns 403

### Requirement: Secret endpoints

`PUT /api/orgs/{org_id}/settings/secrets/{name}` with body `{"value"}` SHALL store a sealed secret. `DELETE` on the same path SHALL remove it. Both SHALL respond with secret status only and SHALL go through build-before-commit. No endpoint SHALL return secret values.

#### Scenario: Wrong key

- **WHEN** an admin stores an OpenRouter key that `/key` rejects with 401
- **THEN** the write is rejected with 422 "OpenRouter key invalid or revoked", and the previous secret stays

### Requirement: Provider lookups

The server SHALL expose the following lookups, for org admins and for space admins of the org's spaces. Each SHALL be served from the provider catalogs and cached for 10 minutes.

| Endpoint                                                                                          | Returns                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GET /api/orgs/{org_id}/settings/ai/models?kind=chat\|embedding\|transcription\|speech\|image&q=` | `id`, `name`, `context_length`, `pricing`, `supported_parameters`, `input_modalities`, `output_modalities`. For providers without a catalog, configured or known model IDs only. |
| `GET /api/orgs/{org_id}/settings/ai/voices?model=`                                                | The model's voices, formats and speed range.                                                                                                                                     |
| `GET /api/orgs/{org_id}/settings/ai/key-health`                                                   | The same report as `doctor`, without key material.                                                                                                                               |

`POST /api/orgs/{org_id}/settings/ai/tts-preview` with `{text?, voice?, speed?, model?}` SHALL synthesize at most 200 characters with the would-be TTS configuration and return the audio. Admins SHALL be limited to 10 previews per minute.

#### Scenario: Speech model search

- **WHEN** an admin searches speech models for "voxtral"
- **THEN** the response lists matching OpenRouter speech models with their voices and formats

#### Scenario: Preview a voice before saving

- **WHEN** an admin previews `voice = "Kore"` on a Gemini TTS model
- **THEN** the audio is returned, and nothing is stored

### Requirement: OpenAPI coverage

Every endpoint in this capability SHALL be documented in the OpenAPI spec, including request and response schemas and every status code, and SHALL be present in the generated Flutter client.

#### Scenario: Spec drift

- **WHEN** the committed `openapi.json` differs from the code
- **THEN** the existing spec-drift test fails
