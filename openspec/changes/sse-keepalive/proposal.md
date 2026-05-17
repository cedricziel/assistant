## Why

The first draft of this proposal claimed the web-ui's SSE endpoints did not call `.keep_alive()` on their `Sse` response, hypothesising that the missing keep-alive caused the client byte heartbeat to false-fire during slow tool calls (the symptom motivating PR #809). Implementation work revealed that hypothesis was **wrong**:

```
  Every Sse::new in crates/web-ui/:
  ─────────────────────────────────
  api/mod.rs:77            (sse_response helper)   .keep_alive(KeepAlive::default())
  a2a/handlers.rs:297                              .keep_alive(KeepAlive::default())
  a2a/handlers.rs:458                              .keep_alive(KeepAlive::default())

  All callers of sse_response() inherit it:
    api/messages.rs:189   POST /api/conversations/{id}/messages
    api/messages.rs:504   second SSE endpoint
    api/messages.rs:755   third SSE endpoint
    api/messages.rs:1181  fourth SSE endpoint
    api/conversations.rs:278
```

Axum's `KeepAlive::default()` emits a `:` comment line every 15 seconds of byte silence. The Flutter client's `withHeartbeatTimeout` in `app/lib/api/api_client.dart:473` resets on any byte (including comment bytes) with a 90-second window — so on paper the architecture should not false-fire on a slow tool call.

Yet PR #809's symptom was real: messages typed during a slow turn ended up stuck in `pendingQueue` until the user backgrounded the app. **Something between "Axum writes the comment byte" and "the Flutter watchdog resets" is broken or not what we think.** Possible causes worth confirming with tests:

1. The comment bytes are buffered (HTTP/2 framing, dio response buffering, a proxy) and don't actually arrive within the 15-second window at the client.
2. The bytes arrive but `withHeartbeatTimeout` isn't actually applied to that specific stream — there's a code path that bypasses it.
3. The bytes arrive and the watchdog resets correctly, but the symptom is unrelated to the byte watchdog (the queue is stuck for a different reason — drain logic, state machine bug).

This change pivots from "add keep-alive" to "**lock in the end-to-end keep-alive contract with tests so we can reason about future stream-health work**". Without these tests, every future change to the SSE infrastructure or the client byte watchdog risks silently breaking the keep-alive path again.

## What Changes

- **Backend integration test** (`crates/web-ui/tests/` or `crates/integration-tests/`): start the web-ui server, connect to `POST /api/conversations/{id}/messages` (or a smaller test SSE endpoint that we can hold open), force a deliberate silence of >20 seconds on the server side, read the raw response bytes, and assert that at least one comment line (`:` followed by a newline) arrives within any 20-second window. The test fails if Axum stops emitting keep-alive, if a wrapper is buffering, or if the helper regresses.
- **Flutter unit test** (`app/test/unit/api/`): drive `withHeartbeatTimeout` with a synthetic byte stream that emits ONLY SSE comment lines (`:\n`) at 10-second intervals for 100 seconds, with a 60-second timeout window. Assert that `withHeartbeatTimeout` does NOT fire its `TimeoutException`. Today there is no test that pins this behaviour; if a future refactor changes the watchdog to "reset only on data: lines", this test catches it.
- **Documentation note** in `crates/web-ui/src/api/mod.rs` near the `sse_response` helper: a one-paragraph contract describing what `KeepAlive::default()` provides (15-second comment interval), what the client relies on (`withHeartbeatTimeout` resets on any byte), and a pointer to the integration test that locks the contract.

This change explicitly **does not** modify any production code in the keep-alive path. It only adds tests and a doc comment. The production behaviour should already be correct.

## Capabilities

### Modified Capabilities

- `web-sse-eventsource`: Add a requirement that the keep-alive contract (server emits a `:` comment within every 15-second window of byte silence; client resets its watchdog on any byte) is covered by automated tests on both sides of the wire. Without these tests, the contract has been silently regression-prone.

## Impact

- **Code**: zero changes to production code. New Rust test file in `crates/web-ui/tests/` or extension to `crates/integration-tests/`. New Dart test in `app/test/unit/api/`. One paragraph of doc comment in `api/mod.rs`.
- **CI**: the new tests run as part of the existing `cargo test` and `flutter test` jobs.
- **Disambiguation for downstream work**: once these tests are passing on `main`, the PR #809 symptom's root cause must be elsewhere (per the elimination logic above). The `chat-stream-progress-ux` and `turn-status-endpoint` changes can move forward without re-investigating whether keep-alive is broken.
- **Doc honesty**: the `api_client.dart` heartbeat doc currently says "the server sends keepalive comments every 30 seconds" but Axum's `KeepAlive::default()` is 15 seconds. Worth correcting that comment as part of this change.
- **Risk**: low. Adding tests around an existing contract.
