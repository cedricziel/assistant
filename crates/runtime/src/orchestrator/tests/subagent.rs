//! Subagent orchestrator integration tests:
//! `run_subagent`, conversation delegation, depth limit, tool filtering,
//! cancellation, LLM-error reporting.

use assistant_storage::{AgentStore, ConversationStore, PersonaStore as _};
use std::time::Duration;

use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::types::agent::AssistantConfig;
use assistant_core::types::conversation::{DEFAULT_MAX_AGENT_DEPTH, Interface, TurnIdentity};
use assistant_core::{AgentReportStatus, AgentSpawn, SubagentRunner};

use super::{
    build, build_with_config, mount_answer, ollama_answer, ollama_thinking, ollama_tool_calls,
    ollama_tool_calls_with_args,
};

#[tokio::test]
async fn subagent_spawn_complete_round_trip() {
    let server = MockServer::start().await;

    // The subagent's LLM will return a final answer directly.
    mount_answer(&server, "subagent result").await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "test-agent-1".into(),
        task: "What is 2+2?".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    assert_eq!(report.status, AgentReportStatus::Completed);
    assert_eq!(report.content, "subagent result");

    // Verify lifecycle was recorded in the DB.
    let agent_store = storage.agent_store();
    let record = agent_store
        .get("test-agent-1")
        .await
        .unwrap()
        .expect("agent record should exist");
    assert_eq!(record.status, assistant_storage::AgentStatus::Completed);
    assert!(record.completed_at.is_some());
    assert_eq!(record.task, "What is 2+2?");
}

