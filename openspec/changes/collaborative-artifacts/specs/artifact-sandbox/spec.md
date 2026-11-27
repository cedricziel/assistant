## Purpose

Safe rendering of HTML and slide artifacts on a self-hosted install, with or
without a separate content origin.

## ADDED Requirements

### Requirement: Rendered content is served from a signed, sandboxed route

`GET /artifact-content/{id}/{version}` SHALL serve rendered content only with a
valid signature from `POST /api/artifacts/{id}/render-url` (HMAC-SHA256 over
artifact id, version, viewer id and expiry; valid at most 2 minutes) or after
share-link validation. The signature SHALL name the viewer, and the route SHALL
resolve that viewer's effective role again on every request, returning `404`
once that role no longer permits `view`. Share links SHALL be revalidated per
request — token hash, org, policy and expiry — so revoking a link or disabling
public links returns `404` on the next request. Signed render URLs SHALL NOT be
issued for link views.

Every response from this route SHALL carry a `Content-Security-Policy` with a
`sandbox` directive that includes `allow-scripts` and excludes
`allow-same-origin`, `allow-popups`, `allow-popups-to-escape-sandbox`,
`allow-top-navigation*`, `allow-forms` and `allow-modals`, and with
`connect-src 'none'`, `form-action 'none'`,
`base-uri 'none'`, `object-src 'none'`, scripts and styles limited to inline
code and the configured CDN allowlist, and images limited to `data:` and
`blob:`. It SHALL also send `X-Content-Type-Options: nosniff`,
`Referrer-Policy: no-referrer` and `Cache-Control: private, no-store`.

#### Scenario: Expired signature

- **WHEN** a render URL is requested after its expiry
- **THEN** the response is `404`

#### Scenario: Grant revoked while a frame is open

- **GIVEN** a viewer loaded a frame for `a1` and their grant on `a1` is removed
- **WHEN** the frame reloads and requests its render URL again
- **THEN** both `POST /api/artifacts/a1/render-url` and the content route return
  `404`

#### Scenario: Link revoked or policy disabled

- **GIVEN** a public link `L` was issued and the policy is turned off
- **WHEN** anyone requests `L`'s content URL
- **THEN** the response is `404` without waiting for an expiry

#### Scenario: Script tries to call the API

- **GIVEN** an `html` artifact whose script calls `fetch("/api/conversations")`
- **WHEN** it is rendered
- **THEN** the browser blocks the request under `connect-src 'none'`

#### Scenario: Script tries to leave the frame

- **GIVEN** an `html` artifact whose script calls `window.open` and submits a
  form
- **WHEN** it is rendered
- **THEN** no popup opens and no navigation or form submission happens, because
  the sandbox omits `allow-popups`, `allow-top-navigation` and `allow-forms`

#### Scenario: Opened directly in a tab

- **WHEN** a user opens a render URL as a top-level page
- **THEN** the document still has an opaque origin and cannot read the app's
  cookies or storage

### Requirement: HTML responses carry the selection bridge

For `html` artifacts the route SHALL append the bridge script to the response —
inside `</body>` when present, otherwise at the end of the document — without
altering the stored version. The bridge SHALL send only `selection` messages
(quoted text with its prefix and suffix) and `open-link` messages (a clicked
URL) to the parent frame.

#### Scenario: Anchoring in an artifact without its own script

- **GIVEN** an `html` artifact of static markup
- **WHEN** a viewer selects a sentence with the comment sidebar open
- **THEN** the app receives the selection with its quote, prefix and suffix, and
  can create a thread anchored to it

#### Scenario: Served content is not a new version

- **WHEN** the route serves an `html` artifact
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

### Requirement: Operators control the content origin and CDN allowlist

`server.toml` `[artifacts]` SHALL accept an optional `content_origin` and a
`cdn_allowlist` (default cdnjs, jsDelivr `/npm/`, unpkg). When
`content_origin` is set, render URLs SHALL use it and the content route SHALL
reject requests addressed to the main origin. An empty allowlist SHALL permit
no external scripts.

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
