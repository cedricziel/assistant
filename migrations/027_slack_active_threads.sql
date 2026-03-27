-- Persist Slack threads where the bot has been @-mentioned so that after a
-- service restart the bot continues to respond in those threads without
-- requiring another @-mention.
--
-- `last_seen_at` is updated on every access so stale threads can be pruned.
CREATE TABLE IF NOT EXISTS slack_active_threads (
    channel_id   TEXT NOT NULL,
    thread_ts    TEXT NOT NULL,
    last_seen_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (channel_id, thread_ts)
);
