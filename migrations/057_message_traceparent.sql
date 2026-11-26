-- Migration 057: record the W3C traceparent of the LLM span that produced an
-- assistant reply, so later out-of-band events (user feedback) can be emitted
-- as span-parented records. NULL for existing rows and when tracing is off.

ALTER TABLE messages ADD COLUMN traceparent TEXT NULL;
