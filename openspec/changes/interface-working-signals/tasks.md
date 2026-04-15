## 1. Core Trait & Runner (assistant-core + assistant-runtime)

- [ ] 1.1 Add `on_message_received(&self, msg: &ChannelMessage) -> Result<()>` default no-op to `ChannelAdapter` trait in `crates/core/src/channel.rs`
- [ ] 1.2 Write unit test confirming the default implementation returns `Ok(())`
- [ ] 1.3 Call `adapter.on_message_received(&channel_msg).await` in `ChannelRunner::run()` after resolving `conv_id`, before `tokio::spawn`; log failures at `warn!` and continue dispatch
- [ ] 1.4 Write `ChannelRunner` test: verify `on_message_received` is called before the per-conversation lock is acquired

## 2. Slack — Hourglass + Agent Status (assistant-interface-slack)

- [ ] 2.1 Add `set_agent_status(channel_id, thread_ts, status, loading_messages) -> Result<()>` to `SlackClient` calling `assistant.threads.setStatus`
- [ ] 2.2 Write unit test for `set_agent_status` using `wiremock` (verify endpoint, body shape, bot token header)
- [ ] 2.3 Override `on_message_received` in `SlackAdapter`: add `:hourglass_flowing_sand:` reaction to the triggering message (best-effort)
- [ ] 2.4 Update `SlackAdapter::on_turn_start`: remove `:hourglass_flowing_sand:` reaction, then add 👀, then call `set_agent_status` with rotating loading messages
- [ ] 2.5 Write adapter-level test for the hourglass add → remove → 👀 + setStatus sequence

## 3. Matrix — Hourglass + Typing (assistant-interface-matrix)

- [ ] 3.1 Add `send_typing(room_id: &str, typing: bool) -> Result<()>` to `MatrixClient`
- [ ] 3.2 Add `add_reaction(room_id: &str, event_id: &str, emoji: &str) -> Result<String>` to `MatrixClient` (returns reaction event ID)
- [ ] 3.3 Add `redact_event(room_id: &str, event_id: &str) -> Result<()>` to `MatrixClient`
- [ ] 3.4 Write unit tests for all three new `MatrixClient` methods using `wiremock`
- [ ] 3.5 Add `pending_reactions: Mutex<HashMap<String, String>>` (platform_id → reaction_event_id) to `MatrixAdapter`
- [ ] 3.6 Override `on_message_received` in `MatrixAdapter`: call `add_reaction` with `"⏳"`, store returned event ID
- [ ] 3.7 Override `on_turn_start` in `MatrixAdapter`: redact stored ⏳ event ID, then call `send_typing(room_id, true)` (both best-effort)
- [ ] 3.8 Override `on_turn_success` and `on_turn_error` in `MatrixAdapter`: call `send_typing(room_id, false)` (best-effort)

## 4. Mattermost — Hourglass + Typing (assistant-interface-mattermost)

- [ ] 4.1 Add `send_typing(channel_id: &str) -> Result<()>` to `MattermostClient`
- [ ] 4.2 Add `add_reaction(user_id: &str, post_id: &str, emoji_name: &str) -> Result<()>` to `MattermostClient`
- [ ] 4.3 Add `remove_reaction(user_id: &str, post_id: &str, emoji_name: &str) -> Result<()>` to `MattermostClient`
- [ ] 4.4 Write unit tests for all three new `MattermostClient` methods using `wiremock`
- [ ] 4.5 Override `on_message_received` in `MattermostAdapter`: call `add_reaction` with `"hourglass_flowing_sand"` using the post ID from `msg.platform_message_id`
- [ ] 4.6 Override `on_turn_start` in `MattermostAdapter`: call `remove_reaction("hourglass_flowing_sand")`, then `send_typing(channel_id)` (both best-effort)

## 5. Nextcloud — Hourglass + Typing (assistant-interface-nextcloud)

- [ ] 5.1 Add a `send_typing(conversation_token: &str, typing: bool) -> Result<()>` helper in the Nextcloud interface (using OCS API with `OCS-APIRequest: true`)
- [ ] 5.2 Add `add_reaction(token: &str, message_id: &str, reaction: &str) -> Result<()>` and `remove_reaction(token: &str, message_id: &str, reaction: &str) -> Result<()>` helpers
- [ ] 5.3 Write unit tests for all three helpers using `wiremock` (verify endpoints and OCS header)
- [ ] 5.4 Override `on_message_received` in `NextcloudAdapter`: call `add_reaction(token, message_id, "⏳")` (best-effort)
- [ ] 5.5 Override `on_turn_start` in `NextcloudAdapter`: call `remove_reaction(token, message_id, "⏳")`, then `send_typing(token, true)` (both best-effort)
- [ ] 5.6 Override `on_turn_success` and `on_turn_error` in `NextcloudAdapter`: call `send_typing(token, false)` (best-effort)

## 6. Validation

- [ ] 6.1 Run `make lint` — zero warnings
- [ ] 6.2 Run `make format` — no diff
- [ ] 6.3 Run `make test` — all tests pass
