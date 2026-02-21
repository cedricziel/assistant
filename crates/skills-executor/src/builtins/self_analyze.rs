//! Builtin handler for self-analyze skill.
//!
//! Queries the TraceStore for statistics on a given skill and returns a
//! formatted analysis summary. Queues a pending skill_refinements row via
//! the RefinementsStore — no raw sqlx needed.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, SkillDef, SkillHandler, SkillOutput};
use assistant_storage::StorageLayer;
use async_trait::async_trait;

pub struct SelfAnalyzeHandler {
    storage: Arc<StorageLayer>,
}

impl SelfAnalyzeHandler {
    pub fn new(storage: Arc<StorageLayer>) -> Self {
        Self { storage }
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

        // Format summary
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
        } else {
            // Queue a pending skill_refinements row via the storage layer.
            let placeholder_md = format!(
                "# Pending analysis for {}\n\nThis row was inserted by self-analyze as a placeholder.\nThe runtime should replace proposed_skill_md with an LLM-generated proposal.",
                skill_name
            );
            let rationale = format!(
                "Automated: {} total executions, {} errors, {:.1}ms avg duration. Deeper analysis requires LLM call.",
                stats.total, stats.error_count, stats.avg_duration_ms
            );

            let refinement_id = self
                .storage
                .refinements_store()
                .insert(&skill_name, &placeholder_md, &rationale)
                .await?;

            lines.push(String::new());
            lines.push(format!(
                "A pending refinement proposal (id: {}) has been queued.",
                refinement_id
            ));
            lines.push("Run '/review' to see and apply it.".to_string());
        }

        Ok(SkillOutput::success(lines.join("\n")))
    }
}
