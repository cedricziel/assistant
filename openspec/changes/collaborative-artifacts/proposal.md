## Why

A Persona's reports, decks and small pages end up as chat text that cannot be
revised, shared or improved by a colleague. Users want the Persona to
**create, maintain, and collaborate on** durable artifacts, the way Claude's
artifacts work (see `research.md`). This change replaces the April
`canvas-artifacts` proposal, which tied artifacts to one conversation, detected
them heuristically, and ruled out HTML and co-editing.

## What Changes

- **Artifacts are space-scoped objects** with a stable id and URL, independent
  of any conversation. Types: `html`, `slides`, `markdown`, `code`, `data`.
- **Every write is a new version.** Writes carry `base_version`; a stale write
  is rejected with the latest version so the writer (Persona or human) merges
  and retries. Restoring an old version creates a new one.
- **Personas use explicit tools**: `artifact-create`, `artifact-read`,
  `artifact-update`, `artifact-edit` (find/replace), `artifact-list`,
  `artifact-comment`. No heuristic detection.
- **HTML and slides render in a sandbox.** The app fills a sandboxed frame with
  a document from an authenticated route — no signed content URL — so a page
  gets an opaque origin, no access to the app's cookies, storage or API, no
  network channel (no fetch, forms or popups), and scripts and styles only from
  a CDN allowlist. Slides are Markdown split on `---`, rendered server-side into
  a self-contained deck with keyboard navigation and a print-to-PDF layout. A
  page can still navigate its own frame to an external URL on the web, which the
  design records as an accepted residual risk.
- **Per-artifact sharing.** Roles `view`, `comment`, `edit`, `manage`. Space
  roles give a baseline; per-artifact grants add org users outside the space.
  Share links come in two kinds: `org` (any signed-in org member) and `public`
  (anyone; off unless an org admin enables it). Links pin a version or follow
  the latest, and can be revoked.
- **Comments** are threads anchored to a version and an optional text range.
  Someone with `edit` can summon the Persona on a thread; it runs a turn with
  the artifact as context and replies or edits.
- **Live updates** through an artifact SSE stream; chat shows an artifact card
  and opens a side panel. Messenger interfaces post a link only.
- Exposed to other agents through MCP and the A2A `Artifact` mapping.

## Non-goals

- Autonomous watching (the Persona reacting to every change unprompted) —
  follows once comments ship.
- Runtime capabilities inside artifacts: storage, model calls, connectors,
  uploads.
- Binary assets (images other than `data:` URIs), PPTX export, real-time
  character-level co-editing (CRDT/OT).

## Capabilities

### New Capabilities

- `artifacts`: data model, versions, optimistic concurrency, tools, REST + SSE.
- `artifact-sharing`: roles, grants, share links, org policy.
- `artifact-sandbox`: sandboxed serving of HTML and rendered slides.
- `artifact-comments`: threads, anchors, Persona summons.
- `artifact-ui`: Flutter viewer/editor, chat card and panel, messenger links.

### Modified Capabilities

None.

## Impact

- `migrations/046_artifacts.sql` (space.db), `migrations/org/010_artifact_shares.sql`.
- `crates/core`, `crates/storage`, `crates/tool-executor`, `crates/runtime`
  (orchestrator event), `crates/web-ui` (routes, render route, public
  `/a/{token}` pages, CSP), `crates/mcp-server`, `crates/interfaces` (link
  formatting).
- `openapi.json`, regenerated Dart client, new `app/lib/features/artifacts/`,
  `webview_flutter` for macOS/iOS. **Visual change:** chat gains an artifact
  card and side panel; screenshot baselines will move.
- New ADR for sandboxed artifact serving.

## Documentation

Needed: a user guide (`docs/artifacts.md`) and operator docs for the CDN
allowlist and public-link policy.
