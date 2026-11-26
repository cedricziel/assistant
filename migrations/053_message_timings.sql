-- Migration 053: per-message turn timing.
--
-- One row per assistant message produced by a measured turn. The table
-- references `conversations` (cascade on delete) but deliberately not
-- `messages`: context compaction rewrites a conversation's messages with
-- `replace_history`, and a foreign key to `messages` would delete the timings
-- of messages that are kept.
CREATE TABLE IF NOT EXISTS message_timings (
    message_id              TEXT    PRIMARY KEY,
    conversation_id         TEXT    NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    time_to_first_token_ms  INTEGER,
    total_ms                INTEGER NOT NULL,
    output_tokens           INTEGER,
    decode_ms               INTEGER
);

CREATE INDEX IF NOT EXISTS idx_message_timings_conversation
    ON message_timings (conversation_id);
