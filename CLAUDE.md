@AGENTS.md

## Active Technologies
- Rust 2021 edition, workspace resolver 2 + `axum` (HTTP), `askama` (templates), `sqlx` + SQLite (storage), (003-observability-ui-improvements)
- SQLite — `distributed_traces` + `logs` tables; no schema migrations needed (003-observability-ui-improvements)

- Rust 2021 edition (workspace edition) + `matrix-sdk` (Matrix client SDK for Rust), `tokio` (async runtime), `anyhow` (error handling), `tracing` (structured logging), `lru` (bounded conversation cache), `uuid` (conversation IDs), `serde` + `toml` (configuration) (002-matrix-interface)
- N/A — conversation context is managed by the existing `Orchestrator`; the `matrix-sdk` crate maintains its own local session store for Matrix sync state (002-matrix-interface)
- Rust 2021 edition + Axum (HTTP server), Askama (server-side templates), tokio::fs (async file I/O), sqlx + SQLite (PersonaStore), HTMX + Stimulus.js (frontend interactivity) (002-persona-editor-ui)
- SQLite (existing `personas` table) + local filesystem (`~/.assistant/agents/{id}/`) (002-persona-editor-ui)

## Recent Changes

- 002-matrix-interface: Added Rust 2021 edition (workspace edition) + `matrix-sdk` (Matrix client SDK for Rust), `tokio` (async runtime), `anyhow` (error handling), `tracing` (structured logging), `lru` (bounded conversation cache), `uuid` (conversation IDs), `serde` + `toml` (configuration)
- 002-persona-editor-ui: Added Rust 2021 edition + Axum (HTTP server), Askama (server-side templates), tokio::fs (async file I/O), sqlx + SQLite (PersonaStore), HTMX + Stimulus.js (frontend interactivity)
