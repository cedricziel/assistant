# Spec Delta

## Purpose

Lets space admins tune a space's AI behavior within the limits their org allows: model, reasoning, web search, images, voice.

## ADDED Requirements

### Requirement: Space AI settings endpoints

The server SHALL expose the following endpoints for the space selected through the usual space context. They SHALL require space-admin access; org admins have it for every space of their org.

| Endpoint                                 | Behavior                                                                                                         |
| ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| `GET /api/spaces/current/settings/ai`    | Returns the allowlisted keys with their effective value, source, inherited value and lock state, plus a version. |
| `PATCH /api/spaces/current/settings/ai`  | Takes a JSON merge patch of space overrides. Requires `If-Match`.                                                |
| `DELETE /api/spaces/current/settings/ai` | Clears every override of the space.                                                                              |

Version conflicts SHALL return 409. Keys outside the allowlist, locked keys and validation or build failures SHALL return 422, with errors per key path.

#### Scenario: Space admin enables web search

- **WHEN** a space admin patches `llm.openrouter.web_search.enabled = true`
- **THEN** that space's next turn uses the web plugin, and other spaces are unchanged

#### Scenario: Member

- **WHEN** a space member without admin rights patches the settings
- **THEN** the server returns 403

### Requirement: Space overrides are stored with the space

Space overrides SHALL be stored in that space's database. They SHALL be removed when the space is deleted, and included when the space is backed up.

#### Scenario: Space deleted

- **WHEN** a space is deleted
- **THEN** its overrides are gone, and no other space is affected
