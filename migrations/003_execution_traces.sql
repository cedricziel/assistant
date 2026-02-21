-- Migration 003: execution traces (foundation for self-improvement)

CREATE TABLE IF NOT EXISTS execution_traces (
    id              TEXT PRIMARY KEY,   -- UUID
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    turn            INTEGER NOT NULL,
    -- Which skill was invoked
    action_skill    TEXT NOT NULL,
    -- JSON-encoded parameters passed to the skill
    action_params   TEXT NOT NULL DEFAULT '{}',
    -- The observation returned by the skill
    observation     TEXT,
    -- Error message if the skill failed (NULL = success)
    error           TEXT,
    -- Execution duration in milliseconds
    duration_ms     INTEGER NOT NULL DEFAULT 0,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_traces_skill ON execution_traces(action_skill, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_traces_conversation ON execution_traces(conversation_id);
