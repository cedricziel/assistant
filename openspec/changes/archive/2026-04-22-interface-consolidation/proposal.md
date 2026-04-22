## Why

The workspace has five separate messenger interface crates (`interface-slack`, `interface-mattermost`, `interface-matrix`, `interface-nextcloud`, `interface-signal`). Each is its own Cargo crate with its own `Cargo.toml`, module tree, and CI entry. Despite following the same `ChannelAdapter` → `InterfaceRunner` pattern, this separation creates overhead:

- **Build & CI cost**: Five crates means five sets of dependency resolution, five clippy targets, and five `cargo test` invocations. Shared helper logic (backoff retry, transcription download, allowlist filtering) is duplicated across crates rather than shared.
- **Feature-flag sprawl in interface-cli**: The CLI conditionally compiles each crate behind feature flags (`slack`, `mattermost`, `matrix`, `nextcloud`, `signal`), adding complexity to `Cargo.toml` and `main.rs` dispatch.
- **Discoverability**: A contributor looking to add a sixth interface must study five separate crate layouts rather than one module with a clear pattern.
- **Type leakage prevention**: Some interface-specific types (e.g. `SignalAdapter`) are publicly exported even though only the runner needs to be public. A single crate makes it natural to keep adapters private.

All five crates already depend on the same shared types from `assistant-core` (`ChannelAdapter`, `ChannelMessage`, `ChannelContent`, `ChannelUser`, `ChannelType`, config structs). The adapters are internal implementation details — only `XxxInterface` (implementing `InterfaceRunner`) is consumed externally.

## What Changes

- Create a new `assistant-interfaces` crate (`crates/interfaces/`) containing one module per platform: `slack`, `mattermost`, `matrix`, `nextcloud`, `signal`.
- Move each adapter, runner, client, config extension, skills, and tools module into the corresponding sub-module.
- Delete the five individual `crates/interface-{slack,mattermost,matrix,nextcloud,signal}` crates.
- Update `crates/interface-cli` to depend on `assistant-interfaces` instead of five separate crates, removing the per-interface feature flags.
- Keep all shared types (`ChannelAdapter`, `ChannelMessage`, `ChannelContent`, `ChannelUser`, `ChannelType`, config structs, `InterfaceRunner`) in `assistant-core` and `assistant-runtime` where they already live.

## Capabilities

### New Capabilities

- `assistant-interfaces`: Single crate housing all messenger interface adapters and runners behind a unified module structure.

### Modified Capabilities

- `assistant-cli` dispatch simplified: direct imports from `assistant_interfaces::{SlackInterface, MattermostInterface, ...}` replace five feature-gated imports.

## Non-goals

- Moving `ChannelAdapter`, `ChannelMessage`, or config types out of `assistant-core` — they already live in the right place.
- Changing the `InterfaceRunner` trait or `ChannelRunner` in `assistant-runtime`.
- Merging `interface-cli` into this crate — the CLI is a binary with its own concerns (REPL, subcommands, MCP server).
- Adding new interfaces (Discord, Telegram, etc.) — that is a follow-on.
- Changing the public API surface: `XxxInterface::new()` signatures and `InterfaceRunner::run()` stay the same.

## Impact

- **Crates removed**: `assistant-interface-slack`, `assistant-interface-mattermost`, `assistant-interface-matrix`, `assistant-interface-nextcloud`, `assistant-interface-signal` (5 crates).
- **Crate added**: `assistant-interfaces` (1 crate).
- **Crates modified**: `assistant-cli` (dependency + dispatch changes), root `Cargo.toml` (workspace members).
- **Dependencies**: No new external dependencies. All existing deps (`reqwest`, `tokio-tungstenite`, `axum`, `hmac`, `sha2`, `lru`, `base64`, `dirs`, `urlencoding`) move into the consolidated crate.
- **No runtime behavior change**: All adapters, skills, and config extensions work identically.
- **Test impact**: Tests move into the new crate's module tests. `wiremock`-based tests remain unchanged.
