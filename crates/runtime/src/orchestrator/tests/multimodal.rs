//! Multimodal content, attachment-collection, image resize, and audio-store
//! integration tests for the orchestrator.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::tool::{ToolHandler, ToolOutput};
use assistant_core::types::agent::AssistantConfig;
use assistant_core::types::conversation::{ExecutionContext, Interface, TurnIdentity};
use assistant_core::{ChatHistoryMessage, ChatRole, ContentBlock, LlmProvider, MessageBus};
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_storage::{StorageLayer, registry::SkillRegistry};
use assistant_tool_executor::ToolExecutor;

use super::super::{Orchestrator, max_image_bytes_for_provider, resize_and_encode};
use super::{
    MockExtTool, build, build_with_executor, mount_answer, ollama_answer, ollama_tool_calls,
    ollama_tool_calls_with_args,
};

#[test]
fn serialize_history_multimodal_user_omits_base64_data() {
    use crate::otel_spans::serialize_history_for_span;

    let history = vec![ChatHistoryMessage::MultimodalUser {
        content: vec![
            ContentBlock::Text("describe this".to_string()),
            ContentBlock::Image {
                media_type: "image/png".to_string(),
                data: "A".repeat(10_000), // large base64 payload
            },
        ],
    }];

    let json_str = serialize_history_for_span(&history);
    assert!(
        !json_str.contains(&"A".repeat(100)),
        "base64 data must NOT appear in span output"
    );
    assert!(
        json_str.contains("image/png"),
        "media_type should be present"
    );
    assert!(
        json_str.contains("size_base64_chars"),
        "size_base64_chars field should be present"
    );
}

#[tokio::test]
async fn prepare_history_with_attachments_emits_multimodal_user() {
    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, _) = build(&server.uri()).await;

    let conv_id = Uuid::new_v4();
    let attachments = vec![ContentBlock::Image {
        media_type: "image/jpeg".to_string(),
        data: "base64data".to_string(),
    }];

    let (_conv_store, history, _turn) = orch
        .prepare_history("look at this", conv_id, attachments, &[], &orch.agent_id)
        .await
        .unwrap();

    // The last message in history should be MultimodalUser.
    let last = history.last().expect("history non-empty");
    match last {
        ChatHistoryMessage::MultimodalUser { content } => {
            assert_eq!(content.len(), 2, "text block + image block");
            assert!(
                matches!(&content[0], ContentBlock::Text(t) if t == "look at this"),
                "first block should be the text"
            );
            assert!(
                matches!(&content[1], ContentBlock::Image { media_type, .. } if media_type == "image/jpeg"),
                "second block should be the image"
            );
        }
        other => panic!("expected MultimodalUser, got {:?}", other),
    }
}

#[tokio::test]
async fn prepare_history_without_attachments_emits_plain_text() {
    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, _) = build(&server.uri()).await;

    let conv_id = Uuid::new_v4();
    let (_conv_store, history, _turn) = orch
        .prepare_history("hello", conv_id, Vec::new(), &[], &orch.agent_id)
        .await
        .unwrap();

    let last = history.last().expect("history non-empty");
    match last {
        ChatHistoryMessage::Text { role, content } => {
            assert_eq!(*role, ChatRole::User);
            assert_eq!(content, "hello");
        }
        other => panic!("expected Text, got {:?}", other),
    }
}

/// A fake tool handler that returns attachments in its output.
struct MockAttachmentTool {
    attachments: Vec<assistant_core::Attachment>,
}

impl MockAttachmentTool {
    fn new(attachments: Vec<assistant_core::Attachment>) -> Self {
        Self { attachments }
    }
}

#[async_trait]
impl ToolHandler for MockAttachmentTool {
    fn name(&self) -> &str {
        "attachment-tool"
    }

    fn description(&self) -> &str {
        "returns attachments for testing"
    }

    fn params_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn run(
        &self,
        _params: HashMap<String, Value>,
        _ctx: &ExecutionContext,
    ) -> anyhow::Result<ToolOutput> {
        Ok(
            ToolOutput::success("generated 1 attachment")
                .with_attachments(self.attachments.clone()),
        )
    }
}

#[tokio::test]
async fn run_turn_collects_attachments_from_tool_output() {
    let server = MockServer::start().await;

    // 1st LLM call: model calls "attachment-tool".
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["attachment-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2nd LLM call: final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("here you go")))
        .mount(&server)
        .await;

    let (orch, _, executor) = build_with_executor(&server.uri()).await;

    // Register our mock tool that returns an attachment.
    let png_bytes = vec![0x89, 0x50, 0x4E, 0x47];
    executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
        assistant_core::Attachment::new("chart.png", "image/png", png_bytes.clone()),
    ])));

    let result = orch
        .run_turn(
            "make a chart",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert_eq!(result.answer, "here you go");
    assert_eq!(
        result.attachments.len(),
        1,
        "expected 1 attachment in TurnResult"
    );
    assert_eq!(result.attachments[0].filename, "chart.png");
    assert_eq!(result.attachments[0].mime_type, "image/png");
    assert_eq!(result.attachments[0].data, png_bytes);
}

#[tokio::test]
async fn run_turn_collects_multiple_attachments_across_tool_calls() {
    let server = MockServer::start().await;

    // Model calls attachment-tool twice in one turn.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ollama_tool_calls(&["attachment-tool", "attachment-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _, executor) = build_with_executor(&server.uri()).await;

    executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
        assistant_core::Attachment::new("file.txt", "text/plain", b"hello".to_vec()),
    ])));

    let result = orch
        .run_turn(
            "go",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        result.attachments.len(),
        2,
        "each tool call should contribute one attachment"
    );
    assert_eq!(result.attachments[0].filename, "file.txt");
    assert_eq!(result.attachments[1].filename, "file.txt");
}

