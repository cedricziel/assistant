## MODIFIED Requirements

### Requirement: Web-ui opens per-space database

The web-ui binary (`assistant webui serve`) SHALL obtain each space's
`StorageLayer` per request from a pool cache keyed by `(org, space)` and built on
`OrgPoolFactory::space_db_path`, and MUST NOT hold a single startup
`StorageLayer` for runtime data (conversations, messages, scheduler). It
MUST NOT accept a `--db-path` flag or `ASSISTANT_DB_PATH` variable, and MUST NOT
use a hard-coded `assistant.db` or `default/default` path for runtime data.

#### Scenario: Web-ui after fresh install

- **WHEN** `assistant webui serve` starts on a host with no prior install and the
  default admin calls `GET /api/conversations`
- **THEN** the request is served from
  `~/.assistant/orgs/default/spaces/default/space.db`, opened through the pool
  cache

#### Scenario: Web-ui after migration

- **WHEN** `assistant webui serve` starts on a host that has been migrated via
  `migrate_database`
- **THEN** the web-ui opens `space.db` on first request and ignores any sibling
  `assistant.db.legacy`

#### Scenario: Two spaces, two databases

- **GIVEN** spaces `A1` and `B1` in different orgs
- **WHEN** members of each call `POST /api/conversations`
- **THEN** each conversation exists only in its own space's `space.db`

#### Scenario: Removed override flag

- **WHEN** `assistant webui serve --db-path /tmp/x.db` is run
- **THEN** argument parsing fails with an unknown-argument error
