## ADDED Requirements

### Requirement: bash timeout is capped

`bash` SHALL clamp `timeout_secs` to `permissions.limits.bash_max_timeout_secs`.

#### Scenario: Oversized timeout clamped

- **WHEN** `timeout_secs = 86400` and the configured maximum is 600
- **THEN** the command SHALL be killed after at most 600 seconds

### Requirement: bash output is stream-capped

`bash` SHALL read stdout and stderr incrementally and stop retaining output at
the configured cap, without buffering the full output first.

#### Scenario: Unbounded producer

- **WHEN** a command prints more than the cap
- **THEN** memory use SHALL stay bounded by the cap AND the result SHALL note
  truncation

### Requirement: bash image auto-attach is workspace-confined

`bash` SHALL auto-attach an image path found in stdout only if its canonical
path lies inside the workspace roots.

#### Scenario: Image outside roots not attached

- **WHEN** stdout contains `/Users/x/.ssh/photo.png`
- **THEN** no attachment SHALL be produced

### Requirement: process sessions are bounded

`process` sessions SHALL expire after `process_ttl_secs` of inactivity, at most
`process_max_sessions` sessions SHALL exist concurrently, and each stored output
line SHALL be truncated at a per-line cap.

#### Scenario: Idle session expires

- **WHEN** a session is idle for longer than the TTL
- **THEN** it SHALL be killed and removed

#### Scenario: Session cap

- **WHEN** the cap is reached and a new session is requested
- **THEN** the request SHALL fail with an error

#### Scenario: Long line truncated

- **WHEN** a child emits a line longer than the per-line cap
- **THEN** the stored line SHALL be truncated with a marker
