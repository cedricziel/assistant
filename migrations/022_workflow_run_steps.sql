-- Per-step execution trace for workflow runs.
CREATE TABLE IF NOT EXISTS workflow_run_steps (
    id                       TEXT PRIMARY KEY,
    run_id                   TEXT NOT NULL
        REFERENCES workflow_runs(id) ON DELETE CASCADE,
    step_index               INTEGER NOT NULL,
    node_id                  TEXT NOT NULL,
    node_kind                TEXT NOT NULL,
    note                     TEXT,
    occurred_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_workflow_run_steps_run
    ON workflow_run_steps(run_id, step_index);
