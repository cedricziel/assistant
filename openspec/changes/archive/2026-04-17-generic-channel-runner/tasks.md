## 1. Extend ChannelAdapter Trait with Hooks

- [x] 1.1 Add `conversation_key(&self, msg: &ChannelMessage) -> String` to `ChannelAdapter` in `crates/core/src/channel.rs` with default `"{sender.platform_id}:{thread_id ?? platform_message_id}"`
- [x] 1.2 Add `fn platform_tools(&self, _msg: &ChannelMessage, _conv_id: Uuid) -> Vec<Arc<dyn ToolHandler>>` to `ChannelAdapter` with default `vec![]`
- [x] 1.3 Add `async fn on_turn_start(&self, _user: &ChannelUser) -> Result<()>` with default `Ok(())`
- [x] 1.4 Add `async fn on_turn_success(&self, _user: &ChannelUser, _result: &TurnResult) -> Result<()>` with default `Ok(())`
- [x] 1.5 Add `async fn on_turn_error(&self, _user: &ChannelUser, _err: &anyhow::Error) -> Result<()>` with default `Ok(())`
- [x] 1.6 Export `TurnResult` from `assistant-core` (re-export from runtime) or thread it through — confirm the right crate boundary
- [x] 1.7 Ensure `crates/core` `Cargo.toml` has `uuid` and `assistant-tool-executor` (or define platform_tools return type as a local alias) — check and adjust deps as needed

## 2. Implement ChannelRunner in crates/runtime

- [x] 2.1 Create `crates/runtime/src/channel_runner.rs` with `ChannelRunner { adapter, orchestrator, conversations: Mutex<LruCache<String, Uuid>>, conv_locks: Mutex<HashMap<Uuid, Arc<Mutex<()>>>> }`
- [x] 2.2 Implement `ChannelRunner::new(adapter: Arc<dyn ChannelAdapter>, orchestrator: Arc<Orchestrator>) -> Self`
- [x] 2.3 Implement `ChannelRunner::run(&self) -> Result<()>`: call `adapter.start()`, enter `loop { select! { shutdown, msg } }`
- [x] 2.4 Implement `resolve_conv_id(&self, key: &str) -> Uuid` using the LRU cache with new-UUID-on-miss
- [x] 2.5 Implement `get_conv_lock(&self, conv_id: Uuid) -> Arc<Mutex<()>>` creating lock on first encounter
- [x] 2.6 Implement `dispatch(&self, msg: ChannelMessage)`: spawn task → acquire lock → `on_turn_start` → `platform_tools` → `run_turn_with_tools` → `send_in_thread/send` → `on_turn_success/on_turn_error`
- [x] 2.7 Add cross-platform shutdown signal helper (reuse or inline the SIGTERM/Ctrl-C future pattern already used in runners)
- [x] 2.8 Export `ChannelRunner` from `crates/runtime/src/lib.rs`
- [x] 2.9 Add `lru` to `crates/runtime/Cargo.toml` if not already present

## 3. Migrate Mattermost to ChannelRunner

- [x] 3.1 Override `conversation_key()` on `MattermostAdapter` to use `{channel_id}:{root_id ?? post_id}`
- [x] 3.2 Override `platform_tools()` on `MattermostAdapter` to call `build_mattermost_tools(...)` using metadata from the message
- [x] 3.3 Replace body of `MattermostInterface::run()` with `ChannelRunner::new(Arc::new(MattermostAdapter::new(cfg)), orchestrator).run().await`
- [x] 3.4 Delete the old dispatch loop, LRU cache, and conv_locks boilerplate from `runner.rs`
- [x] 3.5 Run `make check` — fix any compile errors

## 4. Migrate Matrix to ChannelRunner

- [x] 4.1 Override `conversation_key()` on `MatrixAdapter` to return `sender.platform_id` (room ID only)
- [x] 4.2 Override `platform_tools()` on `MatrixAdapter` to call `build_matrix_tools(room_id, client.clone())`
- [x] 4.3 Replace body of `MatrixInterface::run()` with `ChannelRunner::new(Arc::new(adapter), orchestrator).run().await`
- [x] 4.4 Delete the old dispatch boilerplate from `runner.rs`
- [x] 4.5 Run `make check` — fix any compile errors

## 5. Migrate Nextcloud to ChannelRunner

- [x] 5.1 Verify `NextcloudAdapter::start()` correctly spawns the axum server and returns a `ReceiverStream` — fix if it currently returns an error
- [x] 5.2 Override `platform_tools()` on `NextcloudAdapter` if any Nextcloud-specific tools exist
- [x] 5.3 Override `on_turn_start()` on `NextcloudAdapter` to post the hourglass reaction (if present in current runner)
- [x] 5.4 Override `on_turn_success()` on `NextcloudAdapter` to remove the hourglass reaction (if present)
- [x] 5.5 Replace body of `NextcloudInterface::run()` with `ChannelRunner::new(Arc::new(adapter), orchestrator).run().await`
- [x] 5.6 Delete the old axum-in-runner boilerplate from `runner.rs`
- [x] 5.7 Run `make check` — fix any compile errors

## 6. Migrate Slack to ChannelRunner

- [x] 6.1 Override `conversation_key()` on `SlackAdapter` to encode `{channel_id}:{thread_ts ?? message_ts}`
- [x] 6.2 Override `platform_tools()` on `SlackAdapter` to return the ambient Slack tools (post, react, reply, send-dm, etc.) using metadata from the message
- [x] 6.3 Override `on_turn_start()` on `SlackAdapter` to add the 👀 reaction via `add_reaction()`
- [x] 6.4 Override `on_turn_success()` on `SlackAdapter` to add the ✅ reaction
- [x] 6.5 Override `on_turn_error()` on `SlackAdapter` to post an error message to the channel
- [x] 6.6 Resolve history seeding: move `seed_thread_history()` call into `on_turn_start()` (adapter needs `Arc<StorageLayer>` stored at construction time) — add field to `SlackAdapter` and pass from `SlackInterface`
- [x] 6.7 Replace body of `SlackInterface::run()` with `ChannelRunner::new(Arc::new(adapter), orchestrator).run().await`
- [x] 6.8 Delete the old dispatch loop and boilerplate from `runner.rs`
- [x] 6.9 Run `make check` — fix any compile errors

## 7. Cleanup and Verification

- [x] 7.1 Delete any dead helper functions left behind in each runner (empty `dispatch_message` functions, orphaned imports)
- [x] 7.2 Run `make lint` and fix all clippy warnings
- [x] 7.3 Run `make format`
- [x] 7.4 Run `make test` — all unit tests pass
- [x] 7.5 Run `cargo machete --with-metadata` — no unused deps
- [x] 7.6 Run `make build` — full workspace builds cleanly
- [x] 7.7 Update `AGENTS.md` workspace table entry for `assistant-runtime` to mention `ChannelRunner`
