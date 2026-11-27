## Purpose

Safe rendering of HTML and slide artifacts on a self-hosted install, with no
credential-bearing content URL and no second origin to operate.

## ADDED Requirements

### Requirement: Frames are filled from an authenticated render route

`GET /api/artifacts/{id}/content?format=html` SHALL return the rendered document
for `html` and `slides`, and for `markdown`, `code` and `data` when a link
viewer needs one, only to a caller whose effective role permits `view`, resolved
through `ArtifactCtx` on every request. The app SHALL insert that document into
the frame as `srcdoc` on web and `loadHtmlString` on macOS and iOS. No signed
URL, token or other credential-bearing content URL SHALL exist for app frames.

Public and org link pages SHALL serve the same document at
`/a/{token}/content`, revalidating the token, its org, the public-link policy and
the expiry on every request, and SHALL return `404` once the link is revoked or
expired or the policy is off.

#### Scenario: Grant revoked while a frame is open

- **GIVEN** a viewer has an artifact frame on screen and their grant on `a1` is
  removed
- **WHEN** the frame reloads
- **THEN** `GET /api/artifacts/a1/content?format=html` returns `404`

#### Scenario: Link revoked or policy disabled

- **GIVEN** a public link `L` was issued and the policy is turned off
- **WHEN** anyone requests `L`'s content URL
- **THEN** the response is `404` without waiting for an expiry

#### Scenario: No URL to copy out

- **WHEN** the app renders an `html` artifact
- **THEN** the frame is filled from the render response and no content URL exists
  that the page or its script could reuse

### Requirement: Frame documents are sandboxed by attribute and meta policy

The app SHALL set `sandbox="allow-scripts"` on the frame — never
`allow-same-origin`, `allow-popups`, `allow-popups-to-escape-sandbox`,
`allow-top-navigation*`, `allow-forms` or `allow-modals` — and SHALL prepend a
`Content-Security-Policy` `<meta>` element with `default-src 'none'`, scripts
and styles limited to inline code and the configured CDN allowlist, images and
media limited to `data:` and `blob:`, and `connect-src`, `form-action`,
`base-uri` and `object-src` set to `'none'`. The macOS and iOS webview SHALL
refuse every navigation through its navigation delegate. Link pages served by
the server SHALL return the same policy as response headers, with
`X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer` and
`Cache-Control: no-store`.

#### Scenario: Script tries to call the API

- **GIVEN** an `html` artifact whose script calls `fetch("/api/conversations")`
- **WHEN** it is rendered
- **THEN** the browser blocks the request under `connect-src 'none'`

#### Scenario: Script tries to leave the frame

- **GIVEN** an `html` artifact whose script calls `window.open`, submits a form
  or sets `location` to an external URL on macOS or iOS
- **WHEN** it is rendered
- **THEN** no popup opens, no form submits and the navigation delegate refuses
  the navigation

#### Scenario: Opaque origin

- **WHEN** a script inside the frame reads `document.cookie` or `localStorage`
- **THEN** it sees an opaque origin's empty values, not the app's session

### Requirement: HTML responses carry the selection bridge

For `html` artifacts the render response SHALL append the bridge script — inside
`</body>` when present, otherwise at the end of the document — without altering
the stored version. The bridge SHALL send only `selection` messages (quoted text
with its prefix and suffix) and `open-link` messages (a clicked URL) to the
parent frame.

#### Scenario: Anchoring in an artifact without its own script

- **GIVEN** an `html` artifact of static markup
- **WHEN** a viewer selects a sentence with the comment sidebar open
- **THEN** the app receives the selection with its quote, prefix and suffix, and
  can create a thread anchored to it

#### Scenario: Served content is not a new version

- **WHEN** the render route serves an `html` artifact
- **THEN** the artifact's versions and their content are unchanged by the
  injection

#### Scenario: Artifact link click

- **GIVEN** an `html` artifact containing a link to `https://example.com`
- **WHEN** the viewer clicks it
- **THEN** the app receives `open-link`, validates the scheme and opens the page
  outside the frame after the viewer confirms

#### Scenario: Message from another frame

- **GIVEN** a `postMessage` received on the artifact page
- **WHEN** `event.source` is not the artifact frame's `contentWindow`, or
  `event.origin` is not `"null"`
- **THEN** the message is ignored

### Requirement: Operators control the CDN allowlist

`server.toml` `[artifacts]` SHALL accept a `cdn_allowlist` (default cdnjs,
jsDelivr `/npm/`, unpkg). An empty allowlist SHALL permit no external scripts.

#### Scenario: Offline install

- **GIVEN** `cdn_allowlist = []`
- **WHEN** an artifact loads a script from `https://cdnjs.cloudflare.com`
- **THEN** the CSP blocks it

### Requirement: Slides render as a self-contained deck

A `slides` artifact SHALL be Markdown with optional YAML frontmatter (`theme`,
`aspect`) and slides separated by a line containing only `---`. The server
SHALL render it into one HTML page with keyboard navigation (arrows, space,
Home, End), `#n` deep links, the bridge script, and a print layout of one slide
per page.

#### Scenario: Three slides

- **GIVEN** slides content with two `---` separator lines
- **WHEN** it is rendered
- **THEN** the deck has three slides and `#3` opens the last one

#### Scenario: Print

- **WHEN** a viewer prints the deck
- **THEN** each slide is on its own page

### Requirement: Public-link pages are labelled as user-generated

Pages opened through a `public` link SHALL show the title, version and a
"User-generated content" notice outside the sandboxed frame.

#### Scenario: Public view

- **WHEN** an anonymous visitor opens a public link to an `html` artifact
- **THEN** the notice is visible above the artifact
