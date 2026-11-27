## Purpose

Safe rendering of HTML and slide artifacts on a self-hosted install, with or
without a separate content origin.

## ADDED Requirements

### Requirement: Rendered content is served from a signed, sandboxed route

`GET /artifact-content/{id}/{version}` SHALL serve rendered content only with a
valid signature from `POST /api/artifacts/{id}/render-url` (HMAC-SHA256 over
id, version and expiry; valid at most 5 minutes) or after share-link
validation. Every response from this route SHALL carry a
`Content-Security-Policy` with a `sandbox` directive that does not include
`allow-same-origin`, `connect-src 'none'`, `form-action 'none'`, scripts and
styles limited to inline code and the configured CDN allowlist, and images
limited to `data:` and `blob:`. It SHALL also send
`X-Content-Type-Options: nosniff` and `Referrer-Policy: no-referrer`.

#### Scenario: Expired signature

- **WHEN** a render URL is requested after its expiry
- **THEN** the response is `404`

#### Scenario: Script tries to call the API

- **GIVEN** an `html` artifact whose script calls `fetch("/api/conversations")`
- **WHEN** it is rendered
- **THEN** the browser blocks the request under `connect-src 'none'`

#### Scenario: Opened directly in a tab

- **WHEN** a user opens a render URL as a top-level page
- **THEN** the document still has an opaque origin and cannot read the app's
  cookies or storage

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
Home, End), `#n` deep links, and a print layout of one slide per page.

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
