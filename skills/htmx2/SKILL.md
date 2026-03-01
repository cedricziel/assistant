---
name: htmx2
description: >
  Comprehensive htmx 2.0 patterns for the assistant web UI. Covers the full
  attribute reference, swap modifiers, trigger syntax, SSE streaming, request
  and response headers, View Transitions API, hx-on event handling, security
  defaults, vendoring setup, and migration from htmx 1.x. Use when building
  or modifying any htmx-powered interaction in the web UI.
license: MIT
metadata:
  tier: info
  mutating: "false"
  confirmation-required: "false"
---

# htmx 2.0 Patterns

The assistant web UI uses htmx 2.0 for dynamic interactions without
client-side JavaScript frameworks. The server returns HTML fragments; htmx
swaps them into the DOM. SSE (Server-Sent Events) powers real-time streaming
for the chat interface.

## Vendored Dependencies

htmx and the SSE extension are vendored locally (not loaded from a CDN).
The vendoring system uses integrity-checked downloads with a lockfile.

### Lockfile (`crates/web-ui/vendor.lock`)

```json
{
  "packages": [
    {
      "name": "htmx.org",
      "version": "2.0.4",
      "file": "htmx.min.js",
      "url": "https://unpkg.com/htmx.org@2.0.4/dist/htmx.min.js",
      "sha256": "e209dda5c8235479f3166defc7750e1dbcd5a5c1808b7792fc2e6733768fb447"
    },
    {
      "name": "htmx-ext-sse",
      "version": "2.2.2",
      "file": "sse.js",
      "url": "https://unpkg.com/htmx-ext-sse@2.2.2/sse.js",
      "sha256": "83eca6fa0611fe2b0bf1700b424b88b5eced38ef448ef9760a2ea08fbc875611"
    }
  ]
}
```

### Downloading

```sh
make vendor        # runs vendor.sh, verifies SHA-256
```

Vendored files live in `crates/web-ui/src/static_assets/vendor/` and are
gitignored. They are embedded at compile time via `include_str!()` and served
at fingerprinted URLs with immutable cache headers.

### Updating a Dependency

1. Edit `vendor.lock`: bump version, URL, and SHA-256
2. Run `make vendor` to download and verify
3. Update the version in `static_assets/mod.rs` if the embed path changed
4. Run `make check && make test` to verify

## Module System

htmx 2.0 provides module-specific builds:

| Build          | File          | Use Case                       |
| -------------- | ------------- | ------------------------------ |
| Browser (IIFE) | `htmx.min.js` | Our setup (vendored, embedded) |
| ESM            | `htmx.esm.js` | Bundler / import maps          |
| AMD            | `htmx.amd.js` | RequireJS                      |
| CJS            | `htmx.cjs.js` | Node.js                        |

The SSE extension (`htmx-ext-sse`) has **no ESM build** -- only IIFE. This
is why we use `<script>` tags rather than pure import maps for loading.

## Core Concepts

### 1. Server Returns HTML, Not JSON

Every htmx endpoint returns an HTML fragment -- never JSON. The fragment is a
rendered Askama template (a "partial" that does **not** extend `base.html`).

```rust
// Handler returns a rendered HTML fragment
pub async fn conversation_list(...) -> Response {
    render_template(ConversationListTemplate { conversations })
}
```

```html
<!-- Template: chat/conversation_list.html (no extends) -->
{% for conv in conversations %}
<div
  class="conv-item"
  hx-get="/chat/{{ conv.id }}"
  hx-target="#chat-panel"
  hx-swap="innerHTML"
>
  {{ conv.title }}
</div>
{% endfor %}
```

### 2. Full Pages vs Partials

| Type      | Extends `base.html`? | Has `StaticUrls`? | Used by                 |
| --------- | -------------------- | ----------------- | ----------------------- |
| Full page | Yes                  | Yes               | Direct navigation (GET) |
| Partial   | No                   | No                | htmx requests (XHR)     |

htmx adds the `HX-Request: true` header to all requests. The service worker
skips these requests (they must not be cached as full pages).

## Attribute Reference

### Core Attributes

