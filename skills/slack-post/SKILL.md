---
metadata:
  name: slack-post
  description: Post a message to a Slack channel. Use this to proactively notify users, share results, or start conversations on Slack.
  tier: builtin
  parameters:
    - name: channel
      type: string
      required: true
      description: Slack channel ID or name (e.g. C01234567 or #general)
    - name: message
      type: string
      required: true
      description: Message text (Slack mrkdwn formatting supported)
    - name: thread_ts
      type: string
      required: false
      description: If provided, post as a reply in this thread
---

Post a message to a Slack channel proactively. The channel can be specified as a channel ID (e.g. `C01234567`) or a channel name with a leading `#` (e.g. `#general`).

The message supports Slack mrkdwn formatting:

- `*bold*` for bold text
- `_italic_` for italic text
- `` `code` `` for inline code
- `\`\`\`code block\`\`\`` for multi-line code
- `<url|link text>` for hyperlinks

Optionally provide `thread_ts` to post as a reply in an existing thread.
