-- Add owner_user_id to personas for multi-user ownership.
-- NULL means org-owned (visible to all org members with access).
ALTER TABLE personas ADD COLUMN owner_user_id TEXT;
