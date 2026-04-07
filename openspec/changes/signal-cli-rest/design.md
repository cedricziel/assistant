---
name: signal-cli-rest
description: Design for signal-cli REST adapter replacing presage
type: design
---

## Context

The `interface-signal` crate currently wraps [presage](https://github.com/whisperfish/presage), a Rust implementation of the Signal client protocol. Presage uses non-`Send` types (its `Manager` holds `!Send` state), requires a dedicated SQLite state store for the Signal protocol state, and ships 40k+ lines of transitive dependencies. Because of the non-`Send` constraint, `SignalAdapter` could never be wired into the `ChannelAdapter` / `ChannelRunner` pattern — the presage event loop must live in its own `spawn_local`/`spawn_blocking` context.

[signal-cli-rest-api](https://github.com/bbernhard/signal-cli-rest-api) is a widely-deployed Docker image that wraps the `signal-cli` Java binary and exposes a REST + WebSocket API. Bots talk HTTP/JSON — no Signal protocol knowledge required. OpenFang uses this pattern for all its Signal channels.

## Goals / Non-Goals

**Goals:**

- `SignalAdapter` implements `ChannelAdapter` (fully `Send + Sync`).
- `SignalInterface::run()` delegates to `ChannelRunner`.
- Remove presage and all feature gates.
- Device registration and linking handled externally by the signal-cli daemon.

**Non-Goals:**

- Bundling or managing the signal-cli daemon lifecycle.
- Supporting E2EE beyond what signal-cli exposes by default.
- Maintaining backwards-compatible `store_path` — operators must migrate to the daemon setup.

## Decisions

### 1. Transport: WebSocket receive, REST send

**Decision**: `SignalAdapter::start()` connects to the signal-cli-rest-api WebSocket endpoint `GET /v1/receive/{number}` for inbound messages. `send()` POSTs to `POST /v1/send`.

**Rationale**: The WebSocket provides a long-lived push connection, consistent with Slack Socket Mode and Mattermost WebSocket. REST send is simpler than a persistent outbound connection.

**Alternative considered**: Polling `GET /v1/receive/{number}` (HTTP long-poll). Rejected — the WebSocket is available and avoids repeated polling overhead.

### 2. Authentication: HTTP Basic Auth or none

**Decision**: Support optional HTTP Basic Auth credentials in `SignalConfig` (`api_url` + optional `api_user` / `api_password`). Default is no auth (localhost daemon).

**Rationale**: signal-cli-rest-api supports HTTP Basic Auth via environment variable (`-e API_AUTH_USERNAME`). Most local deployments don't use it but remote deployments should.

### 3. Message format

The signal-cli-rest-api WebSocket delivers JSON envelopes:

```json
{
  "envelope": {
    "source": "+14155550123",
    "sourceNumber": "+14155550123",
    "sourceUuid": "uuid-...",
    "sourceName": "Alice",
    "sourceDevice": 1,
    "timestamp": 1234567890,
    "dataMessage": {
      "timestamp": 1234567890,
      "message": "Hello",
      "groupInfo": { "groupId": "...", "type": "DELIVER" }
    }
  }
}
```

`conversation_key` = `source` (phone number / UUID) for 1:1, or `groupInfo.groupId` for groups.
`platform_id` on `ChannelUser` = `source` (used for reply routing).

### 4. Remove linker / presage feature gate

**Decision**: Delete `linker.rs`, `main.rs`, and the `signal` Cargo feature entirely. Remove `SignalCommand::Link` from the CLI.

**Rationale**: Device setup is now an operator concern (run the signal-cli daemon, register the number). No in-process linking is possible or needed.

### 5. SignalConfig changes

```rust
pub struct SignalConfig {
    pub phone_number: Option<String>,           // bot's number (required)
    pub api_url: Option<String>,                // default: http://localhost:8080
    pub api_user: Option<String>,               // HTTP Basic Auth username (optional)
    pub api_password: Option<String>,           // HTTP Basic Auth password (optional)
    pub allowed_senders: Vec<String>,           // phone numbers / UUIDs allowlist
}
```

`store_path` is removed — the daemon manages storage.

## Risks / Trade-offs

- [Requires external signal-cli daemon] → Mitigation: document in README; provide docker-compose example.
- [WebSocket reconnection] → Mitigation: use the same exponential-backoff pattern as SlackAdapter.
- [Group message routing] → `groupInfo.groupId` used as conversation key; reply goes to the group via `POST /v1/send` with `group_id`.
- [No device linking in-process] → Breaking change for existing presage users; document migration path.

## Migration Plan

1. Operator sets up signal-cli-rest-api daemon, registers their number.
2. Update `config.toml`: replace `[signal] store_path` with `api_url`.
3. Remove `--features signal` from any build scripts.
4. Restart assistant — no database migration needed.
