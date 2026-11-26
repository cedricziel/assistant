//! Turns expose [`LlmCallContext`] to the provider, and compaction follows the
//! turn provider's current context window.

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use assistant_core::identity::{OrgId, UserId};
use assistant_core::llm::call_context::{LlmCallContext, user_hash};
use assistant_core::types::conversation::{Interface, TurnIdentity};
use assistant_core::types::features::CompactionConfig;
use assistant_core::{
    Capabilities, ChatHistoryMessage, LlmProvider, LlmResponse, LlmResponseMeta, StreamChunk,
    ToolSpec, ToolSupport,
};
use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::*;

/// Answers every call and records the call context it ran under.
#[derive(Default)]
struct RecordingProvider {
    seen: Mutex<Vec<Option<LlmCallContext>>>,
    /// 0 means "unknown".
    context_window: AtomicU64,
}

impl RecordingProvider {
    fn answer(&self) -> LlmResponse {
        self.seen.lock().unwrap().push(LlmCallContext::current());
        LlmResponse::FinalAnswer("done".into(), LlmResponseMeta::default())
    }
}

#[async_trait]
impl LlmProvider for RecordingProvider {
    fn capabilities(&self) -> Capabilities {
        let window = self.context_window.load(Ordering::SeqCst);
        Capabilities {
            tools: ToolSupport::Native,
            streaming: true,
            vision: false,
            hosted_tools: vec![],
            context_window_tokens: (window > 0).then_some(window),
        }
    }

    async fn chat(
        &self,
        _system_prompt: &str,
        _history: &[ChatHistoryMessage],
        _tools: &[ToolSpec],
    ) -> anyhow::Result<LlmResponse> {
        Ok(self.answer())
    }

    async fn chat_streaming(
        &self,
        _system_prompt: &str,
        _history: &[ChatHistoryMessage],
        _tools: &[ToolSpec],
        _chunk_sink: Option<mpsc::Sender<StreamChunk>>,
    ) -> anyhow::Result<LlmResponse> {
        Ok(self.answer())
    }
}

fn quiet_config() -> OrgConfig {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    allow_all_tools(&mut config);
    config
}

#[tokio::test]
async fn turn_llm_calls_see_the_conversation_and_user() {
    let provider = Arc::new(RecordingProvider::default());
    let (orch, _) = build_with_llm(provider.clone(), quiet_config(), |o| o).await;
    let conversation_id = Uuid::new_v4();
    let identity = TurnIdentity {
        user_id: Some(UserId::from("usr_abc")),
        org_id: Some(OrgId("org_1".into())),
        space_id: None,
        ..TurnIdentity::default()
    };

    orch.run_turn_with_tools(
        "hi",
        conversation_id,
        Interface::Web,
        vec![],
        None,
        vec![],
        vec![],
        identity,
    )
    .await
    .unwrap();

    let expected = LlmCallContext {
        conversation_id,
        user_hash: Some(user_hash(
            Some(&OrgId("org_1".into())),
            &UserId::from("usr_abc"),
        )),
    };
    let seen = provider.seen.lock().unwrap().clone();
    assert_eq!(
        seen.first(),
        Some(&Some(expected)),
        "the turn's first LLM call runs inside its context: {seen:?}"
    );
}

#[tokio::test]
async fn calls_outside_a_turn_have_no_context() {
    let provider = RecordingProvider::default();
    provider.chat("", &[], &[]).await.unwrap();
    assert_eq!(provider.seen.lock().unwrap().as_slice(), &[None]);
}

#[test]
fn compaction_window_is_the_smaller_of_config_and_provider() {
    let config = CompactionConfig {
        context_window_tokens: 100_000,
        ..CompactionConfig::default()
    };
    let provider = RecordingProvider::default();

    let unknown = Orchestrator::compaction_for(&config, &provider);
    assert_eq!(
        unknown.context_window_tokens, 100_000,
        "unknown window keeps config"
    );

    provider.context_window.store(32_000, Ordering::SeqCst);
    assert_eq!(
        Orchestrator::compaction_for(&config, &provider).context_window_tokens,
        32_000,
        "a window learned after startup applies on the next read"
    );

    provider.context_window.store(500_000, Ordering::SeqCst);
    assert_eq!(
        Orchestrator::compaction_for(&config, &provider).context_window_tokens,
        100_000,
        "a larger provider window never raises the configured limit"
    );
}
