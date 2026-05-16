//! Shared test fixtures for `api::*` handler tests.
//!
//! Each sibling handler module declares its own `#[cfg(test)] mod tests`;
//! all of them pull fixtures from here via
//! `use super::super::test_helpers::*;`.

#![cfg(test)]

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::body::Body;
use http_body_util::BodyExt;
use tokio::sync::RwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use assistant_core::types::agent::AssistantConfig;
use assistant_llm_provider::ollama::client::{LlmClient, LlmClientConfig};
use assistant_llm_provider::retry::RetryConfig;
use assistant_runtime::{CommandRegistry, Orchestrator};
use assistant_storage::{
    AttachmentStore, CommandEventStore, ConversationEventStore, InMemoryConversationBroadcaster,
    RunBroadcaster, SkillRegistry, StorageLayer,
};
use assistant_tool_executor::ToolExecutor;
use assistant_transcription::{TranscriptionProvider, TranscriptionRequest, TranscriptionResult};

use super::{ApiState, api_router};

// -- Stubs ---------------------------------------------------------------------

/// Stub transcription provider that returns a fixed transcript.
pub(crate) struct StubTranscriptionProvider {
    pub(crate) transcript: String,
}

#[async_trait]
impl TranscriptionProvider for StubTranscriptionProvider {
    fn name(&self) -> &str {
        "stub"
    }

    async fn transcribe(
        &self,
        _request: TranscriptionRequest,
    ) -> anyhow::Result<TranscriptionResult> {
        Ok(TranscriptionResult {
            text: self.transcript.clone(),
            language: None,
            duration_secs: None,
        })
    }
}

// -- HTTP / LLM fixtures -------------------------------------------------------

/// Minimal LLM mock: returns a static assistant reply.
pub(crate) async fn mount_llm_reply(server: &MockServer, reply: &str) {
    let body = serde_json::json!({
        "model": "test",
        "message": { "role": "assistant", "content": reply },
        "done": true
    });
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

/// Build an `ApiState` wired to an in-memory DB and a mock LLM server.
pub(crate) async fn test_state(llm_url: &str) -> (ApiState, Arc<StorageLayer>) {
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: llm_url.to_string(),
            timeout_secs: 5,
            retry_config: RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus = Arc::new(storage.message_bus());
    let orchestrator = Arc::new(Orchestrator::new(
        llm,
        storage.clone(),
        executor,
        registry,
        bus,
        &config,
    ));

    // Spawn the turn-processing worker so submit_turn requests are handled.
    let worker_orch = orchestrator.clone();
    tokio::spawn(async move {
        worker_orch.run_worker("test-worker").await;
    });

    let orchestrator_ref = orchestrator.clone();
    let default_model = orchestrator_ref.llm.model_name().to_string();
    let state = ApiState {
        pool: storage.pool.clone(),
        agent_id: Arc::new(RwLock::new("default".to_string())),
        orchestrator,
        push_dispatcher: None,
        transcription_provider: None,
        tts_provider: None,
        audio_store: Arc::new(crate::audio_store::AudioStore::new()),
        event_store: ConversationEventStore::new(storage.pool.clone()),
        run_broadcaster: RunBroadcaster::new(),
        attachment_store: AttachmentStore::new(storage.pool.clone()),
        command_registry: Arc::new(CommandRegistry::new()),
        command_event_store: CommandEventStore::new(storage.pool.clone()),
        conversation_broadcaster: Arc::new(InMemoryConversationBroadcaster::new()),
        conversation_configs: Arc::new(RwLock::new(HashMap::new())),
        orchestrator_ref,
        active_turns: Arc::new(RwLock::new(HashMap::new())),
        default_model,
    };
    (state, storage)
}

/// Create a minimal state with a real event store but a stub orchestrator.
/// No worker task is spawned, so the single in-memory connection is not
/// contended. Use this for tests that only exercise the event log endpoints.
pub(crate) async fn event_log_state() -> (ApiState, Arc<StorageLayer>) {
    let mut config = AssistantConfig::default();
    config.memory.enabled = false;

    let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
    let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
    let llm = Arc::new(
        LlmClient::new(LlmClientConfig {
            model: "test".to_string(),
            base_url: "http://127.0.0.1:1".to_string(),
            timeout_secs: 1,
            retry_config: RetryConfig::disabled(),
        })
        .unwrap(),
    );
    let executor = Arc::new(ToolExecutor::new(
        storage.clone(),
        llm.clone(),
        registry.clone(),
        Arc::new(config.clone()),
    ));
    let bus = Arc::new(storage.message_bus());
    let orchestrator = Arc::new(Orchestrator::new(
        llm,
        storage.clone(),
        executor,
        registry,
        bus,
        &config,
    ));
    // NOTE: No worker task spawned — avoids contention on the single in-memory connection.
    let orchestrator_ref = orchestrator.clone();
    let default_model = orchestrator_ref.llm.model_name().to_string();
    let state = ApiState {
        pool: storage.pool.clone(),
        agent_id: Arc::new(RwLock::new("default".to_string())),
        orchestrator,
        push_dispatcher: None,
        transcription_provider: None,
        tts_provider: None,
        audio_store: Arc::new(crate::audio_store::AudioStore::new()),
        event_store: ConversationEventStore::new(storage.pool.clone()),
        run_broadcaster: RunBroadcaster::new(),
        attachment_store: AttachmentStore::new(storage.pool.clone()),
        command_registry: Arc::new(CommandRegistry::new()),
        command_event_store: CommandEventStore::new(storage.pool.clone()),
        conversation_broadcaster: Arc::new(InMemoryConversationBroadcaster::new()),
        conversation_configs: Arc::new(RwLock::new(HashMap::new())),
        orchestrator_ref,
        active_turns: Arc::new(RwLock::new(HashMap::new())),
        default_model,
    };
    (state, storage)
}

pub(crate) fn app(state: ApiState) -> axum::Router {
    api_router().with_state(state)
}

pub(crate) async fn body_bytes(body: Body) -> Vec<u8> {
    body.collect().await.unwrap().to_bytes().to_vec()
}

pub(crate) async fn body_json(body: Body) -> serde_json::Value {
    let b = body_bytes(body).await;
    serde_json::from_slice(&b).unwrap()
}

// -- Attachment fixtures -------------------------------------------------------

pub(crate) fn tiny_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(1, 1, Rgba([255, 0, 0, 255]));
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
    buf.into_inner()
}

/// Build a multipart body with a single `file` field.
pub(crate) fn multipart_body(
    field_name: &str,
    filename: &str,
    mime: &str,
    data: &[u8],
) -> (String, Vec<u8>) {
    let boundary = "----TestBoundary123";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {mime}\r\n\r\n").as_bytes());
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), body)
}
