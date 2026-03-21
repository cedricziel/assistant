-- Migration 018: scope scheduled tasks and webhooks by assistant agent.

ALTER TABLE scheduled_tasks
    ADD COLUMN agent_id TEXT NOT NULL DEFAULT 'default'
    REFERENCES assistant_agents(id) ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_agent_next
    ON scheduled_tasks(agent_id, enabled, next_run);

ALTER TABLE webhooks
    ADD COLUMN agent_id TEXT NOT NULL DEFAULT 'default'
    REFERENCES assistant_agents(id) ON DELETE RESTRICT;

CREATE INDEX IF NOT EXISTS idx_webhooks_agent_active
    ON webhooks(agent_id, active, created_at DESC);
