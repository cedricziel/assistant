## 1. Core: ChannelAdapter Trait and Unified Types

- [x] 1.1 Add `ChannelType` enum to `crates/core/src/types.rs` (variants: Slack, Mattermost, Matrix, Nextcloud, Signal, Custom(String))
- [x] 1.2 Add `ChannelUser` struct to `crates/core/src/channel.rs` (platform_id, display_name)
- [x] 1.3 Add `ChannelContent` enum to `crates/core/src/channel.rs` (Text, Image, File, FileData, Voice)
- [x] 1.4 Add `ChannelMessage` struct to `crates/core/src/channel.rs` (channel_type, platform_message_id, sender, content, thread_id, timestamp, metadata)
- [x] 1.5 Define `ChannelAdapter` trait in `crates/core/src/channel.rs` with start/send/stop + optional methods with default no-ops
- [x] 1.6 Export new types from `crates/core/src/lib.rs`
- [x] 1.7 Add `tokio-tungstenite` to `[workspace.dependencies]` in root `Cargo.toml`
- [x] 1.8 Write unit tests for `ChannelMessage` construction and `ChannelContent` variants

## 2. Slack: Thin reqwest Client

- [x] 2.1 Remove `slack-morphism` from `crates/interface-slack/Cargo.toml`; add `tokio-tungstenite`
- [x] 2.2 Create `crates/interface-slack/src/client.rs` — `SlackClient` struct wrapping `reqwest::Client` with bot_token + base URL
- [x] 2.3 Implement `apps_connections_open()` in `SlackClient` to fetch the Socket Mode WebSocket URL
- [x] 2.4 Implement `post_message()` in `SlackClient` for `chat.postMessage` (channel, text, optional thread_ts)
- [x] 2.5 Implement `add_reaction()` in `SlackClient` for `reactions.add`
- [x] 2.6 Create `crates/interface-slack/src/adapter.rs` — `SlackAdapter` implementing `ChannelAdapter`
- [x] 2.7 Implement `start()`: open WebSocket to URL from `apps_connections_open()`, return message stream; handle `hello` frames (skip) and `envelope_id` acks
- [x] 2.8 Implement exponential backoff reconnect (1s → 60s, ±10% jitter) in the stream task
- [x] 2.9 Implement allowlist filtering (allowed_channels, allowed_users) inside `start()` stream
- [x] 2.10 Implement `send()` and `send_in_thread()` via `post_message()`
- [x] 2.11 Implement `send_reaction()` via `add_reaction()`
- [x] 2.12 Wire `SlackAdapter` into `SlackInterface::run()` replacing old `SlackCallbackState` + morphism runner
- [x] 2.13 Delete old morphism-based code (`runner.rs` callbacks, `SlackCallbackState`)
- [x] 2.14 Write unit tests using `wiremock` for `post_message()` and envelope ack logic

## 3. Mattermost: Thin reqwest Client

- [ ] 3.1 Remove `mattermost_api` from `crates/interface-mattermost/Cargo.toml`; add `tokio-tungstenite`
- [ ] 3.2 Create `crates/interface-mattermost/src/client.rs` — `MattermostClient` struct (server_url, token, reqwest::Client)
- [ ] 3.3 Implement `get_me()` in `MattermostClient` to resolve the bot's own user ID
- [ ] 3.4 Implement `create_post()` in `MattermostClient` (`POST /api/v4/posts` with channel_id, message, optional root_id)
- [ ] 3.5 Create `crates/interface-mattermost/src/adapter.rs` — `MattermostAdapter` implementing `ChannelAdapter`
- [ ] 3.6 Implement `start()`: connect WebSocket to `wss://<server>/api/v4/websocket`, send auth challenge, stream `posted` events
- [ ] 3.7 Filter self-messages by comparing `user_id` to bot's own ID
- [ ] 3.8 Implement exponential backoff reconnect with re-authentication on disconnect
- [ ] 3.9 Implement `send()` and `send_in_thread()` via `create_post()`
- [ ] 3.10 Wire `MattermostAdapter` into `MattermostInterface::run()` replacing old `MattermostHandler`
- [ ] 3.11 Delete old `mattermost_api`-based handler code
- [ ] 3.12 Write unit tests using `wiremock` for `create_post()` and WebSocket auth logic

