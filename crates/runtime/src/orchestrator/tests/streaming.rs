//! Streaming-path tests for `run_turn_with_tools`: token + thinking event
//! emission via the registered `OrchestratorEvent` sink.

use std::time::Duration;

use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::types::conversation::{Interface, TurnIdentity};

use super::super::OrchestratorEvent;
use super::{build, ollama_answer, ollama_thinking};

/// Build a newline-delimited JSON (NDJSON) streaming body that the Ollama
/// `chat_native_streaming` parser can consume.  Each `token` becomes a
/// `{"message":{"content":token},"done":false}` chunk; a final
/// `{"done":true}` chunk terminates the stream.
fn ollama_streaming_body(tokens: &[&str]) -> String {
    let mut body = String::new();
    for token in tokens {
        let line = json!({
            "model": "test",
            "message": {"role": "assistant", "content": token},
            "done": false,
        });
        body.push_str(&line.to_string());
        body.push('\n');
    }
    // Final chunk signals end-of-stream.
    let done = json!({
        "model": "test",
        "message": {"role": "assistant", "content": ""},
        "done": true,
        "done_reason": "stop",
    });
    body.push_str(&done.to_string());
    body.push('\n');
    body
}

/// `run_turn_with_tools_streaming` must forward tokens from the LLM text
/// stream through the caller-supplied `token_sink`.
#[tokio::test]
async fn run_turn_with_tools_streaming_emits_tokens_through_sink() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(ollama_streaming_body(&["Hello", ",", " world", "!"])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);

    orch.run_turn_with_tools_streaming(
        "hi",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    // Drain all received tokens.
    let mut received = String::new();
    while let Ok(event) = rx.try_recv() {
        if let OrchestratorEvent::Token(t) = event {
            received.push_str(&t);
        }
    }

    assert_eq!(
        received, "Hello, world!",
        "token_sink must receive all LLM text tokens"
    );
}

/// When the token_sink receives tokens in multiple chunks, they must arrive
/// in order and form the complete response.
#[tokio::test]
async fn run_turn_with_tools_streaming_tokens_arrive_in_order() {
    let server = MockServer::start().await;
    let tokens = ["The", " quick", " brown", " fox"];
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ollama_streaming_body(&tokens)))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);

    orch.run_turn_with_tools_streaming(
        "tell me something",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    let mut received_tokens: Vec<String> = Vec::new();
    while let Ok(event) = rx.try_recv() {
        if let OrchestratorEvent::Token(t) = event {
            received_tokens.push(t);
        }
    }

    let joined: String = received_tokens.join("");
    assert_eq!(
        joined, "The quick brown fox",
        "tokens must arrive in order and reconstruct the full text"
    );
}

/// When the worker processes a turn that has BOTH extension tools AND a
/// token_sink registered, it must route to `run_turn_with_tools_streaming`
/// so the sink receives tokens (rather than silently ignoring the sink).
#[tokio::test]
async fn worker_routes_ext_plus_sink_to_streaming_path() {
    let server = MockServer::start().await;
    // Mount a streaming answer: the LLM returns text tokens (FinalAnswer path),
    // which flow through the token_sink.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(ollama_streaming_body(&["streamed", " answer"])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let conv_id = Uuid::new_v4();

    // Spawn the worker.
    let worker_orch = orch.clone();
    let _worker = tokio::spawn(async move {
        worker_orch.run_worker("test-worker").await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Register BOTH a token_sink AND extension tools (the Slack pattern).
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);
    orch.register_token_sink(conv_id, tx).await;
    orch.register_extensions(conv_id, vec![], vec![]).await;

    // submit_turn triggers the worker which should pick up both registrations.
    let _ = orch
        .submit_turn("stream please", conv_id, Interface::Slack, None)
        .await;

    // Allow any in-flight sends to the channel to complete.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut received = String::new();
    while let Ok(event) = rx.try_recv() {
        if let OrchestratorEvent::Token(t) = event {
            received.push_str(&t);
        }
    }

    assert_eq!(
        received, "streamed answer",
        "worker must route to streaming path when ext tools + token_sink are both registered; \
         sink received: {received:?}"
    );
}

/// When the LLM returns a `Thinking` response during a streaming turn,
/// the orchestrator must emit an `OrchestratorEvent::Thinking` event
/// followed by the final answer tokens.
#[tokio::test]
async fn run_turn_streaming_emits_thinking_event() {
    let server = MockServer::start().await;

    // First call: thinking response (empty content, thinking field set).
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_thinking("Let me reason about this")),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call: final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(ollama_streaming_body(&["The", " answer"])),
        )
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);

    orch.run_turn_with_tools_streaming(
        "think first then answer",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    // Collect all events.
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }

    // Must contain a Thinking event.
    let has_thinking = events.iter().any(
        |e| matches!(e, OrchestratorEvent::Thinking(content) if content.contains("Let me reason")),
    );
    assert!(
        has_thinking,
        "Expected OrchestratorEvent::Thinking but got: {:?}",
        events
    );

    // Must also contain Token events from the final answer.
    let mut token_text = String::new();
    for event in &events {
        if let OrchestratorEvent::Token(t) = event {
            token_text.push_str(t);
        }
    }
    assert_eq!(
        token_text, "The answer",
        "Token events should contain the final answer text"
    );
}

/// Ollama response with both tool calls and thinking.
fn ollama_tool_calls_with_thinking(names: &[&str], thought: &str) -> Value {
    let tc: Vec<Value> = names
        .iter()
        .map(|n| json!({ "function": { "name": n, "arguments": {} } }))
        .collect();
    json!({
        "model": "test",
        "message": { "role": "assistant", "content": "", "tool_calls": tc, "thinking": thought },
        "done": true
    })
}

#[tokio::test]
async fn run_turn_streaming_emits_thinking_before_tool_calls() {
    let server = MockServer::start().await;

    // First call: tool call with batch thinking (non-streaming, so thinking
    // is carried in ToolCallResponse.thinking).
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_thinking(
                &["end_turn"],
                "I should end",
            )),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Second call: final answer after tool loop ends.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("Done")))
        .mount(&server)
        .await;

    let (orch, _) = build(&server.uri()).await;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<OrchestratorEvent>(64);

    orch.run_turn_with_tools_streaming(
        "do something",
        Uuid::new_v4(),
        Interface::Slack,
        vec![],
        None,
        vec![],
        tx,
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    // Collect all events.
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }

    // Must contain a Thinking event with the batch thinking content.
    let thinking_idx = events.iter().position(
        |e| matches!(e, OrchestratorEvent::Thinking(content) if content.contains("I should end")),
    );
    assert!(
        thinking_idx.is_some(),
        "Expected OrchestratorEvent::Thinking before tool calls, got: {:?}",
        events
    );
}
