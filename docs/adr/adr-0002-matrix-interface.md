# ADR-0002: Matrix Messaging Interface

- Status: Accepted
- Date: 2026-03-28
- Decision Makers: Core maintainers

## Context

The assistant already supports several messaging interfaces (Slack, Mattermost, Nextcloud Talk, Signal).
Adding Matrix support enables deployment to self-hosted and federated Matrix homeservers, which is a
popular choice in privacy-conscious communities and enterprise environments that operate their own servers.

Matrix (via the matrix-sdk Rust crate) provides a well-maintained, type-safe SDK that maps naturally
onto the existing interface pattern used by the Mattermost and Slack runners.

## Decision

### 1) Library choice: matrix-sdk 0.10

We use `matrix-sdk` (the official Matrix Rust SDK) rather than a lower-level HTTP library.
It handles session state, sync loop management, event deserialization, and room membership transparently.
The 0.10 release is the latest stable series and has a stable async event-handler API.

### 2) Authentication strategy: access-token preferred, password fallback

Two auth paths are supported:

- **Access token** (`access_token` + optional `device_id`): preferred for production deployments.
  The token is issued once (e.g. via `curl` or Element) and stored in `config.toml`. No password
  is transmitted on restart.
- **Password login** (`username` + `password`): convenient for development. The SDK persists the
  session to the SQLite state store after first login so subsequent restarts restore the existing
  session without re-issuing a new device.

### 3) Conversation keying: room_id only

Each Matrix room maps 1-to-1 to a single LLM conversation UUID. Unlike Mattermost (which keys on
`(channel_id, thread_root_id)`), Matrix does not have a first-class threading primitive that is
universally used, so we key on `room_id` alone. This matches the Slack DM and channel model and
provides the simplest mental model: one room = one ongoing conversation.

### 4) State store: matrix-sdk SQLite backend

The `sqlite` feature of `matrix-sdk` is enabled so sync state and session credentials are persisted
across restarts via an embedded SQLite database. The path defaults to
`~/.assistant/matrix-state/` and is configurable via `state_store_path` in `[matrix]`.

### 5) Auto-accept invitations

The runner registers a `StrippedRoomMemberEvent` handler that automatically accepts room invitations
addressed to the bot user. This is consistent with how other bots (Slack, Mattermost) work: the
bot responds everywhere it is invited. Operators can restrict this using the `allowed_rooms` list.

## Consequences

### Positive

- Extends the assistant to Matrix-compatible homeservers (self-hosted, federated, or matrix.org).
- Follows the established interface pattern so operators already familiar with Mattermost config
  can configure Matrix with minimal learning curve.
- Access-token auth avoids storing passwords in config after initial setup.

### Trade-offs

- `matrix-sdk` is a larger dependency than a raw HTTP client; compile times increase.
- E2E encryption is not enabled in this initial version (sqlite feature only; no `e2e-encryption`).
  Encrypted rooms will not receive messages. This is documented as a known limitation.
- The SQLite state store requires a writable directory at runtime.

## Non-Goals

- E2E encryption support (can be added in a later version by enabling the `e2e-encryption` feature).
- Bridging to other protocols via matrix-appservice.
- Multi-homeserver support in a single bot instance.
