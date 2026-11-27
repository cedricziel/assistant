## Context

Facts this design builds on:

- Data-plane routes are flat (`/api/conversations/{id}`); the space comes from
  `X-Space-Id` via the `SpaceCtx` extractor (`crates/web-ui/src/space.rs:202`),
  which maps `Viewer→Read`, `Member→Write`, `SpaceAdmin/OrgAdmin→Admin`.
- There is no per-resource grant table and no `Content-Security-Policy` header
  anywhere in the server.
- Space-DB migrations are `migrations/NNN_*.sql` (latest `045`); org-DB
  migrations are `migrations/org/NNN_*.sql` (latest `009`).
- Tools get storage at construction (`ToolExecutor::new` per space in
  `crates/runtime/src/space_runtime.rs:238`) and identity at run time through
  `ExecutionContext` (`conversation_id`, `user_id`, `org_id`, `space_id`).
- The public base URL is `--public-url` / `ASSISTANT_PUBLIC_URL`
  (`crates/web-ui/src/lib.rs:60`).
- Flutter has no webview or iframe use today; markdown renders with
  `flutter_smooth_markdown`.

## Goals / Non-Goals

**Goals:** durable, versioned artifacts a Persona creates and maintains;
HTML and slides from the first release; per-artifact sharing and links;
human editing and comments with Persona summons; safe rendering on a
self-hosted install with a single hostname.

**Non-Goals:** see the proposal. Notably no in-artifact runtime APIs and no
real-time co-editing — concurrency is per version, not per keystroke.

## Decisions

### D1. Storage

`space.db`, migration `046_artifacts.sql`:

```
artifacts(
  id TEXT PK,               -- UUID
  kind TEXT NOT NULL,       -- html | slides | markdown | code | data
  title TEXT NOT NULL,
  language TEXT,            -- code only
  media_type TEXT,          -- data only (application/json, text/csv)
  owner_user_id TEXT NOT NULL,
  origin_conversation_id TEXT,  -- conversation that created it, nullable
  collab_conversation_id TEXT,  -- conversation used for summons (D7)
  latest_version INTEGER NOT NULL,
  created_at, updated_at)

artifact_versions(
  artifact_id TEXT, version INTEGER, PK(artifact_id, version),
  content TEXT NOT NULL,
  author_kind TEXT NOT NULL,   -- user | persona
  author_id TEXT NOT NULL,     -- user id or persona id
  on_behalf_of TEXT,           -- user who ran the Persona turn
  conversation_id TEXT, message_id TEXT,
  note TEXT,                   -- short change summary
  restored_from INTEGER,
  created_at)

artifact_comments(
  id TEXT PK, artifact_id TEXT, thread_id TEXT,  -- thread_id = root id
  version INTEGER NOT NULL,
  anchor_json TEXT,            -- D6; null = whole artifact
  author_kind, author_id, on_behalf_of,
  body TEXT NOT NULL,
  resolved_at, created_at)
```

Full content per version; text is cheap and restore/diff stay trivial. A
version's content is capped at **5 MiB** (413 above). `ArtifactStore` trait
with `SqliteArtifactStore` and `InMemoryArtifactStore`, contract-tested.

`org.db`, migration `010_artifact_shares.sql` — shares live at org level
because a grantee may not be a member of the artifact's space and a link token
must resolve without a space header:

```
artifact_grants(artifact_id, space_id, user_id, role, granted_by, created_at,
                PK(artifact_id, user_id))
artifact_links(id PK, token_hash UNIQUE, artifact_id, space_id,
               kind,          -- org | public
               role,          -- view | comment (public: view only)
               pinned_version INTEGER,  -- null = latest
               created_by, created_at, expires_at, revoked_at)
artifact_policy(org_id PK, allow_public_links INTEGER NOT NULL DEFAULT 0,
                updated_at)
```

Tokens are 32 random bytes, base64url; only the SHA-256 is stored.

### D2. Optimistic concurrency

`artifact-update`, `artifact-edit` and `PUT /api/artifacts/{id}/content`
require `base_version`. Inside one transaction the store checks
`latest_version == base_version`, inserts `base_version + 1`, and bumps
`latest_version`. On mismatch the API returns **409** with an
`ArtifactConflict` body carrying the latest version and content; the tool
returns `ToolOutput::error` with the same, instructing the Persona to re-apply
its change on top. Restore is `POST /api/artifacts/{id}/restore` with
`{version, base_version}` and writes a new version with `restored_from`.

### D3. Tools

Registered in `register_builtins`, constructed with the space's
`ArtifactStore`. Authorization uses the turn's user (`ExecutionContext.user_id`)
and the same role resolution as the API (D5) — a Persona can never do more
than the user it acts for.

