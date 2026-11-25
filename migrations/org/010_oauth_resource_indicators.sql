-- RFC 8707 resource indicators: the resource an authorization code or refresh
-- token was granted for. NULL means the server's own API.
ALTER TABLE auth_codes ADD COLUMN resource TEXT;
ALTER TABLE refresh_tokens ADD COLUMN resource TEXT;
