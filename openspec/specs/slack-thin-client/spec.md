## ADDED Requirements

### Requirement: Slack adapter uses reqwest and tokio-tungstenite only

The Slack adapter SHALL depend solely on `reqwest` and `tokio-tungstenite` for HTTP and WebSocket transport. The `slack-morphism` crate SHALL NOT be a dependency of `interface-slack`.

#### Scenario: Build succeeds without slack-morphism

- **WHEN** `cargo build -p assistant-interface-slack` is run
- **THEN** compilation succeeds and `slack-morphism` does not appear in the dependency tree

---

### Requirement: Slack Socket Mode WebSocket connection

The Slack adapter SHALL connect to Slack's Socket Mode API by:

1. Requesting a WebSocket URL via `POST https://slack.com/api/apps.connections.open` using the app token.
2. Opening a `tokio-tungstenite` WebSocket to the returned URL.
3. Sending a JSON `{"envelope_id": "<id>", "type": "ack"}` acknowledgement for every received envelope.

#### Scenario: Adapter receives a Socket Mode hello

- **WHEN** the WebSocket connects
- **THEN** the adapter receives a `{"type":"hello"}` frame and does not yield a `ChannelMessage`

#### Scenario: Adapter acknowledges every envelope

- **WHEN** a Socket Mode envelope arrives with an `envelope_id`
- **THEN** the adapter sends `{"envelope_id": "<id>", "type": "ack"}` before yielding the `ChannelMessage`

---

### Requirement: Slack reconnect with exponential backoff

The Slack adapter SHALL reconnect automatically on WebSocket disconnect using exponential backoff starting at 1 second, doubling each attempt up to a maximum of 60 seconds, with random jitter ±10%.

#### Scenario: Reconnect after unexpected close

- **WHEN** the WebSocket connection closes unexpectedly
- **THEN** the adapter waits at least 1 second, then re-requests a new WebSocket URL and reconnects
- **THEN** the stream continues yielding messages without requiring a restart

---

### Requirement: Slack reply via chat.postMessage

The Slack adapter SHALL send replies via `POST https://slack.com/api/chat.postMessage` using the bot token. Thread replies SHALL set `thread_ts` from `ChannelMessage.thread_id`.

#### Scenario: Reply goes to the originating thread

- **WHEN** `send_in_thread()` is called with a `thread_id`
- **THEN** the POST body includes `"thread_ts": "<thread_id>"`

#### Scenario: Direct reply without thread

- **WHEN** `send()` is called with no thread context
- **THEN** the POST body includes the `channel` but omits `thread_ts`

---

### Requirement: Slack allowlist filtering

The Slack adapter SHALL filter incoming messages against configured `allowed_channels` and `allowed_users` lists. Messages from channels or users not on the lists SHALL be silently dropped (no `ChannelMessage` yielded).

#### Scenario: Message from disallowed channel dropped

- **WHEN** a message arrives from a channel not in `allowed_channels` (and `allowed_channels` is non-empty)
- **THEN** no `ChannelMessage` is yielded for that message

#### Scenario: Empty allowlist permits all

- **WHEN** `allowed_channels` is empty
- **THEN** messages from all channels are yielded
