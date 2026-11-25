//! The pipeline that ties static rules to an optional decision model.

use std::sync::Arc;

use tracing::info;

use super::{Decision, PermissionRequest, Resolution, StaticRules, ToolDecider, Verdict};
use crate::types::permissions::PermissionsConfig;

/// Static rules, then an optional decision model.
#[derive(Clone)]
pub struct PermissionPolicy {
    rules: StaticRules,
    model: Option<Arc<dyn ToolDecider>>,
}

impl PermissionPolicy {
    pub fn new(rules: StaticRules) -> Self {
        Self { rules, model: None }
    }

    /// Built-in rules plus the user's `[permissions]` section.
    pub fn from_config(cfg: &PermissionsConfig) -> Self {
        Self::new(StaticRules::from_config(cfg))
    }

    /// Attach the decision model consulted for calls static rules leave open.
    pub fn with_model(mut self, model: Arc<dyn ToolDecider>) -> Self {
        self.model = Some(model);
        self
    }

    pub async fn evaluate(&self, req: &PermissionRequest<'_>) -> Resolution {
        let resolution = self.resolve(req).await;
        info!(
            target: "assistant::permission",
            tool = req.tool_name,
            decider = resolution.decision.decider,
            rule = resolution.decision.rule.as_deref().unwrap_or(""),
            confidence = resolution.decision.confidence.map(f64::from),
            outcome = resolution.outcome.as_str(),
            interactive = req.ctx.interactive,
            "tool permission decision",
        );
        resolution
    }

    async fn resolve(&self, req: &PermissionRequest<'_>) -> Resolution {
        let decision = self.rules.evaluate(req);
        if decision.verdict != Verdict::Abstain {
            return Resolution::from_decision(decision);
        }

        if req.requires_confirmation {
            return Resolution::from_decision(Decision::new(
                Verdict::Ask,
                "static-rules",
                format!(
                    "'{}' requires confirmation and no allow rule matched",
                    req.tool_name
                ),
            ));
        }

        match &self.model {
            Some(model) => Resolution::from_decision(model.decide(req).await),
            None => Resolution::from_decision(Decision::new(
                Verdict::Ask,
                "static-rules",
                format!(
                    "no rule allows '{}' and no decision model is configured",
                    req.tool_name
                ),
            )),
        }
    }
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self::from_config(&PermissionsConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::permission::{Outcome, Rule};
    use crate::types::conversation::{ExecutionContext, Interface};

    struct Scripted {
        verdict: Verdict,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl ToolDecider for Scripted {
        async fn decide(&self, _req: &PermissionRequest<'_>) -> Decision {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Decision::new(self.verdict, "scripted", "scripted").with_confidence(0.95)
        }
    }

    fn ctx() -> ExecutionContext {
        ExecutionContext {
            conversation_id: Uuid::nil(),
            agent_id: "perm-test".into(),
            turn: 0,
            interface: Interface::Scheduler,
            interactive: false,
            allowed_tools: None,
            depth: 0,
            user_id: None,
            org_id: None,
            space_id: None,
        }
    }

    async fn run(
        policy: &PermissionPolicy,
        tool: &str,
        command: &str,
        requires_confirmation: bool,
    ) -> Resolution {
        let ctx = ctx();
        let params = json!({ "command": command });
        policy
            .evaluate(&PermissionRequest {
                tool_name: tool,
                params: &params,
                ctx: &ctx,
                requires_confirmation,
                task_context: Some("task"),
            })
            .await
    }

    fn policy_with(verdict: Verdict, allow: &str) -> (PermissionPolicy, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        let cfg = PermissionsConfig {
            allow: vec![allow.parse::<Rule>().unwrap()],
            ..Default::default()
        };
        let policy = PermissionPolicy::from_config(&cfg).with_model(Arc::new(Scripted {
            verdict,
            calls: Arc::clone(&calls),
        }));
        (policy, calls)
    }

    #[tokio::test]
    async fn static_deny_never_reaches_the_model() {
        let (policy, calls) = policy_with(Verdict::Allow, "bash(*)");
        let r = run(&policy, "bash", "sudo ls", false).await;
        assert_eq!(r.outcome, Outcome::Deny);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn static_allow_skips_the_model() {
        let (policy, calls) = policy_with(Verdict::Deny, "bash(ls*)");
        let r = run(&policy, "bash", "ls -la", false).await;
        assert_eq!(r.outcome, Outcome::Allow);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn undecided_call_goes_to_the_model() {
        let (policy, calls) = policy_with(Verdict::Allow, "bash(ls*)");
        let r = run(&policy, "bash", "make build", false).await;
        assert_eq!(r.outcome, Outcome::Allow);
        assert_eq!(r.decision.decider, "scripted");
        assert_eq!(r.decision.confidence, Some(0.95));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn model_that_does_not_approve_yields_ask() {
        for v in [Verdict::Ask, Verdict::Abstain] {
            let (policy, _) = policy_with(v, "bash(ls*)");
            assert_eq!(
                run(&policy, "bash", "make", false).await.outcome,
                Outcome::Ask
            );
        }
    }

    #[tokio::test]
    async fn undecided_without_a_model_yields_ask() {
        let r = run(&PermissionPolicy::default(), "bash", "make", false).await;
        assert_eq!(r.outcome, Outcome::Ask);
        assert!(r.decision.reason.contains("bash"));
    }

    #[tokio::test]
    async fn requires_confirmation_asks_without_consulting_the_model() {
        let (policy, calls) = policy_with(Verdict::Allow, "bash(ls*)");
        let r = run(&policy, "mcp__x__tool", "{}", true).await;
        assert_eq!(r.outcome, Outcome::Ask);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn explicit_allow_rule_beats_requires_confirmation() {
        let (policy, _) = policy_with(Verdict::Deny, "mcp__x__tool");
        assert_eq!(
            run(&policy, "mcp__x__tool", "{}", true).await.outcome,
            Outcome::Allow
        );
    }
}
