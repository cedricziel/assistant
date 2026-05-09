## 1. Schema + helper (TDD red)

- [ ] 1.1 Add `crates/web-ui/src/errors.rs` with `pub fn json_error(status: StatusCode, message: impl Into<String>) -> Response` and a `#[cfg(test)]` test verifying the body is `{"error":"<msg>"}` with content-type `application/json`. Re-export from `crates/web-ui/src/lib.rs`.
- [ ] 1.2 Add `pub struct ErrorBody { pub error: String }` (with `serde::Serialize` and `utoipa::ToSchema`) in `crates/web-ui/src/openapi.rs`. Add it to the `components(schemas(...))` macro list. Mark `ApiErrorResponse` `#[deprecated(since = "<next>", note = "use ErrorBody")]`.
- [ ] 1.3 Run `cargo test -p assistant-web-ui` — confirm new helper test passes; existing tests still pass.

## 2. Sweep handlers — secured endpoints get `(status = 401, body = ErrorBody)`

- [ ] 2.1 `crates/web-ui/src/api/spaces.rs` — 5 endpoints. Add 401 + 403 (where emitted) to each `responses(...)`. Replace plain-text returns (`(StatusCode::FORBIDDEN, "access denied")` etc.) with `json_error(...)`.
- [ ] 2.2 `crates/web-ui/src/api/users.rs` — 5 endpoints. Same.
- [ ] 2.3 `crates/web-ui/src/api/members.rs` — 4 endpoints. Same.
- [ ] 2.4 `crates/web-ui/src/api/orgs.rs` — 4 endpoints. Same.
- [ ] 2.5 `crates/web-ui/src/api/catalog.rs` — 7 endpoints. Same.
- [ ] 2.6 `crates/web-ui/src/api/interfaces.rs` — 3 endpoints. Same.
- [ ] 2.7 `crates/web-ui/src/api/bindings.rs` and `crates/web-ui/src/api/templates.rs` — fill in any missing 401 responses; convert plain-text bodies.
- [ ] 2.8 `crates/web-ui/src/api/api_keys.rs`, `account.rs`, `analytics.rs`, `commands.rs`, `logs.rs`, `traces.rs` — confirm 401 docs already present; add `body = ErrorBody` to existing 401 entries; convert any remaining plain-text bodies.
- [ ] 2.9 `crates/web-ui/src/api/mod.rs` (conversations), `personas.rs`, `workflows.rs`, `webhooks.rs`, `skills.rs`, `agents.rs`, `push.rs` — most already have 401. Update each existing `(status = 401, description = "Unauthorized")` to include `body = ErrorBody`. Convert any `Json(serde_json::json!({"error": ...}))` to use `json_error` for consistency.
- [ ] 2.10 Verify no remaining plain-text 4xx/5xx responses under `crates/web-ui/src/api/**` via `rg -n '(StatusCode::[A-Z_]+,\s*")\b' crates/web-ui/src/api/`.

## 3. Remove deprecated schema

- [ ] 3.1 Confirm `ApiErrorResponse` has zero references in `crates/web-ui/src/api/**` via `rg ApiErrorResponse crates/web-ui/`.
- [ ] 3.2 Remove `ApiErrorResponse` from the `components(schemas(...))` list in `crates/web-ui/src/openapi.rs`.
- [ ] 3.3 Delete the `pub struct ApiErrorResponse` definition.
- [ ] 3.4 Run `make check` and `make lint` — both green.

## 4. Regenerate OpenAPI + Flutter client

- [ ] 4.1 `make dump-openapi` to update `openapi.json`. Inspect the diff: every secured operation now lists `401` referencing `ErrorBody`.
- [ ] 4.2 `make generate-flutter-client` to regenerate `app/packages/assistant_api/`.
- [ ] 4.3 Commit the regenerated client in a separate commit (`chore(api-client): regenerate from updated spec`) within the same PR for reviewability.

## 5. Spectral ruleset + CI

- [ ] 5.1 Add `openapi-spectral.yaml` at the repo root containing rules `secured-operations-document-401` and `401-uses-error-body`, both `severity: error`, extending `spectral:oas`.
- [ ] 5.2 Add `make lint-openapi` target to `Makefile` running `spectral lint openapi.json --ruleset openapi-spectral.yaml --fail-severity=error`. Document the `npm install -g @stoplight/spectral-cli` prerequisite alongside the `openapi-generator` brew prerequisite in `CLAUDE.md`.
- [ ] 5.3 Add a CI job to `.github/workflows/ci.yml` (or new `openapi.yml`) named `openapi-lint`: setup-node, install pinned spectral-cli, run the make target. Trigger on push-to-main and on PRs touching `openapi.json`, `openapi-spectral.yaml`, or `crates/web-ui/**`.
- [ ] 5.4 Add a second CI step that runs `make dump-openapi` against a fresh build and `git diff --exit-code openapi.json` to catch stale specs.
- [ ] 5.5 Run `make lint-openapi` locally — must exit 0. Run the drift check locally (`make dump-openapi && git diff --exit-code openapi.json`) — must exit 0.

## 6. Smoke + ship

- [ ] 6.1 `make precommit` — full pre-commit suite green.
- [ ] 6.2 Open PR titled `chore(web-ui): standardize error envelopes and lint OpenAPI 401 coverage`. Body links to the two specs. Include before/after `curl` examples showing `Content-Type: application/json` on a 403 response.
- [ ] 6.3 Merge after review. No deploy concerns — change is self-contained in the next release artifact.
- [ ] 6.4 Archive this change with `openspec archive openapi-error-response-coverage` after the next release tags out.
