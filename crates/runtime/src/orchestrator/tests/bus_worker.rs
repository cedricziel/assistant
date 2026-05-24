//! Bus and worker integration tests for the orchestrator:
//! `parse_interface`, `run_worker`, `submit_turn`, correlation propagation,
//! timeout handling, and failed-turn error propagation.

use std::sync::Arc;
use std::time::Duration;

use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::auth::AuthContext;
use assistant_core::types::agent::AssistantConfig;
use assistant_core::types::conversation::{Interface, TurnIdentity};
use assistant_core::{LlmProvider, MessageBus, PublishRequest, bus_messages, topic};
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

use super::super::Orchestrator;
use super::{build, build_with_config, mount_answer, ollama_answer, ollama_tool_calls};

#[test]
fn parse_interface_known_values() {
    use super::super::parse_interface;
    assert_eq!(parse_interface("Cli"), Interface::Cli);
    assert_eq!(parse_interface("cli"), Interface::Cli);
    assert_eq!(parse_interface("Slack"), Interface::Slack);
    assert_eq!(parse_interface("MATTERMOST"), Interface::Mattermost);
    assert_eq!(parse_interface("Signal"), Interface::Signal);
    assert_eq!(parse_interface("mcp"), Interface::Mcp);
}

#[test]
fn parse_interface_unknown_falls_back_to_cli() {
    use super::super::parse_interface;
    assert_eq!(parse_interface("unknown"), Interface::Cli);
    assert_eq!(parse_interface(""), Interface::Cli);
}

