Each numbered section is one PR, in order. Rebase onto `main` before each.

## 1. PR 1 — Store and versions

- [ ] 1.1 ADR `docs/adr/adr-0017-sandboxed-artifacts.md`: space-scoped artifacts, full-content versions, opaque-origin CSP sandbox, signed render URLs
- [ ] 1.2 Failing migration test for `migrations/046_artifacts.sql` (D1 tables, `CHECK` on `kind` and `author_kind`, PK on `(artifact_id, version)`) (confirm RED); write the migration and register it in `crates/storage/src/lib.rs`; GREEN
- [ ] 1.3 Add `Artifact`, `ArtifactKind`, `ArtifactVersion`, `ArtifactConflict` types in `crates/core`; failing serde round-trip tests first
- [ ] 1.4 Failing contract tests in `crates/storage/tests/contract/` for `ArtifactStore`: create → v1, write with matching `base_version` → v+1, stale `base_version` → conflict carrying latest, restore sets `restored_from`, 5 MiB cap, list by space, delete cascades versions and comments (confirm RED)
- [ ] 1.5 Implement `SqliteArtifactStore` (transactional CAS on `latest_version`) and `InMemoryArtifactStore`; expose from `StorageLayer`; GREEN

## 2. PR 2 — Persona tools and chat event

- [ ] 2.1 Failing tests for `artifact-create`, `artifact-read`, `artifact-update`, `artifact-list` using `InMemoryArtifactStore` (confirm RED); implement handlers in `crates/tool-executor/src/builtins/artifacts.rs`; GREEN
- [ ] 2.2 Failing tests for `artifact-edit`: single match applies, zero or multiple matches error naming the entry, stale `base_version` errors with latest content (confirm RED); implement; GREEN
- [ ] 2.3 Failing test: a turn run by a space `Viewer` cannot update (confirm RED); route tool authorization through a shared `ArtifactAccess::resolve` in `crates/core` (space-role part only for now); GREEN
- [ ] 2.4 Register the tools in `register_builtins`; failing orchestrator test (with `ScriptedLlmProvider`) that a successful artifact tool emits `OrchestratorEvent::ArtifactChanged` and persists it to `conversation_events`, and that `artifact-read` emits nothing (confirm RED); implement; GREEN
- [ ] 2.5 Add the artifact guidance section to the system prompt; snapshot test

## 3. PR 3 — REST API and live stream

- [ ] 3.1 Failing route tests for `GET/POST /api/artifacts`, `GET/PATCH/DELETE /api/artifacts/{id}` (Viewer read-only, Member create, 404 for invisible) (confirm RED); implement with utoipa annotations; GREEN
- [ ] 3.2 Failing tests for `PUT /content` (409 with `ArtifactConflict`, 413), `GET /versions[/{n}]`, `POST /restore` (confirm RED); implement; GREEN
- [ ] 3.3 Failing tests for `GET /api/artifacts/{id}/events` (`snapshot`, `version_created`, `deleted`) (confirm RED); implement `ArtifactBroadcast` and the SSE handler with `sse_response()`; GREEN
- [ ] 3.4 Failing test that the chat SSE stream emits `event: artifact` for `ArtifactChanged` (confirm RED); implement; GREEN
- [ ] 3.5 `make dump-openapi`; check the `openapi.json` diff contains only the new operations; `make generate-flutter-client`

## 4. PR 4 — Sandbox and slides

- [ ] 4.1 Failing tests for render-URL signing: valid, expired, tampered version, wrong artifact, and a viewer whose grant was removed between issue and redemption (confirm RED); implement HMAC over `{id, version, sub, exp}` (2 minutes) keyed from `jwt_secret`; GREEN
- [ ] 4.2 Failing route tests for `GET /artifact-content/{id}/{version}`: 404 without signature, role re-resolved per request, link token revalidated per request (revoked link or disabled policy → 404 at once), exact CSP header (sandbox with `allow-scripts` only, `connect-src`/`form-action`/`base-uri`/`object-src` `'none'`), `nosniff`, `no-referrer`, `private, no-store` (confirm RED); implement; GREEN
- [ ] 4.3 Failing tests for `[artifacts]` in `server.toml`: defaults, empty allowlist, `content_origin` makes main-origin requests 404 (confirm RED); implement config and host check; GREEN
- [ ] 4.4 Failing tests for the slides renderer: frontmatter parsing, `---` splitting (not inside fenced code), three slides → three sections, print CSS and bridge script present (confirm RED); implement with `pulldown-cmark` and an embedded template; GREEN
- [ ] 4.5 Render `markdown`/`code`/`data` to HTML for link views; tests first
- [ ] 4.6 Playwright test: an `html` artifact's `fetch('/api/...')` is blocked and `document.cookie` is empty inside the frame
- [ ] 4.7 Failing tests for the injected bridge: a static `html` artifact gets the script before `</body>` (or at the end), stored versions are unchanged, and only `selection`/`open-link` messages are posted (confirm RED); implement the injection and anchor click listener; GREEN
- [ ] 4.8 `make dump-openapi` for `POST /render-url`; `make generate-flutter-client`

## 5. PR 5 — Sharing

