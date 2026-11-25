# Tasks

Each numbered section is one stacked PR, built on `openrouter-full-provider`. Every PR ends with `make lint && make format && make test` passing. PRs that touch `app/` also run `make lint-flutter && make test-flutter`. `openspec validate ai-settings-ui --strict` must stay clean.

## 1. ADR and layer engine

- [ ] 1.1 Write `docs/adr/adr-0017-layered-settings.md` (D9). Verify that it links ADR-0014 and that `docs/adr` lists it.
- [ ] 1.2 Write failing tests in `crates/core/src/settings/` for the merge:
  - deep merge of objects
  - lists and scalars replace
  - `null` removes a value
  - space layer above org layer, org above file, file above defaults

  Confirm they are red. Implement the merge, then verify.

- [ ] 1.3 Write failing tests: validation of the merged value through `OrgConfig` reports dotted paths (via `serde_path_to_error`), including the reasoning cross-field rule. Confirm red. Implement, then verify.
- [ ] 1.4 Write failing tests for source reporting (`default`, `file`, `org`, `space`, `locked`) and the inherited value. Confirm red. Implement, then verify.
- [ ] 1.5 Write failing tests for the space allowlist and locks:
  - a key outside the allowlist is rejected
  - a locked key is rejected on write
  - a stored override that is now locked is reported but not applied

  Confirm red. Implement, then verify.

## 2. Setting secrets

- [ ] 2.1 Write failing storage tests for the `setting_secrets` migration (`migrations/org/011`): CRUD, and ciphertext only. Confirm red. Add the migration and the store, then verify.
- [ ] 2.2 Write failing tests:
  - seal and open with `setting_secret_aad`
  - opening fails on a row copied to another org
  - an unknown key ID is reported as `unreadable`

  Confirm red. Implement, then verify.

- [ ] 2.3 Write failing tests: a stored secret takes precedence over the `org.toml` key, removing it restores the fallback, and the redaction pass leaves no secret in the merged view or in `Debug` output. Confirm red. Implement, then verify.
- [ ] 2.4 Write a failing test that `assistant backup` includes `setting_secrets`. Confirm red, or green if this is already covered, and fix any gap.

## 3. Snapshots and per-space voice

- [ ] 3.1 Write failing tests: `OrgContexts` returns an `ArcSwap` snapshot holding the config, LLM, TTS, transcription, images and decider, and a swap is visible to the next `get`. Confirm red. Implement, then verify.
- [ ] 3.2 Write failing tests: a space's effective config applies its space overrides (`space_ai_settings` migration `046`), and two orgs get different TTS providers. Confirm red. Implement, then verify.
- [ ] 3.3 Write a failing test: a turn keeps its starting snapshot when a swap happens mid-turn (a scripted LLM blocks until the swap). Confirm red. Implement snapshot capture at turn start, then verify.
- [ ] 3.4 Write failing tests: the web-ui audio, voice-send and `voice_response` paths, and the messenger voice handlers, resolve providers from the space. Confirm red. Remove the process-wide providers from `ApiState`, and the `default`-org lookup, then verify.

## 4. Build before commit, and reload

- [ ] 4.1 Write failing tests: a save that fails any provider build leaves the database row and the snapshot unchanged and returns the provider error; `validate` never writes. Confirm red. Implement the settings service, then verify.
- [ ] 4.2 Write failing tests: `assistant reload` and the reload service re-read `org.toml`, and a broken file keeps the running config. Confirm red. Implement the CLI command and the service, then verify.
- [ ] 4.3 Write failing tests for the version compare-and-swap: concurrent saves, the second one conflicts. Confirm red. Implement, then verify.

## 5. Org settings API

- [ ] 5.1 Write failing route tests for `GET/PATCH/DELETE /api/orgs/{org_id}/settings/ai`, `validate` and `reload`:
  - 200 for org admins, 403 otherwise
  - 409 on a stale `If-Match`
  - 422 with per-path errors
  - ETag handling

  Confirm red. Implement the handlers with `#[utoipa::path]` and add them to `ApiDoc`. Verify.

- [ ] 5.2 Write failing route tests for `PUT/DELETE .../settings/secrets/{name}`: status-only responses, unknown names rejected, the key-health check on OpenRouter keys (wiremock), and no secret values in any response or log (checked with a log capture). Confirm red. Implement, then verify.
- [ ] 5.3 Update `openapi.json` and confirm `committed_openapi_json_matches_code` passes.

## 6. Lookups, key health and preview

- [ ] 6.1 Write failing tests for `/settings/ai/models` (by kind and search, cached for 10 minutes on a test clock, static lists for non-catalog providers) and `/voices`. Confirm red. Implement, then verify.
- [ ] 6.2 Write failing tests for `/settings/ai/key-health`: it reuses the doctor logic and contains no key material. Confirm red. Implement, then verify.
- [ ] 6.3 Write failing tests for `/settings/ai/tts-preview`: the 200-character cap, the 10-per-minute rate limit per user, a throwaway provider, and nothing stored. Confirm red. Implement, then verify. Update `openapi.json`.

