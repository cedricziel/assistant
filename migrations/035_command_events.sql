-- Durable slash-command event log.
-- Stores command invocations so they are visible in the conversation timeline
-- but excluded from LLM context (never queried by the orchestrator).
CREATE TABLE IF NOT EXISTS command_events (
    id              TEXT     PRIMARY KEY,
    conversation_id TEXT     NOT NULL,
    event_type      TEXT     NOT NULL DEFAULT 'command',
    command         TEXT     NOT NULL,
    payload         TEXT,
    ack_text        TEXT,
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_command_events_conv
    ON command_events (conversation_id, created_at);
