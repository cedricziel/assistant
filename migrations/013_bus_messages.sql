-- Durable message bus table.
-- Serves as a topic-based work queue with atomic claim semantics.

CREATE TABLE IF NOT EXISTS bus_messages (
    id              TEXT PRIMARY KEY,
    topic           TEXT NOT NULL,
    payload         TEXT NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending','claimed','done','failed')),
    conversation_id TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    claimed_at      DATETIME,
    claimed_by      TEXT
);

-- Primary query path: claim next pending message per topic.
CREATE INDEX IF NOT EXISTS idx_bus_messages_pending
    ON bus_messages(topic, status, created_at)
    WHERE status = 'pending';

-- Observability: list messages by topic and status.
CREATE INDEX IF NOT EXISTS idx_bus_messages_topic_status
    ON bus_messages(topic, status);

-- Housekeeping: find stale claimed messages.
CREATE INDEX IF NOT EXISTS idx_bus_messages_claimed
    ON bus_messages(status, claimed_at)
    WHERE status = 'claimed';

-- Correlation: find messages for a conversation.
CREATE INDEX IF NOT EXISTS idx_bus_messages_conversation
    ON bus_messages(conversation_id)
    WHERE conversation_id IS NOT NULL;
