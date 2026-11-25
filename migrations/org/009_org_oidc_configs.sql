-- Per-org OpenID Connect settings. One row per org with auth_mode = 'oidc'.

CREATE TABLE IF NOT EXISTS org_oidc_configs (
    org_id                TEXT PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    issuer_url            TEXT NOT NULL,
    client_id             TEXT NOT NULL,
    client_secret         TEXT,
    auto_provision        INTEGER NOT NULL DEFAULT 1,
    allowed_email_domains TEXT NOT NULL DEFAULT '[]',  -- JSON: ["acme.com", ...]
    created_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