| Attribute     | Description                                       |
| ------------- | ------------------------------------------------- |
| `hx-get`      | Issues a GET to the specified URL                 |
| `hx-post`     | Issues a POST to the specified URL                |
| `hx-put`      | Issues a PUT to the specified URL                 |
| `hx-patch`    | Issues a PATCH to the specified URL               |
| `hx-delete`   | Issues a DELETE to the specified URL              |
| `hx-trigger`  | Specifies the event that triggers the request     |
| `hx-target`   | CSS selector for the element to swap content into |
| `hx-swap`     | Controls how content is swapped (see below)       |
| `hx-select`   | Select content to swap from the response          |
| `hx-vals`     | Add JSON values to submit with the request        |
| `hx-push-url` | Push a URL into the browser history               |
| `hx-on:*`     | Handle events inline (see hx-on section below)    |

### Additional Attributes

| Attribute         | Description                                        |
| ----------------- | -------------------------------------------------- |
| `hx-boost`        | Progressive enhancement for links and forms        |
| `hx-confirm`      | Show `confirm()` dialog before issuing request     |
| `hx-disable`      | Disable htmx processing for this element           |
| `hx-disabled-elt` | Add `disabled` attribute during in-flight requests |
| `hx-disinherit`   | Control attribute inheritance for children         |
| `hx-encoding`     | Change request encoding type                       |
| `hx-ext`          | Extensions to use for this element                 |
| `hx-headers`      | Add headers to the request                         |
| `hx-include`      | Include additional data in requests                |
| `hx-indicator`    | Element to put `htmx-request` class on             |
| `hx-params`       | Filter parameters submitted with the request       |
| `hx-preserve`     | Keep elements unchanged between requests           |
| `hx-prompt`       | Show `prompt()` before submitting                  |
| `hx-replace-url`  | Replace URL in the browser location bar            |
| `hx-request`      | Configure aspects of the request                   |
| `hx-swap-oob`     | Mark element for out-of-band swap                  |
| `hx-select-oob`   | Select out-of-band content from response           |
| `hx-sync`         | Control request synchronization between elements   |
| `hx-validate`     | Force validation before request                    |

## Swap Strategies (`hx-swap`)

| Value         | Description                                      |
| ------------- | ------------------------------------------------ |
| `innerHTML`   | Replace inner HTML of target (default)           |
| `outerHTML`   | Replace entire target element with response      |
| `textContent` | Replace text content without parsing as HTML     |
| `beforebegin` | Insert response before the target element        |
| `afterbegin`  | Insert response before the first child of target |
| `beforeend`   | Insert response after the last child of target   |
| `afterend`    | Insert response after the target element         |
| `delete`      | Delete the target element regardless of response |
| `none`        | No content swap (OOB items still processed)      |

### Swap Modifiers

Modifiers are appended to the swap value, space-separated:

```html
hx-swap="innerHTML swap:500ms settle:200ms" hx-swap="beforeend
scroll:#messages:bottom" hx-swap="outerHTML transition:true" hx-swap="innerHTML
show:#target:top" hx-swap="outerHTML focus-scroll:true" hx-swap="innerHTML
ignoreTitle:true"
```

| Modifier                          | Description                                         |
| --------------------------------- | --------------------------------------------------- |
| `swap:<time>`                     | Delay before swapping (e.g. `swap:500ms`)           |
| `settle:<time>`                   | Delay between swap and settle (e.g. `settle:200ms`) |
| `scroll:<selector>:<top\|bottom>` | Scroll element after swap                           |
| `show:<selector>:<top\|bottom>`   | Scroll element into viewport                        |
| `transition:true`                 | Use View Transitions API for the swap               |
| `focus-scroll:true`               | Scroll to focused input after swap                  |
| `ignoreTitle:true`                | Don't update page title from response               |

**Examples used in the assistant web UI:**

```html
<!-- Chat: append message and scroll to bottom -->
hx-swap="beforeend scroll:#messages:bottom"

<!-- SSE streaming: replace entire streaming container with final message -->
hx-swap="outerHTML"

<!-- Search: replace list contents -->
hx-swap="innerHTML"
```

## Trigger Syntax (`hx-trigger`)

### Basic Events

```html
hx-trigger="click" hx-trigger="change" hx-trigger="input" hx-trigger="submit"
hx-trigger="load"
<!-- fires on element load -->
hx-trigger="revealed"
<!-- fires when scrolled into viewport -->
hx-trigger="intersect"
<!-- fires on intersection observer -->
```

### Event Modifiers