| Tool               | Params                                                    | Notes                                                           |
| ------------------ | --------------------------------------------------------- | --------------------------------------------------------------- |
| `artifact-create`  | `kind`, `title`, `content`, `language?`, `media_type?`    | returns id, version 1, URL                                      |
| `artifact-read`    | `id`, `version?`                                          | content + open comment threads                                  |
| `artifact-update`  | `id`, `base_version`, `content`, `note?`                  | full replace                                                    |
| `artifact-edit`    | `id`, `base_version`, `edits: [{find, replace}]`, `note?` | each `find` must match exactly once, else error naming the edit |
| `artifact-list`    | `query?`, `limit?`                                        | artifacts the user can see in this space                        |
| `artifact-comment` | `thread_id`, `body`, `resolve?`                           | reply or resolve                                                |

Sharing is deliberately **not** a tool: granting access and minting links are
human decisions. The system prompt gains a short section on when to use an
artifact (durable, shareable output) and that HTML must be self-contained.

### D4. Sandboxed serving (`artifact-sandbox`)

Rendered content is served at `GET /artifact-content/{id}/{version}` outside
`/api`. An iframe cannot send an `Authorization` header, so access is granted by
a short-lived signature: `POST /api/artifacts/{id}/render-url` (authenticated)
returns a URL carrying `sub` (the viewer's user id), `exp` (2 minutes) and an
HMAC-SHA256 `sig` over `{id, version, sub, exp}` keyed from `jwt_secret`. The
content route resolves `sub`'s effective role (D5) again on every request, so a
removed grant or a lost space role returns `404` on the next load instead of
waiting for `exp`. Signed URLs are never issued for link views: share links call
the same route with their token, and the route revalidates the token hash, the
policy and the expiry per request exactly as `/a/{token}` does. Revoking a link
or turning the policy off therefore takes effect immediately.

Every response from this route sets:

```
Content-Security-Policy: sandbox allow-scripts;
  default-src 'none';
  script-src 'unsafe-inline' <cdn allowlist>;
  style-src 'unsafe-inline' https://fonts.googleapis.com <cdn allowlist>;
  font-src https://fonts.gstatic.com data:;
  img-src data: blob:; media-src data: blob:;
  connect-src 'none'; form-action 'none'; base-uri 'none'; object-src 'none';
  frame-ancestors <public url> <content origin>
X-Content-Type-Options: nosniff
Referrer-Policy: no-referrer
Cache-Control: private, no-store
```

The CSP `sandbox` directive (without `allow-same-origin`) gives the document an
**opaque origin even when opened directly**, so its scripts cannot read the
app's cookies or storage, and the browser does not attach credentials to
requests it makes. That makes a single-hostname install safe without a second
domain. `connect-src 'none'` stops fetch and XHR; `form-action 'none'` stops
form posts; `base-uri 'none'` stops a `<base>` tag from redirecting relative
URLs; `object-src 'none'` blocks embeds.

No `allow-popups`, `allow-top-navigation`, `allow-forms` or `allow-modals`: a
page cannot open windows, navigate the app, submit forms or raise `alert` /
`confirm` dialogs, all of which are usable for phishing or for sending artifact
text to an attacker URL. Links inside an artifact are inert in the frame; the
injected bridge (D6) forwards a click to the app as `open-link`, and the app
opens validated `http(s)` URLs outside the frame after a confirmation.

`server.toml` gains:

```toml
[artifacts]
content_origin = "https://artifacts.example.com"  # optional, defence in depth
cdn_allowlist = ["https://cdnjs.cloudflare.com", "https://cdn.jsdelivr.net/npm/", "https://unpkg.com"]
```

When `content_origin` is set, render URLs use it and the content route rejects
requests whose `Host` is the main origin. An offline install sets
`cdn_allowlist = []`.

What is rendered per kind:

- `html` — served as UTF-8 with the bridge script (D6) injected at serve time;
  the stored content is never rewritten.
- `slides` — Markdown, optional YAML frontmatter (`theme: light|dark`,
  `aspect: 16:9|4:3`), slides separated by a line containing only `---`.
  Rendered with `pulldown-cmark` into an embedded deck template (vanilla JS,
  arrow/space/Home/End navigation, `#n` deep links, the same bridge script,
  `@media print` one slide per page for PDF). Fenced `mermaid` blocks are left
  as `<pre class=mermaid>` and rendered only if the CDN allowlist permits the
  mermaid script.
- `markdown`, `code`, `data` — rendered natively by Flutter; the content route
  serves them as HTML (markdown via `pulldown-cmark`, code/data in `<pre>`) only
  for share links, so link viewers without the app see something.

Public-link pages are wrapped in a thin server-rendered frame that shows the
title, version and a "User-generated content" notice above the sandboxed
iframe.

### D5. Roles and resolution (`artifact-sharing`)

Ordered roles: `view < comment < edit < manage`. Effective role for a user =
max of:

- space role: `Viewer→view`, `Member→edit`, `SpaceAdmin`/`OrgAdmin→manage`
- owner (`owner_user_id`) → `manage`
- an `artifact_grants` row
- an `org` link presented in this request → its role (signed-in org members only)

`manage` may change grants, create/revoke links, delete, and transfer ownership.
Grants are only to users of the same org. `public` links require
`artifact_policy.allow_public_links`; disabling the policy makes existing public
links return 404 without deleting them, so re-enabling restores them.

Routes that address one artifact use a new `ArtifactCtx` extractor. It resolves
the artifact's space and the caller's role in this order, taking the first that
applies:

1. `X-Space-Id`, when the caller is a member of that space **or an org admin of
   the org** — the same fallback `SpaceCtx` applies today.
2. A row in `artifact_grants` for the caller.
3. An `org` link token the caller presents (header `X-Artifact-Link` or
   `?link=`), validated for this artifact and the caller's org, which also
   supplies the link's role for this request only.

Otherwise the response is `404`, never `403`, so an invisible artifact is not
confirmed to exist. A link token has authority only when presented; nothing
grants it implicitly. `GET /api/artifacts/shared-with-me` lists grants across
spaces.

### D6. Comment anchors

`anchor_json = {"quote": "...", "prefix": "...", "suffix": "..."}` (text-quote
selector, up to 32 chars of context each side) plus the `version` it was made
on. Clients re-anchor on newer versions by searching for prefix+quote+suffix,
then quote alone; if not found the thread shows as "outdated" but stays open.

For `html` and `slides` the quote is taken from the rendered text through a
bridge script that the **server** adds to the response:

- `slides` renders from the deck template, which contains the bridge.
- `html` is served with a `<script>` element appended before `</body>` (or at
  the end of the document when there is none) plus an anchor click listener on
  the document. The stored version is not modified; the injection happens only
  on this route, and frames that do not receive it — an artifact fetched
  directly from the API — cannot be commented on.

The bridge sends two message kinds to the parent, and nothing else:

- `{type: "selection", text, quote, prefix, suffix}` when the user selects text
  while the comment sidebar is open.
- `{type: "open-link", href}` when the user clicks a link inside the frame.

The parent accepts a message only when `event.source` is the frame's
`contentWindow` and `event.origin` is `"null"` (the opaque origin), and opens an
`open-link` target only for `http`/`https` after the user confirms. Because the
parent cannot read the frame's selection (opaque origin), the quote in the first
message is the only source for anchors on these kinds.

### D7. Summoning the Persona

A user with `edit` posts a comment containing `@<persona-slug>` or presses
"Ask <Persona>". The server publishes `artifact.summon` on the space's scoped
bus with `{artifact_id, thread_id, user_id}`. The space runtime starts a turn
in the artifact's `collab_conversation_id` (created on first summon, owned by
the artifact rather than by any user, hidden from the conversation list) **as
the summoning user**. The turn's first message contains the thread, the anchor,
and the latest version number; the Persona reads, edits with `artifact-edit`,
and replies with `artifact-comment`. Replies record `author_kind = persona`,
`on_behalf_of = user`. Summons are rate-limited to 30 per artifact per hour;
the 31st returns 429.

