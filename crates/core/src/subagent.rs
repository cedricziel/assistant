//! The `SubagentRunner` trait for spawning isolated sub-agent turns.
//!
//! This trait lives in `core` so that `tool-executor` can depend on it
//! without creating a circular dependency with `runtime` (which implements
//! it on [`Orchestrator`]).

use anyhow::Result;
use async_trait::async_trait;

use crate::bus_messages::{AgentReport, AgentSpawn};

/// Runs a sub-agent turn and returns its report.
///
/// The implementor is expected to:
///
/// 1. Create an isolated conversation (new `conversation_id`).
/// 2. Restrict available tools according to [`AgentSpawn::allowed_tools`].
/// 3. Enforce a maximum nesting depth, rejecting the request if exceeded.
/// 4. Execute the sub-agent's task through the normal tool-calling loop.
/// 5. Return an [`AgentReport`] with the outcome.
///
/// # Parameters
///
/// * `spawn` — the sub-agent specification (task, allowed tools, model, etc.)
/// * `parent_depth` — the nesting depth of the calling agent (0 = root).
#[async_trait]
pub trait SubagentRunner: Send + Sync {
    async fn run_subagent(&self, spawn: AgentSpawn, parent_depth: u32) -> Result<AgentReport>;
}
