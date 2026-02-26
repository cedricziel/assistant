-- Durable message bus table.
-- Serves as a topic-based work queue with atomic claim semantics.
-- Topics represent message types; routing uses metadata columns + filtered claims.

CREATE TABLE IF NOT EXISTS bus_messages (
    id              TEXT PRIMARY KEY,
    topic           TEXT NOT NULL,
    payload         TEXT NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK(status IN ('pending','claimed','done','failed')),

    -- identity: who is involved
    user_id         TEXT,
    agent_id        TEXT,

    -- routing: where it goes
    conversation_id TEXT,
    interface       TEXT,
    reply_to        TEXT,

    -- correlation: how messages relate
    correlation_id  TEXT,
    causation_id    TEXT,
    batch_id        TEXT,

    -- lifecycle
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    claimed_at      DATETIME,
    claimed_by      TEXT
);

-- Primary query path: claim next pending message per topic.
CREATE INDEX IF NOT EXISTS idx_bus_messages_pending
    ON bus_messages(topic, status, created_at)
    WHERE status = 'pending';

-- Filtered claim by agent.
CREATE INDEX IF NOT EXISTS idx_bus_messages_agent_pending
    ON bus_messages(topic, agent_id, status, created_at)
    WHERE status = 'pending';

-- Filtered claim by batch (parallel tool execution fan-in).
CREATE INDEX IF NOT EXISTS idx_bus_messages_batch_pending
    ON bus_messages(topic, batch_id, status, created_at)
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

-- Correlation: trace a full request chain.
CREATE INDEX IF NOT EXISTS idx_bus_messages_correlation
    ON bus_messages(correlation_id)
    WHERE correlation_id IS NOT NULL;
