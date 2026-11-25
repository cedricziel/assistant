-- Migration 045: per-skill execution log.
--
-- One row per tool call finalised while a skill was active. Skill
-- self-improvement aggregates these rows; they are product data, not
-- telemetry (see ADR-0012).
CREATE TABLE IF NOT EXISTS skill_executions (
    id              TEXT    PRIMARY KEY,
    skill_name      TEXT    NOT NULL,
    agent_id        TEXT    NOT NULL,
    conversation_id TEXT    NOT NULL,
    tool_name       TEXT    NOT NULL,
    outcome         TEXT    NOT NULL CHECK (outcome IN ('success', 'error')),
    error           TEXT,
    duration_ms     INTEGER NOT NULL,
    created_at      TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_skill_executions_skill_created
    ON skill_executions (skill_name, created_at DESC);
