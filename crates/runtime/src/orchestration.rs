//! Trait facade over [`crate::Orchestrator`].
//!
//! Consumers — `mcp-server`, `web-ui`, and the messenger adapters — depend
//! on this trait so test code can substitute
//! [`StubOrchestrationEngine`] without booting the full ReAct loop.
//!
//! The trait surface is intentionally narrow: only `submit_turn` and
//! `cancel_turn` are crossed between crates. Per-interface streaming
//! (`run_turn_streaming`) and tool/persona configuration stay on the
//! concrete `Orchestrator` because they're internal to the runtime.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use assistant_core::auth::AuthContext;
use assistant_core::types::conversation::Interface;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::Notify;
use uuid::Uuid;

use crate::orchestrator::{CancelOutcome, SubmitTurn, TURN_CANCELLED_MARKER, TurnResult};

/// Trait-based interface for submitting and cancelling assistant turns.
#[async_trait]
pub trait OrchestrationEngine: Send + Sync {
    /// Submit a single prompt and return the assistant's final response.
    async fn submit_turn(
        &self,
        auth: &AuthContext,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult>;

    /// Submit a turn under a caller-chosen `request_id`, so the caller can
    /// pass the same id to [`cancel_turn`](Self::cancel_turn) while the turn
    /// runs.
    async fn submit_turn_with_request_id(
        &self,
        auth: &AuthContext,
        request_id: Uuid,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult>;

    /// Request cancellation of an in-flight turn by request id.
    async fn cancel_turn(&self, request_id: Uuid) -> CancelOutcome;
}

#[async_trait]
impl OrchestrationEngine for crate::Orchestrator {
    async fn submit_turn(
        &self,
        auth: &AuthContext,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult> {
        self.submit(
            SubmitTurn::new(prompt, conversation_id, interface)
                .with_auth(auth)
                .with_timestamp(timestamp),
        )
        .await
    }

    async fn submit_turn_with_request_id(
        &self,
        auth: &AuthContext,
        request_id: Uuid,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult> {
        self.submit(
            SubmitTurn::new(prompt, conversation_id, interface)
                .with_auth(auth)
                .with_request_id(request_id)
                .with_timestamp(timestamp),
        )
        .await
    }

    async fn cancel_turn(&self, request_id: Uuid) -> CancelOutcome {
        crate::Orchestrator::cancel_turn(self, request_id).await
    }
}

/// A single recorded call to [`OrchestrationEngine::submit_turn`].
#[derive(Debug, Clone)]
pub struct RecordedSubmitTurn {
    pub auth: AuthContext,
    /// The caller-chosen request id, when the turn came through
    /// [`OrchestrationEngine::submit_turn_with_request_id`].
    pub request_id: Option<Uuid>,
    pub prompt: String,
    pub conversation_id: Uuid,
    pub interface: Interface,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Default)]
struct StubData {
    responses: Vec<Result<TurnResult, anyhow::Error>>,
    submits: Vec<RecordedSubmitTurn>,
    cancel_outcomes: Vec<CancelOutcome>,
    cancel_calls: Vec<Uuid>,
    hold_turns: bool,
}

/// In-memory [`OrchestrationEngine`] stub for tests.
///
/// Queue turn responses with [`queue_turn`](Self::queue_turn). Empty queue
/// returns a benign `TurnResult` containing `(stub: no more responses)`.
pub struct StubOrchestrationEngine {
    state: Arc<Mutex<StubData>>,
    cancelled: Arc<Notify>,
}

impl StubOrchestrationEngine {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(StubData::default())),
            cancelled: Arc::new(Notify::new()),
        }
    }

    /// Make turns wait until [`cancel_turn`](OrchestrationEngine::cancel_turn)
    /// is called, then fail with [`TURN_CANCELLED_MARKER`], like a real
    /// in-flight turn that is cancelled.
    pub fn hold_turns(self) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.hold_turns = true;
        }
        self
    }

    /// Queue a turn response.
    pub fn queue_turn(self, result: Result<TurnResult, anyhow::Error>) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.responses.push(result);
        }
        self
    }

    /// Queue a `cancel_turn` outcome. Subsequent `cancel_turn` calls
    /// consume the queue in FIFO order; empty queue returns
    /// `CancelOutcome::NotFound`.
    pub fn queue_cancel_outcome(self, outcome: CancelOutcome) -> Self {
        if let Ok(mut s) = self.state.lock() {
            s.cancel_outcomes.push(outcome);
        }
        self
    }

    /// Snapshot of every `submit_turn` call observed.
    pub fn submit_calls(&self) -> Vec<RecordedSubmitTurn> {
        match self.state.lock() {
            Ok(g) => g.submits.clone(),
            Err(p) => p.into_inner().submits.clone(),
        }
    }

    /// Snapshot of every `cancel_turn` request_id observed.
    pub fn cancel_calls(&self) -> Vec<Uuid> {
        match self.state.lock() {
            Ok(g) => g.cancel_calls.clone(),
            Err(p) => p.into_inner().cancel_calls.clone(),
        }
    }
}

