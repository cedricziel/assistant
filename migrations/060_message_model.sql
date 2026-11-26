-- Migration 060: add model to messages
--
-- The model that produced a final assistant reply (the provider's model name),
-- so a regenerated reply can say which model answered. NULL marks user and
-- tool rows and every row written before this migration.

ALTER TABLE messages ADD COLUMN model TEXT;
