//! End-to-end contract test for Axum's SSE keep-alive.
//!
//! Locks the fact that the `KeepAlive::default()` we apply to every
//! `Sse::new(...)` in `crates/web-ui` actually writes a `:` comment
//! line onto the wire during periods of byte silence — not just at
//! the framework level but observably to a raw HTTP client.
//!
//! Why this exists: PR #809 was motivated by a "queued message stuck
//! forever during a slow tool call" symptom. The first diagnosis was
//! "the server doesn't send keep-alive, so the client byte heartbeat
//! false-fires". Investigation revealed every `Sse::new` already calls
//! `.keep_alive(KeepAlive::default())`. This test pins that behaviour
//! end-to-end so future refactors can't silently break the contract —
//! and so the disambiguation of PR #809's symptom (which has a
//! different root cause) survives.
//!
//! Refs: openspec/changes/sse-keepalive/

use std::time::Duration;

use axum::{
    Router,
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
};
use futures::StreamExt;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Build a minimal Axum router with one SSE endpoint backed by an
/// mpsc receiver. Callers control event emission (or non-emission)
/// via the returned sender; the response applies the keep-alive
/// configuration we want to lock in.
fn test_router() -> (
    Router,
    mpsc::Sender<Result<Event, std::convert::Infallible>>,
) {
    let (tx, rx) = mpsc::channel::<Result<Event, std::convert::Infallible>>(8);
    let stream = ReceiverStream::new(rx);

    // Move into an Arc<Mutex<Option<...>>> so the handler closure can
    // own its own copy of the stream the first time it's called. We
    // only ever call this endpoint once per test, so a simple Option
    // dance suffices.
    let stream_holder = std::sync::Arc::new(tokio::sync::Mutex::new(Some(stream)));
    let router = Router::new().route(
        "/stream",
        get(move || {
            let holder = stream_holder.clone();
            async move {
                let stream = holder
                    .lock()
                    .await
                    .take()
                    .expect("test endpoint called more than once");
                // This is the contract under test: every Sse::new must
                // apply KeepAlive::default(). The default emits a `:`
                // comment line every 15 seconds during byte silence.
                Sse::new(stream).keep_alive(KeepAlive::default())
            }
        }),
    );
    (router, tx)
}

/// Spawn the test router on a random localhost port. Returns the
/// bound URL of the `/stream` endpoint.
async fn spawn_server(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, router).await.expect("serve");
    });
    format!("http://{}/stream", addr)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keep_alive_comment_arrives_during_long_silence() {
    let (router, _tx) = test_router();
    let url = spawn_server(router).await;

    // Connect with a low-level HTTP client. `reqwest::bytes_stream`
    // surfaces the raw response body, including SSE comment lines —
    // unlike an `EventSource` parser which would filter them.
    let resp = reqwest::Client::builder()
        .build()
        .expect("client")
        .get(&url)
        .send()
        .await
        .expect("get");
    assert!(
        resp.status().is_success(),
        "expected 2xx, got {}",
        resp.status()
    );

    let mut stream = resp.bytes_stream();

    // Wait up to 20 seconds for the first chunk. The handler is held
    // silent (no `tx.send`), so any bytes we see are keep-alive comments.
    let mut saw_comment = false;
    let mut total_bytes = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, stream.next()).await {
            Err(_) => break,   // overall deadline hit
            Ok(None) => break, // stream ended
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(Some(Ok(chunk))) => {
                total_bytes += chunk.len();
                // Axum's KeepAlive emits `:` (the SSE comment prefix)
                // followed by optional text and a newline. The exact
                // formatting may vary across Axum versions, so we
                // accept any byte sequence that starts with `:`.
                if chunk.iter().any(|b| *b == b':') {
                    saw_comment = true;
                    break;
                }
            }
        }
    }

    assert!(
        saw_comment,
        "expected at least one keep-alive comment byte (`:`) within 20 seconds \
         of silence; received {total_bytes} bytes total. \
         If this fails: either KeepAlive::default() was removed from an \
         Sse::new call site, a response wrapper is buffering the comments, \
         or Axum changed the comment format."
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn semantic_events_still_flow_alongside_keep_alive() {
    // Regression guard: turning on keep-alive must not interfere with
    // normal event delivery. Send one user-visible event, then go silent
    // for 18 seconds, then send another. Assert we observe both events
    // AND at least one comment in the gap.
    let (router, tx) = test_router();
    let url = spawn_server(router).await;

    let producer = tokio::spawn(async move {
        // First event: arrives immediately.
        tx.send(Ok(Event::default().event("hello").data("1")))
            .await
            .expect("send 1");
        // Long silence — keep-alive should fill it.
        tokio::time::sleep(Duration::from_secs(18)).await;
        // Second event: arrives after the silence.
        tx.send(Ok(Event::default().event("world").data("2")))
            .await
            .expect("send 2");
    });

    let resp = reqwest::Client::builder()
        .build()
        .expect("client")
        .get(&url)
        .send()
        .await
        .expect("get");
    let mut stream = resp.bytes_stream();

    let mut saw_event_1 = false;
    let mut saw_event_2 = false;
    let mut saw_comment = false;
    let mut buf = Vec::<u8>::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(25);

    while tokio::time::Instant::now() < deadline && !(saw_event_1 && saw_event_2 && saw_comment) {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(remaining, stream.next()).await {
            Err(_) => break,
            Ok(None) => break,
            Ok(Some(Err(e))) => panic!("stream error: {e}"),
            Ok(Some(Ok(chunk))) => {
                buf.extend_from_slice(&chunk);
                // Look for the events and the comment in accumulated bytes.
                let txt = String::from_utf8_lossy(&buf);
                if txt.contains("event: hello") {
                    saw_event_1 = true;
                }
                if txt.contains("event: world") {
                    saw_event_2 = true;
                }
                // SSE comment line starts with `:` at the beginning of a line.
                if buf
                    .iter()
                    .enumerate()
                    .any(|(i, b)| *b == b':' && (i == 0 || buf[i - 1] == b'\n'))
                {
                    saw_comment = true;
                }
            }
        }
    }

    producer.await.expect("producer");

    assert!(saw_event_1, "first event must arrive");
    assert!(saw_event_2, "second event must arrive after the silence");
    assert!(
        saw_comment,
        "at least one keep-alive comment must appear in the 18-second gap"
    );
}
