# workspace-lint-policy Specification

## Purpose

TBD - created by archiving change workspace-lint-policy. Update Purpose after archive.

## Requirements

### Requirement: workspace-level lint table is the policy source of truth

The root `Cargo.toml` SHALL declare a `[workspace.lints]` table that sets the baseline lint levels for the entire workspace. The table SHALL include at minimum:

```toml
[workspace.lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
unimplemented = "warn"
todo = "warn"

[workspace.lints.rust]
unused_must_use = "deny"
```

Every workspace member crate's `Cargo.toml` SHALL contain `[lints]\nworkspace = true` so the workspace table is inherited. Per-crate `[lints.clippy]` overrides MAY raise or lower individual lints from the workspace baseline.

#### Scenario: a new crate is added

- **WHEN** a new crate is added under `crates/`
- **THEN** its `Cargo.toml` MUST include `[lints]\nworkspace = true`, enforced by a test in the root crate that scans all workspace members.

#### Scenario: a per-crate override is recorded

- **WHEN** a crate cannot meet the workspace baseline yet
- **THEN** its `Cargo.toml` SHALL include an explicit `[lints.clippy]` override for the relaxed lint, annotated with a `# TODO(workspace-lint-policy): ratchet to deny` comment so the override is discoverable.

### Requirement: deny-level crates enforce the panic-free contract

The following crates SHALL enforce `deny` level for `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` in non-test code via `[lints.clippy]` overrides in their respective `Cargo.toml`: `assistant-storage`, `assistant-web-ui`. Additional crates MAY be promoted to deny-level via individual follow-up changes; each promotion SHALL be its own PR that demonstrably leaves `cargo clippy --workspace -- -D warnings` green.

#### Scenario: adding a new unwrap to a deny-level crate fails CI

- **WHEN** a developer adds `.unwrap()`, `.expect()`, or `panic!()` to non-test code in a deny-level crate
- **THEN** `cargo clippy --workspace -- -D warnings` (i.e. `make lint`) fails with the offending file and line cited.

#### Scenario: file-level `cfg_attr` lint blocks are not duplicated

- **WHEN** a deny-level crate enforces the panic-free contract via `[lints.clippy]` in `Cargo.toml`
- **THEN** the corresponding source files SHALL NOT also declare `#![cfg_attr(not(test), deny(clippy::unwrap_used, ...))]` blocks. Crate-level configuration is the single source of truth for that crate's lint level.

### Requirement: test code is exempt from panic-free lints

Test code — `#[cfg(test)]` modules, `tests/` directories, fixture builders — SHALL be exempt from `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic` denials. The exemption mechanism MAY be either: (a) clippy's built-in exemption of `#[cfg(test)]` blocks under `[lints]` inheritance, (b) per-module `#[cfg_attr(test, allow(clippy::unwrap_used))]` in deny-level crates, or (c) a documented equivalent. The chosen mechanism SHALL be documented in `AGENTS.md`.

#### Scenario: a test uses unwrap

- **WHEN** a `#[cfg(test)]` mod in a deny-level crate calls `.unwrap()`
- **THEN** clippy SHALL NOT report an error for that call.

### Requirement: lint policy is documented in AGENTS.md

The `AGENTS.md` "Code Style" section SHALL include a "Lint policy" subsection that:

1. Names the workspace lint table in root `Cargo.toml` as the policy source.
2. Lists the currently-deny-level crates.
3. Explains the per-crate `allow → deny` ratchet contract.
4. References the verification command `make lint`.

#### Scenario: a contributor checks the panic-free policy

- **WHEN** a contributor opens `AGENTS.md` to learn the workspace lint policy
- **THEN** they find the subsection above and can determine without reading `Cargo.toml` which crates currently enforce deny-level panic-free guarantees.

### Requirement: ratcheting a crate from allow to deny is a self-contained change

Promoting a crate from `allow` to `deny` for the panic-free lint set SHALL be a self-contained change comprising: (1) replacement of the offending `.unwrap()`/`.expect()`/`panic!()` calls with `?`-propagated errors or explicit handling, (2) flipping the per-crate override from `"allow"` to `"deny"` (or removing the override entirely if the workspace default is acceptable), (3) confirming `make lint && make test` are green. The change SHALL NOT bundle unrelated refactors.

#### Scenario: a ratchet PR is opened

- **WHEN** a PR claims to ratchet a crate to deny-level
- **THEN** the diff contains only the unwrap-cleanup edits, the `Cargo.toml` lint-level flip, and any necessary test updates; no architectural refactors are included.
