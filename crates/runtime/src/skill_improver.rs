//! Periodic skill self-improvement via scheduled task.
//!
//! Registers a cron task that periodically analyzes skill performance
//! and generates refinements for underperforming skills.  When
//! `auto_apply_refinements` is enabled, improvements are applied
//! immediately with revert-on-regression safety.

use anyhow::Result;
use assistant_core::types::features::LearningConfig;
use assistant_core::{ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse};
use assistant_storage::{
    RefinementStatus, RefinementsStore, SkillRegistry, SkillStatsProvider, StorageLayer,
};
use chrono::Utc;
use tracing::{debug, info, warn};

/// Well-known name for the skill improvement scheduled task.
const TASK_NAME: &str = "skill-self-improve";

/// Register the improvement scheduled task if it doesn't already exist.
///
/// Called at startup when `learning.enabled = true`.
pub async fn register_improvement_task(
    storage: &StorageLayer,
    agent_id: &str,
    config: &LearningConfig,
) -> Result<()> {
    let task_store = storage.scheduled_task_store_for_agent(agent_id);
    if task_store.find_by_name(TASK_NAME).await?.is_some() {
        debug!("Skill improvement task already registered");
        return Ok(());
    }

    // The prompt instructs the assistant to run self-analysis on all skills.
    let prompt = "Run the skill self-improvement cycle: analyze all skills with sufficient \
                  execution data, identify underperforming ones, and generate refinements. \
                  Use the self-analyze tool for each candidate skill."
        .to_string();

    let cron_expr = &config.improvement_cron;

    // Calculate first run time from cron expression.
    let next_run = cron::Schedule::from_str(cron_expr)
        .ok()
        .and_then(|s| s.upcoming(Utc).next());

    task_store
        .insert(TASK_NAME, cron_expr, &prompt, false, next_run)
        .await?;

    info!(
        cron = %cron_expr,
        "Registered skill improvement scheduled task"
    );
    Ok(())
}

/// Run the improvement cycle: check all skills, identify underperformers,
/// generate and optionally apply refinements.
///
/// This can be called from the scheduled task handler or directly.
pub async fn run_improvement_cycle(
    config: &LearningConfig,
    registry: &SkillRegistry,
    stats_provider: &dyn SkillStatsProvider,
    refinements_store: &RefinementsStore,
    llm: &dyn LlmProvider,
) -> Result<()> {
    // First, check recently-applied refinements for regression.
    check_regressions(config, registry, stats_provider, refinements_store).await?;

    // Then, analyze skills eligible for improvement.
    let skills = registry.list().await;
    let mut improved = 0;

    for skill in &skills {
        let stats = match stats_provider
            .stats_for_active_skill(&skill.name, config.min_executions_for_analysis)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                debug!(skill = %skill.name, error = %e, "Failed to fetch stats, skipping");
                continue;
            }
        };

        // Skip skills without enough executions.
        if stats.total < config.min_executions_for_analysis {
            continue;
        }

        // Check error rate threshold.
        let error_rate = if stats.total > 0 {
            stats.error_count as f64 / stats.total as f64
        } else {
            0.0
        };

        if error_rate < config.error_rate_threshold {
            continue;
        }

        info!(
            skill = %skill.name,
            error_rate = %error_rate,
            total = stats.total,
            "Skill underperforming, generating refinement"
        );

        // Generate refinement via LLM.
        match generate_refinement(llm, &skill.name, &skill.body, &stats).await {
            Ok(Some(new_body)) => {
                if config.auto_apply_refinements {
                    // Store previous body for revert capability.
                    refinements_store
                        .insert_with_previous(
                            &skill.name,
                            &new_body,
                            "auto-improvement",
                            &skill.body,
                        )
                        .await?;

                    // Apply the refinement.
                    if let Err(e) = registry.update_skill_body(&skill.name, &new_body).await {
                        warn!(skill = %skill.name, error = %e, "Failed to apply refinement");
                    } else {
                        info!(skill = %skill.name, "Applied auto-refinement");
                        improved += 1;
                    }
                } else {
                    // Store as pending for manual review.
                    refinements_store
                        .insert(&skill.name, &new_body, "auto-improvement")
                        .await?;
                    info!(skill = %skill.name, "Created pending refinement proposal");
                }
            }
            Ok(None) => {
                debug!(skill = %skill.name, "LLM declined to refine this skill");
            }
            Err(e) => {
                warn!(skill = %skill.name, error = %e, "Failed to generate refinement");
            }
        }
    }

    info!(improved, "Skill improvement cycle complete");
    Ok(())
}

