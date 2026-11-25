-- What the last connection attempts to an integration's MCP server showed, per
-- subject. Kept apart from `integrations` so recording a result never touches
-- `updated_at`, which invalidates open connections.
CREATE TABLE IF NOT EXISTS integration_health (
    integration_id     TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    -- '' is the shared credential, or an integration that needs none.
    subject_key        TEXT NOT NULL DEFAULT '',
    subject_user_id    TEXT REFERENCES users(id) ON DELETE CASCADE,
    reachable          INTEGER NOT NULL CHECK (reachable IN (0, 1)),
    last_connected_at  DATETIME,
    tool_count         INTEGER,
    last_error         TEXT,
    checked_at         DATETIME NOT NULL,
    PRIMARY KEY (integration_id, subject_key)
);
