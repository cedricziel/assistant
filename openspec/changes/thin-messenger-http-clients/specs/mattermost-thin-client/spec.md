## ADDED Requirements

### Requirement: Mattermost adapter uses reqwest and tokio-tungstenite only

The Mattermost adapter SHALL depend solely on `reqwest` and `tokio-tungstenite` for HTTP and WebSocket transport. The `mattermost_api` crate SHALL NOT be a dependency of `interface-mattermost`.

#### Scenario: Build succeeds without mattermost_api

- **WHEN** `cargo build -p assistant-interface-mattermost` is run
- **THEN** compilation succeeds and `mattermost_api` does not appear in the dependency tree

---

### Requirement: Mattermost WebSocket event stream

The Mattermost adapter SHALL connect to `wss://<server>/api/v4/websocket` using `tokio-tungstenite`, authenticate with a `{"seq":1,"action":"authentication_challenge","data":{"token":"<token>"}}` frame, and yield `ChannelMessage` for each incoming `posted` event.

#### Scenario: Authentication handshake

- **WHEN** the WebSocket connects
- **THEN** the adapter sends an authentication challenge with the configured token within 1 second

#### Scenario: Posted event becomes ChannelMessage

- **WHEN** a Mattermost WebSocket event with `"event":"posted"` is received
- **THEN** the adapter yields a `ChannelMessage` with `content: ChannelContent::Text(...)` and `sender` populated from the post's `user_id`

#### Scenario: Non-posted events are ignored

- **WHEN** a WebSocket event with `"event":"user_added"` or other non-message events is received
- **THEN** no `ChannelMessage` is yielded

---

### Requirement: Mattermost reconnect with exponential backoff

The Mattermost adapter SHALL reconnect automatically on WebSocket disconnect using exponential backoff starting at 1 second, doubling up to 60 seconds, with random jitter ±10%.

#### Scenario: Reconnect after disconnect

- **WHEN** the WebSocket closes unexpectedly
- **THEN** the adapter reconnects and re-authenticates without requiring a process restart

---

### Requirement: Mattermost reply via REST API

The Mattermost adapter SHALL post replies via `POST <server>/api/v4/posts` with a JSON body containing `channel_id`, `message`, and optionally `root_id` for threaded replies.

#### Scenario: Threaded reply

- **WHEN** `send_in_thread()` is called with a `thread_id`
- **THEN** the POST body includes `"root_id": "<thread_id>"`

#### Scenario: Top-level reply

- **WHEN** `send()` is called without a thread context
- **THEN** the POST body omits `root_id`

---

### Requirement: Mattermost self-message filtering

The Mattermost adapter SHALL ignore messages whose `user_id` matches the configured bot's own user ID to prevent reply loops.

#### Scenario: Bot message dropped

- **WHEN** a `posted` event arrives with `user_id` equal to the bot's own ID
- **THEN** no `ChannelMessage` is yielded
