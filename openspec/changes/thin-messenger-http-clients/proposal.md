## Why

The five messenger interface crates (Slack, Mattermost, Matrix, Nextcloud, Signal) each depend on large, opinionated platform-specific Rust SDKs (`slack-morphism`, `mattermost_api`, `matrix-sdk`, `presage`). These SDKs own the HTTP transport, impose their own async runtimes/abstractions, and make it hard to add new channels or test existing ones. Inspired by OpenFang's `ChannelAdapter` trait, we want to replace them with thin `reqwest`-based HTTP clients and a single shared `ChannelAdapter` trait, so every channel is just a thin wrapper around HTTP calls.

## What Changes

- Introduce a `ChannelAdapter` trait in `assistant-core` (or a new `assistant-channels` crate) with `start()` → `Stream<ChannelMessage>`, `send()`, `stop()`, and optional `send_typing()` / `send_reaction()` / `send_in_thread()`.
- Define unified `ChannelMessage`, `ChannelContent`, and `ChannelUser` types to replace per-interface ad-hoc event structs.
- Rewrite `interface-slack` as a thin reqwest client using Slack's Socket Mode WebSocket + REST APIs — drop `slack-morphism`.
- Rewrite `interface-mattermost` as a thin reqwest client using Mattermost's WebSocket + REST APIs — drop `mattermost_api`.
- Rewrite `interface-matrix` as a thin reqwest client using Matrix Client-Server spec — drop `matrix-sdk`.
- Keep `interface-nextcloud` largely as-is (already uses `reqwest`) but align it to the new trait.
- Keep `interface-signal` largely as-is (protocol-level, not HTTP) but wrap it behind the trait.
- **BREAKING**: Remove the per-interface `run_turn_with_tools` direct coupling — channels emit `ChannelMessage` events; a shared dispatch layer calls the orchestrator.

## Capabilities

### New Capabilities

- `channel-adapter-trait`: Shared `ChannelAdapter` trait + unified message/content/user types used by all messenger interfaces.
- `slack-thin-client`: Slack adapter rewritten as a thin reqwest-based HTTP client, dropping `slack-morphism`.
- `mattermost-thin-client`: Mattermost adapter rewritten as a thin reqwest-based HTTP client, dropping `mattermost_api`.
- `matrix-thin-client`: Matrix adapter rewritten as a thin reqwest-based HTTP client, dropping `matrix-sdk`.

### Modified Capabilities

<!-- No existing openspec specs to modify -->

## Impact

- **Crates affected**: `crates/core` (new trait + types), `crates/interface-slack`, `crates/interface-mattermost`, `crates/interface-matrix`, `crates/interface-nextcloud`, `crates/interface-signal`, `Cargo.toml` (workspace deps).
- **Dependencies removed**: `slack-morphism`, `mattermost_api`, `matrix-sdk` (large transitive trees).
- **Dependencies added**: `reqwest` (already in workspace), `tokio-tungstenite` or `async-tungstenite` for WebSocket connections.
- **No API surface change** for callers of `InterfaceRunner` — the `run()` method signature stays the same.
- **Test impact**: Thin HTTP clients can be tested with `wiremock` (already in workspace), replacing SDK-level mocking.
