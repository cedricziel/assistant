-- Per-org OpenID Connect settings: one optional row per org holding that org's IdP settings.

CREATE TABLE IF NOT EXISTS org_oidc_configs (
    org_id                   TEXT PRIMARY KEY REFERENCES organizations(id) ON DELETE CASCADE,
    issuer_url               TEXT NOT NULL,
    client_id                TEXT NOT NULL,
    -- Sealed by SecretBox; all three are NULL for public clients using PKCE.
    client_secret_ciphertext BLOB,
    client_secret_nonce      BLOB,
    client_secret_key_id     TEXT,
    auto_provision           INTEGER NOT NULL DEFAULT 1,
    allowed_email_domains    TEXT NOT NULL DEFAULT '[]',  -- JSON: ["acme.com", ...]
    created_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
