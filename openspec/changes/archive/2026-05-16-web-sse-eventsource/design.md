## Context

The Flutter web app's chat list is populated via SSE: `streamConversations()` in `app/lib/api/api_client.dart:272` calls `dio.get('/api/conversations/stream', responseType: ResponseType.stream)` and pipes the bytes through `parseConversationSseByteStream()`. The parser yields a `ConversationSnapshotEvent` first (full list of conversations) then delta events (`upserted`, `deleted`).

This works on native because Dio's IO adapter wraps `dart:io HttpClient`, which delivers chunked HTTP responses incrementally — bytes flow into the parser as the server sends them.

On web, Dio uses `BrowserHttpClientAdapter`, which is XHR-based. Even with `responseType: ResponseType.stream`, the adapter does not actually stream. It waits for the response to complete before emitting bytes. SSE connections are explicitly long-lived (server keepalive every 30s), so the response _never completes_ — the snapshot event never reaches the parser.

Verified live on schorschvm: `curl -H 'Authorization: Bearer ...' /api/conversations` returns a JSON array of 685 conversation summaries; the web UI's chat list shows "No conversations yet"; the mac/iOS apps show all 685.

The browser already has a purpose-built SSE client: `EventSource`. It's part of the WHATWG spec, supported in all modern browsers, exposed through `package:web` (already a dependency, used by `space_selection_storage_web.dart`). Switching to it is the right fix.

## Goals / Non-Goals

**Goals:**

- The conversation list MUST populate on web within the same time-to-first-paint as native (≤500ms after the SSE connection opens).
- Native code paths (mac, iOS) MUST be unchanged.
- The new web path MUST use the same `ConversationListEvent` types so the chat provider doesn't need to know which transport delivered the events.
- Authentication on web SSE MUST work without changes to the dio bearer header path (which `EventSource` cannot send).

**Non-Goals:**

- Replacing dio's stream adapter for short-lived SSE endpoints (chat-message streaming, run-event replay). Those work today because they close on completion; deferred until a similar bug surfaces.
- WebSocket transport.
- Server-side fundamental changes — only the auth surface, if needed.
- Reconnection back-off strategies beyond what `EventSource` provides natively (browsers handle automatic reconnect with `Last-Event-ID`).

## Decisions

### Decision 1: Use the browser's native `EventSource` on web; keep dio on native

`EventSource` is purpose-built for SSE: it delivers `event:`/`data:` frames as they arrive, handles reconnection automatically, supports `Last-Event-ID` resume natively. Compared to a polyfill via `fetch + ReadableStream`, it's smaller, better-tested in browsers, and avoids the byte-parsing surface entirely.

Trade-off: `EventSource` cannot send custom request headers (a long-standing browser API limitation). We can't send `Authorization: Bearer ...` on the SSE connection. See Decision 3 for how we authenticate.

**Alternative considered:** `fetch + ReadableStream`. Works, supports headers, but: (a) we'd have to reimplement reconnection, (b) we'd have to parse SSE frames ourselves (we already have a parser, but it's wired through dio types — refactoring is more work than the EventSource path), (c) `EventSource` is the well-trodden path. Rejected.

### Decision 2: Conditional imports — same pattern as `space_selection_storage`

```
api_client.dart
  └─ delegate streamConversations() to platform-specific impl

event_source_stream.dart           ← public interface
event_source_stream_stub.dart      ← native fallback (delegates back to dio)
event_source_stream_web.dart       ← uses package:web EventSource
```

The conditional import (`if (dart.library.js_interop) '..._web.dart'`) routes web builds to the EventSource adapter; everything else gets the stub which calls into the existing dio stream code. Native binaries don't pull in `package:web`'s JS interop.

This pattern is already in use (`space_selection_storage*.dart` from #687), tests exist for it, no new platform-detection mechanism needed.

### Decision 3: JWT via `?access_token=...` query param on the SSE URL

`EventSource` doesn't allow `Authorization` headers. Two ways to authenticate:

- **A) Cookie auth** — the server already issues an HttpOnly `assistant_session` cookie on `/oauth/token`. The browser sends it automatically with `EventSource(url, { withCredentials: true })`. Server middleware accepts it.
- **B) `?access_token=<jwt>` query param** — server middleware (`assistant-auth`) accepts JWT in this param as a fallback to the bearer header.