Because the conversation holds the latest version and every summon thread, a
direct read of it (`GET /api/conversations/{id}` and its messages) requires the
space's `Access::Read` **and** current `view` on the artifact, resolved through
`ArtifactCtx`. Revoking a user's artifact access therefore removes the
transcript at the same time as the artifact, and the first summoner is recorded
as the turn's user, never as the conversation's owner.

### D8. Live updates and chat integration

- `GET /api/artifacts/{id}/events` (SSE via `sse_response()`): `snapshot`,
  `version_created`, `comment_created`, `comment_updated`, `deleted`. Backed by
  an `ArtifactBroadcast` like `ConversationBroadcast`.
- The orchestrator emits `OrchestratorEvent::ArtifactChanged {artifact_id,
version, kind, title}` only when a tool writes a new version —
  `artifact-create`, `artifact-update`, `artifact-edit` or a restore.
  `artifact-read`, `artifact-list` and `artifact-comment` emit nothing, since
  nothing changed. The event is persisted to `conversation_events` and sent on
  the chat SSE stream as `event: artifact`.
- Messenger interfaces append one line per changed artifact to the turn's
  reply: `<title> (v<n>): <public_url>/artifacts/<id>`. The link opens the SPA,
  which requires sign-in and applies D5. If no public URL is configured, the
  line names the artifact without a link.

