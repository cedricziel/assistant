---
name: htmx2
description: >
  htmx 2 patterns used in the assistant web UI. Covers partial HTML
  responses, SSE streaming, swap strategies, fragment templates, and
  the chat streaming architecture. Use when building or modifying any
  htmx-powered interaction in the web UI.
license: MIT
metadata:
  tier: info
  mutating: "false"
  confirmation-required: "false"
---

# htmx 2 Patterns

The assistant web UI uses htmx 2.0 for dynamic interactions without
client-side JavaScript frameworks. The server returns HTML fragments;
htmx swaps them into the DOM. SSE (Server-Sent Events) powers real-time
streaming for the chat interface.

## Dependencies

Loaded in `base.html` (and thus available on all authenticated pages):

```html
<script src="https://unpkg.com/htmx.org@2.0.4/dist/htmx.min.js"></script>
<script src="https://unpkg.com/htmx-ext-sse@2.2.2/sse.js"></script>
```

## Core Concepts

### 1. Server Returns HTML, Not JSON

Every htmx endpoint returns an HTML fragment — never JSON. The fragment
is a rendered Askama template (a "partial" that does **not** extend
`base.html`).

```rust
// Handler returns a rendered HTML fragment
pub async fn conversation_list(...) -> Response {
    render_template(ConversationListTemplate { conversations })
}
```

```html
<!-- Template: chat/conversation_list.html (no {% extends %}) -->
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

| Type      | Extends `base.html`? | Has `app_css_url`? | Used by                 |
| --------- | -------------------- | ------------------ | ----------------------- |
| Full page | Yes                  | Yes                | Direct navigation (GET) |
| Partial   | No                   | No                 | htmx requests (XHR)     |

htmx adds `HX-Request: true` header. The service worker skips these
requests (they must not be cached as full pages).

### 3. Swap Strategies

| Strategy     | Use When                             | Example                         |
| ------------ | ------------------------------------ | ------------------------------- |
| `innerHTML`  | Replace container contents           | Search results, panel content   |
| `outerHTML`  | Replace the element itself           | Streaming → final message       |
| `beforeend`  | Append to container (list, messages) | New message, new conversation   |
| `afterbegin` | Prepend to container                 | New conversation at top of list |

```html
<!-- Search: replace list contents -->
<input
  hx-get="/chat/conversations"
  hx-trigger="input changed delay:300ms"
  hx-target="#conversation-list"
  hx-swap="innerHTML"
/>

<!-- Send message: append to messages, scroll to bottom -->
<form
  hx-post="/chat/{{ id }}/send"
  hx-target="#messages"
  hx-swap="beforeend scroll:#messages:bottom"
>
  <!-- New conversation: prepend to sidebar -->
  <button
    hx-post="/chat/new"
    hx-target="#conversation-list"
    hx-swap="afterbegin"
  ></button>
</form>
```

## SSE Streaming (Chat)

The chat uses htmx's SSE extension for token-by-token streaming from the
LLM.

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
  |<-- event: done\ndata: <html>--|  (final rendered message, replaces stream)
  |                                |
```

### streaming.html (SSE partial)

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
   `data` (a fully rendered message HTML) replaces the entire streaming
   element via `hx-swap="outerHTML"`

### Server-Side Streaming Endpoint

```rust
pub async fn stream_chat(
    Path((id, )): Path<(String, )>,
    Query(params): Query<StreamParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        // Token-by-token from LLM
        while let Some(token) = rx.recv().await {
            yield Ok(Event::default()
                .event("token")
                .data(html_escape(&token)));
        }
        // Final rendered message
        let html = render_template(MessageTemplate { ... });
        yield Ok(Event::default()
            .event("done")
            .data(html));
    };
    Sse::new(stream)
}
```

## Common Patterns

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

- `delay:300ms` debounces typing — waits 300ms after the last keystroke
- `changed` only fires if the value actually changed
- The server receives `?q=search+term` and returns filtered HTML

### Click-to-Load Detail

```html
<div
  class="conv-item"
  hx-get="/chat/{{ conv.id }}"
  hx-target="#chat-panel"
  hx-swap="innerHTML"
>
  {{ conv.title }}
</div>
```

Clicking the item loads the conversation into the panel without a full
page reload.

### Form Submit with Target

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

- Form data is sent as `application/x-www-form-urlencoded`
- Response HTML is appended to `#messages`
- `scroll:#messages:bottom` auto-scrolls the container

### Scroll-to-Bottom Modifier

```
hx-swap="beforeend scroll:#messages:bottom"
```

The `scroll:` modifier tells htmx to scroll the specified element after
the swap completes. Use `#messages:bottom` to keep the chat scrolled to
the latest message.

## Template Organisation

### Full-page templates (extend `base.html`)

These are served on direct navigation. They include the full app shell
(nav, header, content area):

```
templates/chat/page.html          → extends "base.html"
templates/traces/page.html        → extends "base.html"
templates/agents/list.html        → extends "base.html"
```

### Partial templates (standalone fragments)

These are returned by htmx XHR endpoints. They contain only the HTML
fragment to be swapped in:

```
templates/chat/panel.html         → conversation panel content
templates/chat/message.html       → single message bubble
templates/chat/conversation_list.html → sidebar conversation list
templates/chat/streaming.html     → SSE streaming container
```

**Rules:**

- Partials never extend `base.html`
- Partials never include `<html>`, `<head>`, or `<body>` tags
- Partials don't need `app_css_url` (CSS is already loaded by the page)
- Partials can include Askama blocks and conditionals

## Anti-Patterns

### Don't return JSON from htmx endpoints

```rust
// WRONG — htmx expects HTML
Json(serde_json::json!({ "status": "ok" }))

// RIGHT — return rendered HTML fragment
render_template(MyPartialTemplate { ... })
```

### Don't use `hx-get` for state-changing operations

```html
<!-- WRONG — GET should be idempotent -->
<button hx-get="/chat/{{ id }}/delete">Delete</button>

<!-- RIGHT — use POST for mutations -->
<form method="POST" action="/chat/{{ id }}/delete">
  <button type="submit">Delete</button>
</form>
```

### Don't cache htmx partials in the service worker

The service worker already skips requests with `HX-Request: true` header.
If you add custom fetch handling, preserve this behaviour.

### Don't nest `hx-ext="sse"` elements

Only one SSE connection per element tree. The streaming template uses a
single top-level `hx-ext="sse"` with `sse-swap` on both the outer (done)
and inner (token) elements.
