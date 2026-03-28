# Matrix Interface

Connects the assistant to a [Matrix](https://matrix.org) homeserver. The bot joins rooms it is invited to and responds to messages via the matrix-sdk sync loop — no public URL required.

## Setup

### 1. Register a bot account

Create a dedicated Matrix account for the bot on your homeserver. You can use any Matrix client or the homeserver's admin API.

Example with `curl` against a Synapse homeserver:

```sh
curl -X POST "https://matrix.example.com/_matrix/client/v3/register" \
  -H "Content-Type: application/json" \
  -d '{"username": "assistant", "password": "yourpassword", "kind": "user"}'
```

### 2. Obtain an access token

Log in once to get a long-lived access token:

```sh
curl -X POST "https://matrix.example.com/_matrix/client/v3/login" \
  -H "Content-Type: application/json" \
  -d '{"type": "m.login.password", "user": "@assistant:example.com", "password": "yourpassword"}'
```

Copy the `access_token` from the response. Store it securely — you won't need the password again.

### 3. Invite the bot to a room

From any Matrix client, invite the bot account (`@assistant:example.com`) to the room(s) you want it to join. The bot automatically accepts invitations when it is running.

## Configuration

Add a `[matrix]` section to `~/.assistant/config.toml`:

```toml
[matrix]
homeserver_url = "https://matrix.example.com"
username       = "@assistant:example.com"
access_token   = "syt_abc123..."          # preferred

# Optional: use password login instead of an access token
# password = "yourpassword"

# Optional: restrict to specific room IDs (canonical form: !id:server)
# allowed_rooms = ["!abc123:example.com"]

# Optional: restrict to specific Matrix user IDs
# allowed_users = ["@alice:example.com"]

# Optional: custom path for the matrix-sdk SQLite state store
# Defaults to ~/.assistant/matrix-state/
# state_store_path = "/var/lib/assistant/matrix-state"
```

Environment variables take precedence over config file values:

```sh
export MATRIX_HOMESERVER_URL="https://matrix.example.com"
export MATRIX_USERNAME="@assistant:example.com"
export MATRIX_ACCESS_TOKEN="syt_abc123..."
# export MATRIX_PASSWORD="yourpassword"
```

## Running

**Matrix-only mode** (recommended for dedicated bot deployments):

```sh
assistant matrix
# or:
make run-matrix
```

**Alongside other interfaces** (REPL + Matrix + any other configured interface):

```sh
assistant orchestrator run --interfaces matrix
```

The bot logs in, starts the sync loop, and is ready once you see:

```text
INFO assistant_interface_matrix::runner: Matrix bot ready bot_user_id=@assistant:example.com
```

## Authentication

Two authentication paths are supported, tried in this order:

| Method         | When to use                 | How                                                                                                         |
| -------------- | --------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Access token   | Production / always-on bots | Set `access_token` in config or `MATRIX_ACCESS_TOKEN`                                                       |
| Password login | Initial setup or testing    | Set `username` + `password`; session is persisted to the state store so subsequent restarts are token-based |

> The state store (`~/.assistant/matrix-state/` by default) holds the sync token and session data. Deleting it forces a full re-sync on next start.

## Security

- Use `allowed_rooms` and `allowed_users` to restrict which rooms and users the bot responds to. Empty lists (the default) accept all.
- `allowed_rooms` entries must be canonical Matrix room IDs (e.g. `!abc123:example.com`), not room aliases or display names.
- Store `access_token` and `password` in environment variables rather than committing them to `config.toml`.

## Conversation continuity

Each Matrix room ID maps to a stable conversation UUID (held in an LRU cache of up to 10,000 entries). Context is preserved across all messages in the same room for the lifetime of the process.

Direct messages are regular Matrix rooms, so per-DM context isolation is automatic.

## Reconnection

If the homeserver becomes unreachable, the bot retries with exponential backoff starting at 1 second and capping at 60 seconds. No manual intervention is required.