## 4. Matrix: Thin reqwest Client (long-poll sync)

- [ ] 4.1 Remove `matrix-sdk` from `crates/interface-matrix/Cargo.toml`
- [ ] 4.2 Create `crates/interface-matrix/src/client.rs` — `MatrixClient` struct (homeserver_url, access_token, reqwest::Client)
- [ ] 4.3 Implement `sync()` in `MatrixClient`: `GET /_matrix/client/v3/sync?since=<token>&timeout=30000`, return deserialized response
- [ ] 4.4 Implement `send_message()` in `MatrixClient`: `PUT /_matrix/client/v3/rooms/<room_id>/send/m.room.message/<txn_id>`; generate unique txn_id (UUID v4)
- [ ] 4.5 Implement `next_batch` token persistence: read from / write to `~/.assistant/matrix-next-batch-<user_id>.txt`
- [ ] 4.6 Create `crates/interface-matrix/src/adapter.rs` — `MatrixAdapter` implementing `ChannelAdapter`
- [ ] 4.7 Implement `start()`: loop calling `sync()`, parse `m.room.message` / `m.text` events, yield `ChannelMessage`, persist `next_batch` after each call
- [ ] 4.8 Filter own messages (sender == bot MXID) and apply allowed_rooms / allowed_users allowlist
- [ ] 4.9 Implement `send()` via `send_message()`
- [ ] 4.10 Wire `MatrixAdapter` into `MatrixInterface::run()` replacing old SDK-based sync loop
- [ ] 4.11 Delete old `matrix-sdk`-based code (login helpers, state store setup)
- [ ] 4.12 Write unit tests using `wiremock` for sync response parsing and send_message

## 5. Nextcloud: Align to ChannelAdapter Trait

- [ ] 5.1 Create `crates/interface-nextcloud/src/adapter.rs` — `NextcloudAdapter` implementing `ChannelAdapter` wrapping existing webhook handler
- [ ] 5.2 Map existing `WebhookEvent::Create` → `ChannelMessage`; expose webhook as `start()` stream via an internal `tokio::sync::mpsc` channel fed by the axum handler
- [ ] 5.3 Implement `send()` via existing `NextcloudReplyHandler` HTTP call
- [ ] 5.4 Wire `NextcloudAdapter` into `NextcloudInterface::run()`
- [ ] 5.5 Keep HMAC-SHA256 signing logic unchanged

## 6. Signal: Wrap Behind ChannelAdapter Trait

- [ ] 6.1 Create `crates/interface-signal/src/adapter.rs` — `SignalAdapter` implementing `ChannelAdapter` (feature-gated with `signal`)
- [ ] 6.2 Map presage `Received` messages → `ChannelMessage` inside `start()` stream
- [ ] 6.3 Implement `send()` via `manager.send_message()`
- [ ] 6.4 Wire `SignalAdapter` into `SignalInterface::run()`
- [ ] 6.5 Keep presage dependency unchanged (not replacing Signal protocol)

## 7. Cleanup and Verification

- [ ] 7.1 Run `make lint` and fix all clippy warnings
- [ ] 7.2 Run `make format` to ensure consistent formatting
- [ ] 7.3 Run `make test` — all unit tests pass
- [ ] 7.4 Verify `slack-morphism` is absent from `cargo tree` output
- [ ] 7.5 Verify `mattermost_api` is absent from `cargo tree` output
- [ ] 7.6 Verify `matrix-sdk` is absent from `cargo tree` output
- [ ] 7.7 Run `make build` to confirm the full workspace builds cleanly
- [ ] 7.8 Update `AGENTS.md` workspace table with any crate changes
