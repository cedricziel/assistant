//! `AssistantInterface` trait — decouples consumers from the concrete
//! [`Orchestrator`].
//!
//! Implement this trait in test modules via a `MockAssistantInterface` to unit-
//! test API handlers without a full orchestrator + SQLite + LLM stack.
//!
//! # Blanket implementation
//!
//! [`Orchestrator`] implements this trait, so `Arc<Orchestrator>` can be
//! coerced to `Arc<dyn AssistantInterface>` at construction sites.

use anyhow::Result;
use assistant_core::types::conversation::Interface;
use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::orchestrator::{Orchestrator, SubmitTurn, TurnResult};

/// Minimal interface required to drive a conversation turn.
///
/// Consumers (web-ui handlers, integration tests) depend on this trait rather
/// than the concrete [`Orchestrator`], enabling lightweight mock implementations.
#[async_trait]
pub trait AssistantInterface: Send + Sync {
    /// Register a streaming event sink for the given conversation.
    ///
    /// [`OrchestratorEvent`]s emitted during the turn are forwarded to `sink`
    /// as they are produced.  Call this *before*
    /// [`submit_turn`](Self::submit_turn).
    async fn register_token_sink(
        &self,
        conversation_id: Uuid,
        sink: mpsc::Sender<crate::orchestrator::OrchestratorEvent>,
    );

    /// Submit a turn and wait for the assistant's reply.
    ///
    /// Set [`SubmitTurn::request_id`] to let the SSE `run_id` double as the
    /// orchestrator's request id, enabling external cancellation by `run_id`.
    async fn submit(&self, turn: SubmitTurn) -> Result<TurnResult>;

    /// Cancel an in-flight turn by `request_id`. Returns whether a token was
    /// found and triggered.
    ///
    /// Default implementation is a no-op returning `false` — sufficient for
    /// mocks that don't exercise cancellation.
    async fn cancel_turn(&self, _request_id: Uuid) -> bool {
        false
    }

    /// Run the BOOT.md startup hook for the given conversation.
    ///
    /// Returns `Ok(true)` if a boot turn was executed, `Ok(false)` if skipped
    /// (no BOOT.md or the file is empty).
    async fn run_boot(&self, conversation_id: Uuid, interface: Interface) -> Result<bool>;
}

#[async_trait]
impl AssistantInterface for Orchestrator {
    async fn register_token_sink(
        &self,
        conversation_id: Uuid,
        sink: mpsc::Sender<crate::orchestrator::OrchestratorEvent>,
    ) {
        self.register_token_sink(conversation_id, sink).await;
    }

    async fn submit(&self, turn: SubmitTurn) -> Result<TurnResult> {
        self.submit(turn).await
    }

    async fn cancel_turn(&self, request_id: Uuid) -> bool {
        matches!(
            self.cancel_turn(request_id).await,
            crate::orchestrator::CancelOutcome::Cancelled
        )
    }

    async fn run_boot(&self, conversation_id: Uuid, interface: Interface) -> Result<bool> {
        self.run_boot(conversation_id, interface).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Minimal mock that only implements the two required trait methods
    /// (`submit` + `run_boot`). The other trait
    /// methods fall back to the trait's default implementations, which is
    /// what these tests exercise.
    struct MinimalMock {
        recorded_prompt: tokio::sync::Mutex<Option<String>>,
    }

    #[async_trait]
    impl AssistantInterface for MinimalMock {
        async fn register_token_sink(
            &self,
            _conversation_id: Uuid,
            _sink: mpsc::Sender<crate::orchestrator::OrchestratorEvent>,
        ) {
        }

        async fn submit(&self, turn: SubmitTurn) -> Result<TurnResult> {
            *self.recorded_prompt.lock().await = Some(turn.prompt.clone());
            Ok(TurnResult {
                answer: format!("echo: {}", turn.prompt),
                attachments: vec![],
                attachment_ids: vec![],
                message_id: None,
                had_errors: false,
                timing: None,
            })
        }

        async fn run_boot(&self, _conversation_id: Uuid, _interface: Interface) -> Result<bool> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn submit_reaches_the_implementation() {
        let mock = Arc::new(MinimalMock {
            recorded_prompt: tokio::sync::Mutex::new(None),
        });
        let iface: Arc<dyn AssistantInterface> = mock.clone();
        let result = iface
            .submit(SubmitTurn::new("hello", Uuid::new_v4(), Interface::Cli))
            .await
            .unwrap();
        assert_eq!(result.answer, "echo: hello");
        assert_eq!(mock.recorded_prompt.lock().await.as_deref(), Some("hello"));
    }

    #[tokio::test]
    async fn default_cancel_turn_returns_false() {
        let mock = MinimalMock {
            recorded_prompt: tokio::sync::Mutex::new(None),
        };
        let iface: Arc<dyn AssistantInterface> = Arc::new(mock);
        let cancelled = iface.cancel_turn(Uuid::new_v4()).await;
        assert!(!cancelled);
    }
}
