-- Inbound webhook endpoints that trigger workflow runs.
CREATE TABLE IF NOT EXISTS workflow_webhook_endpoints (
    workflow_id              TEXT PRIMARY KEY
        REFERENCES workflows(id) ON DELETE CASCADE,
    token                    TEXT NOT NULL,
    active                   INTEGER NOT NULL DEFAULT 1,
    created_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_workflow_webhook_token
    ON workflow_webhook_endpoints(token);
