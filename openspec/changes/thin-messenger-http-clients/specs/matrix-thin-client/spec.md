## ADDED Requirements

### Requirement: Matrix adapter uses reqwest only (no matrix-sdk)

The Matrix adapter SHALL depend solely on `reqwest` for HTTP transport. The `matrix-sdk` crate SHALL NOT be a dependency of `interface-matrix`.

#### Scenario: Build succeeds without matrix-sdk

- **WHEN** `cargo build -p assistant-interface-matrix` is run
- **THEN** compilation succeeds and `matrix-sdk` does not appear in the dependency tree

---

### Requirement: Matrix sync loop via Client-Server spec

The Matrix adapter SHALL poll `GET /_matrix/client/v3/sync` with a `timeout=30000` query parameter (long-poll), using the access token for authentication. On each response, the adapter SHALL process new room events and persist the `next_batch` token to disk so syncing survives restarts.

#### Scenario: First sync with no saved token

- **WHEN** the adapter starts and no `next_batch` token is persisted
- **THEN** the adapter calls `GET /_matrix/client/v3/sync` without a `since` parameter

#### Scenario: Subsequent sync resumes from token

- **WHEN** a `next_batch` token is persisted from a prior run
- **THEN** the adapter calls `GET /_matrix/client/v3/sync?since=<token>&timeout=30000`

#### Scenario: Text message event yields ChannelMessage

- **WHEN** the sync response includes an `m.room.message` event with `msgtype: m.text`
- **THEN** the adapter yields a `ChannelMessage` with `content: ChannelContent::Text(...)`

---

### Requirement: Matrix next_batch token persistence

The Matrix adapter SHALL persist the `next_batch` token after each successful sync to `~/.assistant/matrix-next-batch-<user_id>.txt` (or a configurable path). On startup the adapter SHALL read this file if it exists.

#### Scenario: Token survives restart

- **WHEN** the adapter shuts down after receiving events
- **THEN** the `next_batch` token is written to the persistence file
- **WHEN** the adapter restarts
- **THEN** the token is read and passed as `since` in the first sync call

---

### Requirement: Matrix reply via sendMessage

The Matrix adapter SHALL send replies via `PUT /_matrix/client/v3/rooms/<room_id>/send/m.room.message/<txn_id>` with a JSON body `{"msgtype":"m.text","body":"<text>"}`.

#### Scenario: Reply sent to correct room

- **WHEN** `send()` is called with a `ChannelUser` whose `platform_id` encodes the room ID
- **THEN** the PUT request targets that room ID

#### Scenario: Unique transaction ID per send

- **WHEN** `send()` is called twice
- **THEN** each PUT request uses a different `txn_id` to prevent duplicate delivery

---

### Requirement: Matrix allowlist filtering

The Matrix adapter SHALL filter incoming messages against configured `allowed_rooms` and `allowed_users` lists. Messages from rooms or users not on the lists SHALL be silently dropped.

#### Scenario: Message from disallowed room dropped

- **WHEN** a sync event arrives from a room not in `allowed_rooms` (and the list is non-empty)
- **THEN** no `ChannelMessage` is yielded

#### Scenario: Bot own-messages filtered

- **WHEN** an `m.room.message` event has a `sender` matching the bot's own MXID
- **THEN** no `ChannelMessage` is yielded
