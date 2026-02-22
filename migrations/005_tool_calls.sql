-- Migration 005: add tool_calls_json to messages
--
-- When the LLM requests tool calls, the assistant turn is stored with this
-- column populated (JSON array of {name, params} objects).  On history reload
-- these rows are reconstructed as AssistantToolCalls variants so the model
-- sees its own tool-call decisions in multi-turn context.

ALTER TABLE messages ADD COLUMN tool_calls_json TEXT;