| Modifier                         | Description                              |
| -------------------------------- | ---------------------------------------- |
| `once`                           | Trigger only once                        |
| `changed`                        | Only fire if value changed               |
| `delay:<time>`                   | Debounce: reset delay on each event      |
| `throttle:<time>`                | Throttle: ignore events during cooldown  |
| `from:<selector>`                | Listen for event on another element      |
| `target:<selector>`              | Filter by event target                   |
| `consume`                        | Stop event from propagating to parents   |
| `queue:<first\|last\|all\|none>` | Queue behavior during in-flight requests |

### Event Filters

Enclose a JavaScript expression in square brackets (requires `allowEval`):

```html
hx-trigger="click[ctrlKey]" hx-trigger="keydown[key==='Enter']"
```

### Multiple Triggers

Comma-separated, each with its own modifiers:

```html
hx-trigger="load, click delay:1s" hx-trigger="input changed delay:300ms,
keydown[key==='Enter']"
```

### SSE Triggers

With the SSE extension, use `sse:<event-name>` as a trigger:

```html
<div hx-ext="sse" sse-connect="/events">
  <div hx-get="/updates" hx-trigger="sse:refresh">...</div>
</div>
```

### Polling

```html
hx-trigger="every 2s" hx-trigger="every 1s [isActive()]"
<!-- conditional polling -->
```

### Patterns Used in the Assistant Web UI

```html
<!-- Debounced search (300ms) -->
<input
  hx-get="/chat/conversations"
  hx-trigger="input changed delay:300ms"
  hx-target="#conversation-list"
  hx-swap="innerHTML"
  name="q"
/>

<!-- Filter dropdown triggers on change -->
<select
  hx-get="/traces"
  hx-trigger="change"
  hx-target="#trace-list"
  name="status"
>
  <!-- Form submit (default trigger for forms) -->
  <form
    hx-post="/chat/{{ id }}/send"
    hx-target="#messages"
    hx-swap="beforeend scroll:#messages:bottom"
  ></form>
</select>
```

## Event Handling (`hx-on:`)

htmx 2.0 uses `hx-on:` attribute syntax (colon-separated) instead of the
deprecated `hx-on` (multi-line) syntax from 1.x.

### Syntax

```html
<!-- Standard DOM events -->
<div hx-on:click="doSomething()">
  <!-- htmx events (must use kebab-case because HTML lowercases attributes) -->
  <div hx-on:htmx:after-swap="handleSwap()">
    <!-- Shorthand: double-colon omits "htmx" prefix -->
    <div hx-on::after-swap="handleSwap()">
      <div hx-on::before-request="showSpinner()">
        <!-- Dash syntax (for JSX compatibility) -->
        <div hx-on--after-swap="handleSwap()"></div>
      </div>
    </div>
  </div>
</div>
```

### Important: Kebab-Case Required

HTML attributes are case-insensitive, so `hx-on:htmx:beforeRequest` will
**not work**. Use `hx-on:htmx:before-request` (kebab-case) instead.

### Our Policy: No Inline JS

In the assistant web UI, we avoid `hx-on:` with inline JavaScript. Instead,
use external JS files that attach event listeners. See the `ux-principles`
skill for the complete no-inline-JS policy and replacement patterns.

## HTTP Headers

### Request Headers (sent by htmx)

| Header                       | Description                       |
| ---------------------------- | --------------------------------- |
| `HX-Request`                 | Always `"true"` for htmx requests |
| `HX-Trigger`                 | `id` of the triggered element     |
| `HX-Trigger-Name`            | `name` of the triggered element   |
| `HX-Target`                  | `id` of the target element        |
| `HX-Current-URL`             | Current browser URL               |
| `HX-Boosted`                 | `"true"` if via `hx-boost`        |
| `HX-Prompt`                  | User response to `hx-prompt`      |
| `HX-History-Restore-Request` | `"true"` for history restoration  |

### Response Headers (sent by server)

