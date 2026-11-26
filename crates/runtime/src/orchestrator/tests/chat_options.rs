//! The agent loop passes per-call chat options to the provider.

use std::sync::{Arc, Mutex};

use assistant_core::types::conversation::{Interface, TurnIdentity};
use assistant_core::{
    Capabilities, ChatHistoryMessage, ChatOptions, LlmProvider, LlmResponse, LlmResponseMeta,
    StreamChunk, ToolSpec, ToolSupport,
};
use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::*;

/// Answers every call and records the options `chat_with` received.
#[derive(Default)]
struct OptionsRecorder {
    seen: Mutex<Vec<ChatOptions>>,
}

#[async_trait]
impl LlmProvider for OptionsRecorder {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            vision: false,
            hosted_tools: vec![],
            context_window_tokens: None,
        }
    }

    async fn chat(
        &self,
        _: &str,
        _: &[ChatHistoryMessage],
        _: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        Ok(LlmResponse::FinalAnswer(
            "plain".into(),
            LlmResponseMeta::default(),
        ))
    }

    async fn chat_streaming(
        &self,
        _: &str,
        _: &[ChatHistoryMessage],
        _: &[ToolSpec],
        _: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        Ok(LlmResponse::FinalAnswer(
            "plain".into(),
            LlmResponseMeta::default(),
        ))
    }

    async fn chat_with(
        &self,
        _: &str,
        _: &[ChatHistoryMessage],
        _: &[ToolSpec],
        options: &ChatOptions,
        _: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        self.seen.lock().unwrap().push(options.clone());
        Ok(LlmResponse::FinalAnswer(
            "done".into(),
            LlmResponseMeta::default(),
        ))
    }
}

async fn first_turn_options(parallel_tool_calls: Option<bool>, streaming: bool) -> ChatOptions {
    let provider = Arc::new(OptionsRecorder::default());
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    config.llm.parallel_tool_calls = parallel_tool_calls;
    allow_all_tools(&mut config);
    let (orch, _) = build_with_llm(provider.clone(), config, |o| o).await;

    if streaming {
        let (tx, _rx) = mpsc::channel(64);
        orch.run_turn_streaming(
            "hi",
            Uuid::new_v4(),
            Interface::Web,
            tx,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();
    } else {
        orch.run_turn(
            "hi",
            Uuid::new_v4(),
            Interface::Web,
            None,
            vec![],
            TurnIdentity::default(),
        )
        .await
        .unwrap();
    }
    let seen = provider.seen.lock().unwrap();
    seen.first()
        .cloned()
        .expect("the agent loop called chat_with")
}

#[tokio::test]
async fn agent_loop_passes_parallel_tool_calls() {
    assert_eq!(
        first_turn_options(Some(false), false)
            .await
            .parallel_tool_calls,
        Some(false)
    );
    assert_eq!(
        first_turn_options(Some(false), true)
            .await
            .parallel_tool_calls,
        Some(false)
    );
}

#[tokio::test]
async fn unset_parallel_tool_calls_keeps_defaults() {
    assert_eq!(
        first_turn_options(None, false).await,
        ChatOptions::default()
    );
}
