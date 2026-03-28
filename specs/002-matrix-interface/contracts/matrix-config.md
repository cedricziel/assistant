# Contract: Matrix Interface Configuration

**Format**: TOML section in `~/.assistant/config.toml`
**Section key**: `[matrix]`
**Environment variable fallbacks**: See per-field notes

## Schema

```toml
[matrix]
# Required: URL of the Matrix homeserver
homeserver_url = "https://matrix.example.com"

# Required: Full Matrix user ID of the bot account
username = "@assistant:example.com"

# One of password or access_token is required
# Use access_token for production (avoids repeated logins)
access_token = "syt_..."         # preferred
# password = "secret"            # alternative: used for initial login + session persist

# Optional: device ID for session restoration (auto-generated if omitted)
# device_id = "ASSISTANTBOT"

# Optional: path for matrix-sdk state store (SQLite)
# Default: ~/.assistant/matrix-state/
# state_store_path = "/var/lib/assistant/matrix-state"

# Optional: restrict to specific room IDs (canonical, e.g. !roomid:server)
# Empty list (default) = accept messages from all rooms
# allowed_rooms = ["!abc123:example.com", "!def456:example.com"]

# Optional: restrict to specific Matrix user IDs
# Empty list (default) = accept messages from all users
# allowed_users = ["@alice:example.com", "@bob:example.com"]
```

## Environment Variable Overrides

All fields can be provided via environment variables (useful for container deployments):

| Config field     | Environment variable    | Notes                       |
| ---------------- | ----------------------- | --------------------------- |
| `homeserver_url` | `MATRIX_HOMESERVER_URL` | Overrides config file value |
| `username`       | `MATRIX_USERNAME`       | Overrides config file value |
| `password`       | `MATRIX_PASSWORD`       | Overrides config file value |
| `access_token`   | `MATRIX_ACCESS_TOKEN`   | Overrides config file value |

## Minimal Viable Config (access token)

```toml
[matrix]
homeserver_url = "https://matrix.example.com"
username = "@assistant:example.com"
access_token = "syt_abc123..."
```

## Minimal Viable Config (password login)

```toml
[matrix]
homeserver_url = "https://matrix.example.com"
username = "@assistant:example.com"
password = "bot-account-password"
```

The session (access token + device ID) will be persisted to `state_store_path` on first successful login to avoid repeated password logins on restart.

## CLI Invocation

```sh
# Matrix-only mode (no interactive REPL)
assistant matrix

# Multi-interface mode (Matrix + other interfaces)
assistant orchestrator run --interfaces matrix,slack
```

## Subcommand

The `assistant matrix` subcommand starts the bot in Matrix-only mode (equivalent to `--interfaces matrix --no-repl`). This is the recommended deployment pattern for a dedicated Matrix bot process.

## Error Conditions

| Condition                                     | Behaviour                                         |
| --------------------------------------------- | ------------------------------------------------- |
| `homeserver_url` missing                      | Startup fails with a clear error message          |
| `username` missing                            | Startup fails with a clear error message          |
| Neither `password` nor `access_token` present | Startup fails with a clear error message          |
| Homeserver unreachable at startup             | Startup fails; operator must resolve connectivity |
| Homeserver becomes unreachable at runtime     | Exponential backoff reconnect (1 s → 60 s cap)    |
| Message in non-allowed room                   | Warning log; silently discarded                   |
| Message from non-allowed user                 | Warning log; silently discarded                   |
