-- Scoped integrations (remote MCP servers) at the org, space and user layer,
-- their encrypted credentials, OAuth client/state tables and the per-org policy.
-- This migration runs against org.db.

CREATE TABLE IF NOT EXISTS integrations (
    id                       TEXT PRIMARY KEY,
    org_id                   TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    layer                    TEXT NOT NULL CHECK (layer IN ('org', 'space', 'user')),
    space_id                 TEXT REFERENCES spaces(id) ON DELETE CASCADE,
    owner_user_id            TEXT REFERENCES users(id) ON DELETE CASCADE,
    catalog_entry_id         TEXT,
    connect_url              TEXT NOT NULL,
    slug                     TEXT NOT NULL CHECK (length(slug) BETWEEN 1 AND 32),
    name                     TEXT NOT NULL,
    auth_kind                TEXT NOT NULL CHECK (auth_kind IN ('none', 'header', 'oauth', 'mixed')),
    header_name              TEXT,
    header_value_prefix      TEXT,
    credential_mode          TEXT NOT NULL CHECK (credential_mode IN ('shared', 'per_user')),
    trust                    TEXT NOT NULL DEFAULT 'confirm' CHECK (trust IN ('confirm', 'trust')),
    enabled                  INTEGER NOT NULL DEFAULT 1,
    oauth_client_id_override TEXT,
    created_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- The owner columns must match the layer.
    CHECK (
        (layer = 'org'   AND space_id IS NULL     AND owner_user_id IS NULL) OR
        (layer = 'space' AND space_id IS NOT NULL AND owner_user_id IS NULL) OR
        (layer = 'user'  AND space_id IS NULL     AND owner_user_id IS NOT NULL)
    ),
    -- User-layer integrations always use the owner's own credential.
    CHECK (layer <> 'user' OR credential_mode = 'per_user')
);

-- COALESCE so NULL owner columns still collide.
CREATE UNIQUE INDEX IF NOT EXISTS idx_integrations_slug
    ON integrations(org_id, layer, COALESCE(space_id, ''), COALESCE(owner_user_id, ''), slug);
CREATE INDEX IF NOT EXISTS idx_integrations_space ON integrations(space_id);
CREATE INDEX IF NOT EXISTS idx_integrations_owner ON integrations(owner_user_id);

-- Org-layer availability. `allow` rows restrict an integration to listed spaces;
-- `opt_out` rows are a space's own opt-out. Both may exist for one space, so
-- opting back in restores the allow-list state.
CREATE TABLE IF NOT EXISTS integration_space_access (
    integration_id  TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    space_id        TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    mode            TEXT NOT NULL CHECK (mode IN ('allow', 'opt_out')),
    PRIMARY KEY (integration_id, space_id, mode)
);

CREATE INDEX IF NOT EXISTS idx_integration_space_access_space
    ON integration_space_access(space_id);

-- Ciphertext only. A NULL subject is the shared credential.
CREATE TABLE IF NOT EXISTS integration_credentials (
    integration_id   TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    subject_user_id  TEXT REFERENCES users(id) ON DELETE CASCADE,
    kind             TEXT NOT NULL CHECK (kind IN ('header', 'oauth')),
    ciphertext       BLOB NOT NULL,
    nonce            BLOB NOT NULL,
    key_id           TEXT NOT NULL,
    expires_at       DATETIME,
    status           TEXT NOT NULL DEFAULT 'ready' CHECK (status IN ('ready', 'auth_expired')),
    updated_at       DATETIME NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_integration_credentials_subject
    ON integration_credentials(integration_id, COALESCE(subject_user_id, ''));

-- One dynamically registered client per org per authorization server.
CREATE TABLE IF NOT EXISTS integration_oauth_clients (
    org_id                  TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    issuer                  TEXT NOT NULL,
    redirect_uri            TEXT NOT NULL,
    client_id               TEXT NOT NULL,
    client_secret_ct        BLOB,
    client_secret_nonce     BLOB,
    client_secret_key_id    TEXT,
    registration_json       TEXT NOT NULL DEFAULT '{}',
    created_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (org_id, issuer, redirect_uri)
);

-- Single-use OAuth state, swept after 10 minutes.
CREATE TABLE IF NOT EXISTS integration_oauth_states (
    state_hash              TEXT PRIMARY KEY,
    integration_id          TEXT NOT NULL REFERENCES integrations(id) ON DELETE CASCADE,
    subject_user_id         TEXT REFERENCES users(id) ON DELETE CASCADE,
    pkce_verifier_ct        BLOB NOT NULL,
    pkce_verifier_nonce     BLOB NOT NULL,
    pkce_verifier_key_id    TEXT NOT NULL,
    created_at              DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_integration_oauth_states_created
    ON integration_oauth_states(created_at);

CREATE TABLE IF NOT EXISTS integration_policy (
    org_id                   TEXT PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    catalog_mode             TEXT NOT NULL DEFAULT 'all' CHECK (catalog_mode IN ('all', 'allowlist', 'none')),
    allowed_catalog_ids      TEXT NOT NULL DEFAULT '[]',
    allow_custom_urls        INTEGER NOT NULL DEFAULT 0,
    allow_user_integrations  INTEGER NOT NULL DEFAULT 1,
    updated_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
