//! Per-skill execution statistics consumed by skill self-improvement.
//!
//! These statistics used to be aggregated from the local SQLite trace store,
//! which no longer exists: telemetry is exported over OTLP only. The
//! [`SkillStatsProvider`] trait is the seam for a product-owned replacement;
//! until one is wired in, [`UnavailableSkillStats`] reports that no
//! statistics are collected.

use anyhow::{Result, bail};
use async_trait::async_trait;

/// Summary statistics about a skill's past executions.
#[derive(Debug, Clone)]
pub struct SkillStats {
    pub skill_name: String,
    pub total: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub avg_duration_ms: f64,
    /// Total prompt/input tokens consumed in the analysis window.
    pub total_input_tokens: i64,
    /// Total completion/output tokens consumed in the analysis window.
    pub total_output_tokens: i64,
    /// Up to 5 most-frequent error messages observed in the window.
    pub common_errors: Vec<String>,
}

/// Source of skill-scoped execution statistics.
#[async_trait]
pub trait SkillStatsProvider: Send + Sync {
    /// Return aggregate stats for the most recent `window` executions of
    /// `skill_name`.
    async fn stats_for_active_skill(&self, skill_name: &str, window: i64) -> Result<SkillStats>;
}

/// Provider used while no execution statistics are collected.
///
/// Fails instead of reporting zero executions so callers surface the gap
/// rather than concluding that a skill is healthy or unused.
pub struct UnavailableSkillStats;

#[async_trait]
impl SkillStatsProvider for UnavailableSkillStats {
    async fn stats_for_active_skill(&self, skill_name: &str, _window: i64) -> Result<SkillStats> {
        bail!("skill execution statistics are not collected; cannot analyze '{skill_name}'")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unavailable_provider_reports_missing_statistics() {
        let err = UnavailableSkillStats
            .stats_for_active_skill("code-review", 50)
            .await
            .expect_err("no statistics are collected");
        assert!(
            err.to_string().contains("code-review"),
            "error should name the skill, got: {err}"
        );
    }
}
