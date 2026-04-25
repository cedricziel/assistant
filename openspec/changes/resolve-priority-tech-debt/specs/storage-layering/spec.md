## ADDED Requirements

### Requirement: storage crate dependency boundary

The `assistant-storage` crate SHALL declare only `assistant-core` as a workspace path dependency. It MUST NOT depend on `assistant-auth`, `assistant-backup`, `assistant-runtime`, `assistant-web-ui`, or any other workspace crate.

#### Scenario: clean compile graph for storage

- **WHEN** `cargo tree -p assistant-storage --workspace --no-default-features` is run
- **THEN** the only workspace crate listed under `assistant-storage`'s direct path dependencies is `assistant-core`

#### Scenario: CI guard against regression

- **WHEN** a future PR adds a workspace path dependency other than `assistant-core` to `crates/storage/Cargo.toml`
- **THEN** a clippy/CI check or a documented review checklist item flags the change

### Requirement: relocation of auth-coupled persistence

Persistence types and queries that are conceptually owned by authentication or backup features (e.g., user/session/token storage helpers, backup metadata) SHALL live in or above the crate that owns the feature, never in `assistant-storage`. `assistant-storage` SHALL expose only generic SQLite primitives, migrations, message-bus, traces, conversations, and other capability-agnostic stores.

#### Scenario: auth persistence ownership

- **WHEN** a developer needs to persist auth-related state
- **THEN** the relevant types and queries live in `assistant-auth` (or a new thin sub-crate consumed by `assistant-auth`), not in `assistant-storage`

#### Scenario: backup persistence ownership

- **WHEN** a developer needs to persist backup metadata
- **THEN** the relevant types and queries live in `assistant-backup`, not in `assistant-storage`

### Requirement: trait-based inversion when sharing is required

When `assistant-storage` needs to expose a generic storage primitive consumed by `assistant-auth` or `assistant-backup`, the contract SHALL be expressed as a trait defined in `assistant-core` (or `assistant-storage` if storage-specific) and implemented in `assistant-storage`. Higher crates SHALL depend on the trait, never on concrete auth/backup-specific types.

#### Scenario: shared primitive via trait

- **WHEN** `assistant-auth` requires a generic key/value or pool primitive provided by storage
- **THEN** the primitive is exposed as a trait, and `assistant-auth` consumes the trait without forcing storage to depend on auth
