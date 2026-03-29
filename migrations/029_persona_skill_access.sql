-- Migration 029: persona skill access control
--
-- Adds a three-mode access model to personas:
--   "all"       — every skill in the registry is available (default)
--   "whitelist" — only explicitly listed skills are available
--   "blacklist" — all skills except explicitly listed ones are available
--
-- The persona_skill_list table stores the per-persona skill name list used
-- in whitelist and blacklist modes.

ALTER TABLE personas ADD COLUMN skill_access_mode TEXT NOT NULL DEFAULT 'all'
    CHECK(skill_access_mode IN ('all', 'whitelist', 'blacklist'));

CREATE TABLE IF NOT EXISTS persona_skill_list (
    persona_id  TEXT NOT NULL REFERENCES personas(id) ON DELETE CASCADE,
    skill_name  TEXT NOT NULL,
    PRIMARY KEY (persona_id, skill_name)
);

CREATE INDEX IF NOT EXISTS idx_persona_skill_list_persona
    ON persona_skill_list(persona_id);
