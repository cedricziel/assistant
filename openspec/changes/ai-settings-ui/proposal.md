# AI Settings UI

## Why

Every AI setting in `openrouter-full-provider` lives in a hand-edited `org.toml`. That file has plain-text keys, is read once, and needs a restart to change. Spaces cannot differ from their org, and users cannot pick their own voice. Voice providers are also process-wide and come from the `default` org; ADR-0014 left that open. Admins and users need to manage AI behavior from the app, at the level they own.

## What Changes

- **Layered settings:** the effective config is built from these layers:
  - built-in defaults
  - `org.toml`
  - org overrides (org.db)
  - space overrides (space.db), limited to an allowlist and to keys the org has not locked
  - user preferences, applied per turn

  Every layer is checked with the same config types. The API reports the source of every value.

- **Encrypted setting secrets:** API keys are stored sealed in org.db with `SecretBox`. They are write-only through the API and take precedence over keys in `org.toml`.
- **Hot reload:** saving settings validates by building the affected providers first. It then swaps the org's context and space runtimes, without a restart and without cutting off running turns.
- **Per-org and per-space voice:** TTS, transcription and `/api/capabilities` resolve per space, instead of process-wide from the `default` org.
- **Org settings API** (org admins): read, patch and reset every AI section for all providers, with a dry-run `validate`, write-only secrets, key health, model and voice lookup from the provider catalogs, and a TTS preview.
- **Space settings API** (space admins): overrides for model, reasoning, web search, images, audio input and output, voice and transcription language, within the org's locks.
- **User preferences API:** spoken replies (`auto`, `always` or `never`), TTS voice, TTS speed and showing reasoning. Stored per user and org.
- **Flutter:**
  - an **AI** tab in Admin (org)
  - an **AI settings** screen per space
  - a **Voice & AI** section in Settings (user)

  These include searchable model and voice pickers, inheritance and lock indicators, reset to inherited, a secret entry field and a voice preview.

## Non-goals

- Editing non-AI settings (`[memory]`, `[skills]`, messenger sections, `server.toml`) in the UI.
- Writing back to `org.toml`. The file stays operator-owned.
- Moving notification preferences to the server.

## Capabilities

### New Capabilities

`settings-layering`, `settings-secrets`, `settings-hot-reload`, `org-ai-settings`, `space-ai-settings`, `user-ai-preferences`, `ai-settings-screens`.

### Modified Capabilities

- `voice-receive`: capabilities are resolved per space and gain new flags. On-demand TTS uses the user's voice and speed.

## Impact

- **Crates:** `core` (layer merge, secret resolution), `storage` (org.db and space.db migrations), `runtime` (reload, per-space voice), `web-ui` (about 20 routes, OpenAPI), `transcription`, `app/` (three screens plus regenerated client).
- **Depends on:** `openrouter-full-provider`.
- **UI:** the Admin screen gains a tab, and Settings gains a section. **Screenshot baselines will move.**
- **ADR:** a new ADR-0017, "Layered org, space and user settings".

**User-facing documentation: needed.** Add `docs/settings.md`, and update `docs/operations/configuration.md`, `docs/openrouter.md` and `docs/transcription.md`.
