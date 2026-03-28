# Quickstart: Matrix Interface

**Branch**: `002-matrix-interface` | **Date**: 2026-03-28

## Prerequisites

1. A running Matrix homeserver (e.g. Synapse at `https://matrix.example.com`).
2. A dedicated Matrix bot account registered on that homeserver.
3. An access token for the bot account (or the account password for initial login).

## 1. Configure the bot

Add a `[matrix]` section to `~/.assistant/config.toml`:

```toml
[matrix]
homeserver_url = "https://matrix.example.com"
username = "@assistant:example.com"
access_token = "syt_abc123..."          # obtain from homeserver admin panel or login API
```

Alternatively set environment variables:

```sh
export MATRIX_HOMESERVER_URL="https://matrix.example.com"
export MATRIX_USERNAME="@assistant:example.com"
export MATRIX_ACCESS_TOKEN="syt_abc123..."
```

## 2. Invite the bot to a room

Using any Matrix client, invite `@assistant:example.com` to the room(s) you want the bot to participate in. The bot will auto-accept invitations.

## 3. Start the bot

**Matrix-only mode** (recommended for dedicated deployments):

```sh
assistant matrix
# or equivalently:
make run-matrix
```

**Multi-interface mode** (alongside CLI REPL and other interfaces):

```sh
assistant orchestrator run --interfaces matrix
```

## 4. Send a message

In your Matrix client, send a message in any room where the bot is a member. Address it directly (by name) or just send a message — by default the bot responds to all messages in rooms it belongs to.

Example:

```
What is the capital of France?
```

The bot will reply in the same room.

## 5. Restrict access (optional)

To limit which rooms or users can trigger the bot, add allowlists:

```toml
[matrix]
homeserver_url = "https://matrix.example.com"
username = "@assistant:example.com"
access_token = "syt_abc123..."
allowed_rooms = ["!abc123:example.com"]
allowed_users = ["@alice:example.com"]
```

## Troubleshooting

| Symptom                               | Likely Cause                | Fix                                                             |
| ------------------------------------- | --------------------------- | --------------------------------------------------------------- |
| `No Matrix homeserver URL configured` | Missing config              | Add `homeserver_url` to config or set `MATRIX_HOMESERVER_URL`   |
| Bot does not respond                  | Room not in `allowed_rooms` | Add the room ID to `allowed_rooms` or clear the list            |
| Bot responds to its own messages      | SDK version issue           | Update `matrix-sdk`; self-filter uses `client.user_id()`        |
| Slow initial startup                  | Full initial sync           | Normal on first run; `state_store_path` reduces this on restart |
| Connection drops frequently           | Homeserver rate limiting    | Reduce traffic; backoff is automatic (up to 60 s)               |
