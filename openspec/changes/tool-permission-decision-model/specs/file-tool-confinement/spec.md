## ADDED Requirements

### Requirement: File tools are confined to workspace roots

`file-read`, `file-write`, `file-edit` and `file-glob` SHALL resolve the
requested path (expanding `~`, normalising `..`, resolving symlinks; for
non-existent targets, resolving the deepest existing ancestor) and SHALL refuse
any path whose real location is outside the persona workspace and
`permissions.workspace_roots`. The check SHALL run inside the tool, independent
of the orchestrator pipeline.

#### Scenario: Path outside roots is refused

- **WHEN** `file-read` is called with `/etc/passwd`
- **THEN** the tool SHALL return an error result and SHALL NOT read the file

#### Scenario: Traversal is refused

- **WHEN** `file-write` is called with `<workspace>/../outside.txt`
- **THEN** the tool SHALL return an error result and SHALL NOT write

#### Scenario: Symlink escape is refused

- **WHEN** a symlink inside the workspace points to a file outside every root
  and `file-read` is called on the symlink
- **THEN** the tool SHALL return an error result

#### Scenario: Extra root permitted

- **WHEN** `workspace_roots` contains `/data/shared` and `file-read` targets
  `/data/shared/a.txt`
- **THEN** the read SHALL proceed
