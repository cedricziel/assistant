-- Migration 046: persist messages as a tree with an active leaf
--
-- `parent_id` links a message to the message it answers (NULL marks a root).
-- It deliberately has no ON DELETE action: the `conversation_id` cascade
-- already removes every row of a conversation in one statement, and a
-- self-referencing cascade along a long chain could exceed SQLite's trigger
-- depth limit.
-- `run_id` records the turn run that wrote a row.
-- `conversations.active_leaf_id` points at the tip of the path the UI shows
-- and the LLM sees. It has no FK so a dead leaf can never block a delete.

ALTER TABLE messages ADD COLUMN parent_id TEXT NULL REFERENCES messages(id);
ALTER TABLE messages ADD COLUMN run_id TEXT NULL;
ALTER TABLE conversations ADD COLUMN active_leaf_id TEXT NULL;

CREATE INDEX IF NOT EXISTS idx_messages_conversation_parent
    ON messages(conversation_id, parent_id);
CREATE INDEX IF NOT EXISTS idx_messages_conversation_run
    ON messages(conversation_id, run_id);

-- Backfill: each existing conversation becomes one chain, ordered the way
-- `load_history` ordered it before the tree existed.
UPDATE messages
SET parent_id = prev.parent
FROM (
    SELECT id,
           LAG(id) OVER (
               PARTITION BY conversation_id
               ORDER BY turn, created_at, rowid
           ) AS parent
    FROM messages
) AS prev
WHERE messages.id = prev.id;

UPDATE conversations
SET active_leaf_id = (
    SELECT m.id
    FROM messages m
    WHERE m.conversation_id = conversations.id
    ORDER BY m.turn DESC, m.created_at DESC, m.rowid DESC
    LIMIT 1
);
