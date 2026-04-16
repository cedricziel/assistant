## Why

Each messenger interface (Slack, Mattermost, Matrix, Nextcloud) has a bespoke runner that reimplements the same ~100 lines of boilerplate: LRU conversation cache, per-conversation mutex serialization, shutdown signal handling, and the stream dispatch loop. Adding a new channel today requires duplicating this boilerplate and wiring a new CLI subcommand by hand. We replace all of that with a single generic `ChannelRunner` that drives any `ChannelAdapter`.

## What Changes

- **New**: `ChannelRunner` struct in `crates/runtime` — the one generic runner that drives any `ChannelAdapter` implementation
- **New**: Lifecycle hooks on `ChannelAdapter` trait — `conversation_key()`, `platform_tools()`, `on_turn_start()`, `on_turn_success()`, `on_turn_error()` — all default no-ops
- **Replaced**: `SlackInterface::run()`, `MattermostInterface::run()`, `MatrixInterface::run()` bespoke runner bodies delegate to `ChannelRunner`
- **Replaced**: `NextcloudInterface::run()` — Nextcloud gets a proper `ChannelAdapter::start()` stream and drops its axum-in-runner approach, delegating to `ChannelRunner`
- **Unchanged**: `SignalInterface::run()` — presage non-Send constraint; stays bespoke
- **CLI**: Per-interface CLI dispatch paths simplified; multi-channel daemon mode becomes trivial

## Capabilities

### New Capabilities

- `channel-runner`: Generic async runner that consumes any `ChannelAdapter`, handling conversation key mapping, per-conversation turn serialization, shutdown, and dispatch to `run_turn_with_tools`
- `channel-adapter-hooks`: Extended `ChannelAdapter` trait methods for per-message conversation keying, platform-specific tools injection, and turn lifecycle callbacks (start/success/error)

### Modified Capabilities

## Impact

- `crates/core/src/channel.rs` — add lifecycle hook methods to `ChannelAdapter` trait (default no-ops, non-breaking)
- `crates/runtime/src/` — new `channel_runner.rs`
- `crates/interface-slack/src/runner.rs` — body replaced, ambient tools and reactions move to adapter hooks
- `crates/interface-mattermost/src/runner.rs` — body replaced
- `crates/interface-matrix/src/runner.rs` — body replaced
- `crates/interface-nextcloud/src/runner.rs` — body replaced; adapter gains real `start()` stream
- `crates/interface-cli/src/main.rs` — interface dispatch simplified
- No dependency changes required
