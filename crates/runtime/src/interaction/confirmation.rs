//! Approvals answered through interactions (ADR-0017, D3).
//!
//! [`InteractiveConfirmation`] is the [`ConfirmationCallback`] of a web turn:
//! it puts the tool call to the turn's owner as a `tool_approval` interaction
//! and maps the owner's answer back. [`RoutedConfirmation`] keeps a REPL's own
//! stdin prompt for `Interface::Cli` turns.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tracing::{info, warn};

use assistant_core::types::conversation::Interface;

use crate::interaction::{
    InteractionBroker, InteractionOutcome, InteractionRequest, cap_arguments,
};
use crate::orchestrator::{
    ApprovalOptions, ConfirmationAnswer, ConfirmationCallback, ConfirmationRequest,
};

/// The `kind` of the interactions this callback creates.
const KIND: &str = "tool_approval";

/// Asks the turn's owner through the [`InteractionBroker`].
///
/// "Always" answers save their rule inside the broker's `respond`, so the
/// resolution this waits for already carries the rule id.
pub struct InteractiveConfirmation {
    broker: Arc<InteractionBroker>,
}

impl InteractiveConfirmation {
    pub fn new(broker: Arc<InteractionBroker>) -> Self {
        Self { broker }
    }
}

/// The decisions an [`ApprovalOptions`] offers, in display order.
fn offered_decisions(options: ApprovalOptions) -> Vec<&'static str> {
    let mut decisions = vec!["allow_once"];
    if options.allow_always {
        decisions.push("allow_always");
    }
    decisions.push("reject_once");
    if options.reject_always {
        decisions.push("reject_always");
    }
    decisions
}

fn approval_request(req: &ConfirmationRequest<'_>) -> Value {
    let (arguments, truncated) = cap_arguments(req.params);
    let mut request = json!({
        "tool_name": req.tool_name,
        "arguments": arguments,
        "arguments_truncated": truncated,
        "reason": req.reason,
        "options": offered_decisions(req.options),
    });
    if let Some(rule) = req.suggested_rule {
        request["suggested_rule"] = json!(rule);
    }
    request
}

fn answer_for(decision: &str) -> ConfirmationAnswer {
    match decision {
        "allow_once" => ConfirmationAnswer::AllowOnce,
        "allow_always" => ConfirmationAnswer::AllowAlways,
        "reject_always" => ConfirmationAnswer::RejectAlways,
        _ => ConfirmationAnswer::RejectOnce,
    }
}

#[async_trait]
impl ConfirmationCallback for InteractiveConfirmation {
    async fn confirm(&self, req: ConfirmationRequest<'_>) -> ConfirmationAnswer {
        let (Some(user), Some(run_id)) = (&req.ctx.user_id, req.ctx.run_id) else {
            warn!(
                tool = req.tool_name,
                "an approval needs a turn owner and a run; denying"
            );
            return ConfirmationAnswer::Cancelled;
        };
        let asked = self
            .broker
            .request(
                InteractionRequest {
                    conversation_id: req.ctx.conversation_id.to_string(),
                    run_id: run_id.to_string(),
                    user_id: user.0.clone(),
                    kind: KIND.into(),
                    tool_call_id: req.tool_call_id.map(str::to_owned),
                    request: approval_request(&req),
                    turn_deadline: None,
                },
                req.event_sink,
            )
            .await;
        match asked {
            Ok((interaction_id, InteractionOutcome::Resolved { response, rule_id })) => {
                let decision = response["decision"].as_str().unwrap_or("reject_once");
                info!(
                    target: "assistant::permission",
                    user = %user.0,
                    tool = req.tool_name,
                    interaction = %interaction_id,
                    decision,
                    rule_id = rule_id.as_deref().unwrap_or(""),
                    "tool approval answered",
                );
                answer_for(decision)
            }
            Ok((_, InteractionOutcome::Expired)) => ConfirmationAnswer::Expired,
            Ok((_, InteractionOutcome::Cancelled)) => ConfirmationAnswer::Cancelled,
            Err(e) => {
                warn!(tool = req.tool_name, error = %e, "approval request failed; denying");
                ConfirmationAnswer::Cancelled
            }
        }
    }
}

/// Sends `Interface::Cli` turns to `cli` and every other turn to `other`.
///
/// A REPL process serves its own terminal and, with interactions on, web turns
/// too. Only the terminal can answer on stdin.
pub struct RoutedConfirmation {
    cli: Arc<dyn ConfirmationCallback>,
    other: Arc<dyn ConfirmationCallback>,
}

