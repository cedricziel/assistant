## ADDED Requirements

### Requirement: ChannelAdapter trait

The system SHALL define a `ChannelAdapter` trait in `assistant-core` with the following async methods:

- `start(&self) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>>` — begins receiving messages from the channel and returns an async stream.
- `send(&self, user: &ChannelUser, content: ChannelContent) -> Result<()>` — delivers a message to the user.
- `stop(&self) -> Result<()>` — gracefully shuts down the adapter.
- `send_typing(&self, user: &ChannelUser) -> Result<()>` — optional; default no-op.
- `send_reaction(&self, user: &ChannelUser, message_id: &str, reaction: &str) -> Result<()>` — optional; default no-op.
- `send_in_thread(&self, user: &ChannelUser, content: ChannelContent, thread_id: &str) -> Result<()>` — optional; default delegates to `send()`.
- `name(&self) -> &str` — returns a human-readable adapter name.
- `channel_type(&self) -> ChannelType` — returns the platform enum variant.

#### Scenario: Adapter starts and yields messages

- **WHEN** `start()` is called on a configured adapter
- **THEN** the returned stream yields `ChannelMessage` items for each incoming platform event

#### Scenario: Optional methods have default no-ops

- **WHEN** an adapter does not override `send_typing()`
- **THEN** calling `send_typing()` returns `Ok(())` without panicking

#### Scenario: Adapter stops cleanly

- **WHEN** `stop()` is called
- **THEN** the stream returned from `start()` terminates and no further messages are yielded

---

### Requirement: ChannelMessage unified event type

The system SHALL define a `ChannelMessage` struct in `assistant-core` with fields:

- `channel_type: ChannelType` — the originating platform.
- `platform_message_id: Option<String>` — platform-native ID for dedup/threading.
- `sender: ChannelUser` — who sent the message.
- `content: ChannelContent` — what was sent.
- `thread_id: Option<String>` — thread or conversation anchor on the platform.
- `timestamp: DateTime<Utc>` — when the message was sent.
- `metadata: HashMap<String, serde_json::Value>` — platform-specific extras (channel name, workspace, etc.).

#### Scenario: Text message round-trip

- **WHEN** a user sends a plain text message on any channel
- **THEN** the adapter yields a `ChannelMessage` with `content: ChannelContent::Text(String)`

#### Scenario: File attachment surfaced

- **WHEN** a user sends a file upload on a supported channel
- **THEN** the adapter yields a `ChannelMessage` with `content: ChannelContent::FileData { data, filename, mime_type }`

---

### Requirement: ChannelContent enum

The system SHALL define a `ChannelContent` enum in `assistant-core` with variants:

- `Text(String)`
- `Image { url: String, caption: Option<String> }`
- `File { url: String, filename: String }`
- `FileData { data: Vec<u8>, filename: String, mime_type: String }`
- `Voice { url: String, duration_seconds: Option<u32> }`

#### Scenario: Text content serialization

- **WHEN** `ChannelContent::Text("hello".into())` is used in a send call
- **THEN** the adapter sends the exact string to the platform without wrapping

---

### Requirement: ChannelUser type

The system SHALL define a `ChannelUser` struct in `assistant-core` with fields:

- `platform_id: String` — platform-native user identifier.
- `display_name: Option<String>` — human-readable name, if available.

#### Scenario: User identity preserved across message and reply

- **WHEN** a `ChannelMessage` is received and a reply is sent via `send()`
- **THEN** the `ChannelUser` from the incoming message is passed to `send()` and the platform routes the reply correctly

---

### Requirement: ChannelType enum

The system SHALL define a `ChannelType` enum in `assistant-core` with variants matching the `Interface` enum for parity, including at minimum: `Slack`, `Mattermost`, `Matrix`, `Nextcloud`, `Signal`, and `Custom(String)`.

#### Scenario: Channel type identifies the platform

- **WHEN** an adapter returns a `ChannelMessage`
- **THEN** `channel_type` matches the adapter's `channel_type()` return value
