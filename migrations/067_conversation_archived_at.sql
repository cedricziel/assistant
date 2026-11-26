-- Archive a conversation: put it away without deleting it.
--
-- `archived_at` records when the conversation was archived (NULL = not
-- archived). Archived conversations leave the main list but stay searchable.
-- A conversation is never pinned and archived at once; the store enforces it
-- in the single UPDATE that changes either column.

ALTER TABLE conversations ADD COLUMN archived_at DATETIME;
