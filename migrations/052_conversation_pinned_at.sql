-- Pin a conversation to the top of its owner's list.
--
-- `pinned_at` records when the conversation was pinned (NULL = not pinned).
-- The list orders pinned conversations first, newest pin first. Conversations
-- are owned by one user, so the column is per-user without a join table.

ALTER TABLE conversations ADD COLUMN pinned_at DATETIME;