#[tokio::test]
async fn run_worker_processes_turn_request() {
    let server = MockServer::start().await;
    mount_answer(&server, "bus response").await;

    let (orch, _storage) = build(&server.uri()).await;

    // Spawn the worker in the background.
    let orch_worker = orch.clone();
    let worker = tokio::spawn(async move {
        orch_worker.run_worker("test-worker").await;
    });

    // Publish a TurnRequest to the bus.
    let conv_id = Uuid::new_v4();
    let turn_req = bus_messages::TurnRequest {
        prompt: "hello from bus".to_string(),
        conversation_id: conv_id,
        extension_tools: vec![],
        timestamp: None,
        traceparent: None,
        attachment_ids: vec![],
        user_id: None,
        org_id: None,
        space_id: None,
    };
    orch.bus()
        .publish(
            PublishRequest::new(
                topic::TURN_REQUEST,
                serde_json::to_value(&turn_req).unwrap(),
            )
            .with_agent_id("default")
            .with_conversation_id(conv_id)
            .with_interface("Cli"),
        )
        .await
        .unwrap();

    // Poll for the worker to process and publish the result instead of
    // a fixed sleep, which can be flaky under CI load.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let results = loop {
        let r = orch.bus().list(topic::TURN_RESULT, None, 10).await.unwrap();
        if !r.is_empty() {
            break r;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for TurnResult"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(results.len(), 1, "expected one TurnResult on the bus");
    let result: bus_messages::TurnResult =
        serde_json::from_value(results[0].payload.clone()).unwrap();
    assert_eq!(result.conversation_id, conv_id);
    assert_eq!(result.content, "bus response");

    // The original request should be acked (done).
    let pending = orch
        .bus()
        .list(
            topic::TURN_REQUEST,
            Some(assistant_core::MessageStatus::Pending),
            10,
        )
        .await
        .unwrap();
    assert!(pending.is_empty(), "turn request should be acked");

    worker.abort();
}

#[tokio::test]
async fn submit_turn_publishes_and_waits_for_result() {
    let server = MockServer::start().await;
    mount_answer(&server, "submitted answer").await;

    let (orch, _storage) = build(&server.uri()).await;

    // Spawn the worker so it can process the submitted turn.
    let orch_worker = orch.clone();
    tokio::spawn(async move {
        orch_worker.run_worker("test-worker").await;
    });

    let conv_id = Uuid::new_v4();
    let result = orch
        .submit_turn(
            &AuthContext::system(),
            "hello via submit",
            conv_id,
            Interface::Cli,
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.answer, "submitted answer");
}

#[tokio::test]
async fn worker_propagates_submit_correlation_fields_to_turn_result() {
    let server = MockServer::start().await;
    mount_answer(&server, "correlated response").await;

    let (orch, _storage) = build(&server.uri()).await;

    let orch_worker = orch.clone();
    let worker = tokio::spawn(async move {
        orch_worker.run_worker("test-worker").await;
    });

    let conv_id = Uuid::new_v4();
    let request_id = Uuid::new_v4();
    let turn_req = bus_messages::TurnRequest {
        prompt: "hello correlation".to_string(),
        conversation_id: conv_id,
        extension_tools: vec![],
        timestamp: None,
        traceparent: None,
        attachment_ids: vec![],
        user_id: None,
        org_id: None,
        space_id: None,
    };

    let request_msg_id = orch
        .bus()
        .publish(
            PublishRequest::new(
                topic::TURN_REQUEST,
                serde_json::to_value(&turn_req).unwrap(),
            )
            .with_agent_id("default")
            .with_conversation_id(conv_id)
            .with_interface("Cli")
            .with_correlation_id(request_id)
            .with_batch_id(request_id),
        )
        .await
        .unwrap();

    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let results = loop {
        let r = orch.bus().list(topic::TURN_RESULT, None, 10).await.unwrap();
        if !r.is_empty() {
            break r;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for correlated TurnResult"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    let result_msg = &results[0];
    assert_eq!(
        result_msg.batch_id,
        Some(request_id),
        "TURN_RESULT should preserve the request batch_id"
    );
    assert_eq!(
        result_msg.correlation_id,
        Some(request_id),
        "TURN_RESULT should preserve the request correlation_id"
    );
    assert_eq!(
        result_msg.causation_id,
        Some(request_msg_id),
        "TURN_RESULT should set causation_id to the TURN_REQUEST message id"
    );

    worker.abort();
}

// ── Worker integration tests ────────────────────────────────────────────────

/// Test that submit_turn works correctly when the worker is running.
/// This reproduces the bug where Slack/Mattermost-only modes didn't spawn
/// the worker, causing submit_turn to timeout waiting for a result.
#[tokio::test]
async fn submit_turn_with_worker_returns_result() {
    let server = MockServer::start().await;
    mount_answer(&server, "hello from worker").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    // Spawn the worker in the background (as the CLI would do).
    let worker_orch = orch.clone();
    let _worker = tokio::spawn(async move {
        worker_orch.run_worker("test-worker").await;
    });

    // Give the worker a moment to start.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result = orch
        .submit_turn(
            &AuthContext::system(),
            "test message",
            conv_id,
            Interface::Slack,
            None,
        )
        .await;

    assert!(
        result.is_ok(),
        "submit_turn should succeed when worker is running"
    );
    assert_eq!(result.unwrap().answer, "hello from worker");
}

/// Test that submit_turn times out when no worker is running.
/// This verifies the bug condition that was fixed.
#[tokio::test]
async fn submit_turn_without_worker_times_out() {
    let server = MockServer::start().await;
    mount_answer(&server, "this should not be received").await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    // No worker spawned — simulates the bug condition.
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        orch.submit_turn(
            &AuthContext::system(),
            "test message",
            conv_id,
            Interface::Slack,
            None,
        ),
    )
    .await;

    assert!(
        result.is_err() || result.unwrap().is_err(),
        "submit_turn should fail/timeout when worker is not running"
    );
}

/// Test that with_submit_timeout sets a custom deadline that is respected.
///
/// We configure a 1-second timeout and verify that submit_turn (with no worker
/// running) completes within a short wall-clock window — i.e. it honours the
/// configured value rather than the 3-hour default.
#[tokio::test]
async fn with_submit_timeout_respected() {
    let server = MockServer::start().await;
    mount_answer(&server, "not used").await;

    // Build without wrapping in Arc so we can apply with_submit_timeout first.
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: server.uri(),
            timeout_secs: 10,
            retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(
        Orchestrator::new(llm, storage, executor.clone(), registry, bus, &config)
            .with_submit_timeout(1), // 1-second deadline
    );
    executor.set_subagent_runner(orch.clone());

    let conv_id = Uuid::new_v4();
    let start = std::time::Instant::now();
    // Wrap in a generous outer timeout to prevent the test hanging if the
    // inner deadline is accidentally the 3-hour default.
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        orch.submit_turn(
            &AuthContext::system(),
            "test message",
            conv_id,
            Interface::Slack,
            None,
        ),
    )
    .await;
    let elapsed = start.elapsed();

    // The inner submit_turn must have timed out (no worker running).
    assert!(
        result.is_ok() && result.unwrap().is_err(),
        "submit_turn should return Err when no worker is running"
    );
    // And it should have done so close to the configured 1-second deadline.
    assert!(
        elapsed < Duration::from_secs(5),
        "submit_turn should respect the 1-second timeout, not the 3-hour default (elapsed: {elapsed:?})"
    );
}

#[tokio::test]
async fn failed_turn_propagates_error() {
    // Verifies that when the LLM returns an error, run_turn propagates Err.
    // The error path is also where we set OtelStatus::Error on the turn span.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let (orch, _storage) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    let result = orch
        .run_turn(
            "test",
            conv_id,
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await;
    assert!(
        result.is_err(),
        "expected run_turn to return Err on LLM failure"
    );
}

/// Verify that when submit_turn times out it cancels the in-flight worker
/// turn via the CancellationToken registered in turn_cancellations.
///
/// The mock LLM is configured to delay its response by 5 s.  submit_turn
/// is given a 1-second deadline.  We assert:
///  1. submit_turn returns an error within ~2 s (the deadline fires).
///  2. The worker does NOT continue processing: if it did it would produce a
///     turn.result message that would linger on the bus, which we check for.
#[tokio::test]
async fn timeout_cancels_in_flight_worker_turn() {
    let server = MockServer::start().await;
    // Delay the LLM response so the turn is still running when submit_turn times out.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(5))
                .set_body_json(ollama_answer("delayed answer")),
        )
        .mount(&server)
        .await;

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: server.uri(),
            timeout_secs: 30,
            retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(
        Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry,
            bus,
            &config,
        )
        .with_submit_timeout(1), // 1-second deadline
    );
    executor.set_subagent_runner(orch.clone());

    // Spawn the worker so it picks up and processes the turn.
    let worker_orch = orch.clone();
    let _worker = tokio::spawn(async move {
        worker_orch.run_worker("test-worker").await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let conv_id = Uuid::new_v4();
    let start = std::time::Instant::now();
    let result = orch
        .submit_turn(
            &AuthContext::system(),
            "slow task",
            conv_id,
            Interface::Slack,
            None,
        )
        .await;
    let elapsed = start.elapsed();

    // submit_turn must have timed out with an error.
    assert!(result.is_err(), "submit_turn should error on timeout");
    assert!(
        elapsed < Duration::from_secs(4),
        "submit_turn should have stopped within ~1 s, not waited for the 5-s LLM response (elapsed: {elapsed:?})"
    );

    // Give the worker a moment — if cancellation worked there should be no
    // lingering turn.result message on the bus.
    tokio::time::sleep(Duration::from_millis(200)).await;
    let leftover = storage
        .message_bus()
        .claim_filtered(
            assistant_core::topic::TURN_RESULT,
            "check",
            &assistant_core::ClaimFilter::new(),
        )
        .await
        .unwrap();
    assert!(
        leftover.is_none(),
        "worker should not have published a turn.result after cancellation"
    );
}

/// `run_turn_with_tools` uses `run_turn_with_tools_impl` which has its own
/// LLM call site.  Verify that an HTTP-500 from the LLM propagates as `Err`
/// through the extension-tool path (separate from `run_turn_core`).
#[tokio::test]
async fn failed_turn_with_tools_llm_error_propagates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;

    let result = orch
        .run_turn_with_tools(
            "test",
            Uuid::new_v4(),
            Interface::Slack,
            vec![],
            None,
            vec![],
            vec![],
            TurnIdentity::default(),
        )
        .await;
    assert!(
        result.is_err(),
        "run_turn_with_tools must return Err on LLM HTTP-500"
    );
}

/// `run_turn_with_tools` must return `Err` when the LLM loops forever
/// (always returns tool calls, never a final answer).
#[tokio::test]
async fn failed_turn_with_tools_max_iterations_returns_error() {
    let server = MockServer::start().await;
    // LLM never produces a final answer — always returns a tool call.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["unknown-tool"])),
        )
        .mount(&server)
        .await;

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    config.llm.max_iterations = 2;
    let (orch, _) = build_with_config(&server.uri(), config).await;

    let result = orch
        .run_turn_with_tools(
            "loop forever",
            Uuid::new_v4(),
            Interface::Cli,
            vec![],
            None,
            vec![],
            vec![],
            TurnIdentity::default(),
        )
        .await;
    match result {
        Ok(_) => panic!("should fail when max iterations reached"),
        Err(e) => assert!(
            e.to_string().contains("Max iterations"),
            "error should mention max iterations: {e}"
        ),
    }
}

