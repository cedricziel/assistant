## Why

The web-UI OpenAPI surface has uneven 401 documentation (e.g. `api/spaces.rs`, `api/users.rs`, `api/members.rs`, `api/catalog.rs`, `api/interfaces.rs` declare `security(...)` but list zero `401` responses), and the actual error response shapes disagree with the project's stated convention. `AGENTS.md` says all errors use `{"error": "...message..."}`, yet several handlers return plain text bodies (`(StatusCode::FORBIDDEN, "access denied")`) and `crates/web-ui/src/openapi.rs:152` defines an unrelated `ApiErrorResponse { code, message }` schema that doesn't match either reality. A future client (third-party SDK, CI lint, or the Flutter 401 interceptor that just landed) cannot rely on the spec to know "what does an unauthorized response actually look like." This change closes that gap and adds a CI rule so it can't regress.

## What Changes

- Define a single error envelope schema `ErrorBody { error: String }` in `crates/web-ui/src/openapi.rs`, matching the documented convention. Mark the legacy `ApiErrorResponse` as deprecated and remove its references once unused.
- Add an internal helper `pub fn json_error(status, msg) -> Response` in the web-ui crate that returns `(status, Json(json!({"error": msg})))`. Replace every plain-text error return in `crates/web-ui/src/api/**` with the helper. Where handlers already use `Json(serde_json::json!({"error": ...}))`, leave them alone (they already match).
- For every endpoint whose `#[utoipa::path]` declares `security(("bearer_token" = []), ...)`, add `(status = 401, description = "Unauthorized", body = ErrorBody)` to the `responses(...)` block. Same for `403` where it's currently undocumented but produced.
- Re-export `openapi.json` via `make dump-openapi` and regenerate the Flutter client via `make generate-flutter-client`.
- Add a Spectral ruleset `openapi-spectral.yaml` at the repo root and a CI step (`.github/workflows/ci.yml`) that runs `spectral lint openapi.json --ruleset openapi-spectral.yaml --fail-severity=error`. The ruleset enforces: every `securityRequirement`-bearing operation MUST list a `401` response, AND that `401`'s body MUST `$ref` the `ErrorBody` schema.

## Capabilities

### New Capabilities

- `openapi-error-envelope`: The contract for HTTP error responses (shape, status mapping, OpenAPI documentation rules) and the helper that enforces it server-side.
- `openapi-spectral-ci`: Spectral lint rules that gate the committed `openapi.json` in CI, with a focus on auth response coverage.

### Modified Capabilities

(none — no existing capability owns OpenAPI hygiene yet.)

## Impact

- **Code touched (server)**: `crates/web-ui/src/openapi.rs` (add `ErrorBody`, deprecate `ApiErrorResponse`); every file under `crates/web-ui/src/api/` (uniform `responses(401|403)` annotations, plain-string → JSON envelope conversions); `crates/web-ui/src/lib.rs` or a new `errors.rs` for the `json_error` helper.
- **Code touched (client)**: `app/packages/assistant_api/` regenerated from the new `openapi.json`; the manual `ApiClient` wrapper unchanged.
- **CI**: new step in `.github/workflows/ci.yml` (or a dedicated `openapi.yml`) running Spectral. New file `openapi-spectral.yaml` in repo root.
- **Behavior change**: error response _bodies_ on a handful of currently-plain-text endpoints become JSON envelopes. Any consumer that parsed plain strings will need to read `body.error`. This is not considered a breaking change because (a) the Flutter app does not parse error bodies today (only status codes), (b) no external SDK exists, and (c) the new shape matches the long-documented project convention.
- **Non-goals**:
  - Rewriting `OAuthErrorResponse` (RFC 6749 §5.2 has its own `{error, error_description}` shape that we keep verbatim).
  - Adding generic 4xx/5xx schemas to non-secured endpoints (out of scope; the rule targets auth-protected operations).
  - Generating typed Dart error classes — openapi-generator's dio template doesn't support per-response error classes; the interceptor matches by status code regardless.
  - Changing any response status codes.
- **User-facing documentation needed**: No. This is a developer/contract hygiene change. A short note in the next release's changelog under "internal" is sufficient.