#[tokio::test]
async fn run_turn_no_attachments_when_tools_return_none() {
    let server = MockServer::start().await;
    mount_answer(&server, "pong").await;

    let (orch, _, _) = build_with_executor(&server.uri()).await;

    let result = orch
        .run_turn(
            "hello",
            Uuid::new_v4(),
            Interface::Cli,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert!(
        result.attachments.is_empty(),
        "no tool calls means no attachments"
    );
}

#[tokio::test]
async fn run_turn_streaming_collects_attachments() {
    let server = MockServer::start().await;

    // 1st LLM call: model calls "attachment-tool".
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["attachment-tool"])),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // 2nd LLM call: final answer.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
        .mount(&server)
        .await;

    let (orch, _, executor) = build_with_executor(&server.uri()).await;
    executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
        assistant_core::Attachment::new("report.pdf", "application/pdf", vec![0x25, 0x50]),
    ])));

    let (tx, mut rx) = tokio::sync::mpsc::channel::<super::super::OrchestratorEvent>(64);

    // Drain events in background.
    tokio::spawn(async move { while rx.recv().await.is_some() {} });

    let result = orch
        .run_turn_streaming(
            "generate report",
            Uuid::new_v4(),
            Interface::Cli,
            tx,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();

    assert_eq!(result.attachments.len(), 1);
    assert_eq!(result.attachments[0].filename, "report.pdf");
    assert_eq!(result.attachments[0].mime_type, "application/pdf");
}

#[tokio::test]
async fn run_turn_with_tools_collects_attachments_from_extension() {
    let server = MockServer::start().await;

    // Model calls the extension tool then reply then end_turn.
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                ("ext-attach", json!({})),
                ("reply", json!({"text": "done"})),
                ("end_turn", json!({"reason": "done"})),
            ])),
        )
        .mount(&server)
        .await;

    let (orch, _, _) = build_with_executor(&server.uri()).await;

    // Create an extension tool that returns attachments.
    struct ExtAttachTool;

    #[async_trait]
    impl ToolHandler for ExtAttachTool {
        fn name(&self) -> &str {
            "ext-attach"
        }
        fn description(&self) -> &str {
            "returns an attachment"
        }
        fn params_schema(&self) -> Value {
            json!({"type": "object", "properties": {}, "required": []})
        }
        async fn run(
            &self,
            _params: HashMap<String, Value>,
            _ctx: &ExecutionContext,
        ) -> anyhow::Result<ToolOutput> {
            Ok(ToolOutput::success("image generated").with_attachment(
                assistant_core::Attachment::new("img.png", "image/png", vec![1, 2, 3]),
            ))
        }
    }

    let reply_handler = Arc::new(MockExtTool::new("reply"));
    let ext_attach = Arc::new(ExtAttachTool);

    // run_turn_with_tools returns Ok(()) — we can't inspect attachments
    // directly, but we verify the call succeeds without panicking and
    // that the extension tool is executed (reply is called).
    orch.run_turn_with_tools(
        "make image",
        Uuid::new_v4(),
        Interface::Slack,
        vec![
            ext_attach as Arc<dyn ToolHandler>,
            reply_handler.clone() as Arc<dyn ToolHandler>,
        ],
        None,
        vec![],
        vec![],
        TurnIdentity::default(),
    )
    .await
    .unwrap();

    assert_eq!(
        reply_handler.calls(),
        1,
        "reply tool should have been called"
    );
}

#[test]
fn resize_and_encode_small_image_returns_base64_without_resize() {
    use base64::Engine as _;

    let raw = b"tiny image bytes";
    let encoded = resize_and_encode(raw, "image/png", "anthropic");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .unwrap();
    assert_eq!(
        decoded, raw,
        "small images should be returned as-is (base64-encoded)"
    );
}

#[test]
fn resize_and_encode_unknown_mime_returns_base64() {
    use base64::Engine as _;

    let raw = b"some pdf content";
    let encoded = resize_and_encode(raw, "application/pdf", "openai");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .unwrap();
    assert_eq!(decoded, raw, "unknown MIME types should be returned as-is");
}

#[test]
fn max_image_bytes_anthropic_is_5mb() {
    assert_eq!(max_image_bytes_for_provider("anthropic"), 5 * 1024 * 1024);
}

#[test]
fn max_image_bytes_openai_is_20mb() {
    assert_eq!(max_image_bytes_for_provider("openai"), 20 * 1024 * 1024);
}

#[test]
fn max_image_bytes_default_is_conservative() {
    assert_eq!(max_image_bytes_for_provider("ollama"), 5 * 1024 * 1024);
}

// ── AudioStore integration ──────────────────────────────────────────────

#[tokio::test]
async fn with_audio_store_makes_store_available() {
    let store = Arc::new(assistant_transcription::AudioStore::new());
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm: Arc<dyn LlmProvider> = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: "http://localhost:1".to_string(),
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
    let orch = Orchestrator::new(llm, storage, executor, registry, bus, &config)
        .with_audio_store(store.clone());

    assert!(
        orch.audio_store.is_some(),
        "audio_store should be set after with_audio_store"
    );

    // Verify the store is the same instance.
    let id = store
        .insert(b"fake-mp3".to_vec(), "audio/mpeg".to_string())
        .await;
    let retrieved = orch.audio_store.as_ref().unwrap().get(id).await;
    assert!(
        retrieved.is_some(),
        "orchestrator's audio store should share state with the provided store"
    );
}

#[tokio::test]
async fn audio_store_none_by_default() {
    let server = MockServer::start().await;
    mount_answer(&server, "ok").await;
    let (orch, _) = build(&server.uri()).await;
    assert!(
        orch.audio_store.is_none(),
        "audio_store should be None by default"
    );
}
