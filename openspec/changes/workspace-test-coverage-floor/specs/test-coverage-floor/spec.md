## ADDED Requirements

### Requirement: per-crate line-coverage floor

Every crate in the workspace SHALL maintain `>= 80%` line coverage as
measured by `cargo llvm-cov --workspace --json` per-package. The floor
applies to all production crates; the exclusions are documented in
`coverage.toml`.

Excluded crates (initial set):

- `assistant-integration-tests` — contains no production library code
  (its `src/lib.rs` is a stub for shared test helpers).
- `assistant-a2a-json-schema` — generated `serde` types mirror the A2A
  JSON schema; coverage targets on typed data are not meaningful.

#### Scenario: workspace coverage CI gate is configured

- **WHEN** the GitHub Actions `coverage.yml` workflow runs on a PR
- **THEN** it executes `cargo llvm-cov --workspace --json --output-path coverage.json`
  and invokes `tools/check_coverage.sh` to assert per-crate coverage

#### Scenario: a crate falls below 80%

- **WHEN** any non-excluded crate's `lines.percent` in `coverage.json`
  is below `80.0`
- **THEN** the CI gate exits non-zero with a message listing the crate,
  its current coverage, and the delta to 80%

#### Scenario: an excluded crate is added or removed

- **WHEN** a contributor proposes adding or removing a crate from the
  exclusion list
- **THEN** the change MUST justify the exclusion in the PR description
  (generated code, harness-only, or equivalent rationale)

### Requirement: report-only mode during rollout

The CI gate SHALL support a per-crate report-only allowlist. Crates on
the allowlist SHALL print their coverage delta in the CI log but MUST NOT
fail the build for falling below the floor.

#### Scenario: crate on the report-only allowlist

- **WHEN** a crate listed in the allowlist measures below 80%
- **THEN** the gate prints the coverage and the delta but exits 0

#### Scenario: a crate is promoted to enforced

- **WHEN** a contributor removes a crate from the report-only allowlist
- **THEN** the gate enforces 80% for that crate on all subsequent PRs;
  the removal is permanent — re-adding a crate to the allowlist requires
  a separate OpenSpec change

### Requirement: coverage measurement excludes generated code

The `coverage.toml` configuration SHALL exclude `build.rs`-generated
files, `*.pb.rs` artifacts, and macro-expanded `#[derive(...)]`
boilerplate from the line-coverage calculation.

#### Scenario: hand-written code remains in scope

- **WHEN** `cargo llvm-cov` runs against the workspace
- **THEN** all hand-written `.rs` files under `crates/*/src/**` count
  toward coverage, while generated files identified in `coverage.toml`
  are excluded

### Requirement: `make coverage` reproduces CI locally

The root `Makefile` SHALL expose a `coverage` target that runs the same
`cargo llvm-cov` invocation as CI and prints a per-crate table of
current coverage percentages.

#### Scenario: developer runs `make coverage`

- **WHEN** a developer runs `make coverage` after editing code
- **THEN** the output table lists every crate with its current coverage,
  the floor (80%), the delta, and the report-only status, sorted by
  delta ascending

### Requirement: contributor guidance is documented

The `AGENTS.md` testing section SHALL document the 80% floor, link to
the `make coverage` workflow, and reference this spec.

#### Scenario: new crate is added to the workspace

- **WHEN** a contributor adds a new crate under `crates/`
- **THEN** the crate inherits the 80% floor immediately; if the initial
  PR cannot reach the floor, the crate MUST be added to the report-only
  allowlist with a follow-up task to reach the floor
