-- Migration 048: saved tool permission rules.
--
-- A rule is one allow or deny line in the permission rule grammar that a user
-- saved from an approval prompt ("allow always"). Rules apply to every turn the
-- user owns, including unattended ones. They are checked after the built-in and
-- configured deny rules, never instead of them (see the tool-approval-prompts
-- design, D4).
--
-- `source_interaction_id` names the approval the rule was saved from. It is
-- informational and has no foreign key: the interactions table is separate and
-- rules must outlive their interaction.
CREATE TABLE IF NOT EXISTS tool_permission_rules (
    id                    TEXT PRIMARY KEY,
    user_id               TEXT NOT NULL,
    effect                TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    rule                  TEXT NOT NULL,
    created_at            TEXT NOT NULL,
    source_interaction_id TEXT,
    UNIQUE (user_id, effect, rule)
);

CREATE INDEX IF NOT EXISTS idx_tool_permission_rules_user
    ON tool_permission_rules (user_id, created_at);
