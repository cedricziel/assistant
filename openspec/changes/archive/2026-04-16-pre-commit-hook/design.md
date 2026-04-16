# Design: Pre-commit Hook

## Architecture

The pre-commit hook is a single bash script at `.githooks/pre-commit`. It runs
sequentially through checks, exiting on first failure (fail-fast).

```
git commit
    │
    ▼
.githooks/pre-commit
    │
    ├─ preflight: cargo, flutter, dart on PATH
    ├─ cargo fmt --all -- --check
    ├─ (cd app && dart run dart_pre_commit)
    ├─ cargo clippy --workspace -- -D warnings
    ├─ cargo machete --with-metadata
    └─ (cd app && flutter test)
```

## Key Decisions

### Use `dart_pre_commit` package

Instead of hand-rolling `dart format` + `flutter analyze`, we use the
`dart_pre_commit` pub package. It provides formatting, static analysis,
dependency freshness, and OSV vulnerability scanning with zero config.

Configured implicitly via existing `analysis_options.yaml` (which already
excludes `packages/assistant_api/`).

### Fail-fast ordering

Checks are ordered cheapest to most expensive so developers get fast feedback
on trivial issues (formatting) before waiting for slow checks (tests).

### Hard fail on missing tools

No graceful degradation. If `cargo`, `flutter`, or `dart` is missing, the hook
fails immediately with a clear error message. This ensures every committer has
the full toolchain.

### Makefile targets

New targets mirror the hook steps for manual use:

- `lint-flutter` — runs `dart run dart_pre_commit` in `app/`
- `test-flutter` — runs `flutter test` in `app/`
- `precommit` — runs the full hook sequence for manual invocation

## Integration Points

- `.githooks/pre-commit` — the hook script itself
- `Makefile` — new targets for manual use
- `app/pubspec.yaml` — `dart_pre_commit` dev dependency
- `app/analysis_options.yaml` — already excludes generated code (no changes needed)
