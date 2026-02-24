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

| Scope              | Purpose                                                              | Required by                                                 |
| ------------------ | -------------------------------------------------------------------- | ----------------------------------------------------------- |
| `chat:write`       | Post replies and proactive messages                                  | core, `slack-post`, `slack-send-dm`                         |
| `chat:write:bot`   | Delete/update bot messages (optional — own messages only by default) | `slack-delete-message`, `slack-update-message`              |
| `reactions:write`  | Add/remove emoji reactions                                           | core (👀 ack), `slack-react`                                |
| `channels:history` | Read message history in public channels                              | core (thread hydration), `slack-get-history`                |
| `groups:history`   | Read message history in private channels                             | core (thread hydration), `slack-get-history`                |
| `im:history`       | Read DM message history                                              | core (thread hydration), `slack-get-history`                |
| `channels:read`    | List public channels the bot is in                                   | `slack-list-channels`                                       |
| `groups:read`      | List private channels the bot is in                                  | `slack-list-channels`                                       |
| `im:read`          | List direct-message conversations                                    | `slack-list-channels` (types=im)                            |
| `mpim:read`        | List group direct-message conversations                              | `slack-list-channels` (types=mpim)                          |
| `im:write`         | Open/create DM channels (`conversations.open`)                       | `slack-send-dm`                                             |
| `users:read`       | Resolve names → IDs; fetch user profiles                             | `slack-send-dm`, `slack-list-channels`, `slack-lookup-user` |
| `users:read.email` | Read user email addresses in profile lookups (optional)              | `slack-lookup-user`                                         |

### 4. Subscribe to events

**Event Subscriptions → Enable → Subscribe to bot events:**

- `message.channels` — messages in public channels
- `message.groups` — messages in private channels the bot is in
- `message.im` — direct messages

### 5. Install the app

Install to your workspace. Copy the **Bot Token** (`xoxb-...`).

### 6. Invite the bot to a channel

```text
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

## Ambient tools

When Slack is configured the assistant automatically gains these tools (available in all interfaces, not just Slack turns):

| Tool                   | What it does                                            | Key parameters                      |
| ---------------------- | ------------------------------------------------------- | ----------------------------------- |
| `slack-post`           | Post a message to any channel                           | `channel`, `message`, `thread_ts`?  |
| `slack-send-dm`        | Send a direct message to a user                         | `user`, `message`, `thread_ts`?     |
| `slack-list-channels`  | List channels the bot is a member of                    | `types`?, `limit`?                  |
| `slack-get-history`    | Read recent messages from a channel or thread           | `channel`, `limit`?, `thread_ts`?   |
| `slack-react`          | Add or remove an emoji reaction on a message            | `channel`, `ts`, `emoji`, `action`? |
| `slack-update-message` | Edit the text of a message the bot previously posted    | `channel`, `ts`, `message`          |
| `slack-delete-message` | Delete a message the bot previously posted              | `channel`, `ts`                     |
| `slack-lookup-user`    | Look up a user's profile (name, email, timezone, title) | `user`                              |

> Parameters marked `?` are optional.

## Security

- Use `allowed_channels` and `allowed_users` allowlists to restrict which channels/users the bot responds to. Empty lists mean all are accepted.

## Triggers

- Plain text messages in any channel/thread the bot can read start a turn.
- Emoji reactions (`reaction_added`) are also forwarded as synthetic messages, so a quick ":eyes:" or ":question:" reaction can nudge the assistant without typing.

## Conversation continuity

Each `(channel_id, thread_ts)` pair maps to a stable conversation UUID, so context is preserved across all messages in the same Slack thread.
