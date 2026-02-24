-- Migration 008: relax the tier CHECK constraint so that skills loaded from
-- SKILL.md directories (which have no execution tier) can be stored with the
-- "knowledge" label.  SQLite does not support ALTER COLUMN, so we recreate
-- the table without the CHECK constraint.

CREATE TABLE skills_new (
    name            TEXT PRIMARY KEY,
    description     TEXT NOT NULL,
    dir_path        TEXT NOT NULL,
    tier            TEXT NOT NULL,
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    source_type     TEXT NOT NULL DEFAULT 'builtin',
    license         TEXT,
    metadata_json   TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO skills_new
    SELECT name, description, dir_path, tier, enabled, source_type, license, metadata_json, created_at, updated_at
    FROM skills;

DROP TABLE skills;

ALTER TABLE skills_new RENAME TO skills;
