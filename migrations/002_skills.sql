-- Migration 002: skill registry metadata

CREATE TABLE IF NOT EXISTS skills (
    name            TEXT PRIMARY KEY,
    description     TEXT NOT NULL,
    dir_path        TEXT NOT NULL,
    tier            TEXT NOT NULL CHECK(tier IN ('prompt', 'script', 'wasm', 'builtin')),
    enabled         BOOLEAN NOT NULL DEFAULT TRUE,
    -- Where this skill came from: "builtin" | "user" | "project" | "installed"
    source_type     TEXT NOT NULL DEFAULT 'builtin',
    license         TEXT,
    metadata_json   TEXT,  -- Full frontmatter metadata as JSON
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
