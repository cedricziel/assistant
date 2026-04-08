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
use assistant_core::Interface;
use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::orchestrator::{Orchestrator, TurnResult};

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

    /// Submit a user turn and wait for the assistant's reply.
    async fn submit_turn(
        &self,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<TurnResult>;

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

    async fn submit_turn(
        &self,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<TurnResult> {
        self.submit_turn(prompt, conversation_id, interface, timestamp)
            .await
    }

    async fn run_boot(&self, conversation_id: Uuid, interface: Interface) -> Result<bool> {
        self.run_boot(conversation_id, interface).await
    }
}
