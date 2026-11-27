## Purpose

Per-artifact access: roles, grants to individual users, revocable share links,
and the org policy for public links.

## ADDED Requirements

### Requirement: Effective role combines space role, ownership, grants and links

Artifact roles SHALL be ordered `view < comment < edit < manage`. A user's
effective role SHALL be the highest of: their space role (`Viewer→view`,
`Member→edit`, `SpaceAdmin` or `OrgAdmin→manage`), `manage` if they own the
artifact, their grant on the artifact, and the role of an `org` link presented
with the request. An org admin SHALL keep access to every space in their org
without a membership, exactly as the space extractor allows today, and therefore
holds `manage` on its artifacts. A request for an artifact the user cannot
`view` SHALL return `404`, not `403`.

An `org` link SHALL carry authority only while it is presented: `ArtifactCtx`
SHALL accept `X-Artifact-Link` or `?link=` for the artifact it names, validate
it for the caller's org, and use its role for that request alone.

#### Scenario: Grant to a non-member

- **GIVEN** user `u2` in the same org is not a member of space `eng`
- **WHEN** a `manage` user grants `u2` `comment` on artifact `a1` in `eng`
- **THEN** `u2` sees `a1` in `GET /api/artifacts/shared-with-me`, can read it
  and comment, and gets `404` for every other artifact in `eng`

#### Scenario: Grant cannot lower a space role

- **GIVEN** `u3` is a `Member` of the artifact's space
- **WHEN** `u3` is granted `view`
- **THEN** `u3` still has `edit`

#### Scenario: Org admin without a membership

- **GIVEN** `u4` is an org admin and not a member of space `eng`
- **WHEN** `u4` sends `GET /api/artifacts/a1` with that space's `X-Space-Id`
- **THEN** the response is `200` with `manage`

#### Scenario: Org link to a non-member

- **GIVEN** `u5` belongs to the artifact's org, is not a space member and holds
  no grant
- **WHEN** `u5` comments using an `org` link token that names `a1`
- **THEN** the comment is accepted with the link's `comment` role
- **AND** a request without that token returns `404`

### Requirement: Only manage may change access

Creating, changing and removing grants and links, deleting the artifact, and
transferring ownership SHALL require `manage`. Grants SHALL only name users of
the artifact's org.

#### Scenario: Editor tries to share

- **WHEN** a user with `edit` sends `PUT /api/artifacts/a1/grants/u4`
- **THEN** the response is `403`

### Requirement: Share links are per artifact, revocable and optionally pinned

A share link SHALL have a kind (`org` or `public`), a role (`view` or
`comment`; `public` links only `view`), an optional pinned version, and an
optional expiry. The server SHALL store only a SHA-256 hash of the link token.
An `org` link SHALL require the viewer to be signed in to the artifact's org.
A revoked or expired link SHALL return `404`.

#### Scenario: Pinned link

- **GIVEN** a link pinned to version 2 and `a1` now at version 5
- **WHEN** someone opens the link
- **THEN** they see version 2 and no newer version is reachable through it

#### Scenario: Org link opened by an outsider

- **WHEN** a user signed in to a different org opens an `org` link
- **THEN** the response is `404`

### Requirement: Public links are gated by org policy

An org admin SHALL control `allow_public_links` via
`GET/PUT /api/artifacts/policy`; it SHALL default to off. Creating a `public`
link while it is off SHALL return `403`. Turning it off SHALL make existing
public links return `404` without deleting them; turning it back on restores
them.

#### Scenario: Policy toggled

- **GIVEN** public link `L` exists and the policy is turned off
- **WHEN** anyone opens `L`
- **THEN** the response is `404`
- **AND** after the policy is turned on again, `L` works
