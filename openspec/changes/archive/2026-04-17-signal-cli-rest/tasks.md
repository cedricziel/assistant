## 1. Config Changes

- [x] 1.1 Add `api_url: Option<String>`, `api_user: Option<String>`, `api_password: Option<String>` to `SignalConfig` in `crates/core/src/types.rs`; remove `store_path`
- [x] 1.2 Add `SignalConfigExt` helper methods `resolved_api_url() -> String` (default `http://localhost:8080`), `resolved_phone_number() -> Option<String>`, `resolved_api_user/password() -> Option<String>` to `crates/interface-signal/src/config.rs`
- [x] 1.3 Update `config.toml` sample comments to reflect new fields

## 2. signal-cli REST Adapter

- [x] 2.1 Rewrite `crates/interface-signal/src/adapter.rs`: `SignalAdapter` stores `SignalConfig`; add `stop_tx/stop_rx` watch channel
- [x] 2.2 Implement `ChannelAdapter::start()`: connect to `ws://{api_url}/v1/receive/{phone_number}` WebSocket with optional Basic Auth header; yield `ChannelMessage` for each inbound `dataMessage`; auto-reconnect with exponential backoff
- [x] 2.3 Implement `ChannelAdapter::send()`: POST JSON to `{api_url}/v1/send` with `{"number": phone, "recipients": [platform_id], "message": text}` (or `group_id` when `thread_id` is set)
- [x] 2.4 Implement `ChannelAdapter::send_in_thread()`: same as `send()` but uses `group_id` from `thread_id`
- [x] 2.5 Implement `ChannelAdapter::stop()`: signal stop watch
- [x] 2.6 Implement `conversation_key()`: return `group_id` if present in metadata, else `source`
- [x] 2.7 Add `on_turn_error()` hook to send an error message back to the sender
- [x] 2.8 Add unit tests for `parse_envelope()` helper (happy path, group message, missing fields)

## 3. Remove Presage

- [x] 3.1 Delete `crates/interface-signal/src/linker.rs`
- [x] 3.2 Delete `crates/interface-signal/src/main.rs`
- [x] 3.3 Rewrite `crates/interface-signal/src/runner.rs`: `SignalInterface::run()` = `ChannelRunner::new(Arc::new(SignalAdapter::new(config)?), orchestrator).run().await`
- [x] 3.4 Remove `presage`, `presage-store-sqlite`, `qr2term` from `crates/interface-signal/Cargo.toml`; remove `[features]` signal gate; add `tokio-tungstenite`, `tokio-stream`
- [x] 3.5 Remove `assistant-bus-nats` optional dep and `nats` feature from `interface-signal/Cargo.toml` (move to crates that need it)
- [x] 3.6 Update `crates/interface-signal/src/lib.rs`: remove `pub use linker::link_device`, re-export `SignalAdapter`

## 4. CLI Cleanup

- [x] 4.1 Remove `#[cfg(feature = "signal")]` guards from `crates/interface-cli/src/main.rs`
- [x] 4.2 Remove `SignalCommand::Link` subcommand and `cmd_signal` handler from the CLI
- [x] 4.3 Update the orchestrator startup block to use the same pattern as Slack/Mattermost/Matrix (no `--features signal` requirement)
- [x] 4.4 Remove `signal` feature from `interface-cli/Cargo.toml` if present

## 5. Verification

- [x] 5.1 Run `make check` — fix any compile errors
- [x] 5.2 Run `make lint` — fix all clippy warnings
- [x] 5.3 Run `make format`
- [x] 5.4 Run `make test` — all tests pass
- [x] 5.5 Run `cargo machete --with-metadata` — no unused deps
- [x] 5.6 Update `AGENTS.md` to reflect the signal-cli-rest change
