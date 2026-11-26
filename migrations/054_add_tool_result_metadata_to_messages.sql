-- Migration 054: persist tool-call metadata on tool rows
--
-- `tool_call_id` is the provider-supplied id of the call a tool row answers
-- and joins it to its entry in the assistant row's `tool_calls_json`.
-- `tool_status` is `ok`, `error` or `denied`. `tool_data_json` is the
-- serialised structured render data, capped by dispatch. It is shown to
-- clients only and never enters LLM context.
-- Nullable, no backfill: rows from before this migration keep NULL.

ALTER TABLE messages ADD COLUMN tool_call_id TEXT NULL;
ALTER TABLE messages ADD COLUMN tool_status TEXT NULL;
ALTER TABLE messages ADD COLUMN tool_data_json TEXT NULL;
