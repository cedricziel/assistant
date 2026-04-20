-- Migration 036: add active_skill column to distributed_traces for skill-scoped tracing

ALTER TABLE distributed_traces ADD COLUMN active_skill TEXT;

CREATE INDEX idx_distributed_traces_active_skill
    ON distributed_traces(active_skill, start_time DESC);
