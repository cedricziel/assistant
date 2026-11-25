# Design

## Context

Today's behavior and constraints, as of the commit this change starts from:

- `OrgConfig` is loaded from `org.toml` by `load_org_config` (`crates/core/src/config.rs:57`). `OrgContexts::get` caches it, together with the LLM, forever. `OrgContexts::evict` has no production caller.
- `SpaceRuntimes` caches one runtime per space in a `OnceCell`. `SpaceRuntimes::evict` runs only when a space is deleted.
- TTS and transcription are built once from the `default` org (`crates/web-ui/src/lib.rs:259-307`, `:473-484`). They are shared by every org. ADR-0014 left this open.
- `SecretBox` (`crates/core/src/integrations/secret.rs`) is a seal and open primitive. Each feature owns its own sealed columns and its own AAD helper.
- org.db migrations live in `migrations/org/NNN_*.sql`, registered in `org_storage.rs`. space.db migrations live in `migrations/NNN_*.sql`, registered in `storage/src/lib.rs`.
- Authorization uses two patterns:
  - org admin: `AuthContext::is_org_admin()` plus an `org_id` match, as in the helper in `org_oidc.rs`
  - space admin: `SpaceCtx::require(Access::Admin)`
- Every route must carry `#[utoipa::path]`, be listed in `ApiDoc`, and match the committed `openapi.json`.
- Flutter has adaptive widgets in `app/lib/shared/platform/widgets.dart`. The Admin screen is a three-tab `TabController`. Notification preferences are client-only.

## Goals / Non-Goals

**Goals:**

- One merge and validation path that serves the file, the API, reloads and `doctor`.
- No unvalidated configuration ever runs.
- Running turns are never interrupted.
- Secrets are never readable over the API.

**Non-Goals:**

- A generic settings framework for non-AI sections. The layer engine is generic over `toml::Value`, but only the AI sections are exposed.
- Per-persona model overrides.

## Decisions

### D1. Overrides are JSON merge patches over a `toml::Value` of the file

- **Merge:** `org.toml` is parsed to a `toml::Value`. The org override (JSON, stored in org.db `org_ai_settings(org_id, overrides_json, locks_json, version, updated_by, updated_at)`) is converted to `toml::Value` and deep-merged, following RFC 7386: `null` deletes. The space override (space.db `space_ai_settings(overrides_json, version, updated_by, updated_at)`) is merged on top, after the allowlist and lock filter.
- **Validation:** the result is deserialized into `OrgConfig`. `serde` path errors (from the `serde_path_to_error` crate, a new workspace dependency) become key paths.
- **Source tracking:** a parallel walk of the layers records the source of each leaf.

_Alternative:_ typed override structs with `Option` on every field. Rejected because they duplicate every config type and drift with each new key.

### D2. The space allowlist and locks are data in core

`assistant_core::settings::SPACE_OVERRIDABLE` is a list of key-path patterns. Locks are stored with the org row. The filter runs on both read and write. On write, a disallowed key is a 422. On read, a stored override that is now locked is reported but not applied.

### D3. Secrets use a dedicated table and AAD

- **Storage:** org.db `setting_secrets(org_id, scope, name, ciphertext, nonce, key_id, updated_by, updated_at)`, primary key `(org_id, scope, name)`. Scope is `org` for now, so space-level keys can come later without a migration.
- **Sealing:** `SecretBox::seal` with `setting_secret_aad(org_id, scope, name)` (prefix `assistant:setting-secret:v1`).
- **Resolution:** during the merge, the opened secret is injected at its key path. Values stay `Secret` (redacted `Debug`). Before the merged `toml::Value` is used for source reporting, a redaction pass replaces every secret path with a `{"set": bool}` marker.

### D4. Building and swapping the org context and space runtimes

`OrgContexts` stores `ArcSwap<OrgContext>` per org. `OrgContext` grows to hold the effective config, the LLM, the TTS provider, the transcription provider, the image provider and the decider policy. Space runtimes hold `Arc<SpaceEffective>`, built from the org context plus the space overrides, so each space gets its own voice and model.

A save runs these steps:

1. Load the current layers and apply the patch.
2. Validate.
3. Build every affected provider with `async` builders, including catalog checks. The builds run outside any lock.
4. Commit the database row with version compare-and-swap.
5. Swap the `ArcSwap`.

A turn takes an `Arc` snapshot at its start and keeps it until the end, so running turns are unaffected. Messenger runners keep their connections: they look up the current snapshot per message instead of capturing providers at startup.

This retires the process-wide `tts_provider` and `transcription_provider` in `ApiState`, and the `default`-org lookup. Handlers resolve them from the space.

