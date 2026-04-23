-- Add sender_user_id to messages for multi-user attribution.
-- NULL for assistant/system/tool messages and legacy data.
ALTER TABLE messages ADD COLUMN sender_user_id TEXT;
