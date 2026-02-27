-- Outgoing webhook endpoints with HMAC-SHA256 verification support.
CREATE TABLE IF NOT EXISTS webhooks (
    id            TEXT PRIMARY KEY,
    name          TEXT    NOT NULL,
    url           TEXT    NOT NULL,
    secret        TEXT    NOT NULL,
    event_types   TEXT    NOT NULL DEFAULT '[]',   -- JSON array of subscribed topics
    active        INTEGER NOT NULL DEFAULT 1,
    verified_at   DATETIME,
    created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
