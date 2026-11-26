//! `acquire_exclusive_for_branch`: branching while a turn waits for the user
//! cancels that turn; branching while a turn works is refused (D7a).

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use assistant_core::types::agent::OrgConfig;
use assistant_storage::{
    InMemoryInteractionStore, InteractionStatus, InteractionStore, NewInteraction,
};

use crate::orchestrator::{BRANCH_CANCEL_WAIT, BranchConflict, Orchestrator, TurnLease, TurnMode};

use super::build_customized;

async fn orchestrator(store: Arc<InMemoryInteractionStore>) -> Arc<Orchestrator> {
    let mut config = OrgConfig::default();
    config.memory.enabled = false;
    let (orch, _) = build_customized("http://127.0.0.1:1", config, move |o| {
        o.with_interaction_store(store)
    })
    .await;
    orch
}

async fn pending(store: &InMemoryInteractionStore, conv: Uuid, run: Uuid, kind: &str) -> String {
    let row = store
        .create(NewInteraction {
            id: Uuid::new_v4().to_string(),
            conversation_id: conv.to_string(),
            run_id: run.to_string(),
            user_id: "alice".into(),
            kind: kind.into(),
            tool_call_id: None,
            request: json!({}),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        })
        .await
        .unwrap();
    row.id
}

/// A turn that holds a shared lease until `Orchestrator::cancel_turn` fires
/// its token, as a real run does.
struct FakeRun {
    request_id: Uuid,
    token: CancellationToken,
}

async fn start_run(orch: &Arc<Orchestrator>, conv: Uuid, release_on_cancel: bool) -> FakeRun {
    let request_id = Uuid::new_v4();
    let token = CancellationToken::new();
    let lease: TurnLease = orch
        .turn_registry()
        .try_begin(conv, request_id, TurnMode::Shared)
        .unwrap();
    orch.turn_cancellations
        .write()
        .await
        .register(request_id, token.clone());
    let waiter = token.clone();
    tokio::spawn(async move {
        if release_on_cancel {
            waiter.cancelled().await;
        } else {
            std::future::pending::<()>().await;
        }
        drop(lease);
    });
    FakeRun { request_id, token }
}

#[tokio::test]
async fn a_free_conversation_gets_the_lease_at_once() {
    let orch = orchestrator(Arc::new(InMemoryInteractionStore::new())).await;
    let conv = Uuid::new_v4();
    let lease = orch
        .acquire_exclusive_for_branch(conv, Uuid::new_v4())
        .await;
    assert!(lease.is_ok(), "no lease held, so the branch may proceed");
}

#[tokio::test]
async fn a_run_waiting_for_an_approval_is_cancelled_and_the_lease_granted() {
    let store = Arc::new(InMemoryInteractionStore::new());
    let orch = orchestrator(store.clone()).await;
    let conv = Uuid::new_v4();
    let run = start_run(&orch, conv, true).await;
    pending(&store, conv, run.request_id, "tool_approval").await;

    let lease = orch
        .acquire_exclusive_for_branch(conv, Uuid::new_v4())
        .await;

    assert!(lease.is_ok(), "the waiting run was cancelled and released");
    assert!(run.token.is_cancelled(), "cancelled through cancel_turn");
}

#[tokio::test]
async fn the_cancelled_runs_interactions_resolve_as_cancelled() {
    let store = Arc::new(InMemoryInteractionStore::new());
    let orch = orchestrator(store.clone()).await;
    let conv = Uuid::new_v4();
    let run = start_run(&orch, conv, true).await;
    let id = pending(&store, conv, run.request_id, "tool_approval").await;

    let _lease = orch
        .acquire_exclusive_for_branch(conv, Uuid::new_v4())
        .await
        .unwrap();

    let row = store.get(&id).await.unwrap().unwrap();
    assert_eq!(row.status, InteractionStatus::Cancelled);
}

