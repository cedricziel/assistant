## Why

The conversation list is populated via `GET /api/conversations/stream` (SSE). On native (mac/iOS), Dio's IO adapter delivers chunked HTTP correctly and the snapshot event reaches the chat provider within milliseconds. On the web, Dio uses `BrowserHttpClientAdapter` (XHR-based) which buffers the entire response body until the connection closes — and SSE connections never close (server keepalives every 30s). The snapshot event therefore **never fires on web**, and the chat list shows "No conversations yet" indefinitely even when the API returns hundreds of conversations. Confirmed live on the schorschvm deployment: `curl /api/conversations` returns 685 entries; web UI shows 0; mac/iOS native shows 685.

## What Changes

- Add a small `EventSourceConversationStream` adapter in `app/lib/api/` that uses the browser's native `EventSource` API (via `package:web`) for the conversation-list SSE stream on web. Same parsed `ConversationListEvent` output shape as the dio path — drop-in replacement.
- Conditional-import wires it: `streamConversations()` keeps the dio path on native; on web it delegates to `EventSource`. Same pattern we used for `space_selection_storage`.
- Auth: `EventSource` does not natively support custom headers. Pass the JWT as a `?access_token=...` query parameter (already supported by the JWT middleware via the existing API key pattern, or add it). Same security posture as the bearer header — TLS in flight, not exposed beyond the SSE response.
- Tests: a unit test that pumps a stream of SSE bytes through the parser and asserts `ConversationSnapshotEvent` + delta events arrive correctly. Web-specific E2E is out of scope (covered by manual smoke).

## Capabilities

### New Capabilities

- `web-sse-eventsource`: How the Flutter web app consumes SSE responses (specifically the conversation-list stream) using the browser's native `EventSource` API instead of dio's broken-on-web stream adapter.

### Modified Capabilities

(none — `web-401-recovery`, `space-selector-resilience`, and `web-session-resilience` continue unchanged.)

## Impact

- **Code touched**: `app/lib/api/api_client.dart` (route `streamConversations` through a platform-aware helper), new `app/lib/api/event_source_stream_stub.dart` and `app/lib/api/event_source_stream_web.dart`. Possibly `crates/web-ui/src/api/mod.rs` to accept JWT via `?access_token=...` query param if the auth middleware doesn't already.
- **Tests**: unit test for the SSE byte-stream parser plus a thin test that the conditional import resolves correctly on each platform.
- **Behavior change on web**: conversation list populates within ~100ms of login (matching native).
- **Native behavior**: unchanged.
- **Non-goals**:
  - Replacing dio's stream adapter for non-SSE endpoints (chat message streaming, run replays). Those work because they short-lived — they close on completion. Could migrate later if the same bug bites.
  - WebSocket transport. SSE-over-EventSource is sufficient and simpler.
  - Server-side changes beyond accepting the JWT in a query param if needed.
- **User-facing documentation needed**: No.
