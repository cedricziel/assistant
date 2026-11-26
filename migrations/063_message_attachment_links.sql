-- Migration 063: share attachments across message versions
--
-- `message_attachments.message_id` binds an attachment to exactly one message,
-- and its bytes are keyed by the attachment's own id. Editing a message
-- creates a sibling that must show the same attachments, so each extra
-- reference is one row here. Nothing is copied on disk.
--
-- Both foreign keys cascade: deleting a conversation removes its messages,
-- which removes the links; deleting an attachment removes its links.

CREATE TABLE IF NOT EXISTS message_attachment_links (
    message_id    TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    attachment_id TEXT NOT NULL REFERENCES message_attachments(id) ON DELETE CASCADE,
    PRIMARY KEY (message_id, attachment_id)
);

CREATE INDEX IF NOT EXISTS idx_attachment_links_attachment
    ON message_attachment_links(attachment_id);