### D9. Flutter

`app/lib/features/artifacts/` with MVVM per project conventions:

- `ArtifactViewer` — native markdown/code/data; for `html`/`slides` an
  `ArtifactFrame` that uses `HtmlElementView` + `<iframe sandbox="allow-scripts">`
  on web and `webview_flutter` on macOS/iOS, loading the signed render URL.
- `ArtifactEditor` — source editor (monospace, line numbers) with save →
  `PUT .../content`; on 409 it shows a diff of the user's text vs latest and
  offers "Reapply on latest" or "Discard mine".
- Version picker with diff, comment sidebar, share dialog (grants + links).
- Chat: `ArtifactCard` in the message list; tapping opens the panel (side by
  side ≥900 px, full-screen route below). Routes `/artifacts`, `/artifacts/:id`.

### D10. MCP and A2A

The MCP server gets the tools automatically via `executor.to_specs()`, and adds
`artifacts://{id}` resources (latest content) to `resources/list|read`. A2A
tasks include an `Artifact` per changed artifact: `parts[0]` text with the
content and `media_type`, `metadata = {kind, version, url}`.

### REST surface

| Method               | Path                                            | Role                         |
| -------------------- | ----------------------------------------------- | ---------------------------- |
| GET                  | `/api/artifacts`                                | view (space)                 |
| POST                 | `/api/artifacts`                                | space Member                 |
| GET                  | `/api/artifacts/shared-with-me`                 | —                            |
| GET / PATCH / DELETE | `/api/artifacts/{id}`                           | view / edit (title) / manage |
| PUT                  | `/api/artifacts/{id}/content`                   | edit                         |
| GET                  | `/api/artifacts/{id}/versions`, `/versions/{n}` | view                         |
| POST                 | `/api/artifacts/{id}/restore`                   | edit                         |
| POST                 | `/api/artifacts/{id}/render-url`                | view                         |
| GET                  | `/api/artifacts/{id}/events`                    | view                         |
| GET / POST           | `/api/artifacts/{id}/comments`                  | view / comment               |
| PATCH                | `/api/artifacts/{id}/comments/{cid}`            | author or edit (resolve)     |
| POST                 | `/api/artifacts/{id}/summon`                    | edit                         |
| GET / PUT / DELETE   | `/api/artifacts/{id}/grants[/{user_id}]`        | manage                       |
| GET / POST / DELETE  | `/api/artifacts/{id}/links[/{link_id}]`         | manage                       |
| GET / PUT            | `/api/artifacts/policy`                         | org admin                    |
| GET                  | `/a/{token}` (public)                           | link                         |
| GET                  | `/artifact-content/{id}/{version}` (signed)     | signature                    |

Shapes follow `openspec/config.yaml`: `ArtifactSummary`, `ArtifactDetail`,
`ArtifactVersionSummary`, `ArtifactVersionDetail`, `CreateArtifactRequest`,
`UpdateArtifactContentRequest`, `ArtifactConflict`, etc.

## Risks / Trade-offs

- **Malicious HTML from prompt injection.** A Persona fed hostile input can
  write hostile HTML. Mitigated by the opaque-origin sandbox, `connect-src
'none'`, no credentials, and the public-link notice. Residual risk: phishing
  UI inside a shared page — public links are off by default.
- **An artifact script can navigate its own frame away.** No CSP stops a page
  from setting `location`, so a script can send text it already renders to an
  attacker URL, and can carry its own render URL with it. Popups, top
  navigation, forms and dialogs are blocked; a render URL grants one immutable
  version, expires in 2 minutes and is rechecked against the viewer's role at
  redemption, so a copied URL is not a durable credential. Content the page
  already holds cannot be hidden from its own script — the boundary is that the
  page gets nothing else.
- **Revocation.** Role and link changes are rechecked per request, so there is
  no post-revocation window for a viewer who reloads or refetches. A page
  already rendered on screen keeps showing what it rendered; closing the frame
  is the user's action.
- **CDN dependency.** Allowlisted CDNs see viewer IPs. Operators can empty the
  allowlist.
- **Opaque origin breaks `localStorage` in artifacts.** Acceptable; in-artifact
  storage is a non-goal and would come through a `postMessage` bridge later.
- **Full-content versions grow `space.db`.** 5 MiB cap; version pruning can be
  added later.
- **`webview_flutter` on macOS** is newer than on iOS; task 6.1 verifies it
  before the UI work depends on it.

## Migration Plan

New tables only. No migration of the old `canvas-artifacts` design (never
implemented).

## Open Questions

None blocking. Autonomous watching is deferred to a follow-up change.
