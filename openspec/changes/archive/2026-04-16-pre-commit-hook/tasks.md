# Tasks: Pre-commit Hook

- [x] Add `dart_pre_commit` as a dev dependency in `app/pubspec.yaml` and run `flutter pub get`
- [x] Rewrite `.githooks/pre-commit` with preflight checks, Rust checks, `dart_pre_commit`, and `flutter test` in fail-fast order
- [x] Add `lint-flutter`, `test-flutter`, and `precommit` targets to the Makefile
- [x] Update CLAUDE.md / AGENTS.md to document the new Flutter hook checks and Makefile targets
