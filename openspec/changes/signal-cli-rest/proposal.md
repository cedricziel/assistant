---
name: signal-cli-rest
description: Replace presage Signal integration with signal-cli REST API client
type: proposal
---

## Why

The current Signal integration uses `presage`, a full Rust Signal protocol implementation with non-`Send` types, a heavy SQLite state store, and compile-time feature gating — making it incompatible with the `ChannelAdapter` / `ChannelRunner` pattern used by all other interfaces. Replacing it with a thin HTTP/WebSocket client against the [signal-cli REST API](https://github.com/bbernhard/signal-cli-rest-api) follows the same thin-client philosophy as the Slack/Mattermost/Matrix migration and eliminates the `presage` dependency entirely.

## What Changes

- **BREAKING**: Remove the `signal` cargo feature and all `presage` / `presage-store-sqlite` dependencies from `interface-signal`.
- **BREAKING**: Remove `link_device()` / `SignalCommand::Link` from the CLI — device registration is now handled externally by the signal-cli daemon.
- Replace `runner.rs` presage loop with a `SignalAdapter` implementing `ChannelAdapter` backed by the signal-cli REST API.
- `SignalAdapter::start()` opens a WebSocket to `GET /v1/receive/{number}` (signal-cli-rest-api WebSocket endpoint) and yields `ChannelMessage` items — no more non-Send types, no feature gate.
- `SignalAdapter::send()` POSTs to `POST /v1/send`.
- `SignalInterface::run()` becomes a thin `ChannelRunner::new(adapter, orchestrator).run().await`.
- `SignalConfig` gains `api_url` (default `http://localhost:8080`) and loses `store_path` (managed by the signal-cli daemon).
- Remove `linker.rs` and `main.rs` from the crate (no longer needed).
- Update `interface-cli` to remove `#[cfg(feature = "signal")]` guards and the `signal link` subcommand.

## Capabilities

### New Capabilities

- `signal-rest-adapter` — `SignalAdapter` implementing `ChannelAdapter` via signal-cli REST API

### Modified Capabilities

- `signal-config` — extend `SignalConfig` with `api_url`, remove `store_path`
