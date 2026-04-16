-- Attachment metadata for files stored on the filesystem.
-- Actual bytes live at ~/.assistant/agents/{agent_id}/attachments/{conversation_id}/{id}.{ext}
CREATE TABLE IF NOT EXISTS message_attachments (
    id              TEXT PRIMARY KEY,
    message_id      TEXT REFERENCES messages(id),
    conversation_id TEXT NOT NULL,
    agent_id        TEXT NOT NULL,
    filename        TEXT NOT NULL,
    mime_type       TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_attachments_message ON message_attachments(message_id);
CREATE INDEX IF NOT EXISTS idx_attachments_conversation ON message_attachments(conversation_id);
