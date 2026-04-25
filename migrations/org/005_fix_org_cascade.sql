-- Fix inconsistent ON DELETE policy: catalog_items and interface_instances
-- reference organizations(id) without CASCADE, while all space-scoped FKs
-- use ON DELETE CASCADE.  SQLite cannot ALTER COLUMN, so we recreate.

-- 1. catalog_items: add ON DELETE CASCADE on org_id
CREATE TABLE IF NOT EXISTS catalog_items_new (
    id              TEXT PRIMARY KEY,
    org_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    resource_type   TEXT NOT NULL,
    name            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(org_id, resource_type, name)
);

INSERT OR IGNORE INTO catalog_items_new SELECT * FROM catalog_items;
DROP TABLE catalog_items;
ALTER TABLE catalog_items_new RENAME TO catalog_items;

CREATE INDEX IF NOT EXISTS idx_catalog_items_org_type ON catalog_items(org_id, resource_type);

-- 2. interface_instances: add ON DELETE CASCADE on org_id
CREATE TABLE IF NOT EXISTS interface_instances_new (
    id              TEXT PRIMARY KEY,
    org_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    space_id        TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    interface_type  TEXT NOT NULL,
    config          TEXT NOT NULL DEFAULT '{}',
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO interface_instances_new SELECT * FROM interface_instances;
DROP TABLE interface_instances;
ALTER TABLE interface_instances_new RENAME TO interface_instances;

CREATE INDEX IF NOT EXISTS idx_iface_instances_space ON interface_instances(space_id);
