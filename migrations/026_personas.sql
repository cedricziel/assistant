-- Migration 026: rename assistant context table to personas.

ALTER TABLE assistant_agents RENAME TO personas;

DROP INDEX IF EXISTS idx_assistant_agents_single_default;

CREATE UNIQUE INDEX IF NOT EXISTS idx_personas_single_default
    ON personas(is_default)
    WHERE is_default = 1;
