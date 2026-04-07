# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

## API Client Generation

The Flutter app's `app/packages/assistant_api/` package is **generated code** — never edit it manually. The workflow when changing API endpoints:

```sh
make dump-openapi           # re-export openapi.json from the running server
make generate-flutter-client # regenerate Dart/dio client from openapi.json
```

Requires `openapi-generator` (`brew install openapi-generator`).

## Build Integration

The Rust build embeds the Flutter web app via `rust-embed`. `cargo build -p assistant-cli` (or `make build`) triggers `build.rs`, which runs `flutter build web --release` in `app/`. This means:

- A Flutter SDK must be on `PATH` for a full Rust build.
- Use `make check` (`cargo check`) to skip Flutter and validate Rust only.

## Runtime Data

All runtime data lives under `~/.assistant/`:

- `assistant.db` — SQLite database (WAL mode)
- `config.toml` — copied from `config.toml` at repo root on first run
- `skills/<name>/SKILL.md` — user-created skills
- `agents/<id>/` — per-persona agent workspace

## OpenAPI Spec

`openapi.json` at the repo root is the committed spec snapshot. Keep it up to date when modifying routes in `crates/web-ui`.
