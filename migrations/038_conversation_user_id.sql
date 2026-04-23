-- Add user_id to conversations for multi-user scoping.
-- NULL means legacy/unscoped (backward compatible).
ALTER TABLE conversations ADD COLUMN user_id TEXT;

CREATE INDEX IF NOT EXISTS idx_conversations_user_id
    ON conversations(user_id);