#[tokio::test]
async fn a_pending_question_or_surface_wait_counts_like_an_approval() {
    for kind in ["question", "surface_wait"] {
        let store = Arc::new(InMemoryInteractionStore::new());
        let orch = orchestrator(store.clone()).await;
        let conv = Uuid::new_v4();
        let run = start_run(&orch, conv, true).await;
        pending(&store, conv, run.request_id, kind).await;

        let lease = orch
            .acquire_exclusive_for_branch(conv, Uuid::new_v4())
            .await;

        assert!(lease.is_ok(), "{kind} is a wait for the user");
    }
}

#[tokio::test]
async fn a_working_run_is_a_conflict_and_nothing_is_cancelled() {
    let store = Arc::new(InMemoryInteractionStore::new());
    let orch = orchestrator(store.clone()).await;
    let conv = Uuid::new_v4();
    let waiting = start_run(&orch, conv, true).await;
    let working = start_run(&orch, conv, true).await;
    let id = pending(&store, conv, waiting.request_id, "tool_approval").await;

    let result = orch
        .acquire_exclusive_for_branch(conv, Uuid::new_v4())
        .await;

    assert_eq!(result.unwrap_err(), BranchConflict);
    assert!(
        !waiting.token.is_cancelled(),
        "the waiting run is untouched"
    );
    assert!(
        !working.token.is_cancelled(),
        "the working run is untouched"
    );
    let row = store.get(&id).await.unwrap().unwrap();
    assert_eq!(row.status, InteractionStatus::Pending);
}

#[tokio::test]
async fn a_run_with_no_interaction_at_all_is_a_conflict() {
    let orch = orchestrator(Arc::new(InMemoryInteractionStore::new())).await;
    let conv = Uuid::new_v4();
    let run = start_run(&orch, conv, true).await;

    let result = orch
        .acquire_exclusive_for_branch(conv, Uuid::new_v4())
        .await;

    assert_eq!(result.unwrap_err(), BranchConflict);
    assert!(!run.token.is_cancelled());
}

#[tokio::test]
async fn an_interaction_of_another_conversation_does_not_count() {
    let store = Arc::new(InMemoryInteractionStore::new());
    let orch = orchestrator(store.clone()).await;
    let conv = Uuid::new_v4();
    let run = start_run(&orch, conv, true).await;
    pending(&store, Uuid::new_v4(), run.request_id, "tool_approval").await;

    let result = orch
        .acquire_exclusive_for_branch(conv, Uuid::new_v4())
        .await;

    assert_eq!(result.unwrap_err(), BranchConflict);
}

#[tokio::test]
async fn a_run_that_keeps_its_lease_past_the_ceiling_is_a_conflict() {
    let store = Arc::new(InMemoryInteractionStore::new());
    let orch = orchestrator(store.clone()).await;
    let conv = Uuid::new_v4();
    let run = start_run(&orch, conv, false).await;
    pending(&store, conv, run.request_id, "tool_approval").await;
    // Pause only now: the database pool needs real time to open.
    tokio::time::pause();

    let started = tokio::time::Instant::now();
    let result = orch
        .acquire_exclusive_for_branch(conv, Uuid::new_v4())
        .await;

    assert_eq!(result.unwrap_err(), BranchConflict);
    assert!(run.token.is_cancelled(), "the cancel was still sent");
    let waited = started.elapsed();
    assert!(
        waited >= BRANCH_CANCEL_WAIT && waited < BRANCH_CANCEL_WAIT + Duration::from_secs(1),
        "waited {waited:?}, expected about {BRANCH_CANCEL_WAIT:?}"
    );
}

// -- a real turn, a real broker --

mod blocked_turn {
    use async_trait::async_trait;

    use assistant_core::auth::AuthContext;
    use assistant_core::types::conversation::Interface;
    use assistant_core::{
        LlmProvider, LlmResponse, LlmResponseMeta, ToolCallItem, ToolCallResponse,
    };
    use assistant_llm_provider::scripted::ScriptedLlmProvider;

    use crate::interaction::{
        InteractionBroker, InteractionKinds, InteractionOutcome, InteractionRequest,
    };
    use crate::orchestrator::{
        ConfirmationAnswer, ConfirmationCallback, ConfirmationRequest, SubmitTurn,
        TURN_CANCELLED_MARKER,
    };

