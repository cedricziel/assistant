-- Delivery tracking for the durable message bus.
--
-- delivery_count: incremented on every claim so consumers can cap redeliveries
--   (see MAX_TURN_REDELIVERIES).
-- visible_at: when set, the message is not claimable until this time. Written
--   by nack_delayed to implement redelivery backoff; NULL means immediately
--   visible.

ALTER TABLE bus_messages ADD COLUMN delivery_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE bus_messages ADD COLUMN visible_at DATETIME;
