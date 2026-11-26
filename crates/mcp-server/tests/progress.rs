//! `run-prompt` reports progress and stops its turn when the request is
//! cancelled.

mod common;

use std::sync::Arc;
use std::time::Duration;

use assistant_runtime::StubOrchestrationEngine;
use common::*;
use rmcp::ServiceExt;
use rmcp::model::{CallToolRequest, ClientRequest, ProgressNotificationParam, ProtocolVersion};
use rmcp::service::PeerRequestOptions;
use serde_json::json;
use tokio::sync::mpsc;

/// Waits up to five seconds for `condition` to hold.
async fn eventually(mut condition: impl FnMut() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("condition holds within five seconds");
}

async fn next_progress(rx: &mut mpsc::UnboundedReceiver<Received>) -> ProgressNotificationParam {
    match next_received(rx).await {
        Received::Progress(progress) => progress,
        other => panic!("expected progress, got {other:?}"),
    }
}

#[tokio::test]
async fn run_prompt_reports_progress_at_start_and_completion() {
    let stub = StubOrchestrationEngine::new().queue_turn(Ok(turn("done")));
    let server = default_stubs()
        .orchestrator(Arc::new(stub))
        .server(allow_everything());
    let (recorder, mut received) = Recorder::new(ProtocolVersion::V_2025_11_25);
    let client = recorder.serve(spawn_server(server)).await.unwrap();

    let handle = client
        .send_request_with_option(
            ClientRequest::CallToolRequest(CallToolRequest::new(call_params(
                "run-prompt",
                json!({"prompt": "hi"}),
            ))),
            PeerRequestOptions::no_options(),
        )
        .await
        .unwrap();
    let token = handle.progress_token.clone();
    handle.await_response().await.unwrap();

    let started = next_progress(&mut received).await;
    let finished = next_progress(&mut received).await;
    assert_eq!(started.progress_token, token);
    assert_eq!(finished.progress_token, token);
    assert_eq!(started.progress, 0.0, "{started:?}");
    assert_eq!(finished.progress, 1.0, "{finished:?}");
    assert_eq!(finished.total, Some(1.0), "{finished:?}");
}

#[tokio::test]
async fn cancelling_run_prompt_cancels_the_turn() {
    let stub = Arc::new(StubOrchestrationEngine::new().hold_turns());
    let server = default_stubs()
        .orchestrator(stub.clone())
        .server(allow_everything());
    let (recorder, mut received) = Recorder::new(ProtocolVersion::V_2025_11_25);
    let client = recorder.serve(spawn_server(server)).await.unwrap();

    let handle = client
        .send_cancellable_request(
            ClientRequest::CallToolRequest(CallToolRequest::new(call_params(
                "run-prompt",
                json!({"prompt": "hi"}),
            ))),
            PeerRequestOptions::no_options(),
        )
        .await
        .unwrap();
    eventually(|| !stub.submit_calls().is_empty()).await;
    let started = next_progress(&mut received).await;
    handle.cancel(Some("user stopped".into())).await.unwrap();

    eventually(|| !stub.cancel_calls().is_empty()).await;
    let submitted = stub.submit_calls()[0].request_id;
    let cancelled = stub.cancel_calls()[0];
    assert_eq!(submitted, Some(cancelled));

    // Nothing more arrives for the cancelled request.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        received.try_recv().is_err(),
        "only {started:?} was expected"
    );
}
