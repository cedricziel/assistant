## ADDED Requirements

### Requirement: All API error responses use the `{"error": "..."}` envelope

Every HTTP response with a 4xx or 5xx status code emitted by handlers under `crates/web-ui/src/api/**` SHALL have a body matching the JSON envelope `{"error": "...message..."}`. Plain-text bodies and alternate shapes are NOT permitted.

#### Scenario: Forbidden access returns the envelope

- **WHEN** a handler rejects a request with `403 Forbidden`
- **THEN** the response body SHALL be `application/json` with shape `{"error": "<reason>"}` AND SHALL NOT be a plain-text string

#### Scenario: Internal server error returns the envelope

- **WHEN** a handler returns `500 Internal Server Error`
- **THEN** the response body SHALL be `application/json` with shape `{"error": "<reason>"}`

#### Scenario: Unauthorized returns the envelope

- **WHEN** an authenticated endpoint rejects a request with `401 Unauthorized` (e.g., expired token)
- **THEN** the response body SHALL be `application/json` with shape `{"error": "<reason>"}`

#### Scenario: OAuth2 endpoints exempt

- **WHEN** an endpoint under `/oauth/...` returns an error per RFC 6749 §5.2
- **THEN** the response body MAY use the OAuth-defined `{error, error_description, error_uri}` shape AND this requirement does NOT apply

### Requirement: `ErrorBody` schema is the single source of truth

The OpenAPI document SHALL declare exactly one schema named `ErrorBody` in `components.schemas` representing the error envelope, with shape `{ "type": "object", "required": ["error"], "properties": { "error": { "type": "string" } } }`.

#### Scenario: ErrorBody schema present

- **WHEN** the committed `openapi.json` is loaded
- **THEN** `components.schemas.ErrorBody` SHALL exist AND match the shape above

#### Scenario: Legacy ApiErrorResponse removed once unused

- **WHEN** no `#[utoipa::path]` macro references `ApiErrorResponse` anywhere in `crates/web-ui/src/api/**`
- **THEN** `ApiErrorResponse` SHALL be removed from `components(schemas(...))` in `crates/web-ui/src/openapi.rs` AND its `pub struct` definition deleted

### Requirement: Every secured operation documents 401 with `ErrorBody`

Every OpenAPI operation that declares a non-empty `security` requirement (bearer token or OAuth2 scopes) SHALL list a `401` response whose `application/json` body schema `$ref`s `#/components/schemas/ErrorBody`.

#### Scenario: Bearer-secured GET documents 401

- **WHEN** an operation declares `security(("bearer_token" = []))`
- **THEN** its `responses(...)` block SHALL include `(status = 401, description = "Unauthorized", body = ErrorBody)`

#### Scenario: OAuth-secured POST documents 401

- **WHEN** an operation declares `security(("oauth2" = ["..."]))`
- **THEN** its `responses(...)` block SHALL include `(status = 401, description = "Unauthorized", body = ErrorBody)`

#### Scenario: Public operation does NOT need 401

- **WHEN** an operation declares an empty `security(())` (or no security at all)
- **THEN** documenting `401` is NOT required (though permitted if the handler can return one)

### Requirement: Server provides a single helper for emitting error responses

The `crates/web-ui` crate SHALL expose `pub fn json_error(status: StatusCode, message: impl Into<String>) -> axum::response::Response` that returns the documented envelope. New handler code SHALL use this helper rather than constructing tuple responses ad hoc.

#### Scenario: Helper returns the envelope

- **GIVEN** `json_error(StatusCode::FORBIDDEN, "access denied")`
- **THEN** the resulting response SHALL have status `403`, content-type `application/json`, and body `{"error":"access denied"}`

#### Scenario: Existing handlers migrated

- **WHEN** the change lands
- **THEN** every occurrence of `(StatusCode::X, "<plain text>").into_response()` under `crates/web-ui/src/api/**` SHALL be replaced by `json_error(StatusCode::X, "<plain text>")` AND every `(StatusCode::X, Json(serde_json::json!({"error": ...}))).into_response()` SHALL be replaced by the same helper for consistency
