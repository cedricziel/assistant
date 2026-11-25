-- Per-user session generation. Access tokens carry the generation they were
-- signed under (`tv` claim); bumping it invalidates every token issued before.
ALTER TABLE users ADD COLUMN token_version INTEGER NOT NULL DEFAULT 0;
