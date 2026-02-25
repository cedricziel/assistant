-- Migration 010: add token usage columns to distributed_traces
-- Populated from gen_ai.usage.input_tokens / gen_ai.usage.output_tokens span attributes.

ALTER TABLE distributed_traces ADD COLUMN input_tokens  INTEGER;
ALTER TABLE distributed_traces ADD COLUMN output_tokens INTEGER;
