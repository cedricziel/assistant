**Scope adjustment**: this PR closes the 401 documentation gap on the multi-org endpoints (the user's actual concern) and lands the `ErrorBody` schema + `json_error` helper + Spectral CI rule. Body-shape normalization across the well-documented files (mod.rs, personas.rs, workflows.rs, etc.) and `ApiErrorResponse` removal (still used by A2A) move to a follow-up.

## 1. Schema + helper

- [x] 1.1 Added `crates/web-ui/src/errors.rs` with `json_error(status, msg)` returning the documented envelope. Two `#[cfg(test)]` tests verifying body shape and `impl Into<String>` ergonomics.
- [x] 1.2 Added `pub struct ErrorBody { pub error: String }` in `crates/web-ui/src/openapi.rs` and registered it in `components(schemas(...))`. `ApiErrorResponse` retained for A2A — removal deferred.
- [x] 1.3 `cargo test -p assistant-web-ui errors::` — 2 new tests pass; existing tests untouched.

## 2. Sweep secured endpoints — gap files

- [ ] 2.1 `crates/web-ui/src/api/spaces.rs` — 5 endpoints. Add `(status = 401, description = "Unauthorized", body = ErrorBody)`. Convert plain-text returns to `json_error`.
- [ ] 2.2 `crates/web-ui/src/api/users.rs` — 5 endpoints. Same.
- [ ] 2.3 `crates/web-ui/src/api/members.rs` — 4 endpoints. Same.
- [ ] 2.4 `crates/web-ui/src/api/orgs.rs` — 4 endpoints. Same.
- [ ] 2.5 `crates/web-ui/src/api/catalog.rs` — 7 endpoints. Same.
- [ ] 2.6 `crates/web-ui/src/api/interfaces.rs` — 3 endpoints. Same.
- [ ] 2.7 `crates/web-ui/src/api/bindings.rs` and `crates/web-ui/src/api/templates.rs` — fill in any missing 401 responses; convert plain-text bodies.

## 2b. Follow-up (NOT this PR)

- [ ] 2.8 `api_keys.rs`, `account.rs`, `analytics.rs`, `commands.rs`, `logs.rs`, `traces.rs` — add `body = ErrorBody` to existing 401 entries.
- [ ] 2.9 `mod.rs` (conversations), `personas.rs`, `workflows.rs`, `webhooks.rs`, `skills.rs`, `agents.rs`, `push.rs` — update existing 401 entries to `body = ErrorBody`.
- [ ] 2.10 Cross-tree audit: no remaining plain-text 4xx/5xx in `crates/web-ui/src/api/**`.

## 3. Remove deprecated schema (DEFERRED)

`ApiErrorResponse` is still used by `crates/web-ui/src/a2a/handlers.rs`. Removal needs A2A migration first; out of scope here.

## 4. Regenerate OpenAPI + Flutter client

- [ ] 4.1 `make dump-openapi` to update `openapi.json`.
- [ ] 4.2 `make generate-flutter-client` to regenerate `app/packages/assistant_api/`.
- [ ] 4.3 Commit the regenerated client in a separate commit within the same PR.

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
