# Spec: Pre-commit Hook

## Requirements

### REQ-1: Preflight toolchain check

The hook MUST verify that `cargo`, `flutter`, and `dart` are available on PATH
before running any checks. If any is missing, it MUST exit with a non-zero code
and a clear error message naming the missing tool.

### REQ-2: Rust formatting check

The hook MUST run `cargo fmt --all -- --check` and fail if any files are
unformatted.

### REQ-3: Dart pre-commit checks

The hook MUST run `dart run dart_pre_commit` from the `app/` directory. This
covers dart formatting, static analysis, dependency freshness, and OSV
vulnerability scanning.

### REQ-4: Rust linting

The hook MUST run `cargo clippy --workspace -- -D warnings` and fail on any
warning.

### REQ-5: Unused Rust dependency check

The hook MUST run `cargo machete --with-metadata` and fail if unused
dependencies are found.

### REQ-6: Flutter tests

The hook MUST run `flutter test` from the `app/` directory and fail if any test
fails.

### REQ-7: Fail-fast execution

The hook MUST stop on the first failing check. Checks MUST run in order from
cheapest to most expensive (as listed in REQ-1 through REQ-6).

### REQ-8: Dev dependency

`dart_pre_commit` MUST be added as a dev dependency in `app/pubspec.yaml`.

### REQ-9: Makefile targets

The Makefile MUST include:

- `lint-flutter` — runs `dart run dart_pre_commit` in `app/`
- `test-flutter` — runs `flutter test` in `app/`
- `precommit` — runs all hook checks in order for manual invocation
