## Purpose

Durable, versioned, space-scoped artifacts that Personas and people create and
revise, with optimistic concurrency, Persona tools, REST and live updates.

## ADDED Requirements

### Requirement: Artifacts are space-scoped versioned objects

An artifact SHALL belong to exactly one space and have a UUID, a `kind`
(`html`, `slides`, `markdown`, `code`, `data`), a title, an owner user, and a
`latest_version`. It SHALL exist independently of any conversation; deleting a
conversation MUST NOT delete artifacts it created. Every content write SHALL
create a new immutable version numbered `latest_version + 1`, recording the
author (`user` or `persona`), the user the Persona acted for, and the
originating conversation and message when there is one.

#### Scenario: Create

- **WHEN** a space Member sends `POST /api/artifacts` with kind `html`, a title
  and content
- **THEN** the response is `201` with an `ArtifactDetail` at version 1 owned by
  that user

#### Scenario: Conversation deleted

- **GIVEN** conversation `c1` created artifact `a1`
- **WHEN** `c1` is deleted
- **THEN** `a1` and all its versions remain readable

#### Scenario: Content too large

- **WHEN** a write carries content larger than 5 MiB
- **THEN** the response is `413` and no version is created

### Requirement: Writes are rejected when based on a stale version

Every content write SHALL carry `base_version`. The store SHALL create the new
version only if `base_version` equals `latest_version`, atomically. Otherwise
the API SHALL return `409` with an `ArtifactConflict` body containing the latest
version number and content, and the tools SHALL return a tool error containing
the same.

#### Scenario: Two writers from the same base

- **GIVEN** `a1` is at version 3
- **WHEN** a user saves with `base_version: 3` and then the Persona calls
  `artifact-update` with `base_version: 3`
- **THEN** the user's save creates version 4
- **AND** the Persona's call fails with the content of version 4 and creates
  nothing

#### Scenario: Restore

- **GIVEN** `a1` is at version 5
- **WHEN** an editor sends `POST /api/artifacts/a1/restore` with
  `{version: 2, base_version: 5}`
- **THEN** version 6 is created with version 2's content and
  `restored_from: 2`, and versions 1–5 are unchanged

### Requirement: Personas manage artifacts through explicit tools

The tool executor SHALL provide `artifact-create`, `artifact-read`,
`artifact-update`, `artifact-edit`, `artifact-list` and `artifact-comment`. A
tool call SHALL be authorized as the user running the turn, using the same role
resolution as the API. The runtime MUST NOT create artifacts from heuristics
over model output. No tool SHALL grant access or create share links.

#### Scenario: Targeted edit

- **GIVEN** `a1` at version 2 contains `<h1>Q3</h1>` exactly once
- **WHEN** the Persona calls `artifact-edit` with `base_version: 2` and
  `edits: [{find: "<h1>Q3</h1>", replace: "<h1>Q4</h1>"}]`
- **THEN** version 3 is created with only that change

#### Scenario: Ambiguous find

- **WHEN** an `edits` entry's `find` matches zero times or more than once
- **THEN** the tool returns an error naming that entry and creates no version

#### Scenario: Persona limited by the user's role

- **GIVEN** user `u1` is a space `Viewer` with no grant on `a1`
- **WHEN** a turn run by `u1` calls `artifact-update` on `a1`
- **THEN** the tool returns an error and creates no version

### Requirement: Artifact changes stream to viewers and chat

`GET /api/artifacts/{id}/events` SHALL stream `snapshot`, `version_created`,
`comment_created`, `comment_updated` and `deleted` events to anyone with `view`.
When an artifact tool succeeds during a turn, the orchestrator SHALL emit an
`artifact` event `{artifact_id, version, kind, title}` on the chat stream and
persist it to the conversation event log.

#### Scenario: Viewer sees a new version

- **GIVEN** a user has the events stream for `a1` open
- **WHEN** the Persona creates version 4 in another conversation
- **THEN** the stream delivers `version_created` with version 4

### Requirement: Artifacts are reachable over MCP and A2A

The MCP server SHALL list each visible artifact as resource `artifacts://{id}`
returning its latest content. A2A task responses SHALL include an `Artifact`
for every artifact changed in the task, with the content in `parts[0]`, and
`metadata` holding `kind`, `version` and `url`.

#### Scenario: MCP read

- **WHEN** an MCP client calls `resources/read` for `artifacts://a1`
- **THEN** it receives the latest content of `a1` with its media type