/// Check recently-applied refinements for regression and revert if needed.
async fn check_regressions(
    config: &LearningConfig,
    registry: &SkillRegistry,
    stats_provider: &dyn SkillStatsProvider,
    refinements_store: &RefinementsStore,
) -> Result<()> {
    let accepted = refinements_store
        .list_by_status(&RefinementStatus::Accepted)
        .await?;

    for refinement in accepted {
        // Only check refinements that have a previous body (auto-applied).
        let Some(ref previous_body) = refinement.previous_skill_md else {
            continue;
        };

        // Get post-apply stats over the revert window.
        let stats = stats_provider
            .stats_for_active_skill(&refinement.target_skill, config.revert_window)
            .await?;

        // Not enough post-apply data yet.
        if stats.total < config.revert_window {
            continue;
        }

        let post_error_rate = if stats.total > 0 {
            stats.error_count as f64 / stats.total as f64
        } else {
            0.0
        };

        // Compare to a baseline (we use the threshold as baseline since we don't
        // store pre-apply error rate separately).
        if post_error_rate > config.error_rate_threshold + config.revert_regression_threshold {
            warn!(
                skill = %refinement.target_skill,
                post_error_rate,
                "Regression detected, reverting refinement"
            );

            // Revert the skill body.
            if let Err(e) = registry
                .update_skill_body(&refinement.target_skill, previous_body)
                .await
            {
                warn!(skill = %refinement.target_skill, error = %e, "Failed to revert skill body");
                continue;
            }

            refinements_store
                .set_status(refinement.id, &RefinementStatus::Reverted)
                .await?;
            info!(skill = %refinement.target_skill, "Reverted skill to previous version");
        } else {
            // No regression — confirm the refinement.
            refinements_store
                .set_status(refinement.id, &RefinementStatus::Confirmed)
                .await?;
            debug!(skill = %refinement.target_skill, "Confirmed refinement (no regression)");
        }
    }

    Ok(())
}

/// System prompt for generating skill refinements.
const REFINE_SYSTEM_PROMPT: &str = r#"You are a skill optimization engine. Given a skill's current body and its performance statistics, generate an improved version that addresses the observed issues.

If the skill cannot be meaningfully improved, respond with: {"improved": false}

Otherwise respond with:
{"improved": true, "body": "<the complete improved skill body in markdown>"}

Focus on:
- Clearer instructions that reduce error rates
- Better tool selection guidance
- Missing error handling steps
- Improved precondition checks

Do NOT change the fundamental purpose of the skill. Only refine HOW it accomplishes its goal."#;