impl RoutedConfirmation {
    pub fn new(cli: Arc<dyn ConfirmationCallback>, other: Arc<dyn ConfirmationCallback>) -> Self {
        Self { cli, other }
    }
}

#[async_trait]
impl ConfirmationCallback for RoutedConfirmation {
    async fn confirm(&self, req: ConfirmationRequest<'_>) -> ConfirmationAnswer {
        if matches!(req.ctx.interface, Interface::Cli) {
            self.cli.confirm(req).await
        } else {
            self.other.confirm(req).await
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use uuid::Uuid;

    use assistant_core::identity::UserId;
    use assistant_core::types::conversation::ExecutionContext;
    use assistant_storage::{
        InMemoryInteractionStore, InMemoryToolPermissionRuleStore, InteractionStatus,
        InteractionStore, ToolPermissionRuleStore,
    };

    use crate::interaction::{InteractionKinds, Responder};

    use super::*;

    struct Fixture {
        confirmation: Arc<InteractiveConfirmation>,
        broker: Arc<InteractionBroker>,
        store: Arc<InMemoryInteractionStore>,
        rules: Arc<InMemoryToolPermissionRuleStore>,
        ctx: ExecutionContext,
    }

    fn fixture() -> Fixture {
        let store = Arc::new(InMemoryInteractionStore::new());
        let rules = Arc::new(InMemoryToolPermissionRuleStore::new());
        let broker = Arc::new(
            InteractionBroker::new(store.clone(), Arc::new(InteractionKinds::with_builtin()))
                .with_rules(rules.clone()),
        );
        let ctx = ExecutionContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".into(),
            turn: 0,
            interface: Interface::Web,
            interactive: true,
            run_id: Some(Uuid::new_v4()),
            allowed_tools: None,
            depth: 0,
            user_id: Some(UserId("alice".into())),
            org_id: None,
            space_id: None,
        };
        Fixture {
            confirmation: Arc::new(InteractiveConfirmation::new(broker.clone())),
            broker,
            store,
            rules,
            ctx,
        }
    }

    /// Ask for approval to run `command` in the background.
    fn ask(
        f: &Fixture,
        command: &str,
        options: ApprovalOptions,
        suggested: Option<&'static str>,
    ) -> tokio::task::JoinHandle<ConfirmationAnswer> {
        let confirmation = f.confirmation.clone();
        let ctx = f.ctx.clone();
        let params = json!({ "command": command });
        tokio::spawn(async move {
            confirmation
                .confirm(ConfirmationRequest {
                    tool_name: "bash",
                    params: &params,
                    ctx: &ctx,
                    tool_call_id: Some("call-1"),
                    reason: "no rule allows it",
                    options,
                    suggested_rule: suggested,
                    event_sink: None,
                })
                .await
        })
    }

    async fn pending(f: &Fixture) -> assistant_storage::InteractionRow {
        loop {
            let rows = f
                .store
                .list_for_conversation(
                    &f.ctx.conversation_id.to_string(),
                    Some(InteractionStatus::Pending),
                )
                .await
                .unwrap();
            if let Some(row) = rows.into_iter().next() {
                return row;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn answer(f: &Fixture, id: &str, decision: &str) {
        let run = f.ctx.run_id.unwrap().to_string();
        let conversation = f.ctx.conversation_id.to_string();
        f.broker
            .respond(
                &Responder {
                    user_id: "alice",
                    conversation_id: &conversation,
                    run_id: &run,
                },
                id,
                json!({ "decision": decision }),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn the_interaction_carries_the_call_the_options_and_the_suggested_rule() {
        let f = fixture();
        let task = ask(
            &f,
            "git status --short",
            ApprovalOptions::with_always(),
            Some("bash(git status*)"),
        );
        let row = pending(&f).await;

        assert_eq!(row.kind, "tool_approval");
        assert_eq!(row.user_id, "alice");
        assert_eq!(row.run_id, f.ctx.run_id.unwrap().to_string());
        assert_eq!(row.tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(
            row.request,
            json!({
                "tool_name": "bash",
                "arguments": {"command": "git status --short"},
                "arguments_truncated": false,
                "reason": "no rule allows it",
                "options": ["allow_once", "allow_always", "reject_once", "reject_always"],
                "suggested_rule": "bash(git status*)",
            })
        );
        answer(&f, &row.id, "allow_once").await;
        assert_eq!(task.await.unwrap(), ConfirmationAnswer::AllowOnce);
    }

    #[tokio::test]
    async fn once_only_requests_offer_no_always_and_no_rule() {
        let f = fixture();
        let task = ask(&f, "rm -rf build", ApprovalOptions::once_only(), None);
        let row = pending(&f).await;
        assert_eq!(row.request["options"], json!(["allow_once", "reject_once"]));
        assert!(row.request.get("suggested_rule").is_none());
        answer(&f, &row.id, "reject_once").await;
        assert_eq!(task.await.unwrap(), ConfirmationAnswer::RejectOnce);
    }

    #[tokio::test]
    async fn every_decision_maps_to_its_answer() {
        for (decision, expected) in [
            ("allow_once", ConfirmationAnswer::AllowOnce),
            ("allow_always", ConfirmationAnswer::AllowAlways),
            ("reject_once", ConfirmationAnswer::RejectOnce),
            ("reject_always", ConfirmationAnswer::RejectAlways),
        ] {
            let f = fixture();
            let task = ask(
                &f,
                "git status",
                ApprovalOptions::with_always(),
                Some("bash(git status*)"),
            );
            let row = pending(&f).await;
            answer(&f, &row.id, decision).await;
            assert_eq!(task.await.unwrap(), expected, "{decision}");
        }
    }

    #[tokio::test]
    async fn allow_always_leaves_a_saved_rule_before_the_tool_runs() {
        let f = fixture();
        let task = ask(
            &f,
            "git status",
            ApprovalOptions::with_always(),
            Some("bash(git status*)"),
        );
        let row = pending(&f).await;
        answer(&f, &row.id, "allow_always").await;
        assert_eq!(task.await.unwrap(), ConfirmationAnswer::AllowAlways);
        let saved = f.rules.list_for_user("alice").await.unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].rule, "bash(git status*)");
    }

    #[tokio::test]
    async fn cancelling_the_run_cancels_the_approval() {
        let f = fixture();
        let task = ask(&f, "make", ApprovalOptions::once_only(), None);
        pending(&f).await;
        f.store
            .cancel_pending_for_run(&f.ctx.run_id.unwrap().to_string())
            .await
            .unwrap();
        assert_eq!(task.await.unwrap(), ConfirmationAnswer::Cancelled);
    }

    #[tokio::test]
    async fn a_turn_without_an_owner_is_denied_without_asking() {
        let mut f = fixture();
        f.ctx.user_id = None;
        let answer = ask(&f, "make", ApprovalOptions::once_only(), None)
            .await
            .unwrap();
        assert_eq!(answer, ConfirmationAnswer::Cancelled);
        assert!(
            f.store
                .list_for_conversation(&f.ctx.conversation_id.to_string(), None)
                .await
                .unwrap()
                .is_empty()
        );
    }

    /// Records which turns reached it.
    struct Fixed(ConfirmationAnswer);

    #[async_trait]
    impl ConfirmationCallback for Fixed {
        async fn confirm(&self, _req: ConfirmationRequest<'_>) -> ConfirmationAnswer {
            self.0
        }
    }

    #[tokio::test]
    async fn cli_turns_go_to_the_terminal_and_the_rest_to_the_broker() {
        let router = RoutedConfirmation::new(
            Arc::new(Fixed(ConfirmationAnswer::AllowOnce)),
            Arc::new(Fixed(ConfirmationAnswer::RejectOnce)),
        );
        let mut ctx = fixture().ctx;
        let params = json!({});
        for (interface, expected) in [
            (Interface::Cli, ConfirmationAnswer::AllowOnce),
            (Interface::Web, ConfirmationAnswer::RejectOnce),
            (Interface::Slack, ConfirmationAnswer::RejectOnce),
        ] {
            ctx.interface = interface;
            let got = router
                .confirm(ConfirmationRequest {
                    tool_name: "bash",
                    params: &params,
                    ctx: &ctx,
                    tool_call_id: None,
                    reason: "",
                    options: ApprovalOptions::once_only(),
                    suggested_rule: None,
                    event_sink: None,
                })
                .await;
            assert_eq!(got, expected, "{:?}", ctx.interface);
        }
    }
}