#[tokio::test]
async fn conversation_can_delegate_to_anonymous_subagent() {
    let server = MockServer::start().await;

    let task = "Search the web for rust async testing tips";

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("delegate anonymously"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                "agent-spawn",
                json!({
                    "task": task,
                    "allowed_tools": ["web-search"]
                }),
            )])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains(task))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("research-result")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("research-result"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_answer("parent acknowledged anonymous result")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    let turn = orch
        .run_turn(
            "please delegate anonymously",
            conv_id,
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();
    assert_eq!(turn.answer, "parent acknowledged anonymous result");

    let records = storage
        .agent_store()
        .list_by_parent_conversation(&conv_id.to_string())
        .await
        .unwrap();
    assert_eq!(records.len(), 1, "expected one delegated subagent record");
    assert_eq!(records[0].parent_conversation_id, conv_id.to_string());
    assert_eq!(records[0].parent_agent_id.as_deref(), Some("default"));

    let child_conv_id = Uuid::parse_str(&records[0].conversation_id).unwrap();
    let anonymous_scope = format!("anonymous::{}", records[0].id);
    let child_conv = storage
        .conversation_store_for_agent(&anonymous_scope)
        .get_conversation(child_conv_id)
        .await
        .unwrap()
        .expect("anonymous subagent conversation should be scoped to isolated persona");
    assert_eq!(child_conv.agent_id, anonymous_scope);

    let reqs = server.received_requests().await.unwrap();
    assert_eq!(
        reqs.len(),
        3,
        "expected parent->subagent->parent LLM call sequence"
    );
}

#[tokio::test]
async fn conversation_can_delegate_to_existing_agent_context() {
    let server = MockServer::start().await;

    let task = "Draft a launch blurb for next week";

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("use marketing"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                "agent-spawn",
                json!({
                    "agent_id": "marketing",
                    "task": task
                }),
            )])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains(task))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("marketing-result")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .and(body_string_contains("marketing-result"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_answer("parent acknowledged marketing result")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;
    storage
        .persona_store()
        .ensure_exists("default")
        .await
        .unwrap();
    storage
        .persona_store()
        .ensure_exists("marketing")
        .await
        .unwrap();

    let conv_id = Uuid::new_v4();
    let turn = orch
        .run_turn(
            "please use marketing",
            conv_id,
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();
    assert_eq!(turn.answer, "parent acknowledged marketing result");

    let delegated = storage
        .agent_store()
        .list_by_parent_conversation(&conv_id.to_string())
        .await
        .unwrap();
    assert_eq!(delegated.len(), 1, "expected one delegated subagent record");
    assert_eq!(delegated[0].parent_conversation_id, conv_id.to_string());
    assert_eq!(delegated[0].parent_agent_id.as_deref(), Some("default"));

    let child_conv_id = Uuid::parse_str(&delegated[0].conversation_id).unwrap();
    let child_conv = storage
        .conversation_store_for_agent("marketing")
        .get_conversation(child_conv_id)
        .await
        .unwrap()
        .expect("persona-bound subagent conversation should be scoped to target persona");
    assert_eq!(child_conv.agent_id, "marketing");

    let marketing = storage
        .agent_store()
        .get("marketing")
        .await
        .unwrap()
        .expect("marketing subagent record should exist");
    assert_eq!(marketing.status, assistant_storage::AgentStatus::Completed);
    assert_eq!(marketing.task, task);
}

#[tokio::test]
async fn subagent_nesting_depth_limit_enforced() {
    let server = MockServer::start().await;
    mount_answer(&server, "should not reach here").await;

    let (orch, _) = build(&server.uri()).await;

    // Spawn at max depth — should be rejected.
    let spawn = AgentSpawn {
        agent_id: "deep-agent".into(),
        task: "too deep".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch
        .run_subagent(spawn, DEFAULT_MAX_AGENT_DEPTH)
        .await
        .unwrap();

    assert_eq!(report.status, AgentReportStatus::Failed);
    assert!(
        report.content.contains("depth"),
        "error should mention depth: {}",
        report.content
    );

    // No LLM call should have been made.
    let reqs = server.received_requests().await.unwrap();
    assert!(
        reqs.is_empty(),
        "no LLM calls should be made when depth limit is exceeded"
    );
}

#[tokio::test]
async fn subagent_tool_filtering_restricts_tools() {
    let server = MockServer::start().await;

    // Subagent LLM tries to call "bash" which is NOT in the allowed list.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["bash"])))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call returns final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "restricted-agent".into(),
        task: "try to use bash".into(),
        system_prompt: None,
        model: None,
        // Only allow file-read — bash should be rejected.
        allowed_tools: vec!["file-read".into()],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    // The subagent should still complete (the LLM got a rejection
    // observation and then returned a final answer).
    assert_eq!(report.status, AgentReportStatus::Completed);
    assert_eq!(report.content, "done");

    // Verify the first LLM call had the restricted tool set —
    // the request should only contain "file-read", not "bash".
    let reqs = server.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 2);
    let body: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    let tool_names: Vec<String> = body["tools"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|t| t["function"]["name"].as_str().map(String::from))
        .collect();
    assert!(
        tool_names.contains(&"file-read".to_string()),
        "file-read should be in tool specs: {tool_names:?}"
    );
    assert!(
        !tool_names.contains(&"bash".to_string()),
        "bash should NOT be in tool specs: {tool_names:?}"
    );
}

#[tokio::test]
async fn subagent_cancellation_stops_loop() {
    let server = MockServer::start().await;

    // The subagent LLM returns tool calls indefinitely, so the subagent
    // would loop forever if not cancelled.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_tool_calls(&["unknown-tool"]))
                // Add a small delay so the cancel has time to trigger
                .set_body_json(ollama_tool_calls(&["unknown-tool"])),
        )
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "cancel-me".into(),
        task: "infinite loop task".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    // Cancel the agent before it starts by pre-cancelling.
    // We can't easily cancel mid-loop in a unit test, but we can
    // test that the cancel_agent mechanism works by:
    // 1. Registering the token manually would require access to internals.
    // Instead, test cancel_agent returns false for unknown agents.
    let cancelled = orch.cancel_agent("nonexistent").await.unwrap();
    assert!(
        !cancelled,
        "cancelling nonexistent agent should return false"
    );

    // Test the actual cancellation flow: spawn in a task, cancel shortly after.
    let orch2 = orch.clone();
    let handle = tokio::spawn(async move { orch2.run_subagent(spawn, 0).await.unwrap() });

    // Give the subagent a moment to start and register the token.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let cancelled = orch.cancel_agent("cancel-me").await.unwrap();
    assert!(cancelled, "should find and cancel the running agent");

    // Wait for the subagent to finish.
    let report = handle.await.unwrap();
    assert_eq!(
        report.status,
        AgentReportStatus::Cancelled,
        "subagent should report Cancelled status, got: {:?}",
        report.status
    );

    // Verify lifecycle recorded as cancelled.
    let agent_store = storage.agent_store();
    let record = agent_store
        .get("cancel-me")
        .await
        .unwrap()
        .expect("agent record should exist");
    assert_eq!(record.status, assistant_storage::AgentStatus::Cancelled);
}

#[tokio::test]
async fn subagent_llm_error_records_failed_status() {
    let server = MockServer::start().await;

    // LLM returns a 500 error.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "error-agent".into(),
        task: "this will fail".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    assert_eq!(report.status, AgentReportStatus::Failed);
    assert!(report.content.contains("LLM error"));

    let agent_store = storage.agent_store();
    let record = agent_store
        .get("error-agent")
        .await
        .unwrap()
        .expect("agent record should exist");
    assert_eq!(record.status, assistant_storage::AgentStatus::Failed);
}
#[tokio::test]
async fn subagent_thinking_step_persisted_to_db() {
    let server = MockServer::start().await;

    // First call: LLM returns a thinking step (empty content + thinking field).
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_thinking("deep thought")))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call: LLM returns a normal final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, storage) = build(&server.uri()).await;

    let spawn = AgentSpawn {
        agent_id: "thinking-agent".into(),
        task: "think about it".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };
    let anon_scope = format!("anonymous::{}", spawn.agent_id);

    let report = orch.run_subagent(spawn, 0).await.unwrap();
    assert_eq!(report.status, AgentReportStatus::Completed);

    // Retrieve the subagent's conversation_id from the agent record.
    let agent_store = storage.agent_store();
    let record = agent_store
        .get("thinking-agent")
        .await
        .unwrap()
        .expect("agent record should exist");
    let conv_id =
        Uuid::parse_str(&record.conversation_id).expect("conversation_id should be a valid UUID");

    // Load persisted messages and verify the thinking step is present.
    // Anonymous subagents (persona_bound: false) scope their conversation under
    // "anonymous::<agent_id>", so we must use the matching store to find messages.
    let conv_store = storage.conversation_store_for_agent(&anon_scope);
    let messages = conv_store.load_history(conv_id).await.unwrap();
    let thinking_msg = messages.iter().find(|m| m.content.contains("<think>"));
    assert!(
        thinking_msg.is_some(),
        "thinking step should be persisted to DB; messages: {:?}",
        messages.iter().map(|m| &m.content).collect::<Vec<_>>()
    );
    assert!(
        thinking_msg.unwrap().content.contains("deep thought"),
        "persisted thinking message should contain the thought text"
    );
}