_Alternative:_ evict and rebuild lazily. Rejected because the first turn after a save would pay the build cost and could fail after the save already succeeded.

### D5. Optimistic concurrency

Each settings row has an integer `version`. GET returns it as an `ETag`, PATCH requires `If-Match`, and a mismatch returns 409 with the current state. Secrets bump the org version, so a stale form cannot silently undo a key change.

### D6. Lookups go through the providers' catalogs

The lookups reuse `OpenRouterCatalog` from the OpenRouter change: chat models, embeddings, speech models, image models and voices. Other providers return a static list of known models and accept free text. The lookups are cached in memory for 10 minutes per org and key. TTS preview builds a throwaway provider from the would-be config, and uses a per-user token bucket of 10 requests per minute.

### D7. User preferences

Preferences live in org.db `user_ai_preferences(org_id, user_id, prefs_json, updated_at)`, with a foreign-key cascade when the user is removed. At turn start, the orchestrator receives them through `TurnIdentity`, which already identifies the user. They then drive:

- the `spoken_replies` decision (combined with the voice-turn flag from the OpenRouter change)
- voice and speed overrides on TTS and audio-output requests, validated against the effective model with a fallback to the space voice
- the `show_reasoning` filter on `OrchestratorEvent::Thinking` delivery

Reasoning is still persisted in every case.

### D8. Flutter structure

The new code lives in `app/lib/features/ai_settings/`:

- `models/`: a view model per level
- `org_ai_settings_notifier.dart`, `space_ai_settings_notifier.dart` and `user_ai_preferences_notifier.dart`: `AsyncNotifier`s
- `widgets/`:
  - `SettingField` (value, source badge, lock, reset)
  - `ModelPicker` (searchable, and accepts free text)
  - `VoicePicker` (with preview)
  - `SecretField`
  - `SectionCard`
- `org_ai_tab.dart`, `space_ai_settings_screen.dart` and `voice_ai_section.dart`

The Admin screen gets a fourth tab, and its tab-specific floating action button stays null for that tab. The Settings screen gets a new section built from `voice_ai_section.dart`, so `settings_screen.dart` does not grow. Each section is described by a field spec: key path, control type, provider predicate. The form is therefore data-driven, and adding a key needs no new widget.

### D9. ADR-0017

A new ADR, "Layered org, space and user settings", records:

- the layer order
- DB overrides over the operator-owned `org.toml`
- sealed setting secrets
- snapshot swap
- per-space voice providers (closing the question ADR-0014 left open)

## Risks / Trade-offs

- [Override drift: an operator edits `org.toml` while DB overrides mask it] → The source badges show it, `GET` returns the file value as the inherited value, and `doctor` lists the keys an org override masks.
- [A build fails after an upstream outage, so saves fail] → The error names the provider and the cause. Catalog-unreachable cases already degrade per the OpenRouter specs instead of failing the build.
- [Snapshots keep old providers alive during long turns] → Providers are cheap `Arc`s and are dropped when the last turn finishes.
- [Messengers keep their connections across credential changes] → Messenger credentials are not AI settings and are out of scope. Only provider lookups are snapshotted.
- [Preview could be used as a free TTS endpoint] → It is admin-only, limited to 200 characters, and rate-limited.

## Migration Plan

The change adds three org.db migrations (`010_org_ai_settings`, `011_setting_secrets`, `012_user_ai_preferences`) and one space.db migration (`046_space_ai_settings`). Without any rows, behavior equals today's, except that voice providers become per org, which fixes non-default orgs silently using the default org's voice. Rollback means reverting the change. The extra tables are ignored by older code.

## Delivery (stacked PRs, after `openrouter-full-provider`)

1. ADR-0017, plus the layer engine in core: merge, validate, sources, allowlist and locks.
2. Setting secrets: table, AAD, resolution and redaction.
3. `OrgContext` snapshot with `ArcSwap`, per-space effective config, per-space voice providers, and `ApiState` cleanup.
4. Build-before-commit service, plus `assistant reload`.
5. Org settings API (GET, PATCH, DELETE, validate, reload, secrets).
6. Lookups, key health and TTS preview.
7. Space settings API.
8. User preferences API, plus applying preferences per turn.
9. `voice-receive` capabilities per space.
10. `make dump-openapi` and `make generate-flutter-client`.
11. Flutter shared widgets (`SettingField`, `ModelPicker`, `VoicePicker`, `SecretField`).
12. Flutter org AI tab.
13. Flutter space AI settings screen.
14. Flutter Voice & AI section.
15. Playwright baselines and docs.
