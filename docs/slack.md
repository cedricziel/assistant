# Slack Interface

Connects the assistant to Slack via [Socket Mode](https://api.slack.com/apis/socket-mode) — no public URL required.

## Setup

### 1. Create a Slack app

Go to [api.slack.com/apps](https://api.slack.com/apps) → **Create New App** → **From scratch**.

### 2. Enable Socket Mode

**App Settings → Socket Mode → Enable**

This generates an **App-Level Token** (`xapp-...`) with scope `connections:write`. Copy it.

### 3. Add Bot Token scopes

**OAuth & Permissions → Bot Token Scopes:**

| Scope              | Purpose                  |
| ------------------ | ------------------------ |
| `chat:write`       | Post replies             |
| `channels:history` | Read messages            |
| `channels:read`    | List channels (optional) |
| `im:history`       | Read DMs (optional)      |

### 4. Subscribe to events

**Event Subscriptions → Enable → Subscribe to bot events:**

- `message.channels` — messages in public channels
- `message.im` — direct messages (optional)

### 5. Install the app

Install to your workspace. Copy the **Bot Token** (`xoxb-...`).

### 6. Invite the bot to a channel

```
/invite @your-bot-name
```

## Configuration

Add to `~/.assistant/config.toml`:

```toml
[slack]
bot_token = "xoxb-..."   # Bot OAuth token
app_token = "xapp-..."   # App-level token (Socket Mode)

# Optional: restrict to specific channels or users
# allowed_channels = ["C0123456789"]
# allowed_users    = ["U0123456789"]
```

Environment variables are also supported and take precedence over the config file:

```sh
export SLACK_BOT_TOKEN=xoxb-...
export SLACK_APP_TOKEN=xapp-...
```

## Running

```sh
cargo run -p assistant-interface-slack
# or after cargo install:
assistant-slack run
```

## Security

- `shell-exec` is blocked for all messages arriving via Slack — the safety gate enforces this regardless of config.
- `allowed_channels` and `allowed_users` allowlists restrict which channels/users the bot responds to. Empty lists mean all are accepted.

## Conversation continuity

Each `(channel_id, user_id)` pair maps to a stable conversation UUID, so context is preserved across multiple messages in the same channel.
