## Purpose

Comment threads on artifacts, anchored to text, and summoning the Persona to
act on a thread.

## ADDED Requirements

### Requirement: Comments are threads anchored to a version

A user with `comment` SHALL be able to start a thread on an artifact, optionally
anchored to a text-quote selector (`quote`, `prefix`, `suffix`) on a specific
version, and reply to existing threads. The thread's author or a user with
`edit` SHALL be able to resolve it. A thread whose anchor cannot be found in the
latest version SHALL be reported as `outdated` and stay open.

#### Scenario: Anchor survives an edit elsewhere

- **GIVEN** a thread anchored to "revenue grew 12%" on version 3
- **WHEN** version 4 changes a different paragraph
- **THEN** the thread is still anchored to that text in version 4

#### Scenario: Anchor removed

- **WHEN** version 5 deletes the anchored text
- **THEN** the thread is listed as `outdated`

#### Scenario: Viewer cannot comment

- **WHEN** a user with only `view` posts a comment
- **THEN** the response is `403`

### Requirement: Editors can summon the Persona on a thread

A user with `edit` SHALL be able to summon the space's Persona on a thread, by
mentioning `@<persona-slug>` in a comment or with
`POST /api/artifacts/{id}/summon`. The server SHALL start a turn, as that user,
in the artifact's collaboration conversation, with the thread, its anchor and
the latest version as context. The Persona's replies SHALL be recorded with
`author_kind = persona` and `on_behalf_of` the summoning user. Summons SHALL be
limited to 30 per artifact per hour; beyond that the API returns `429`.

#### Scenario: Summon edits and replies

- **GIVEN** a thread "make this chart a bar chart" on `a1` at version 2
- **WHEN** an editor summons the Persona on it
- **THEN** the Persona creates version 3 and replies on the thread

#### Scenario: Commenter cannot summon

- **WHEN** a user with `comment` mentions `@persona`
- **THEN** the comment is saved and no turn starts

#### Scenario: Collaboration conversation is not listed

- **WHEN** the first summon creates the collaboration conversation
- **THEN** it does not appear in `GET /api/conversations`