Choose **(B)**. Reasons:

- The cookie path requires CORS `withCredentials` to be set, plus `Access-Control-Allow-Credentials: true` and a non-`*` origin on the server. The current server emits `Access-Control-Allow-Origin: *` for the Flutter web bundle (verified earlier), which is incompatible with credentials. Switching to a specific origin per-deploy is brittle.
- The JWT in a query param has the same exposure as the bearer header: visible only to the client and server, encrypted in flight by TLS. Browser history _could_ log the URL, but JWTs are short-lived (1h default) and the SSE URL is hit programmatically — it doesn't go in the address bar.
- Future `web-cookie-auth` change (queued) would replace this with cookie auth across the board. Until then, query-param is the simplest correct path.

Server change required: extend the auth middleware to accept `?access_token=<jwt>` on `/api/conversations/stream` as an alternative to the `Authorization` header. If the middleware already supports this for API keys (`?api_key=...`), we can mirror that pattern.

**Alternative considered:** Sending a short-lived signed URL (e.g., HMAC over the path + expiry, ?signature=...). More secure (one-shot use), but also more code and not necessary at this trust level. Rejected.

### Decision 4: Reuse the existing SSE event parser, just in a different shape

The current parser is `parseConversationSseByteStream(Stream<List<int>>) -> Stream<ConversationListEvent>`. `EventSource` already pre-parses SSE frames into JavaScript `MessageEvent` objects with `event.type` and `event.data` (string). We don't need byte-level parsing on the EventSource path — we just dispatch on event name and `jsonDecode(event.data)`.

A small adapter in `event_source_stream_web.dart` listens for the relevant event names (`snapshot`, `upserted`, `deleted`) and converts each to the corresponding `ConversationListEvent`. ~50 lines of code, no shared parsing surface.

### Decision 5: No retries on the wrapper — `EventSource` does it natively

`EventSource` reconnects automatically on disconnect with browser-default backoff (~3s) and resumes via `Last-Event-ID` if the server sends `id:` lines. Our existing chat-provider reconnect loop (`chat_provider.dart:122-135`) is dio-specific and ignored on the EventSource path.

We will continue to surface "stream errored" via the same `_onStreamError` handler so the UI banner behavior is consistent across platforms.

## Risks / Trade-offs

- **Token in URL is logged by some intermediate proxies.** Mitigation: schorschvm is behind Pangolin (TLS-terminated, single hop). For broader deployments, document that JWT-in-URL is in use; suggest cookie auth (`web-cookie-auth` follow-up) for shared infrastructure.
- **`EventSource.close()` doesn't propagate cancellation through the existing `StreamSubscription` API cleanly.** Mitigation: the adapter wraps a `StreamController` and forwards close events; tests cover the dispose path.
- **`Last-Event-ID` resume requires the server to issue `id:` lines.** Currently the conversation-event SSE doesn't (see `parseConversationSseByteStream` `'id:' lines are consumed and ignored`). Adding `id:` is server-side and out of scope for this change — until then, reconnect just gets a fresh snapshot, which is fine for conversation-list semantics.
- **CORS preflight on `?access_token=...`.** GET with `text/event-stream` accept doesn't need preflight; query params don't trigger one either. No change required.

## Migration Plan

1. Land the change. Web bundle ships with the new adapter; native unchanged.
2. Verify on schorschvm: open chat, confirm conversation list populates within ~500ms of login. Open DevTools Network tab → see one EventSource connection to `/api/conversations/stream?access_token=...`.
3. **Rollback**: revert; native path stays intact, web reverts to broken-but-known state. No data migration.

## Open Questions

- Does the existing auth middleware already accept `?access_token=...`? If not, this change adds it. Want a single shared pattern across all SSE endpoints, not a one-off for conversation-list.
- Should `streamMessages` (the chat reply SSE) also migrate? Currently works because it's short-lived. Defer.
