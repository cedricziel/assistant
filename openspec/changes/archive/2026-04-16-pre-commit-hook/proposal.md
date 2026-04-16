# Pre-commit Hook: Rust + Flutter Quality Gate

## Problem

The existing pre-commit hook (`.githooks/pre-commit`) only covers Rust checks
(cargo fmt, clippy, machete). The Flutter app under `app/` has no pre-commit
quality gate — formatting, analysis, and test regressions can slip through to CI.

## Proposal

Extend the pre-commit hook to enforce quality checks for **both** Rust and
Flutter, and add corresponding Makefile targets for manual use.

## Design Decisions

- **Always run all checks** — no selective skipping based on changed files. If
  you commit to this repo, you need both toolchains.
- **Hard fail on missing toolchains** — `cargo`, `flutter`, and `dart` must be
  on PATH. No graceful degradation.
- **Fail fast** — stop on first failing check, ordered cheap-to-expensive.
- **Use `dart_pre_commit`** — the
  [`dart_pre_commit`](https://pub.dev/packages/dart_pre_commit) package replaces
  hand-rolled `dart format` + `flutter analyze` calls. It provides formatting,
  analysis, dependency freshness checks, and OSV vulnerability scanning
  out-of-the-box with zero config and optional tuning via `pubspec.yaml`.
  Generated code in `app/packages/assistant_api/` is excluded via
  `analysis_options.yaml` (already in place).
- **Include flutter test** — tests run as the final (most expensive) step,
  separate from `dart_pre_commit` since the package doesn't cover tests.

## Check Order (fail-fast, cheap → expensive)

| Step | Check                                        | ~Time   |
| ---- | -------------------------------------------- | ------- |
| 1    | Preflight: verify `cargo`, `flutter`, `dart` | instant |
| 2    | `cargo fmt --all -- --check`                 | ~0s     |
| 3    | `dart run dart_pre_commit` (from `app/`)     | ~5s     |
| 4    | `cargo clippy --workspace -- -D warnings`    | ~10s    |
| 5    | `cargo machete --with-metadata`              | ~3s     |
| 6    | `flutter test` (from `app/`)                 | ~30s+   |

## Dependencies Added

- `dart_pre_commit` as a dev dependency in `app/pubspec.yaml`

## Files Changed

- `.githooks/pre-commit` — rewrite with preflight + Flutter checks
- `Makefile` — add `lint-flutter`, `test-flutter`, and `precommit` targets
- `app/pubspec.yaml` — add `dart_pre_commit` dev dependency

## Out of Scope

- Commit message linting (conventional commits enforcement)
- Running Rust tests in the hook (kept for CI)
- Husky / lefthook or any third-party hook manager
