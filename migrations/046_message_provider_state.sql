-- Migration 046: add provider_state_json to messages
--
-- Opaque, provider-specific state that must be sent back with an assistant
-- tool-call turn, such as OpenRouter's reasoning_details. Stored as JSON
-- {provider, model, data}; NULL for every other message.

ALTER TABLE messages ADD COLUMN provider_state_json TEXT;
