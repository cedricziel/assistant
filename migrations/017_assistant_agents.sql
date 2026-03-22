-- Migration 017: assistant agent contexts

CREATE TABLE IF NOT EXISTS assistant_agents (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    is_default  INTEGER NOT NULL DEFAULT 0 CHECK(is_default IN (0, 1)),
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_assistant_agents_single_default
    ON assistant_agents(is_default)
    WHERE is_default = 1;

INSERT OR IGNORE INTO assistant_agents (id, name, is_default)
VALUES ('default', 'Default', 1);

UPDATE assistant_agents
SET is_default = CASE WHEN id = 'default' THEN 1 ELSE 0 END,
    updated_at = CURRENT_TIMESTAMP
WHERE NOT EXISTS (
    SELECT 1 FROM assistant_agents WHERE is_default = 1
);

ALTER TABLE conversations
    ADD COLUMN agent_id TEXT NOT NULL DEFAULT 'default';

CREATE INDEX IF NOT EXISTS idx_conversations_agent_updated
    ON conversations(agent_id, updated_at DESC);
