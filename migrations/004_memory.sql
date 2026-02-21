-- Migration 004: memory store, skill refinements, scheduled tasks

-- Key-value memory entries
CREATE TABLE IF NOT EXISTS memory_entries (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    -- Who/what set this value: "user" | "assistant" | skill name
    source      TEXT NOT NULL DEFAULT 'user',
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Self-improvement proposals (from self-analyze skill)
CREATE TABLE IF NOT EXISTS skill_refinements (
    id                  TEXT PRIMARY KEY,   -- UUID
    target_skill        TEXT NOT NULL,
    -- The full proposed SKILL.md content
    proposed_skill_md   TEXT NOT NULL,
    -- LLM-generated explanation for the change
    rationale           TEXT NOT NULL,
    -- Lifecycle: pending → accepted | rejected
    status              TEXT NOT NULL DEFAULT 'pending'
                        CHECK(status IN ('pending', 'accepted', 'rejected')),
    -- Set when reviewed via /review
    review_note         TEXT,
    created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_at         DATETIME
);

CREATE INDEX IF NOT EXISTS idx_refinements_skill ON skill_refinements(target_skill, status);

-- Scheduled recurring tasks (cron-style, from schedule-task skill)
CREATE TABLE IF NOT EXISTS scheduled_tasks (
    id          TEXT PRIMARY KEY,   -- UUID
    name        TEXT NOT NULL,
    -- Standard cron expression: "0 9 * * *" = 9am daily
    cron_expr   TEXT NOT NULL,
    -- The prompt to run through the ReAct loop
    prompt      TEXT NOT NULL,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    last_run    DATETIME,
    next_run    DATETIME,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
