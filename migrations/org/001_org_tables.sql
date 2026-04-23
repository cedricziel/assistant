-- Organization, user, space, and membership tables for multi-tenant storage.
-- This migration runs against org.db (separate from assistant.db).

CREATE TABLE IF NOT EXISTS organizations (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    auth_mode   TEXT NOT NULL DEFAULT 'password',
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY,
    org_id          TEXT NOT NULL REFERENCES organizations(id),
    email           TEXT NOT NULL,
    name            TEXT NOT NULL DEFAULT '',
    password_hash   TEXT NOT NULL DEFAULT '',
    idp_issuer      TEXT,
    idp_subject     TEXT,
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(org_id, email)
);

CREATE INDEX IF NOT EXISTS idx_users_org_id ON users(org_id);
CREATE INDEX IF NOT EXISTS idx_users_idp ON users(idp_issuer, idp_subject);

CREATE TABLE IF NOT EXISTS spaces (
    id          TEXT PRIMARY KEY,
    org_id      TEXT NOT NULL REFERENCES organizations(id),
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(org_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_spaces_org_id ON spaces(org_id);

CREATE TABLE IF NOT EXISTS space_memberships (
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    space_id    TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    role        TEXT NOT NULL DEFAULT 'member',
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, space_id)
);

CREATE INDEX IF NOT EXISTS idx_memberships_space_id ON space_memberships(space_id);