/// Use the LLM to generate an improved skill body.
async fn generate_refinement(
    llm: &dyn LlmProvider,
    skill_name: &str,
    current_body: &str,
    stats: &assistant_storage::TraceStats,
) -> Result<Option<String>> {
    let user_msg = format!(
        "Skill: {skill_name}\n\n\
         Current body:\n```\n{current_body}\n```\n\n\
         Performance stats (last {} executions):\n\
         - Total: {}\n\
         - Errors: {} ({:.0}% error rate)\n\
         - Avg duration: {:.0}ms\n\
         - Common errors: {}\n\n\
         Generate an improved version.",
        stats.total,
        stats.total,
        stats.error_count,
        if stats.total > 0 {
            stats.error_count as f64 / stats.total as f64 * 100.0
        } else {
            0.0
        },
        stats.avg_duration_ms,
        if stats.common_errors.is_empty() {
            "none".to_string()
        } else {
            stats.common_errors.join(", ")
        },
    );

    let history = vec![ChatHistoryMessage::Text {
        role: ChatRole::User,
        content: user_msg,
    }];

    let response = llm.chat(REFINE_SYSTEM_PROMPT, &history, &[]).await?;

    let text = match &response {
        LlmResponse::FinalAnswer(t, _) => t.trim(),
        LlmResponse::Thinking(t, _) => t.trim(),
        _ => return Ok(None),
    };

    let decision: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| anyhow::anyhow!("LLM returned invalid JSON: {e}"))?;

    let improved = decision
        .get("improved")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !improved {
        return Ok(None);
    }

    let body = decision
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if body.is_empty() {
        return Ok(None);
    }

    Ok(Some(body))
}

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use assistant_core::{
        Capabilities, ChatHistoryMessage, LlmResponse, LlmResponseMeta, StreamChunk, ToolSpec,
        ToolSupport,
    };
    use assistant_storage::TraceStats;
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    /// Mock LLM that returns a canned JSON response.
    struct MockLlm {
        response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn capabilities(&self) -> Capabilities {
            Capabilities {
                tools: ToolSupport::None,
                streaming: false,
                vision: false,
                hosted_tools: vec![],
                context_window_tokens: None,
            }
        }

        async fn chat(
            &self,
            _system_prompt: &str,
            _history: &[ChatHistoryMessage],
            _tools: &[ToolSpec],
        ) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse::FinalAnswer(
                self.response.clone(),
                LlmResponseMeta {
                    model: None,
                    input_tokens: None,
                    output_tokens: None,
                    finish_reason: None,
                    response_id: None,
                },
            ))
        }

        async fn chat_streaming(
            &self,
            _system_prompt: &str,
            _history: &[ChatHistoryMessage],
            _tools: &[ToolSpec],
            _token_sink: Option<mpsc::Sender<StreamChunk>>,
        ) -> anyhow::Result<LlmResponse> {
            Ok(LlmResponse::FinalAnswer(
                self.response.clone(),
                LlmResponseMeta {
                    model: None,
                    input_tokens: None,
                    output_tokens: None,
                    finish_reason: None,
                    response_id: None,
                },
            ))
        }
    }

    /// Mock stats provider that returns configurable stats.
    struct MockStatsProvider {
        stats: TraceStats,
    }

    #[async_trait]
    impl SkillStatsProvider for MockStatsProvider {
        async fn stats_for_active_skill(
            &self,
            _skill_name: &str,
            _window: i64,
        ) -> anyhow::Result<TraceStats> {
            Ok(self.stats.clone())
        }
    }

    #[tokio::test]
    async fn test_register_improvement_task_creates_task() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let config = LearningConfig::default();

        register_improvement_task(&storage, "test-agent", &config)
            .await
            .unwrap();

        let task_store = storage.scheduled_task_store_for_agent("test-agent");
        let task = task_store.find_by_name(TASK_NAME).await.unwrap();
        assert!(task.is_some(), "task should be registered");
        assert_eq!(task.unwrap().cron_expr, config.improvement_cron);
    }

    #[tokio::test]
    async fn test_register_improvement_task_idempotent() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let config = LearningConfig::default();

        register_improvement_task(&storage, "test-agent", &config)
            .await
            .unwrap();
        register_improvement_task(&storage, "test-agent", &config)
            .await
            .unwrap();

        let task_store = storage.scheduled_task_store_for_agent("test-agent");
        let all = task_store.list_all().await.unwrap();
        let count = all.iter().filter(|t| t.name == TASK_NAME).count();
        assert_eq!(count, 1, "should not create duplicates");
    }

    #[tokio::test]
    async fn test_improvement_cycle_skips_below_min_executions() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());

        // Create a skill in the registry.
        registry
            .create_user_skill("test-skill", "A test skill", "Do something")
            .await
            .unwrap();

        let config = LearningConfig {
            min_executions_for_analysis: 10,
            error_rate_threshold: 0.2,
            ..LearningConfig::default()
        };

        // Stats show only 3 executions (below threshold of 10).
        let stats_provider = MockStatsProvider {
            stats: TraceStats {
                skill_name: "test-skill".to_string(),
                total: 3,
                success_count: 1,
                error_count: 2,
                avg_duration_ms: 100.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                common_errors: vec![],
            },
        };

        let refinements_store = RefinementsStore::new(storage.pool.clone());
        let llm = MockLlm {
            response: r#"{"improved": true, "body": "improved body"}"#.to_string(),
        };

        run_improvement_cycle(
            &config,
            &registry,
            &stats_provider,
            &refinements_store,
            &llm,
        )
        .await
        .unwrap();

        // No refinement should have been created because executions < threshold.
        let pending = refinements_store
            .list_by_status(&RefinementStatus::Pending)
            .await
            .unwrap();
        let accepted = refinements_store
            .list_by_status(&RefinementStatus::Accepted)
            .await
            .unwrap();
        assert!(pending.is_empty(), "no pending refinements expected");
        assert!(accepted.is_empty(), "no accepted refinements expected");
    }

    #[tokio::test]
    async fn test_improvement_cycle_generates_refinement_for_high_error_rate() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());

        // Create a skill in the registry.
        registry
            .create_user_skill("flaky-skill", "A flaky skill", "Original body")
            .await
            .unwrap();

        let config = LearningConfig {
            min_executions_for_analysis: 5,
            error_rate_threshold: 0.2,
            auto_apply_refinements: false, // store as pending
            ..LearningConfig::default()
        };

        // Stats show high error rate (60%).
        let stats_provider = MockStatsProvider {
            stats: TraceStats {
                skill_name: "flaky-skill".to_string(),
                total: 10,
                success_count: 4,
                error_count: 6,
                avg_duration_ms: 200.0,
                total_input_tokens: 100,
                total_output_tokens: 50,
                common_errors: vec!["timeout".to_string()],
            },
        };

        let refinements_store = RefinementsStore::new(storage.pool.clone());
        let llm = MockLlm {
            response:
                r##"{"improved": true, "body": "# Improved flaky-skill\n\nBetter instructions."}"##
                    .to_string(),
        };

        run_improvement_cycle(
            &config,
            &registry,
            &stats_provider,
            &refinements_store,
            &llm,
        )
        .await
        .unwrap();

        // A pending refinement should have been created.
        let pending = refinements_store
            .list_by_status(&RefinementStatus::Pending)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1, "one pending refinement expected");
        assert_eq!(pending[0].target_skill, "flaky-skill");
        assert!(
            pending[0]
                .proposed_skill_md
                .contains("Improved flaky-skill")
        );
    }

    #[tokio::test]
    async fn test_check_regressions_reverts_on_high_error_rate() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());

        // Create a skill with the "improved" body.
        registry
            .create_user_skill("regressing-skill", "A skill", "Improved body")
            .await
            .unwrap();

        let config = LearningConfig {
            error_rate_threshold: 0.2,
            revert_window: 5,
            revert_regression_threshold: 0.1,
            ..LearningConfig::default()
        };

        // Insert an accepted refinement with previous body stored.
        let refinements_store = RefinementsStore::new(storage.pool.clone());
        refinements_store
            .insert_with_previous(
                "regressing-skill",
                "Improved body",
                "auto-improvement",
                "Original body",
            )
            .await
            .unwrap();

        // Stats show post-apply error rate of 0.5 (exceeds threshold + regression_threshold = 0.3).
        let stats_provider = MockStatsProvider {
            stats: TraceStats {
                skill_name: "regressing-skill".to_string(),
                total: 10,
                success_count: 5,
                error_count: 5,
                avg_duration_ms: 150.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                common_errors: vec![],
            },
        };

        check_regressions(&config, &registry, &stats_provider, &refinements_store)
            .await
            .unwrap();

        // The refinement should now be reverted.
        let reverted = refinements_store
            .list_by_status(&RefinementStatus::Reverted)
            .await
            .unwrap();
        assert_eq!(reverted.len(), 1, "one reverted refinement expected");
        assert_eq!(reverted[0].target_skill, "regressing-skill");
    }

    #[tokio::test]
    async fn test_check_regressions_confirms_when_no_regression() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());

        // Create a skill.
        registry
            .create_user_skill("stable-skill", "A skill", "Improved body")
            .await
            .unwrap();

        let config = LearningConfig {
            error_rate_threshold: 0.2,
            revert_window: 5,
            revert_regression_threshold: 0.1,
            ..LearningConfig::default()
        };

        // Insert an accepted refinement with previous body.
        let refinements_store = RefinementsStore::new(storage.pool.clone());
        refinements_store
            .insert_with_previous(
                "stable-skill",
                "Improved body",
                "auto-improvement",
                "Original body",
            )
            .await
            .unwrap();

        // Stats show low error rate (0.1) — below threshold + regression_threshold (0.3).
        let stats_provider = MockStatsProvider {
            stats: TraceStats {
                skill_name: "stable-skill".to_string(),
                total: 10,
                success_count: 9,
                error_count: 1,
                avg_duration_ms: 100.0,
                total_input_tokens: 0,
                total_output_tokens: 0,
                common_errors: vec![],
            },
        };

        check_regressions(&config, &registry, &stats_provider, &refinements_store)
            .await
            .unwrap();

        // The refinement should now be confirmed.
        let confirmed = refinements_store
            .list_by_status(&RefinementStatus::Confirmed)
            .await
            .unwrap();
        assert_eq!(confirmed.len(), 1, "one confirmed refinement expected");
        assert_eq!(confirmed[0].target_skill, "stable-skill");
    }
}