| Header                    | Description                              |
| ------------------------- | ---------------------------------------- |
| `HX-Location`             | Client-side redirect without full reload |
| `HX-Push-Url`             | Push URL into history stack              |
| `HX-Redirect`             | Full client-side redirect                |
| `HX-Refresh`              | `"true"` to trigger full page refresh    |
| `HX-Replace-Url`          | Replace current URL in location bar      |
| `HX-Reswap`               | Override swap strategy                   |
| `HX-Retarget`             | CSS selector to override target element  |
| `HX-Reselect`             | CSS selector to select part of response  |
| `HX-Trigger`              | Trigger client-side events               |
| `HX-Trigger-After-Settle` | Trigger events after settle step         |
| `HX-Trigger-After-Swap`   | Trigger events after swap step           |

### Using Response Headers in Rust (Axum)

```rust
use axum::http::HeaderValue;

// Redirect after form submission
let mut response = render_template(SuccessTemplate {});
response.headers_mut().insert(
    "HX-Redirect",
    HeaderValue::from_str("/webhooks")?,
);
```

## CSS Classes (added by htmx)

| Class            | Description                                        |
| ---------------- | -------------------------------------------------- |
| `htmx-request`   | Applied during in-flight request                   |
| `htmx-added`     | Applied to new content before swap, removed after  |
| `htmx-swapping`  | Applied to target before swap                      |
| `htmx-settling`  | Applied to target after swap, removed after settle |
| `htmx-indicator` | Toggles visibility when `htmx-request` is present  |

**Loading indicator pattern:**

```html
<button hx-get="/data" hx-indicator="#spinner">Load</button>
<span id="spinner" class="htmx-indicator">Loading...</span>
```

## View Transitions API

htmx 2.0 supports the View Transitions API for animated swaps:

```html
<!-- Per-element -->
<div hx-get="/page2" hx-swap="innerHTML transition:true">
  <!-- Global (via meta tag) -->
  <meta name="htmx-config" content='{"globalViewTransitions":true}' />
</div>
```

The assistant web UI does not currently use View Transitions but the
infrastructure supports it.

## SSE Streaming (Chat)

The chat uses htmx's SSE extension for token-by-token streaming from the LLM.

### Architecture

```
Browser                          Server
  |                                |
  |--- POST /chat/{id}/send ----->|  (htmx form submit)
  |<-- HTML: streaming.html ------|  (contains SSE connection)
  |                                |
  |--- GET /chat/{id}/stream ---->|  (SSE connection opened by htmx)
  |<-- event: token\ndata: "Hi"---|  (token-by-token)
  |<-- event: token\ndata: " th"--|
  |<-- event: done\ndata: <html>--|  (final rendered message)
  |                                |
```

### SSE Extension Attributes

| Attribute     | Description                                      |
| ------------- | ------------------------------------------------ |
| `sse-connect` | URL of the SSE server (EventSource)              |
| `sse-swap`    | Event name to swap into the DOM                  |
| `sse-close`   | Event name that closes the connection gracefully |

### streaming.html (SSE Partial)

```html
<div
  class="message assistant"
  hx-ext="sse"
  sse-connect="/chat/{{ id }}/stream?turn={{ turn }}"
  sse-swap="done"
  hx-swap="outerHTML"
>
  <div class="msg-role">assistant</div>
  <div class="msg-meta">thinking...</div>
  <div class="msg-content" sse-swap="token" hx-swap="beforeend"></div>
</div>
```

**How it works:**

1. `hx-ext="sse"` activates the SSE extension on this element
2. `sse-connect` opens an EventSource to the streaming endpoint
3. `sse-swap="token"` on the inner div: each `event: token` appends its
   `data` to `.msg-content` via `hx-swap="beforeend"`
4. `sse-swap="done"` on the outer div: when `event: done` arrives, its
   `data` (fully rendered message HTML) replaces the entire streaming
   element via `hx-swap="outerHTML"`

### SSE Extension Events

| Event                   | Description                                       |
| ----------------------- | ------------------------------------------------- |
| `htmx:sseOpen`          | SSE connection established                        |
| `htmx:sseError`         | SSE connection failed                             |
| `htmx:sseBeforeMessage` | Before event data is swapped (cancelable)         |
| `htmx:sseMessage`       | After event data has been swapped                 |
| `htmx:sseClose`         | Connection closed (node missing/replaced/message) |

### Server-Side Streaming (Axum)

```rust
pub async fn stream_chat(...) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        while let Some(token) = rx.recv().await {
            yield Ok(Event::default()
                .event("token")
                .data(html_escape(&token)));
        }
        let html = render_template(MessageTemplate { ... });
        yield Ok(Event::default()
            .event("done")
            .data(html));
    };
    Sse::new(stream)
}
```

