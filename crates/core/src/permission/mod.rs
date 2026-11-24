//! Tool-permission pipeline.
//!
//! Every tool call is evaluated before it runs:
//!
//! 1. [`StaticRules`] — deny / ask / allow rules, workspace confinement and
//!    built-in hard-deny lists. Deny always wins and never reaches a model.
//! 2. An optional decision model (any [`ToolDecider`]) for calls the static
//!    rules leave undecided.
//! 3. Fallback: the runtime asks a human (interactive turns) or denies.
//!
//! [`PermissionPolicy::evaluate`] runs steps 1–2 and reports an [`Outcome`];
//! step 3 lives in the runtime because it needs the confirmation callback.

mod path;
mod policy;
mod rule;
mod static_rules;

use async_trait::async_trait;
use serde_json::Value;

use crate::types::conversation::ExecutionContext;

pub use path::{PathOutsideRoots, WorkspaceRoots, glob_literal_dir, resolve_real_path};
pub use policy::PermissionPolicy;
pub use rule::{Rule, wildcard_match};
pub use static_rules::StaticRules;

/// Everything a decider may look at for one tool call.
#[derive(Debug, Clone, Copy)]
pub struct PermissionRequest<'a> {
    pub tool_name: &'a str,
    pub params: &'a Value,
    pub ctx: &'a ExecutionContext,
    /// `ToolHandler::requires_confirmation` of the target tool.
    pub requires_confirmation: bool,
    /// The user's task text for this turn, when known.
    pub task_context: Option<&'a str>,
}

/// A single decider's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny,
    /// A human must decide.
    Ask,
    /// No opinion; the next stage decides.
    Abstain,
}

/// A verdict plus the audit trail for it.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub verdict: Verdict,
    /// Which decider produced this (`"static-rules"`, `"decision-model"`, ...).
    pub decider: &'static str,
    /// The matched rule text or model id.
    pub rule: Option<String>,
    pub confidence: Option<f32>,
    /// Human-readable reason; returned to the LLM on denial.
    pub reason: String,
}

impl Decision {
    pub fn new(verdict: Verdict, decider: &'static str, reason: impl Into<String>) -> Self {
        Self {
            verdict,
            decider,
            rule: None,
            confidence: None,
            reason: reason.into(),
        }
    }

    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence);
        self
    }
}

/// Final pipeline outcome for one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Allow,
    Deny,
    /// Undecided: interactive turns defer to a human, others are denied.
    Ask,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Allow => "allow",
            Outcome::Deny => "deny",
            Outcome::Ask => "ask",
        }
    }
}

/// The pipeline result: the outcome and the decision that produced it.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
    pub outcome: Outcome,
    pub decision: Decision,
}

impl Resolution {
    /// Map a decider's verdict to an outcome; no opinion means a human decides.
    pub fn from_decision(decision: Decision) -> Self {
        let outcome = match decision.verdict {
            Verdict::Allow => Outcome::Allow,
            Verdict::Deny => Outcome::Deny,
            Verdict::Ask | Verdict::Abstain => Outcome::Ask,
        };
        Self { outcome, decision }
    }
}

/// One stage of the pipeline.
#[async_trait]
pub trait ToolDecider: Send + Sync {
    async fn decide(&self, req: &PermissionRequest<'_>) -> Decision;
}
