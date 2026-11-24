---
name: test-coverage
description: Per-crate test coverage floor (80%) and the coverage.toml rollout ratchet. Use when running `make coverage`, reading coverage gate output, or promoting/excluding a crate in coverage.toml.
---

# Test Coverage Floor

The workspace targets `>= 80%` line coverage per crate, measured by
`cargo llvm-cov` and gated in CI via `.github/workflows/coverage.yml`.

```sh
# Local: produce coverage.json and run the per-crate gate.
make coverage

# Inputs:
#   coverage.toml          — floor, excluded crates, report-only allowlist, file excludes
#   tools/check_coverage.sh — gate script (parses coverage.toml + coverage.json)
```

During rollout (tracked by openspec/changes/workspace-test-coverage-floor/),
every crate sits on the `[report_only]` list in `coverage.toml`. The gate
prints each crate's coverage delta but does not fail CI. As a crate reaches
the floor sustainably, remove it from `report_only` — that flips the gate
to enforcing for that crate, and any subsequent PR that drops it below 80%
fails the build. Promotion is a one-way ratchet; re-adding a crate requires
a separate OpenSpec change.

Permanently excluded crates (no production library code or generated types)
live in `[excluded_crates]` and never gate CI.
