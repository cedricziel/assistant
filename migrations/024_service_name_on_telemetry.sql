-- Migration 024: persist resource service.name on span/log rows

ALTER TABLE distributed_traces
ADD COLUMN service_name TEXT;

ALTER TABLE logs
ADD COLUMN service_name TEXT;

CREATE INDEX IF NOT EXISTS idx_distributed_traces_service
    ON distributed_traces(service_name, start_time DESC);

CREATE INDEX IF NOT EXISTS idx_logs_service
    ON logs(service_name, timestamp DESC);
