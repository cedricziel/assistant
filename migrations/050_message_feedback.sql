-- Migration 050: per-user feedback on assistant messages.
--
-- One row per (message, user). The user's thumbs rating and optional comment
-- are product data and stay in the space database; deleting a message
-- deletes its feedback.
CREATE TABLE IF NOT EXISTS message_feedback (
    message_id  TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL,
    rating      TEXT NOT NULL CHECK (rating IN ('positive', 'negative')),
    comment     TEXT,
    created_at  DATETIME NOT NULL,
    updated_at  DATETIME NOT NULL,
    PRIMARY KEY (message_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_message_feedback_user
    ON message_feedback (user_id);
