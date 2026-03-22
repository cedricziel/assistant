-- Migration 019: add assistant agent scope to metric points.

ALTER TABLE metric_points
    ADD COLUMN agent_id TEXT NOT NULL DEFAULT 'default';

CREATE INDEX IF NOT EXISTS idx_mp_agent_time
    ON metric_points(agent_id, recorded_at);

CREATE INDEX IF NOT EXISTS idx_mp_agent_name_time
    ON metric_points(agent_id, metric_name, recorded_at);
