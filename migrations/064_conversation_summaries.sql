-- Migration 064: conversation summaries for non-destructive compaction.
--
-- A summary stands in for every message of a path up to and including
-- `through_message_id`. It is a side table, not a tree node, so it never shows
-- in the message list and never shifts `turn`. A summary applies to a path
-- only when `through_message_id` lies on it, so a branch that diverged earlier
-- gets no summary.
--
-- `through_message_id` deliberately has no foreign key: `replace_history`
-- (context compaction until the next change removes it) deletes message rows,
-- and a cascade would take the summaries with them. The `conversation_id`
-- cascade removes every summary of a deleted conversation.
CREATE TABLE IF NOT EXISTS conversation_summaries (
    id                  TEXT     PRIMARY KEY,
    conversation_id     TEXT     NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    through_message_id  TEXT     NOT NULL,
    content             TEXT     NOT NULL,
    created_at          DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_conversation_summaries_conversation
    ON conversation_summaries (conversation_id, created_at);
