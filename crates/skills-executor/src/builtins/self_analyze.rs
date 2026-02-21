//! Builtin handler for self-analyze skill.
//!
//! Queries the TraceStore for statistics on a given skill, sends those stats
//! along with the current SKILL.md body to the LLM, and stores the resulting
//! improved SKILL.md as a pending refinement proposal.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use assistant_llm::{ChatHistoryMessage, ChatRole, LlmClient, LlmResponse};
use assistant_storage::{SkillRegistry, StorageLayer};
use async_trait::async_trait;
use tracing::{debug, warn};

pub struct SelfAnalyzeHandler {
    storage: Arc<StorageLayer>,
    llm: Arc<LlmClient>,
    registry: Arc<SkillRegistry>,
}

impl SelfAnalyzeHandler {
    pub fn new(storage: Arc<StorageLayer>, llm: Arc<LlmClient>, registry: Arc<SkillRegistry>) -> Self {
        Self { storage, llm, registry }
    }
}

#[async_trait]
impl SkillHandler for SelfAnalyzeHandler {
    fn skill_name(&self) -> &str {
        "self-analyze"
    }

    async fn execute(
        &self,
        _def: &SkillDef,
        params: HashMap<String, serde_json::Value>,
        _ctx: &ExecutionContext,
    ) -> Result<SkillOutput> {
        let skill_name = match params.get("skill_name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                return Ok(SkillOutput::error(
                    "Missing required parameter 'skill_name'",
                ));
            }
        };

        let window: i64 = params.get("window").and_then(|v| v.as_i64()).unwrap_or(50);

        let trace_store = self.storage.trace_store();

        // Fetch aggregate stats
        let stats = match trace_store.stats_for_skill(&skill_name, window).await {
            Ok(s) => s,
            Err(e) => {
                return Ok(SkillOutput::error(format!(
                    "Failed to fetch trace stats for '{}': {}",
                    skill_name, e
                )));
            }
        };

        // Format summary for the user
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("Self-analysis for skill: {}", skill_name));
        lines.push(format!(
            "Analysis window: {} most-recent executions",
            window
        ));
        lines.push(String::new());
        lines.push(format!("  Total executions : {}", stats.total));
        lines.push(format!("  Successes        : {}", stats.success_count));
        lines.push(format!("  Failures         : {}", stats.error_count));

        if stats.total > 0 {
            let success_rate = (stats.success_count as f64 / stats.total as f64) * 100.0;
            lines.push(format!("  Success rate     : {:.1}%", success_rate));
        }

        lines.push(format!(
            "  Avg duration     : {:.1} ms",
            stats.avg_duration_ms
        ));

        if !stats.common_errors.is_empty() {
            lines.push(String::new());
            lines.push("  Most common errors:".to_string());
            for err in &stats.common_errors {
                lines.push(format!("    - {}", err));
            }
        }

        if stats.total == 0 {
            lines.push(String::new());
            lines.push(format!(
                "No execution history found for '{}'. Run the skill a few times first.",
                skill_name
            ));
            return Ok(SkillOutput::success(lines.join("\n")));
        }

        // Look up the current SKILL.md body from the registry
        let current_body = if let Some(def) = self.registry.get(&skill_name).await {
            def.body.clone()
        } else {
            String::new()
        };

        // Build a self-improvement prompt and ask the LLM for a better SKILL.md
        debug!(skill = %skill_name, "Requesting LLM-generated skill refinement");

        let system_prompt = "You are an expert at writing clear, precise AI skill instructions. \
            You will receive execution statistics and the current SKILL.md body for a skill. \
            Respond with an improved SKILL.md body (the Markdown section only, without frontmatter) \
            that would help the AI use this skill more effectively and avoid past errors. \
            Be concise and actionable.";

        let error_summary = if stats.common_errors.is_empty() {
            "None observed.".to_string()
        } else {
            stats.common_errors.join("; ")
        };

        let user_prompt = format!(
            "Skill: {}\n\nExecution statistics (last {} runs):\n\
            - Total: {}\n- Successes: {}\n- Failures: {}\n\
            - Avg duration: {:.1} ms\n- Common errors: {}\n\n\
            Current SKILL.md instructions:\n---\n{}\n---\n\n\
            Please write an improved version of the instructions section only.",
            skill_name,
            window,
            stats.total,
            stats.success_count,
            stats.error_count,
            stats.avg_duration_ms,
            error_summary,
            current_body,
        );

        let sub_history = vec![ChatHistoryMessage {
            role: ChatRole::User,
            content: user_prompt,
        }];

        let proposed_skill_md = match self.llm.chat(system_prompt, &sub_history, &[]).await {
            Ok(LlmResponse::FinalAnswer(text)) => text,
            Ok(LlmResponse::Thinking(text)) => text,
            Ok(LlmResponse::ToolCall { name, .. }) => {
                warn!(
                    skill = %skill_name,
                    tool = %name,
                    "LLM returned tool call during self-analyze; using fallback"
                );
                format!(
                    "# {}\n\n(LLM returned a tool call instead of text. Run self-analyze again.)",
                    skill_name
                )
            }
            Err(e) => {
                warn!(skill = %skill_name, err = %e, "LLM call failed during self-analyze");
                return Ok(SkillOutput::error(format!(
                    "LLM call failed while generating improvement proposal: {e}"
                )));
            }
        };

        let rationale = format!(
            "Automated analysis: {} total executions, {} errors, {:.1}ms avg. \
            LLM-generated improvement proposal.",
            stats.total, stats.error_count, stats.avg_duration_ms
        );

        let refinement_id = self
            .storage
            .refinements_store()
            .insert(&skill_name, &proposed_skill_md, &rationale)
            .await?;

        lines.push(String::new());
        lines.push(format!(
            "Refinement proposal generated (id: {}).",
            refinement_id
        ));
        lines.push("Run '/review' in the CLI to inspect and apply it.".to_string());

        Ok(SkillOutput::success(lines.join("\n")))
    }
}
