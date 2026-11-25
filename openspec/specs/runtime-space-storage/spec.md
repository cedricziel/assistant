## Purpose

Where the orchestrator and the web server keep runtime data: one database per space under the install root.

## Requirements

### Requirement: Orchestrator opens per-space database

The orchestrator binary (`assistant orchestrator run`) SHALL resolve its
primary `StorageLayer` path via `OrgPoolFactory::space_db_path("default",
"default")` and MUST NOT fall back to `~/.assistant/assistant.db` for
runtime data.

#### Scenario: Fresh install starts orchestrator

- **WHEN** an operator runs `assistant orchestrator run` on a host with no
  prior `~/.assistant/` directory
- **THEN** the orchestrator creates
  `~/.assistant/orgs/default/spaces/default/space.db` and opens it as the
  primary `StorageLayer`
- **AND** no file named `~/.assistant/assistant.db` is created at any point

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

#### Scenario: Two spaces, two databases

- **GIVEN** spaces `A1` and `B1` in different orgs
- **WHEN** members of each call `POST /api/conversations`
- **THEN** each conversation exists only in its own space's `space.db`

#### Scenario: Removed override flag

- **WHEN** `assistant webui serve --db-path /tmp/x.db` is run
- **THEN** argument parsing fails with an unknown-argument error
