-- A2A protocol tasks, persisted so they survive restart.
--
-- The full A2A Task (status, history, artifacts, metadata) is stored as a JSON
-- blob in `task_json`; the flat columns exist for cheap filtering and ordering.
-- A2A push-notification configs and live SSE subscribers remain process-local
-- (ephemeral) and are intentionally not persisted here.
CREATE TABLE IF NOT EXISTS a2a_tasks (
    id         TEXT PRIMARY KEY,
    agent_id   TEXT NOT NULL,
    context_id TEXT NOT NULL,
    state      TEXT NOT NULL,
    task_json  TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_a2a_tasks_context ON a2a_tasks (agent_id, context_id);
CREATE INDEX IF NOT EXISTS idx_a2a_tasks_updated ON a2a_tasks (agent_id, updated_at);
