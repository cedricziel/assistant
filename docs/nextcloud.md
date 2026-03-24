# Nextcloud Talk Interface

Connects the assistant to [Nextcloud Talk](https://nextcloud.com/talk/) as a webhook-based bot. The assistant runs an HTTP server that receives events from the Nextcloud Talk server and replies via the Bot REST API.

## Prerequisites

- Nextcloud server with **Talk 17.1+** (Nextcloud 27.1+) — the `bots-v1` capability
- Shell access to the Nextcloud server (required for `occ` bot registration)
- A publicly reachable URL for the webhook endpoint (or a reverse proxy / tunnel)

## Setup

### 1. Choose a webhook URL

The assistant runs an HTTP server (default `0.0.0.0:8080`) that Nextcloud will POST events to. This URL must be reachable from the Nextcloud server.

Examples:

- Direct: `https://assistant.example.com:8080/webhook`
- Behind a reverse proxy: `https://assistant.example.com/webhook`
- Local development with a tunnel: `https://<tunnel-id>.ngrok.io/webhook`

The server accepts webhooks on both `/` and `/webhook`.

### 2. Register the bot on Nextcloud

On the Nextcloud server, register the bot via the `occ` CLI:

```sh
sudo -u www-data php occ talk:bot:install \
  --feature webhook \
  "Assistant Bot" \
  "<shared-secret>" \
  "https://assistant.example.com/webhook" \
  "AI assistant bot"
```

| Argument            | Description                                                           |
| ------------------- | --------------------------------------------------------------------- |
| `--feature webhook` | Enables webhook delivery (required)                                   |
| First positional    | Display name shown when the bot posts                                 |
| Second positional   | Shared secret for HMAC-SHA256 signing — choose a strong random string |
| Third positional    | Webhook URL the Nextcloud server will POST events to                  |
| Fourth positional   | Description shown to moderators                                       |

> The `--feature reaction` flag can optionally be added to receive reaction events.

### 3. Enable the bot in conversations

By default, newly installed bots are available but not enabled in any conversation. A conversation moderator enables the bot via:

- **Talk settings** (in the Nextcloud web UI) for each conversation, or
- The REST API: `POST /ocs/v2.php/apps/spreed/api/v1/bot/{token}/{botId}`

### 4. Configure the assistant

Add to `~/.assistant/config.toml`:

```toml
[nextcloud]
server_url = "https://nextcloud.example.com"
secret = "<shared-secret>"          # must match the occ install command
listen_addr = "0.0.0.0:8080"       # default; change if needed

# Optional: restrict to specific conversations or users
# allowed_channels = ["conversation-token-1", "conversation-token-2"]
# allowed_users    = ["alice", "bob"]
```

Environment variables are also supported and take precedence over the config file:

```sh
export NEXTCLOUD_SERVER_URL=https://nextcloud.example.com
export NEXTCLOUD_TALK_SECRET=<shared-secret>
```

## Running

### Standalone mode

```sh
# Via make
make run-nextcloud

# Via cargo
cargo run -p assistant-cli --features nextcloud -- orchestrator run --interfaces nextcloud --no-repl
```

### Background mode (REPL + Nextcloud)

When the `[nextcloud]` section is present in `config.toml`, the Nextcloud interface starts automatically in the background alongside the REPL:

```sh
make run
```

### As a systemd user service

If installed via the `.deb` or `.rpm` package:

```sh
# Enable and start
systemctl --user enable --now assistant-nextcloud

# View logs
journalctl --user -u assistant-nextcloud -f

# Persist across reboots (even when not logged in)
loginctl enable-linger $USER
```

## How it works

### Message flow

```text
User posts in Nextcloud Talk
  -> Nextcloud server POSTs Activity Streams 2.0 event to webhook URL
    -> Assistant verifies HMAC-SHA256 signature
    -> Assistant processes message through the Orchestrator
      -> System prompt, tools, skills, memory, ReAct loop
    -> Assistant replies via POST /bot/{token}/message (signed)
  <- Reply appears in the Nextcloud Talk conversation
```

### Per-turn tools

During each turn the LLM has access to:

| Tool              | What it does                                    | Key parameters       |
| ----------------- | ----------------------------------------------- | -------------------- |
| `reply`           | Post a reply in the current conversation        | `message`, `silent`? |
| `nextcloud-react` | Add an emoji reaction to the triggering message | `reaction`           |

> Parameters marked `?` are optional.

### UX indicators

- On receiving a message, the bot adds an hourglass reaction to acknowledge receipt
- After the turn completes, the hourglass is removed

### Conversation continuity

Each conversation token maps to a stable conversation UUID, so context is preserved across all messages in the same Nextcloud Talk conversation.

## Reverse proxy example (nginx)

```nginx
server {
    listen 443 ssl;
    server_name assistant.example.com;

    ssl_certificate     /etc/ssl/certs/assistant.pem;
    ssl_certificate_key /etc/ssl/private/assistant.key;

    location /webhook {
        proxy_pass http://127.0.0.1:8080/webhook;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Security

- **HMAC-SHA256 verification**: Every incoming webhook is verified against the shared secret before processing. Invalid signatures are rejected with HTTP 401.
- **Allowlists**: Use `allowed_channels` and `allowed_users` to restrict which conversations and users the bot responds to. Empty lists accept all.
- **Bot message filtering**: Messages from bots are ignored to prevent loops.
- **Non-interactive confirmation**: Mutating tool calls use auto-deny confirmation (same as Slack/Mattermost).

## Troubleshooting

| Symptom                           | Likely cause                                                                         |
| --------------------------------- | ------------------------------------------------------------------------------------ |
| Bot never receives messages       | Webhook URL not reachable from Nextcloud server; bot not enabled in the conversation |
| HTTP 401 on all webhooks          | Shared secret mismatch between `occ talk:bot:install` and `config.toml`              |
| Bot receives but never replies    | `server_url` in config is wrong or unreachable from the assistant host               |
| "Failed to add reaction" warnings | Bot may lack the `reaction` feature flag; non-fatal                                  |

Check the Nextcloud admin panel under **Talk → Bots** to see the bot's error count and last error message.
