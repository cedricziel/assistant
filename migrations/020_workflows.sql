-- Workflow graphs with trigger/action nodes and loop-safe execution limits.
CREATE TABLE IF NOT EXISTS workflows (
    id                       TEXT PRIMARY KEY,
    agent_id                 TEXT NOT NULL,
    name                     TEXT NOT NULL,
    description              TEXT NOT NULL DEFAULT '',
    graph_json               TEXT NOT NULL,
    active                   INTEGER NOT NULL DEFAULT 1,
    max_steps                INTEGER NOT NULL DEFAULT 200,
    max_visits_per_node      INTEGER NOT NULL DEFAULT 25,
    created_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_workflows_agent_active_created
    ON workflows(agent_id, active, created_at DESC);

CREATE TABLE IF NOT EXISTS workflow_runs (
    id                       TEXT PRIMARY KEY,
    workflow_id              TEXT NOT NULL
        REFERENCES workflows(id) ON DELETE CASCADE,
    agent_id                 TEXT NOT NULL,
    trigger_type             TEXT NOT NULL,
    trigger_payload_json     TEXT NOT NULL DEFAULT '{}',
    status                   TEXT NOT NULL DEFAULT 'running'
        CHECK(status IN ('running', 'completed', 'failed', 'cancelled', 'max_steps_exceeded', 'max_visits_exceeded')),
    started_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at              DATETIME,
    error_message            TEXT
);

CREATE INDEX IF NOT EXISTS idx_workflow_runs_lookup
    ON workflow_runs(agent_id, workflow_id, started_at DESC);