impl Default for StubOrchestrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl StubOrchestrationEngine {
    async fn record_and_answer(
        &self,
        auth: &AuthContext,
        request_id: Option<Uuid>,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult> {
        let hold = {
            let mut state = match self.state.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            state.submits.push(RecordedSubmitTurn {
                auth: auth.clone(),
                request_id,
                prompt: prompt.to_string(),
                conversation_id,
                interface,
                timestamp,
            });
            state.hold_turns
        };
        if hold {
            self.cancelled.notified().await;
            return Err(anyhow::anyhow!(TURN_CANCELLED_MARKER));
        }
        let mut state = match self.state.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if state.responses.is_empty() {
            return Ok(TurnResult {
                answer: "(stub: no more responses)".to_string(),
                attachments: Vec::new(),
                attachment_ids: Vec::new(),
                message_id: None,
                had_errors: false,
                timing: None,
            });
        }
        state.responses.remove(0)
    }
}

#[async_trait]
impl OrchestrationEngine for StubOrchestrationEngine {
    async fn submit_turn(
        &self,
        auth: &AuthContext,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult> {
        self.record_and_answer(auth, None, prompt, conversation_id, interface, timestamp)
            .await
    }

    async fn submit_turn_with_request_id(
        &self,
        auth: &AuthContext,
        request_id: Uuid,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<TurnResult> {
        self.record_and_answer(
            auth,
            Some(request_id),
            prompt,
            conversation_id,
            interface,
            timestamp,
        )
        .await
    }

    async fn cancel_turn(&self, request_id: Uuid) -> CancelOutcome {
        let outcome = {
            let mut state = match self.state.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            state.cancel_calls.push(request_id);
            if state.cancel_outcomes.is_empty() {
                CancelOutcome::NotFound
            } else {
                state.cancel_outcomes.remove(0)
            }
        };
        self.cancelled.notify_one();
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_records_and_consumes_responses() {
        let stub = StubOrchestrationEngine::new().queue_turn(Ok(TurnResult {
            answer: "hello".into(),
            attachments: Vec::new(),
            attachment_ids: Vec::new(),
            message_id: None,
            had_errors: false,
            timing: None,
        }));
        let r = stub
            .submit_turn(
                &AuthContext::system(),
                "hi",
                Uuid::new_v4(),
                Interface::Mcp,
                None,
            )
            .await
            .unwrap();
        assert_eq!(r.answer, "hello");
        let calls = stub.submit_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].prompt, "hi");
        assert!(matches!(calls[0].interface, Interface::Mcp));
    }

    #[tokio::test]
    async fn stub_empty_queue_returns_benign_result() {
        let stub = StubOrchestrationEngine::new();
        let r = stub
            .submit_turn(
                &AuthContext::system(),
                "hi",
                Uuid::new_v4(),
                Interface::Cli,
                None,
            )
            .await
            .unwrap();
        assert!(r.answer.contains("stub"));
    }

    #[tokio::test]
    async fn cancel_records_and_returns_queued() {
        let stub = StubOrchestrationEngine::new().queue_cancel_outcome(CancelOutcome::Cancelled);
        let req_id = Uuid::new_v4();
        let outcome = stub.cancel_turn(req_id).await;
        assert!(matches!(outcome, CancelOutcome::Cancelled));
        assert_eq!(stub.cancel_calls(), vec![req_id]);
    }

    #[tokio::test]
    async fn cancel_empty_queue_returns_not_found() {
        let stub = StubOrchestrationEngine::new();
        let outcome = stub.cancel_turn(Uuid::new_v4()).await;
        assert!(matches!(outcome, CancelOutcome::NotFound));
    }

    #[tokio::test]
    async fn submit_with_request_id_records_the_id() {
        let stub = StubOrchestrationEngine::new();
        let request_id = Uuid::new_v4();
        stub.submit_turn_with_request_id(
            &AuthContext::system(),
            request_id,
            "hi",
            Uuid::new_v4(),
            Interface::Mcp,
            None,
        )
        .await
        .unwrap();
        stub.submit_turn(
            &AuthContext::system(),
            "again",
            Uuid::new_v4(),
            Interface::Mcp,
            None,
        )
        .await
        .unwrap();
        let ids: Vec<_> = stub.submit_calls().iter().map(|c| c.request_id).collect();
        assert_eq!(ids, vec![Some(request_id), None]);
    }

    #[tokio::test]
    async fn held_turn_fails_as_cancelled_once_cancel_turn_is_called() {
        let stub = Arc::new(StubOrchestrationEngine::new().hold_turns());
        let request_id = Uuid::new_v4();
        let turn = tokio::spawn({
            let stub = stub.clone();
            async move {
                stub.submit_turn_with_request_id(
                    &AuthContext::system(),
                    request_id,
                    "hi",
                    Uuid::new_v4(),
                    Interface::Mcp,
                    None,
                )
                .await
            }
        });
        while stub.submit_calls().is_empty() {
            tokio::task::yield_now().await;
        }
        stub.cancel_turn(request_id).await;
        let error = turn.await.unwrap().err().expect("cancelled turn fails");
        assert!(error.to_string().contains(TURN_CANCELLED_MARKER), "{error}");
    }
}
