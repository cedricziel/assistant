-- Push subscription storage for Web Push / VAPID notifications.
-- Each row represents one browser push subscription endpoint.
-- Subscriptions are upserted by endpoint URL and deleted on 410 Gone responses.
CREATE TABLE IF NOT EXISTS push_subscriptions (
    id         INTEGER  PRIMARY KEY AUTOINCREMENT,
    endpoint   TEXT     NOT NULL UNIQUE,
    p256dh     TEXT     NOT NULL,
    auth       TEXT     NOT NULL,
    created_at DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
