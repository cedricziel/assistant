## 1. Server: accept `?access_token=...` on SSE endpoints

- [x] 1.1 Audit `crates/auth/src/middleware.rs` (and friends) — does the auth middleware already accept `?access_token=<jwt>` as a fallback to the bearer header? If yes, skip 1.2–1.3. If no:
- [x] 1.2 Extend the bearer-token extractor to also check `request.uri().query()` for `access_token=<value>` when no `Authorization` header is present.
- [x] 1.3 Add unit tests: valid token in query → 200 with populated `AuthContext`; invalid token in query → 401; query-token AND header → header wins (defensive).
- [x] 1.4 Run `cargo test -p assistant-auth` — all green.

## 2. Failing tests first (TDD red)

- [x] 2.1 Add `app/test/unit/api/event_source_stream_test.dart`: a fake `_FakeEventSource` that exposes `dispatchSnapshot`/`dispatchUpserted`/`dispatchDeleted` methods. Assert the adapter wraps it correctly and yields the right `ConversationListEvent` types.
- [x] 2.2 Run `flutter test` — confirm the test fails because `EventSourceConversationStream` does not exist.

## 3. Web adapter

- [x] 3.1 Add `app/lib/api/event_source_stream.dart`: the public surface — a `Stream<ConversationListEvent> openEventSourceStream({required String url, required String token})`-style function and the conditional-import pattern (`stub` + `web`).
- [x] 3.2 Add `app/lib/api/event_source_stream_stub.dart`: a stub that throws `UnsupportedError` when called on non-web (it should never be called there).
- [x] 3.3 Add `app/lib/api/event_source_stream_web.dart`: uses `package:web`'s `EventSource`. Listens for `event: snapshot`, `event: upserted`, `event: deleted` via `addEventListener`. JSON-decodes `event.data` and yields the matching `ConversationListEvent`. On `error` with `readyState == CLOSED`, emits `ApiAuthException`.
- [x] 3.4 Update `ApiClient.streamConversations` to delegate to the platform adapter on web; keep the dio path otherwise. Pass the token via `?access_token=...`.
- [x] 3.5 Confirm test 2.1 turns green.

## 4. Smoke + ship

- [x] 4.1 `flutter analyze --fatal-infos` → 0 issues.
- [x] 4.2 `flutter test` → all green.
- [x] 4.3 `flutter build web --release` → succeeds.
- [x] 4.4 Manual smoke against schorschvm: open chat, confirm the conversation list populates within ~500ms of login. DevTools Network → see one `EventSource` connection (not XHR).
- [x] 4.5 PR: `feat(app): use EventSource for SSE on web`. Body links the four scenarios.
- [x] 4.6 Merge and deploy via apt update.
- [x] 4.7 Archive: `openspec archive web-sse-eventsource`.
