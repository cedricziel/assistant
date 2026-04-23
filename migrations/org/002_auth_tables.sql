-- OAuth2 clients, API keys, and auth state tables.
-- These bridge the assistant-auth crate's store traits to persistent storage.

CREATE TABLE IF NOT EXISTS api_keys (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    key_hash        TEXT NOT NULL UNIQUE,
    key_prefix      TEXT NOT NULL,
    org_id          TEXT NOT NULL REFERENCES organizations(id),
    space_roles_json TEXT NOT NULL DEFAULT '{}',
    scopes_json     TEXT NOT NULL DEFAULT '[]',
    expires_at      DATETIME,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash);

CREATE TABLE IF NOT EXISTS oauth_clients (
    client_id                   TEXT PRIMARY KEY,
    client_name                 TEXT NOT NULL,
    client_secret_hash          TEXT,
    redirect_uris_json          TEXT NOT NULL DEFAULT '[]',
    grant_types_json            TEXT NOT NULL DEFAULT '[]',
    token_endpoint_auth_method  TEXT NOT NULL DEFAULT 'none',
    created_at                  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS auth_codes (
    code_hash       TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL,
    client_id       TEXT NOT NULL,
    redirect_uri    TEXT NOT NULL,
    pkce_challenge  TEXT,
    scopes_json     TEXT NOT NULL DEFAULT '[]',
    expires_at      DATETIME NOT NULL
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    token_hash  TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    client_id   TEXT NOT NULL,
    scopes_json TEXT NOT NULL DEFAULT '[]',
    expires_at  DATETIME NOT NULL,
    consumed    INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_client
    ON refresh_tokens(user_id, client_id);

CREATE TABLE IF NOT EXISTS device_codes (
    device_code_hash TEXT PRIMARY KEY,
    user_code        TEXT NOT NULL UNIQUE,
    client_id        TEXT NOT NULL,
    scopes_json      TEXT NOT NULL DEFAULT '[]',
    authorized_user  TEXT,
    expires_at       DATETIME NOT NULL
);