## 7. Space settings API

- [ ] 7.1 Write failing route tests for `GET/PATCH/DELETE /api/spaces/current/settings/ai`:
  - space admin and org admin allowed, member gets 403
  - allowlist and lock errors return 422
  - 409 on version conflicts
  - the effective value applies on the next turn

  Confirm red. Implement, then verify.

- [ ] 7.2 Write failing tests: deleting a space removes its overrides, and space backup includes them. Confirm red, or green if already covered. Fix any gap, and update `openapi.json`.

## 8. User preferences

- [ ] 8.1 Write failing tests for the `user_ai_preferences` migration (`012`), including the cascade when a user is removed, and for `GET/PATCH /api/users/me/preferences`: validation against the effective TTS voices and speed range, and the voice list returned. Confirm red. Implement, then verify.
- [ ] 8.2 Write failing orchestrator tests covering:
  - `spoken_replies` `never`, `always` and `auto` against voice and text turns
  - voice and speed overrides, with fallback to the space voice when the model does not accept them
  - `show_reasoning = false` drops thinking events but still persists reasoning

  Confirm red. Implement via `TurnIdentity`, then verify.

- [ ] 8.3 Write a failing test: `GET /api/messages/{id}/audio` uses the caller's voice and speed. Confirm red. Implement, then verify. Update `openapi.json`.

## 9. Capabilities per space

- [ ] 9.1 Write failing tests for `/api/capabilities`: values per space across two orgs, the new flags `image_generation`, `audio_input` and `spoken_replies`, and the updated `voice-receive` scenarios. Confirm red. Implement, then verify. Update `openapi.json`.

## 10. API client

- [ ] 10.1 Run `make dump-openapi` and `make generate-flutter-client`. Verify that `app/packages/assistant_api/` compiles (`make lint-flutter`) and contains the settings, preferences and capabilities types.

## 11. Flutter shared widgets

- [ ] 11.1 Write failing widget tests for `SettingField`: source badge text and semantics label, lock toggle, reset action, and disabled locked state. Confirm red. Implement in `app/lib/features/ai_settings/widgets/`, then verify.
- [ ] 11.2 Write failing widget tests for `ModelPicker`: search, loading, error with retry, and free-text entry when the catalog fails. Also for `VoicePicker`: voice list, and a preview that plays the returned audio (mocked). Confirm red. Implement, then verify.
- [ ] 11.3 Write failing widget tests for `SecretField`: the Set, Not set and Unreadable states, an obscured input that clears after save, and the Remove confirmation. Confirm red. Implement, then verify.

## 12. Flutter org AI tab

- [ ] 12.1 Write failing unit tests for `OrgAiSettingsNotifier`: loading, success, error, validate-before-save, a 409 that leads to a reload prompt, and 422 errors mapped to fields. Confirm red. Implement, then verify.
- [ ] 12.2 Write failing widget tests for `OrgAiTab`:
  - provider-dependent sections, where switching to OpenRouter shows the OpenRouter sections
  - an invalid save shows the field error
  - key health renders
  - the tab is hidden for non-admins

  Confirm red. Implement it as a data-driven form from field specs, add the fourth Admin tab, then verify.

## 13. Flutter space AI settings screen

- [ ] 13.1 Write failing unit tests for `SpaceAiSettingsNotifier`: loading, success, error and conflict. Confirm red. Implement, then verify.
- [ ] 13.2 Write failing widget tests for `SpaceAiSettingsScreen`: inherited values, override switches, the locked-row message, reset all, and entry points from the space menu and the Admin Spaces tab. Confirm red. Implement the screen and add the router route, then verify.

## 14. Flutter Voice & AI section

- [ ] 14.1 Write failing unit tests for `UserAiPreferencesNotifier`: loading, success and error. Confirm red. Implement, then verify.
- [ ] 14.2 Write failing widget tests for `VoiceAiSection`: the spoken-replies selector, the voice picker limited to the returned voices, the speed slider range, the show-reasoning switch, and the no-TTS explanation. Confirm red. Implement it as its own widget file, wire it into `settings_screen.dart`, then verify.
- [ ] 14.3 Write a failing widget test: chat hides thinking when `show_reasoning` is false. Confirm red. Implement, then verify.

## 15. Visual baselines and docs

- [ ] 15.1 Add Playwright scenarios for the Admin AI tab, the space AI settings screen and the Settings Voice & AI section. Update the screenshot baselines intentionally (see the `e2e-testing` skill), and describe the visual diff in the PR.
- [ ] 15.2 Write `docs/settings.md`, a guide covering:
  - the layers and their order
  - locks
  - secrets
  - reload
  - preferences

  Update `docs/operations/configuration.md`, `docs/openrouter.md` and `docs/transcription.md`. Check every claim against the code.

- [ ] 15.3 Run `make lint && make format && make test && make lint-flutter && make test-flutter`, and `openspec validate ai-settings-ui --strict`. All must be clean.
