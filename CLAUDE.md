@AGENTS.md

## Active Technologies

- Flutter 3.x (stable) + Dart 3.x, `flutter_riverpod 3.x`, `go_router 17.x`, `dio 5.x`, `flutter_secure_storage 10.x`, `assistant_api` (generated Dart client via openapi-generator dart-dio) (005-flutter-frontend)
- Rust: `rust-embed` (embeds `app/build/web/` into binary), `mime_guess` (MIME detection for SPA handler), `build.rs` runs `flutter build web --release` at compile time (005-flutter-frontend)
- Platforms: web (embedded in binary, served at `/`) + macOS native `.app` (005-flutter-frontend)

- Rust 2021 edition, workspace resolver 2 + `tar 0.4`, `flate2 1`, `sha2 0.10`, `hex 0.4` (new); `serde`, `serde_json`, `tokio`, `anyhow`, `tracing`, `chrono` (existing workspace deps) (004-backup-restore)
- SQLite at `~/.assistant/assistant.db` (WAL-checkpointed before copy); no new tables (004-backup-restore)

- Rust 2021 edition, workspace resolver 2 + `sqlx` (SQLite, hand-rolled migrations), `axum` (HTTP), `askama` (003-skill-management)
- SQLite — `~/.assistant/assistant.db`; skills also written to `~/.assistant/skills/<name>/SKILL.md` (003-skill-management)

- Rust 2021 edition, workspace resolver 2 + `axum` (HTTP), `askama` (templates), `sqlx` + SQLite (storage), (003-observability-ui-improvements)
- SQLite — `distributed_traces` + `logs` tables managed via schema migrations (003-observability-ui-improvements)

- Rust 2021 edition (workspace edition) + `matrix-sdk` (Matrix client SDK for Rust), `tokio` (async runtime), `anyhow` (error handling), `tracing` (structured logging), `lru` (bounded conversation cache), `uuid` (conversation IDs), `serde` + `toml` (configuration) (002-matrix-interface)
- N/A — conversation context is managed by the existing `Orchestrator`; the `matrix-sdk` crate maintains its own local session store for Matrix sync state (002-matrix-interface)
- Rust 2021 edition + Axum (HTTP server), Askama (server-side templates), tokio::fs (async file I/O), sqlx + SQLite (PersonaStore), HTMX + Stimulus.js (frontend interactivity) (002-persona-editor-ui)
- SQLite (existing `personas` table) + local filesystem (`~/.assistant/agents/{id}/`) (002-persona-editor-ui)

## Recent Changes

- 003-skill-management: `SkillRegistry` gains `create_user_skill`, `update_user_skill`, `delete_user_skill`, `list_for_persona`. New `PersonaSkillAccessStore` manages per-persona access modes (all/whitelist/blacklist) and skill lists. Migrations 028 (body_text column) and 029 (persona_skill_access). Web UI at `/skills` with full CRUD. CLI `assistant skill list/show/create/delete/generate` (`generate` drafts a SKILL.md from a plain-language description using the agentskills-spec builtin) and `assistant persona skill-mode/skill-add/skill-remove`. Orchestrator filters skills by persona access mode.
- 002-matrix-interface: Added Rust 2021 edition (workspace edition) + `matrix-sdk` (Matrix client SDK for Rust), `tokio` (async runtime), `anyhow` (error handling), `tracing` (structured logging), `lru` (bounded conversation cache), `uuid` (conversation IDs), `serde` + `toml` (configuration)
- 002-persona-editor-ui: Added Rust 2021 edition + Axum (HTTP server), Askama (server-side templates), tokio::fs (async file I/O), sqlx + SQLite (PersonaStore), HTMX + Stimulus.js (frontend interactivity)