    use super::super::build_with_llm;
    use super::*;

    /// Approvals go through the broker, as a web turn's do.
    struct BrokerApproval(Arc<InteractionBroker>);

    #[async_trait]
    impl ConfirmationCallback for BrokerApproval {
        async fn confirm(&self, req: ConfirmationRequest<'_>) -> ConfirmationAnswer {
            let Some(run_id) = req.ctx.run_id else {
                return ConfirmationAnswer::RejectOnce;
            };
            let asked = self
                .0
                .request(
                    InteractionRequest {
                        conversation_id: req.ctx.conversation_id.to_string(),
                        run_id: run_id.to_string(),
                        user_id: "alice".into(),
                        kind: "tool_approval".into(),
                        tool_call_id: req.tool_call_id.map(str::to_string),
                        request: json!({
                            "tool_name": req.tool_name, "arguments": req.params,
                            "arguments_truncated": false, "reason": req.reason,
                            "options": ["allow_once", "reject_once"],
                        }),
                        turn_deadline: None,
                    },
                    None,
                )
                .await;
            match asked {
                Ok((_, InteractionOutcome::Resolved { .. })) => ConfirmationAnswer::AllowOnce,
                Ok((_, InteractionOutcome::Cancelled)) => ConfirmationAnswer::Cancelled,
                Ok((_, InteractionOutcome::Expired)) | Err(_) => ConfirmationAnswer::Expired,
            }
        }
    }

    #[tokio::test]
    async fn branching_cancels_a_turn_blocked_on_a_bash_approval() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("marker");
        let scripted = Arc::new(ScriptedLlmProvider::new().with_canned_responses(vec![
            LlmResponse::ToolCalls(ToolCallResponse {
                items: vec![ToolCallItem {
                    name: "bash".into(),
                    params: json!({ "command": format!("touch {}", marker.display()) }),
                    id: Some("call_1".into()),
                }],
                meta: LlmResponseMeta::default(),
                thinking: None,
                provider_state: None,
            }),
            LlmResponse::FinalAnswer("done".into(), LlmResponseMeta::default()),
        ]));
        let llm: Arc<dyn LlmProvider> = scripted.clone();
        let store = Arc::new(InMemoryInteractionStore::new());
        let broker = Arc::new(InteractionBroker::new(
            store.clone(),
            Arc::new(InteractionKinds::with_builtin()),
        ));
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        let (orch, _) = build_with_llm(llm, config, {
            let store = store.clone();
            move |o| {
                o.with_interaction_store(store)
                    .with_confirmation_callback(Arc::new(BrokerApproval(broker)))
            }
        })
        .await;
        let worker = orch.clone();
        tokio::spawn(async move { worker.run_worker("test-worker").await });

        let conv = Uuid::new_v4();
        let run = Uuid::new_v4();
        let turn = {
            let orch = orch.clone();
            tokio::spawn(async move {
                orch.submit(
                    SubmitTurn::new("list files", conv, Interface::Web)
                        .with_auth(&AuthContext::system())
                        .with_request_id(run)
                        .with_interactive(true),
                )
                .await
            })
        };
        let pending_id = loop {
            let rows = store
                .list_for_conversation(&conv.to_string(), Some(InteractionStatus::Pending))
                .await
                .unwrap();
            if let Some(row) = rows.first() {
                break row.id.clone();
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        };

        let lease = orch
            .acquire_exclusive_for_branch(conv, Uuid::new_v4())
            .await;

        assert!(lease.is_ok(), "the blocked turn was cancelled");
        let row = store.get(&pending_id).await.unwrap().unwrap();
        assert_eq!(row.status, InteractionStatus::Cancelled);
        assert!(!marker.exists(), "the gated bash call never ran");
        let ended = tokio::time::timeout(Duration::from_secs(10), turn)
            .await
            .expect("the cancelled turn ends")
            .unwrap();
        let Err(err) = ended else {
            panic!("a cancelled turn is an error");
        };
        assert!(err.to_string().contains(TURN_CANCELLED_MARKER), "{err}");
        assert_eq!(scripted.call_count(), 1, "the model was not asked again");
    }
}
