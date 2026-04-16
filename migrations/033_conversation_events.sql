-- Durable conversation event log.
-- Stores every orchestrator event emitted during a streaming turn so clients
-- can reconnect mid-run and replay missed events from a sequence cursor.
-- Events are ephemeral: expires_at drives TTL-based pruning.
CREATE TABLE IF NOT EXISTS conversation_events (
    id              INTEGER  PRIMARY KEY AUTOINCREMENT,
    run_id          TEXT     NOT NULL,
    conversation_id TEXT     NOT NULL,
    sequence        INTEGER  NOT NULL,
    event_type      TEXT     NOT NULL,  -- "run_started" | "token" | "status" | "tool_result" | "skill_complete" | "audio_ready" | "done" | "error"
    payload         TEXT     NOT NULL,  -- JSON blob
    created_at      DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    expires_at      DATETIME NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_conv_events_seq
    ON conversation_events (run_id, sequence);

CREATE INDEX IF NOT EXISTS idx_conv_events_conv
    ON conversation_events (conversation_id, run_id);

CREATE INDEX IF NOT EXISTS idx_conv_events_expires
    ON conversation_events (expires_at);
