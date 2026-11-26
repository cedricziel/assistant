# Spec Delta

## Purpose
Define how the assistant discovers and uses skills that external MCP servers publish through the Skills extension.

## ADDED Requirements

### Requirement: Skill discovery
For each connected server configured under `[mcp]` in `server.toml` that advertises the Skills extension, the client SHALL list its skills and make them available to Personas that can use that server, named with the server's prefix so they cannot shadow local skills.

#### Scenario: Server publishes a skill
- **WHEN** a connected server lists a skill `deploy`
- **THEN** Personas using that server see a skill named with that server's prefix and `deploy`

#### Scenario: Server without the extension
- **WHEN** a server does not advertise the Skills extension
- **THEN** the client does not call `skills/list` on it

#### Scenario: Integration-pool server
- **WHEN** a server connected through the integration pool advertises the Skills extension
- **THEN** the client does not list its skills, and Personas see none of them

### Requirement: Manifest verification
Before a remote skill is used, the client SHALL fetch its files and verify each file's size and digest against the manifest, and SHALL reject the skill when any check fails or its frontmatter does not match the manifest.

#### Scenario: Digest mismatch
- **WHEN** a skill file's content does not match its manifest digest
- **THEN** the skill is not offered and a warning names the server and skill

### Requirement: Untrusted content
Remote skills SHALL be treated as untrusted: their bundled scripts SHALL NOT run without the permission policy's confirmation, and a manifest change SHALL require re-verification.

#### Scenario: Skill manifest changes
- **WHEN** a server's skill manifest changes after verification
- **THEN** the client re-verifies the skill before its next use