### SSE Rules

- Only one SSE connection per element tree (don't nest `hx-ext="sse"`)
- The SSE extension has automatic reconnection with exponential backoff
- Use `sse-close` to gracefully terminate a stream
- Multiple events can be listened for: `sse-swap="event1,event2"`
- Child elements can use `hx-trigger="sse:<event>"` to trigger requests

## Security Defaults (htmx 2.0)

htmx 2.0 changed several defaults for security:

| Config                    | htmx 1.x Default | htmx 2.0 Default   | Notes                        |
| ------------------------- | ---------------- | ------------------ | ---------------------------- |
| `selfRequestsOnly`        | `false`          | `true`             | Only same-origin requests    |
| `allowEval`               | `true`           | `true`             | Needed for trigger filters   |
| `allowScriptTags`         | `true`           | `true`             | Process script tags in swaps |
| `scrollBehavior`          | `'smooth'`       | `'instant'`        | Scroll jump behavior         |
| `methodsThatUseUrlParams` | `["get"]`        | `["get","delete"]` | DELETE uses URL params       |

**`selfRequestsOnly: true`** means htmx will not make cross-domain requests
by default. This is correct for the assistant web UI since all endpoints are
same-origin.

### CSP Considerations

If you add a Content-Security-Policy header:

- `script-src` must allow htmx and extensions (our vendored scripts)
- If using `hx-on:` with inline JS, you need `'unsafe-inline'` or a nonce
  (we avoid this by not using inline JS)
- If using trigger filters (`[ctrlKey]`), `allowEval` must be `true` and
  CSP must allow `'unsafe-eval'` -- avoid trigger filters if possible

## Configuration

Configure htmx via a `<meta>` tag or JavaScript:

```html
<!-- Via meta tag -->
<meta name="htmx-config" content='{"defaultSwapStyle":"innerHTML"}' />

<!-- Via JavaScript -->
<script>
  htmx.config.defaultSwapStyle = "innerHTML";
</script>
```

Key configuration options:

| Option                   | Default     | Description                        |
| ------------------------ | ----------- | ---------------------------------- |
| `defaultSwapStyle`       | `innerHTML` | Default swap strategy              |
| `defaultSwapDelay`       | `0`         | Delay before swap (ms)             |
| `defaultSettleDelay`     | `20`        | Delay between swap and settle (ms) |
| `selfRequestsOnly`       | `true`      | Only allow same-origin requests    |
| `scrollBehavior`         | `instant`   | `instant`, `smooth`, or `auto`     |
| `globalViewTransitions`  | `false`     | Enable View Transitions globally   |
| `allowEval`              | `true`      | Allow eval (needed for filters)    |
| `allowScriptTags`        | `true`      | Process scripts in swapped content |
| `historyCacheSize`       | `10`        | Number of pages in history cache   |
| `defaultFocusScroll`     | `false`     | Scroll to focused input after swap |
| `includeIndicatorStyles` | `true`      | Load built-in indicator CSS        |

## Template Organization

### Full-Page Templates (extend `base.html`)

Served on direct navigation. Include the full app shell:

```
templates/chat/page.html          --> extends "base.html"
templates/traces/page.html        --> extends "base.html"
templates/agents/list.html        --> extends "base.html"
templates/logs/page.html          --> extends "base.html"
templates/analytics/page.html     --> extends "base.html"
templates/webhooks/list.html      --> extends "base.html"
```

### Partial Templates (standalone fragments)

Returned by htmx XHR endpoints. Only the HTML fragment to swap:

```
templates/chat/panel.html         --> conversation panel content
templates/chat/message.html       --> single message bubble
templates/chat/conversation_list.html --> sidebar conversation list
templates/chat/streaming.html     --> SSE streaming container
```

**Rules:**

- Partials never extend `base.html`
- Partials never include `<html>`, `<head>`, or `<body>` tags
- Partials don't need `StaticUrls` (CSS/JS already loaded by the page)
- Partials can include Askama blocks and conditionals

## Common Patterns

### Clickable Row (data-href)

Instead of `onclick`, use `data-href` with external JS:

```html
<tr class="is-clickable" tabindex="0" data-href="/traces/{{ id }}">
  <td>{{ name }}</td>
</tr>
```

The `app.js` file attaches click and Enter-key listeners to all
`[data-href]` elements.

### Confirmation Dialog

Instead of `onclick="return confirm()"`, use `hx-confirm`:

```html
<button
  hx-delete="/webhooks/{{ id }}"
  hx-confirm="Delete this webhook? This cannot be undone."
  hx-target="closest .webhook-detail"
  hx-swap="outerHTML"
>
  Delete
</button>
```

### Expandable Section

Instead of JS-powered accordion, use `<details>`/`<summary>`:

```html
<details>
  <summary>Tool call: file-read</summary>
  <pre class="tool-output">{{ output }}</pre>
</details>
```

### Filter Dropdown

Instead of `onchange`, use `hx-get` with `hx-trigger="change"`:

```html
<select
  hx-get="/traces"
  hx-trigger="change"
  hx-target="#trace-list"
  hx-swap="innerHTML"
  name="status"
>
  <option value="">All</option>
  <option value="ok">OK</option>
  <option value="error">Error</option>
</select>
```

### Debounced Search

```html
<input
  type="search"
  hx-get="/chat/conversations"
  hx-trigger="input changed delay:300ms"
  hx-target="#conversation-list"
  hx-swap="innerHTML"
  name="q"
  placeholder="Search..."
/>
```

### Form Submit with Scroll

```html
<form
  hx-post="/chat/{{ id }}/send"
  hx-target="#messages"
  hx-swap="beforeend scroll:#messages:bottom"
>
  <textarea name="content"></textarea>
  <button type="submit">Send</button>
</form>
```

## Migration from htmx 1.x

Key changes when upgrading from htmx 1.x to 2.0:

### Breaking Changes

1. **Extensions removed from core**: All extensions (SSE, WS, etc.) are
   now separate packages. You must load them independently.

2. **`hx-on` syntax changed**: The multi-line `hx-on="event: handler"`
   syntax is deprecated. Use `hx-on:event-name="handler"` instead.
   Event names must be **kebab-case** (HTML lowercases attributes).

3. **Legacy `hx-sse`/`hx-ws` removed**: Use the extension versions
   (`hx-ext="sse"` + `sse-connect`/`sse-swap`).

4. **`makeFragment()` returns `DocumentFragment`**: Always returns
   `DocumentFragment`, not `Element`.

5. **Extension API change**: `selectAndSwap` replaced with `swap` method.

### Default Changes

| Setting                   | 1.x       | 2.0                 |
| ------------------------- | --------- | ------------------- |
| `scrollBehavior`          | `smooth`  | `instant`           |
| `selfRequestsOnly`        | `false`   | `true`              |
| `methodsThatUseUrlParams` | `["get"]` | `["get", "delete"]` |

### IE Support

IE is no longer supported in htmx 2.0. Use htmx 1.x if you need IE support.

## Anti-Patterns

### Don't return JSON from htmx endpoints

```rust
// WRONG
Json(serde_json::json!({ "status": "ok" }))

// RIGHT
render_template(MyPartialTemplate { ... })
```

### Don't use GET for state-changing operations

```html
<!-- WRONG -->
<button hx-get="/delete/{{ id }}">Delete</button>

<!-- RIGHT -->
<button hx-delete="/items/{{ id }}" hx-confirm="Are you sure?">Delete</button>
```

### Don't cache htmx partials in the service worker

The SW skips requests with `HX-Request: true`. Preserve this behavior.

### Don't nest `hx-ext="sse"` elements

Only one SSE connection per element tree.

### Don't use inline JavaScript with hx-on

```html
<!-- AVOID in this project -->
<div hx-on::after-swap="document.getElementById('x').scrollTo(0, 999)">
  <!-- PREFER: external JS file with event listener -->
  <!-- In app.js or page-specific JS: -->
  <!-- document.body.addEventListener("htmx:afterSwap", handler) -->
</div>
```

### Don't use `hx-vars` (deprecated)

Use `hx-vals` instead:

```html
<!-- WRONG (deprecated) -->
<div hx-vars="myVal:computeValue()">
  <!-- RIGHT -->
  <div hx-vals='{"myVal": "static-value"}'>
    <!-- Or with JS (requires eval): -->
    <div hx-vals="js:{myVal: computeValue()}"></div>
  </div>
</div>
```
