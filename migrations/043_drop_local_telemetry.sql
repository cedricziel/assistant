-- Migration 043: drop local telemetry storage.
--
-- Spans, logs and metrics are exported over OTLP only. The SQLite exporter
-- that populated these tables no longer exists. Indexes go with their tables.

DROP TABLE IF EXISTS metric_points;
DROP TABLE IF EXISTS metric_scopes;
DROP TABLE IF EXISTS resources;
DROP TABLE IF EXISTS logs;
DROP TABLE IF EXISTS distributed_traces;
