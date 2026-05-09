## ADDED Requirements

### Requirement: Spectral ruleset committed at repo root

A Spectral ruleset SHALL exist at `openapi-spectral.yaml` in the repository root, extending `spectral:oas` and adding project-specific rules. The ruleset SHALL be readable without additional configuration files.

#### Scenario: Ruleset file is discoverable

- **WHEN** a contributor clones the repo
- **THEN** `openapi-spectral.yaml` SHALL exist at the root AND SHALL be valid YAML loadable by `@stoplight/spectral-cli`

### Requirement: Lint rule enforces 401 documentation on secured operations

The ruleset SHALL include a rule named `secured-operations-document-401` with `severity: error`. The rule MUST flag any operation whose `security` array is non-empty and whose `responses` object does not contain a `"401"` key.

#### Scenario: Missing 401 fails the lint

- **GIVEN** an OpenAPI operation declaring `security: [{ bearer_token: [] }]` AND `responses` containing only `200`
- **WHEN** `spectral lint openapi.json --ruleset openapi-spectral.yaml --fail-severity=error` runs
- **THEN** the command SHALL exit with non-zero status AND the output SHALL identify the offending operation

#### Scenario: Documented 401 passes

- **GIVEN** an OpenAPI operation that declares a security requirement AND lists `responses.401`
- **WHEN** Spectral runs
- **THEN** that operation SHALL NOT trigger the `secured-operations-document-401` rule

### Requirement: Lint rule enforces `ErrorBody` schema on 401 responses

The ruleset SHALL include a rule named `401-uses-error-body` with `severity: error`. The rule MUST flag any `responses.401` whose `application/json.schema` does not `$ref` `#/components/schemas/ErrorBody`.

#### Scenario: 401 with wrong schema fails

- **GIVEN** an operation whose `responses.401.content.application/json.schema` `$ref`s `ApiErrorResponse`
- **WHEN** Spectral runs
- **THEN** the lint SHALL fail with a reference to the offending operation

#### Scenario: 401 with no body schema fails

- **GIVEN** an operation whose `responses.401` has no `content` field
- **WHEN** Spectral runs
- **THEN** the lint SHALL fail (the rule treats missing content as a violation)

#### Scenario: Correct ErrorBody passes

- **GIVEN** `responses.401.content.application/json.schema.$ref == "#/components/schemas/ErrorBody"`
- **WHEN** Spectral runs
- **THEN** the operation SHALL pass

### Requirement: CI runs Spectral on every push and PR

The repository's CI configuration SHALL run `spectral lint openapi.json --ruleset openapi-spectral.yaml --fail-severity=error` on every push to `main` and every pull request that touches `openapi.json`, `openapi-spectral.yaml`, or any file under `crates/web-ui/`.

#### Scenario: PR violating the rules fails CI

- **WHEN** a pull request adds a new endpoint without a `401` response AND CI runs
- **THEN** the OpenAPI lint job SHALL fail AND merging SHALL be blocked by the required check

#### Scenario: Local make target available

- **WHEN** a contributor runs `make lint-openapi`
- **THEN** the same Spectral command as CI SHALL execute against the local `openapi.json` AND exit with the same status

### Requirement: OpenAPI spec stays in sync with handlers

`make dump-openapi` SHALL produce an `openapi.json` byte-identical to the committed file when run against an unchanged tree. CI SHALL detect drift.

#### Scenario: CI catches stale openapi.json

- **WHEN** a PR modifies routes in `crates/web-ui` without re-running `make dump-openapi`
- **THEN** CI SHALL re-export the spec, diff against the committed file, and fail if they differ

#### Scenario: Generated client stays in sync

- **WHEN** `openapi.json` changes
- **THEN** `make generate-flutter-client` SHALL be re-run AND the regenerated `app/packages/assistant_api/` SHALL be committed in the same PR
