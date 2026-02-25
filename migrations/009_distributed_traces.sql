-- Migration 009: distributed trace storage sourced from OpenTelemetry spans

CREATE TABLE IF NOT EXISTS distributed_traces (
    span_id         TEXT PRIMARY KEY,
    trace_id        TEXT NOT NULL,
    parent_span_id  TEXT,
    name            TEXT NOT NULL,
    conversation_id TEXT REFERENCES conversations(id) ON DELETE CASCADE,
    turn            INTEGER,
    tool_name       TEXT,
    tool_status     TEXT,
    tool_observation TEXT,
    tool_error      TEXT,
    duration_ms     INTEGER NOT NULL DEFAULT 0,
    start_time      DATETIME NOT NULL,
    end_time        DATETIME NOT NULL,
    attributes      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_distributed_traces_tool
    ON distributed_traces(tool_name, start_time DESC);
CREATE INDEX IF NOT EXISTS idx_distributed_traces_trace
    ON distributed_traces(trace_id);

-- Migrate any legacy execution_traces rows into the new format.
INSERT OR IGNORE INTO distributed_traces (
    span_id,
    trace_id,
    parent_span_id,
    name,
    conversation_id,
    turn,
    tool_name,
    tool_status,
    tool_observation,
    tool_error,
    duration_ms,
    start_time,
    end_time,
    attributes
)
SELECT
    id AS span_id,
    id AS trace_id,
    NULL AS parent_span_id,
    'legacy.execution_trace' AS name,
    conversation_id,
    turn,
    action_skill AS tool_name,
    CASE WHEN error IS NULL THEN 'ok' ELSE 'error' END AS tool_status,
    observation AS tool_observation,
    error AS tool_error,
    duration_ms,
    created_at AS start_time,
    created_at AS end_time,
    json_object(
        'action_params', json(action_params),
        'source', 'legacy.execution_trace'
    ) AS attributes
FROM execution_traces;

DROP INDEX IF EXISTS idx_traces_skill;
DROP INDEX IF EXISTS idx_traces_conversation;
DROP TABLE IF EXISTS execution_traces;