#[tokio::test]
async fn subagent_max_iterations_returns_failed_status() {
    let server = MockServer::start().await;

    // LLM always returns tool calls, never a final answer.
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

    let spawn = AgentSpawn {
        agent_id: "loop-agent".into(),
        task: "infinite loop".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: None,
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();

    assert_eq!(
        report.status,
        AgentReportStatus::Failed,
        "subagent should report Failed when max iterations reached"
    );
    assert!(
        report.content.contains("max iterations"),
        "error should mention max iterations: {}",
        report.content
    );
}

#[tokio::test]
async fn subagent_forwards_inner_events_to_parent_sink() {
    let server = MockServer::start().await;

    // The subagent's LLM returns a final answer directly.
    mount_answer(&server, "subagent done").await;

    let (orch, _storage) = build(&server.uri()).await;

    // Register a parent event sink keyed by a fake parent conversation_id.
    let parent_conv_id = Uuid::new_v4();
    let (parent_tx, mut parent_rx) =
        tokio::sync::mpsc::channel::<super::super::OrchestratorEvent>(128);
    orch.register_token_sink(parent_conv_id, parent_tx).await;

    let spawn = AgentSpawn {
        agent_id: "streaming-sub".into(),
        task: "Say hello".into(),
        system_prompt: None,
        model: None,
        allowed_tools: vec![],
        persona_bound: false,
        parent_conversation_id: Some(parent_conv_id),
        parent_agent_id: None,
    };

    let report = orch.run_subagent(spawn, 0).await.unwrap();
    assert_eq!(report.status, AgentReportStatus::Completed);

    // Collect all events from the parent sink.
    let mut events = Vec::new();
    while let Ok(event) = parent_rx.try_recv() {
        events.push(event);
    }

    // Must contain SubagentStarted.
    assert!(
        events.iter().any(|e| matches!(
            e,
            super::super::OrchestratorEvent::SubagentStarted { agent_id, .. }
            if agent_id == "streaming-sub"
        )),
        "Expected SubagentStarted event, got: {:?}",
        events
    );

    // Must contain SubagentCompleted.
    assert!(
        events.iter().any(|e| matches!(
            e,
            super::super::OrchestratorEvent::SubagentCompleted { agent_id, status, .. }
            if agent_id == "streaming-sub" && status == "ok"
        )),
        "Expected SubagentCompleted event, got: {:?}",
        events
    );
}
