-- Migration 065: persist the MCP App resolved for a tool call
--
-- `tool_app_json` is the serialised `McpApp` (server, `ui://` resource URI,
-- HTML, CSP and permissions) an external MCP tool declared for its result.
-- It is shown to clients only, never enters LLM context, and is not subject
-- to the `tool_data_json` size cap. Nullable, no backfill.

ALTER TABLE messages ADD COLUMN tool_app_json TEXT NULL;
