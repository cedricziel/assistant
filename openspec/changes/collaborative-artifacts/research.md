# Research: Claude artifacts

Notes on how Claude's artifacts work (as of November 2026), compared with this
change's April proposal. The goal has shifted: the Persona should **create,
maintain, and collaborate on** artifacts, so the proposal's "co-editing is a
non-goal" no longer holds.

## How Claude artifacts work

Sources: [Claude Code artifacts docs](https://code.claude.com/docs/en/artifacts),
[What are artifacts](https://support.claude.com/en/articles/17153992-what-are-artifacts-and-how-do-i-use-them),
[Publishing and sharing](https://support.claude.com/en/articles/9547008-publishing-and-sharing-artifacts),
[Artifacts in Claude Code](https://claude.com/blog/artifacts-in-claude-code).

### An artifact is an object, not a chat message

- Each artifact has a stable URL and exists apart from the conversation that made
  it. A later session continues an artifact only when given its URL. Without the
  URL, Claude creates a new one.
- Content is one self-contained page (HTML or Markdown). claude.ai adds typed
  templates: Docs, Slides, Design, Data, Sheets, Whiteboards.
- New artifacts are private until shared.

### The model publishes explicitly

- Claude decides to publish, or the user asks. Nothing in the docs suggests
  detecting artifacts from code blocks in the response.
- Every publish to the same URL creates a new version. Users can restore earlier
  versions and choose to share a specific version or always the latest.
- Open viewers refresh in place on each publish.

### Concurrency is optimistic

- If someone else has published since Claude last read the artifact, the publish
  is refused. Claude receives the newer version, merges its change onto it, and
  publishes again.
- Claude must read an artifact before it can overwrite one it did not create in
  the current session.

### Collaboration has three roles

- Roles are **view**, **comment**, and **edit**. Audiences are a single user,
  specific people, the organization, or a public link.
- Comments are threads on the artifact. Claude acts on a thread only when someone
  with edit access sends it to Claude (`@claude` or "Send to Claude"). Claude's
  replies are attributed to Claude acting for that user.
- Watching: after a publish, Claude subscribes to comments and replies without
  being prompted. It pauses after 60 activations per hour.
- In docs, a user can select text and choose "Edit with Claude" to request a
  targeted edit.

### Pages can do more than display

These capabilities matter less for us, but they show the direction:

- Persistent storage, either per viewer or shared between viewers.
- Calls to the model from inside the page, billed to the viewer.
- Connector (MCP) data, fetched with the viewer's own connections.
- File uploads.
- Limits: a 16 MiB page; claude.ai storage of 100 MB or 25k documents.

### Security

- Pages are served from a separate sandboxed origin under a strict CSP. Scripts
  load only from a CDN allowlist. Network access is limited to the page's own
  origin, and external images are blocked.
- Viewers outside the organization see an "unverified content" banner, and
  connector calls do not run for them.

## Gaps in the April proposal

| Topic        | April proposal                                            | Claude's model                          | Recommendation                                                                                                                                                                                                                             |
| ------------ | --------------------------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Scope        | Belongs to a conversation (`conversation_id NOT NULL`)    | Standalone object with a stable URL     | Make artifacts space-scoped in `space.db` and link them to the conversations and messages that touched them.                                                                                                                               |
| Creation     | Heuristics (code block over 20 lines, `file-read` output) | Explicit publish                        | Drop the heuristics. Add `artifact-*` tools to the tool executor.                                                                                                                                                                          |
| Editing      | Full content per version                                  | Republish with a stale-copy check       | Keep full content per version. Writes carry `base_version`; a stale write is rejected with the latest version so the Persona merges and retries. Add a targeted `artifact-edit` (find/replace) so small changes don't cost a full rewrite. |
| Human edits  | Not supported                                             | Editors edit directly or through Claude | Add a Flutter editor for Markdown and code. A human save is a version like any other, under the same concurrency check.                                                                                                                    |
| Comments     | None                                                      | Threads; Claude summoned by an editor   | Store threads per artifact, anchored to a version and an optional text range. An `@persona` mention starts an orchestrator turn that runs with the artifact as context.                                                                    |
| Watching     | None                                                      | Claude subscribes after publish         | Publish `artifact.*` events on the message bus. Opt-in Persona watches start turns, with a rate limit.                                                                                                                                     |
| Permissions  | None                                                      | view / comment / edit                   | Map onto space roles first. Defer per-artifact grants and public links.                                                                                                                                                                    |
| Live updates | Per-turn SSE chunks                                       | Viewers refresh on publish              | Add an artifact-scoped SSE stream next to the chat stream, so viewers outside the conversation see updates.                                                                                                                                |
| HTML         | Non-goal                                                  | Main format, behind a sandboxed origin  | Keep HTML out of the first release. Safe hosting needs a separate origin and a CSP, which is hard to provide on a self-hosted install.                                                                                                     |
| Other agents | Through A2A                                               | Through MCP and URL                     | Expose the same operations through `crates/mcp-server` and keep the A2A `Artifact` mapping.                                                                                                                                                |

## Proposed tool surface

All tools are scoped to the current space:

- `artifact-create` takes a name, a type (`markdown`, `code`, `data`), an optional
  language, and the content. It returns an id and version 1.
- `artifact-read` takes an id and an optional version. It returns the content,
  the version, and any open comment threads.
- `artifact-update` replaces the content. It requires `base_version`.
- `artifact-edit` applies find/replace edits. It requires `base_version`.
- `artifact-list` searches artifacts by name.
- `artifact-comment` replies to a comment thread or resolves it.

## Decisions taken

1. HTML and slides ship in the first release.
2. Sharing is per artifact: grants to users plus revocable share links.
3. Watching follows after comments; the first release only supports summoning.
4. Messenger interfaces post a link, never inline content.
