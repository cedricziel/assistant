-- Migration 047: turn interactions.
--
-- One row per question a running turn asks its user (tool approval, free-form
-- question). The row is the source of truth for authorisation, idempotency,
-- liveness (`waiter_seen_at`), the pending list and receipts after reload.
-- Resolution is a compare-and-set on `status = 'pending'`.
-- Timestamps are RFC 3339 UTC with millisecond precision, so they compare
-- correctly as text.
CREATE TABLE IF NOT EXISTS interactions (
    id                  TEXT    PRIMARY KEY,
    conversation_id     TEXT    NOT NULL,
    run_id              TEXT    NOT NULL,
    user_id             TEXT    NOT NULL,
    kind                TEXT    NOT NULL,
    tool_call_id        TEXT,
    request             TEXT    NOT NULL,  -- JSON
    status              TEXT    NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending', 'resolved', 'cancelled', 'expired')),
    response            TEXT,              -- JSON
    rule_id             TEXT,
    created_at          TEXT    NOT NULL,
    expires_at          TEXT    NOT NULL,
    resolved_at         TEXT,
    waiter_seen_at      TEXT    NOT NULL,
    resolution_emitted  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_interactions_conversation_created
    ON interactions (conversation_id, created_at);

CREATE INDEX IF NOT EXISTS idx_interactions_run_status
    ON interactions (run_id, status);
