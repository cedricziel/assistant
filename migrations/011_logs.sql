-- Migration 011: OpenTelemetry log record storage
-- Bridges tracing events into persistent OTel log records for the logs UI.

CREATE TABLE IF NOT EXISTS logs (
    id                  TEXT PRIMARY KEY,
    timestamp           DATETIME NOT NULL,
    observed_timestamp  DATETIME,
    severity_number     INTEGER,
    severity_text       TEXT,
    body                TEXT,
    trace_id            TEXT,
    span_id             TEXT,
    target              TEXT,
    attributes          TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_logs_timestamp
    ON logs(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_severity
    ON logs(severity_number, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_logs_trace
    ON logs(trace_id);
CREATE INDEX IF NOT EXISTS idx_logs_target
    ON logs(target, timestamp DESC);
