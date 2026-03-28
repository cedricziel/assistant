@AGENTS.md

## Active Technologies
- Rust 2021 edition (workspace edition) + `matrix-sdk` (Matrix client SDK for Rust), `tokio` (async runtime), `anyhow` (error handling), `tracing` (structured logging), `lru` (bounded conversation cache), `uuid` (conversation IDs), `serde` + `toml` (configuration) (002-matrix-interface)
- N/A — conversation context is managed by the existing `Orchestrator`; the `matrix-sdk` crate maintains its own local session store for Matrix sync state (002-matrix-interface)

## Recent Changes
- 002-matrix-interface: Added Rust 2021 edition (workspace edition) + `matrix-sdk` (Matrix client SDK for Rust), `tokio` (async runtime), `anyhow` (error handling), `tracing` (structured logging), `lru` (bounded conversation cache), `uuid` (conversation IDs), `serde` + `toml` (configuration)