- [ ] 5.1 Failing migration test for `migrations/org/010_artifact_shares.sql` (grants, links with unique `token_hash`, policy) (confirm RED); write it; GREEN
- [ ] 5.2 Failing contract tests for `ArtifactShareStore` (Sqlite + InMemory): grant upsert/remove, link create stores only a hash, revoke, expiry, policy default off (confirm RED); implement; GREEN
- [ ] 5.3 Failing unit tests for full `ArtifactAccess::resolve` covering the role matrix (space role × owner × grant × org link; 404 not 403) (confirm RED); implement; GREEN
- [ ] 5.4 Failing tests for the `ArtifactCtx` extractor: member via `X-Space-Id`, org admin without a membership, grantee without membership, org link token supplied per request, other org → 404 (confirm RED); implement and switch artifact routes and tools to it; GREEN
- [ ] 5.5 Failing route tests for grants, links, `shared-with-me` and policy (only `manage` changes access; public link 403 when policy off; toggling policy hides and restores links) (confirm RED); implement; GREEN
- [ ] 5.6 Failing tests for `GET /a/{token}`: pinned version, latest, revoked/expired → 404, org link needs same-org sign-in, disabled policy → 404 immediately, content route revalidates the token, public frame shows the notice (confirm RED); implement in `public_routes`; GREEN
- [ ] 5.7 `make dump-openapi`; `make generate-flutter-client`

## 6. PR 6 — Comments and summons

- [ ] 6.1 Failing contract tests for comment storage: thread, reply, resolve permissions, anchor JSON round trip (confirm RED); implement in `ArtifactStore`; GREEN
- [ ] 6.2 Failing tests for re-anchoring (prefix+quote+suffix, quote only, `outdated`) in `crates/core` (confirm RED); implement; GREEN
- [ ] 6.3 Failing route tests for `GET/POST /comments`, `PATCH /comments/{cid}` and SSE `comment_*` events (confirm RED); implement; GREEN
- [ ] 6.4 Failing tests for summons: `@slug` from an editor publishes `artifact.summon`, a commenter's mention does not, 31st summon in an hour → 429 (confirm RED); implement `POST /summon` and mention parsing; GREEN
- [ ] 6.5 Failing runtime test: `artifact.summon` starts a turn as the summoning user in a hidden collaboration conversation owned by the artifact, the Persona's reply is stored with `author_kind = persona`, and reading the conversation requires current artifact `view` (revoked editor → 404) (confirm RED); implement; GREEN
- [ ] 6.6 Add the `artifact-comment` tool; tests first
- [ ] 6.7 `make dump-openapi`; `make generate-flutter-client`

## 7. PR 7 — Flutter viewer and chat card

- [ ] 7.1 Spike: confirm `webview_flutter` loads a sandboxed page on macOS and iOS; add the dependency; record the outcome in the PR
- [ ] 7.2 `ArtifactRepository` + `ArtifactViewModel` with unit tests for loading, success, error and empty
- [ ] 7.3 `ArtifactFrame` (web `HtmlElementView` iframe with `sandbox`, webview on Apple) with a widget test; render-URL refresh on expiry, test first
- [ ] 7.4 `ArtifactViewer` for native kinds with widget tests per kind
- [ ] 7.5 `ArtifactCard` in the message list and the side panel / full-screen route at the 900 px breakpoint, with widget tests; routes `/artifacts`, `/artifacts/:id`
- [ ] 7.6 Update Playwright screenshot baselines for chat and add viewer baselines; note the visual diff in the PR

## 8. PR 8 — Flutter editing, versions, comments, sharing

- [ ] 8.1 `ArtifactEditor` with save and the 409 conflict view; ViewModel unit tests (success, conflict, error) and widget tests
- [ ] 8.2 Version picker with diff; widget tests
- [ ] 8.3 Comment sidebar with anchored threads, `outdated` state and "Ask <Persona>"; selection bridge from the injected script validated by `event.source` and `event.origin == "null"`, plus `open-link` confirmation; tests
- [ ] 8.4 Share dialog (grants, org/public links, pin, revoke) shown only for `manage`; tests
- [ ] 8.5 Admin screen toggle for `allow_public_links`; tests
- [ ] 8.6 Update Playwright baselines; note the visual diff in the PR

## 9. PR 9 — Messengers, MCP, A2A, docs

- [ ] 9.1 Failing tests that Slack, Matrix, Mattermost, Nextcloud, Signal and CLI replies end with the artifact link line, and name-only without a public URL (confirm RED); implement in the shared reply formatting; GREEN
- [ ] 9.2 Failing tests for `artifacts://{id}` in MCP `resources/list|read` (confirm RED); implement; GREEN
- [ ] 9.3 Failing test that A2A task responses include an `Artifact` per changed artifact (confirm RED); implement the conversion; GREEN
- [ ] 9.4 Write `docs/artifacts.md` (user guide) and add `[artifacts]` and the public-link policy to `docs/operations/configuration.md`; update `docs/glossary.md`

## 10. Verification

- [ ] 10.1 `make lint && make format && make test`
- [ ] 10.2 `make lint-flutter && make test-flutter`
