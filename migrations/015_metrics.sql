-- OpenTelemetry metrics persistence.
--
-- Resources and instrumentation scopes are normalized into join tables
-- to avoid duplicating identical metadata on every data point row.

-- Shared resource identity, deduplicated by content fingerprint.
CREATE TABLE IF NOT EXISTS resources (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    fingerprint TEXT    NOT NULL UNIQUE,
    attributes  TEXT    NOT NULL DEFAULT '{}',
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Instrumentation scopes, deduplicated by (name, version).
CREATE TABLE IF NOT EXISTS metric_scopes (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    name    TEXT    NOT NULL,
    version TEXT    NOT NULL DEFAULT '',
    UNIQUE(name, version)
);

-- Metric data points: unified table for counters, gauges, and histograms.
CREATE TABLE IF NOT EXISTS metric_points (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_id   INTEGER NOT NULL REFERENCES resources(id),
    scope_id      INTEGER NOT NULL REFERENCES metric_scopes(id),
    metric_name   TEXT    NOT NULL,
    metric_kind   TEXT    NOT NULL,       -- 'counter', 'up_down_counter', 'gauge', 'histogram'
    unit          TEXT,
    description   TEXT,
    -- Counter / Gauge scalar value
    value         REAL,
    -- Histogram aggregates
    count         INTEGER,
    sum           REAL,
    min           REAL,
    max           REAL,
    bounds        TEXT,                   -- JSON array of bucket boundaries
    bucket_counts TEXT,                   -- JSON array of per-bucket counts
    -- Per-data-point attributes (full OTel fidelity)
    attributes    TEXT    NOT NULL DEFAULT '{}',
    start_time    TEXT,                   -- start of the delta interval
    recorded_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    -- Denormalized hot-path columns extracted from attributes
    model         TEXT,                  -- gen_ai.request.model
    provider      TEXT,                  -- gen_ai.provider.name
    operation     TEXT,                  -- gen_ai.operation.name
    skill         TEXT,
    interface     TEXT
);

CREATE INDEX IF NOT EXISTS idx_mp_name_time  ON metric_points(metric_name, recorded_at);
CREATE INDEX IF NOT EXISTS idx_mp_model_time ON metric_points(model, recorded_at);
CREATE INDEX IF NOT EXISTS idx_mp_recorded   ON metric_points(recorded_at);
CREATE INDEX IF NOT EXISTS idx_mp_resource   ON metric_points(resource_id);
CREATE INDEX IF NOT EXISTS idx_mp_scope      ON metric_points(scope_id);
CREATE INDEX IF NOT EXISTS idx_mp_kind       ON metric_points(metric_kind, metric_name);
