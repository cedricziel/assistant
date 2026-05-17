## 1. Audit (already complete — record findings)

- [x] 1.1 Audit `crates/web-ui/src/` for every `Sse::new(...)` call site. Result: 3 sites total — `api/mod.rs:77` (the `sse_response` helper), `a2a/handlers.rs:297`, `a2a/handlers.rs:458`. All three already apply `.keep_alive(KeepAlive::default())`. The 4 callers of `sse_response()` in `api/messages.rs` and the 1 caller in `api/conversations.rs` inherit it transparently.
- [x] 1.2 Audit the Flutter client's `withHeartbeatTimeout` (`app/lib/api/api_client.dart:473`). Result: it resets on any byte arriving on the underlying stream (the `source.listen((data) { resetWatchdog(); … })` branch). Byte-level reset is already correct on paper.

## 2. Backend integration test

- [x] 2.1 Wrote `crates/web-ui/tests/sse_keepalive_contract.rs` — starts a minimal Axum server with one Sse endpoint backed by an mpsc receiver, applies `.keep_alive(KeepAlive::default())`, holds the receiver silent.
- [x] 2.2 Test connects via `reqwest::Client` and reads the raw `bytes_stream()` — no SSE parser between us and the wire.
- [x] 2.3 First test (`keep_alive_comment_arrives_during_long_silence`) asserts a chunk containing `:` arrives within 20s of total silence. Second test (`semantic_events_still_flow_alongside_keep_alive`) asserts events still flow on either side of a long silence AND a comment arrives in the gap.
- [x] 2.4 Both tests pass on the current codebase (18s wall clock for the first, ~20s for the second). **Confirmed**: keep-alive comments are written to the wire end-to-end. PR #809's symptom is NOT caused by missing or buffered keep-alive.
- [x] 2.5 `cargo test -p assistant-web-ui --test sse_keepalive_contract` — 2 passed, 0 failed.

## 3. Flutter unit test

- [x] 3.1 Wrote `app/test/unit/api/heartbeat_timeout_test.dart`. Uses `fake_async` (added as a direct dev_dependency) to drive a synthetic byte stream emitting `:\n` every 10 seconds for 100 simulated seconds with a 60-second watchdog.
- [x] 3.2 Test 1 asserts 10 chunks delivered and no `TimeoutException`.
- [x] 3.3 Test 2 (negative): 70 seconds of silence with a 60-second watchdog — asserts `TimeoutException`.
- [x] 3.4 Added a bonus test 3: mixed pattern of comment and `data:` chunks at 15-second cadence with a 30-second watchdog — all 5 chunks deliver, no error.
- [x] 3.5 `flutter test test/unit/api/heartbeat_timeout_test.dart` — 3 passed.

## 4. Documentation honesty

- [x] 4.1 Rewrote the `withHeartbeatTimeout` docstring in `app/lib/api/api_client.dart` — corrects the 15-second interval, explains reset-on-any-byte, and points to both test files that lock the contract.
- [x] 4.2 Rewrote the `sse_response()` docstring in `crates/web-ui/src/api/mod.rs` — explicit contract, both proxy-timeout and client-watchdog rationale, pointer to `tests/sse_keepalive_contract.rs`.

## 5. Final verification

- [x] 5.1 `cargo test -p assistant-web-ui --test sse_keepalive_contract` — green (2/2).
- [x] 5.2 `flutter test test/unit/api/heartbeat_timeout_test.dart` — green (3/3).
- [x] 5.3 Disambiguation recorded in proposal.md "Why" section. **Confirmed**: keep-alive is verified end-to-end at both wire level and client-watchdog level. PR #809's "queued message stuck forever" symptom is NOT caused by missing/buffered keep-alive. Root cause must be elsewhere — likely in the queue / drain state machine in `chat_provider.dart`. Investigation continues in the `chat-stream-progress-ux` change and any follow-up triage on PR #809's salvageable fixes (the `isSending` guard in `_recoverStalledStream` and the `attemptReconnect` drain kick remain correct independent fixes).
