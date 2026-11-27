-- Whose browser a push subscription belongs to, so notifications about a turn
-- (a tool approval or a question waiting for an answer) reach the turn's
-- owner only. Nullable: subscriptions stored before this column keep
-- receiving the broadcast pushes but no user-targeted ones. No backfill.
ALTER TABLE push_subscriptions ADD COLUMN user_id TEXT;
CREATE INDEX IF NOT EXISTS idx_push_subscriptions_user ON push_subscriptions (user_id);