// -- Cancellation -------------------------------------------------------------

/// Cancelling an unknown `request_id` is a no-op and reports `NotFound`.
#[tokio::test]
async fn cancel_turn_unknown_request_id_returns_not_found() {
    let server = MockServer::start().await;
    mount_answer(&server, "unused").await;
    let (orch, _) = build(&server.uri()).await;

    let outcome = orch.cancel_turn(Uuid::new_v4()).await;
    assert_eq!(
        outcome,
        crate::orchestrator::CancelOutcome::NotFound,
        "cancelling an unregistered request_id must report NotFound"
    );
}

/// Cancelling a turn that's been launched with a caller-supplied request_id
/// reports `Cancelled`, aborts the in-flight worker future, and unblocks
/// `submit_turn_with_request_id` promptly with the `turn_cancelled` marker.
#[tokio::test]
async fn cancel_turn_aborts_inflight_submit() {
    // LLM never responds — without cancellation submit_turn would wait the
    // full configured submit_timeout. We assert the cancel returns it early.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(60)))
        .mount(&server)
        .await;

    let mut config = AssistantConfig::default();
    config.memory.enabled = false;
    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: server.uri(),
            timeout_secs: 120,
            retry_config: assistant_llm_provider::retry::RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
    let orch = Arc::new(
        Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry,
            bus,
            &config,
        )
        .with_submit_timeout(30), // generous; the test must finish well before this
    );
    executor.set_subagent_runner(orch.clone());

    // Spawn the worker so the bus message gets picked up and dispatched.
    let orch_worker = orch.clone();
    tokio::spawn(async move {
        orch_worker.run_worker("test-worker").await;
    });

    let conv_id = Uuid::new_v4();
    let request_id = Uuid::new_v4();

    // Fire submit_turn_with_request_id in the background — it will block on
    // the slow LLM call until we cancel.
    let orch_for_submit = orch.clone();
    let handle = tokio::spawn(async move {
        orch_for_submit
            .submit_turn_with_request_id(
                &AuthContext::system(),
                request_id,
                "say hi",
                conv_id,
                Interface::Web,
                None,
                vec![],
            )
            .await
    });

    // Wait until the orchestrator has registered the cancellation token —
    // i.e. submit_turn_internal has reached the body. Bounded to keep the
    // test from hanging if registration ever regresses.
    let registration_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !orch.is_turn_in_flight(request_id).await {
        if tokio::time::Instant::now() > registration_deadline {
            panic!("turn never registered its cancellation token");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Cancel.
    let outcome = orch.cancel_turn(request_id).await;
    assert_eq!(
        outcome,
        crate::orchestrator::CancelOutcome::Cancelled,
        "cancel_turn must report Cancelled when a token is registered"
    );

    // submit_turn_with_request_id should return promptly with the cancelled
    // marker — well under the 30 s submit_timeout configured above.
    let result = tokio::time::timeout(Duration::from_secs(10), handle)
        .await
        .expect("submit must return within 10 s of cancel")
        .expect("join task");
    let err = match result {
        Ok(_) => panic!("submit_turn must return Err on cancel"),
        Err(e) => e,
    };
    assert!(
        err.to_string()
            .contains(crate::orchestrator::TURN_CANCELLED_MARKER),
        "cancelled submit must carry the marker; got: {err}"
    );

    // The token should be deregistered once submit_turn returns, so a
    // second cancel attempt is a no-op.
    assert_eq!(
        orch.cancel_turn(request_id).await,
        crate::orchestrator::CancelOutcome::NotFound,
        "request_id should be deregistered after submit_turn returns"
    );
}
