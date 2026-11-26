-- Migration 062: add finish_reason to messages
--
-- Why a persisted assistant reply ended early. The only value written today is
-- 'cancelled', on the reply saved for a turn the user stopped. NULL marks an
-- ordinary message, including every row written before this migration.

ALTER TABLE messages ADD COLUMN finish_reason TEXT;
