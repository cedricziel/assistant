-- Subagent lifecycle tracking.
CREATE TABLE IF NOT EXISTS agents (
    id                      TEXT    PRIMARY KEY,
    parent_agent_id         TEXT,
    parent_conversation_id  TEXT    NOT NULL,
    conversation_id         TEXT    NOT NULL,
    task                    TEXT    NOT NULL,
    status                  TEXT    NOT NULL DEFAULT 'running',
    depth                   INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT    NOT NULL,
    completed_at            TEXT,
    result_summary          TEXT
);

CREATE INDEX IF NOT EXISTS idx_agents_parent_conversation
    ON agents (parent_conversation_id);
CREATE INDEX IF NOT EXISTS idx_agents_status
    ON agents (status);
