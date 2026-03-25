-- Migration 025: make distributed_traces standalone (no conversation FK)

BEGIN TRANSACTION;

CREATE TABLE distributed_traces_new (
    span_id          TEXT PRIMARY KEY,
    trace_id         TEXT NOT NULL,
    parent_span_id   TEXT,
    name             TEXT NOT NULL,
    conversation_id  TEXT,
    turn             INTEGER,
    tool_name        TEXT,
    tool_status      TEXT,
    tool_observation TEXT,
    tool_error       TEXT,
    duration_ms      INTEGER NOT NULL DEFAULT 0,
    start_time       DATETIME NOT NULL,
    end_time         DATETIME NOT NULL,
    attributes       TEXT NOT NULL,
    input_tokens     INTEGER,
    output_tokens    INTEGER,
    service_name     TEXT
);

INSERT INTO distributed_traces_new (
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
    attributes,
    input_tokens,
    output_tokens,
    service_name
)
SELECT
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
    attributes,
    input_tokens,
    output_tokens,
    service_name
FROM distributed_traces;

DROP TABLE distributed_traces;
ALTER TABLE distributed_traces_new RENAME TO distributed_traces;

CREATE INDEX idx_distributed_traces_tool
    ON distributed_traces(tool_name, start_time DESC);
CREATE INDEX idx_distributed_traces_trace
    ON distributed_traces(trace_id);
CREATE INDEX idx_distributed_traces_service
    ON distributed_traces(service_name, start_time DESC);

COMMIT;
