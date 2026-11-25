//! Per-skill execution statistics consumed by skill self-improvement.
//!
//! [`SkillStatsProvider`] is the seam between the improvement cycle and
//! wherever executions are recorded. Every
//! [`SkillExecutionStore`](crate::SkillExecutionStore) is one.

use anyhow::Result;
use async_trait::async_trait;

use crate::skill_executions::{SkillExecution, SkillOutcome};

/// Summary statistics about a skill's past executions.
#[derive(Debug, Clone)]
pub struct SkillStats {
    pub skill_name: String,
    pub total: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub avg_duration_ms: f64,
    /// Total prompt/input tokens consumed in the analysis window.
    /// Zero: executions do not record token usage.
    pub total_input_tokens: i64,
    /// Total completion/output tokens consumed in the analysis window.
    /// Zero: executions do not record token usage.
    pub total_output_tokens: i64,
    /// Up to 5 most-frequent error messages observed in the window.
    pub common_errors: Vec<String>,
}

impl SkillStats {
    /// Maximum number of distinct error messages reported.
    const MAX_COMMON_ERRORS: usize = 5;

    /// Aggregate `executions` (all belonging to `skill_name`).
    pub fn summarize(skill_name: &str, executions: &[SkillExecution]) -> Self {
        let total = executions.len() as i64;
        let error_count = executions
            .iter()
            .filter(|e| e.outcome == SkillOutcome::Error)
            .count() as i64;
        let avg_duration_ms = if executions.is_empty() {
            0.0
        } else {
            executions.iter().map(|e| e.duration_ms as f64).sum::<f64>() / executions.len() as f64
        };

        // First-seen order breaks frequency ties deterministically.
        let mut counts: Vec<(&str, usize)> = Vec::new();
        for error in executions.iter().filter_map(|e| e.error.as_deref()) {
            match counts.iter_mut().find(|(seen, _)| *seen == error) {
                Some((_, n)) => *n += 1,
                None => counts.push((error, 1)),
            }
        }
        counts.sort_by_key(|&(_, n)| std::cmp::Reverse(n));

        Self {
            skill_name: skill_name.to_string(),
            total,
            success_count: total - error_count,
            error_count,
            avg_duration_ms,
            total_input_tokens: 0,
            total_output_tokens: 0,
            common_errors: counts
                .into_iter()
                .take(Self::MAX_COMMON_ERRORS)
                .map(|(error, _)| error.to_string())
                .collect(),
        }
    }
}

/// Source of skill-scoped execution statistics.
#[async_trait]
pub trait SkillStatsProvider: Send + Sync {
    /// Return aggregate stats for the most recent `window` executions of
    /// `skill_name`.
    async fn stats_for_active_skill(&self, skill_name: &str, window: i64) -> Result<SkillStats>;
}
