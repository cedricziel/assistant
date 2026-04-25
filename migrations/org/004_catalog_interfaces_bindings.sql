-- Catalog items, subscriptions, interface instances, and persona bindings.
-- This migration runs against org.db.

CREATE TABLE IF NOT EXISTS catalog_items (
    id              TEXT PRIMARY KEY,
    org_id          TEXT NOT NULL REFERENCES organizations(id),
    resource_type   TEXT NOT NULL,  -- 'skill', 'template', 'interface'
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(org_id, resource_type, name)
);

CREATE INDEX IF NOT EXISTS idx_catalog_items_org_type ON catalog_items(org_id, resource_type);

CREATE TABLE IF NOT EXISTS catalog_subscriptions (
    id                TEXT PRIMARY KEY,
    space_id          TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    catalog_item_id   TEXT NOT NULL REFERENCES catalog_items(id) ON DELETE CASCADE,
    created_at        DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(space_id, catalog_item_id)
);

CREATE INDEX IF NOT EXISTS idx_catalog_subs_space ON catalog_subscriptions(space_id);

CREATE TABLE IF NOT EXISTS interface_instances (
    id              TEXT PRIMARY KEY,
    org_id          TEXT NOT NULL REFERENCES organizations(id),
    space_id        TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    interface_type  TEXT NOT NULL,
    config          TEXT NOT NULL DEFAULT '{}',
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_iface_instances_space ON interface_instances(space_id);

CREATE TABLE IF NOT EXISTS persona_bindings (
    id                      TEXT PRIMARY KEY,
    space_id                TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    persona_id              TEXT NOT NULL,
    interface_instance_id   TEXT NOT NULL REFERENCES interface_instances(id) ON DELETE CASCADE,
    created_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(persona_id, interface_instance_id)
);

CREATE INDEX IF NOT EXISTS idx_bindings_space ON persona_bindings(space_id);
