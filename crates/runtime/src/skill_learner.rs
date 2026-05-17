//! Post-turn autonomous skill creation.
//!
//! After each turn completes, the orchestrator can spawn a background
//! evaluation that decides whether the completed task should become a
//! reusable skill.  An LLM "judge" reviews the conversation history and
//! either proposes a new skill (name + description + body) or passes.
//!
//! This module is gated behind `LearningConfig.enabled &&
//! LearningConfig.auto_create_skills`.

use assistant_storage::ConversationStore;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::types::features::LearningConfig;
use assistant_core::{ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse};
use assistant_storage::{SkillRegistry, StorageLayer};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Context collected from the completed turn, used by the heuristic gate
/// and the LLM judge.
#[allow(dead_code)]
pub(crate) struct TurnContext {
    pub conversation_id: Uuid,
    pub agent_id: String,
    pub tool_count: usize,
    pub had_errors: bool,
    pub active_skill: Option<String>,
    pub history: Vec<ChatHistoryMessage>,
}

/// Spawn a fire-and-forget background task that evaluates whether the
/// just-completed turn should produce a new skill.
pub(crate) fn spawn_post_turn_eval(
    ctx: TurnContext,
    config: LearningConfig,
    storage: Arc<StorageLayer>,
    registry: Arc<SkillRegistry>,
    llm: Arc<dyn LlmProvider>,
) {
    tokio::spawn(async move {
        if let Err(e) = evaluate_and_create(ctx, &config, storage, registry, llm).await {
            warn!(error = %e, "Post-turn skill evaluation failed");
        }
    });
}

/// Heuristic gate: quick checks before calling the LLM judge.
fn passes_heuristic_gate(ctx: &TurnContext, config: &LearningConfig) -> bool {
    // Don't create skills for turns that already had a skill loaded.
    if ctx.active_skill.is_some() {
        debug!("Skill learner: skipping — turn already had an active skill");
        return false;
    }

    // Must have enough tool calls to be interesting.
    if ctx.tool_count < config.min_tool_calls_for_skill {
        debug!(
            tool_count = ctx.tool_count,
            threshold = config.min_tool_calls_for_skill,
            "Skill learner: skipping — too few tool calls"
        );
        return false;
    }

    // Turns with errors are less likely to produce good skill templates.
    if ctx.had_errors {
        debug!("Skill learner: skipping — turn had errors");
        return false;
    }

    true
}

/// System prompt for the LLM judge that decides skill creation.
const JUDGE_SYSTEM_PROMPT: &str = r#"You are a skill extraction engine. Your job is to decide whether a completed AI assistant conversation turn should become a reusable "skill" — a template that helps the assistant perform similar tasks in the future.

A good skill candidate:
- Represents a repeatable pattern (not a one-off question)
- Involves multiple tool calls working together toward a goal
- Would benefit from documented steps and context
- Is general enough to be useful across different inputs

Respond with EXACTLY one JSON object (no markdown fences, no extra text):
- If the turn should become a skill:
  {"create": true, "name": "<kebab-case-name>", "description": "<one-line description>", "body": "<markdown body with steps and guidance>"}
- If the turn should NOT become a skill:
  {"create": false, "reason": "<brief explanation>"}

Rules for the skill:
- name: kebab-case, 2-5 words, descriptive (e.g., "code-review", "debug-test-failure")
- description: one sentence explaining what the skill does
- body: markdown instructions that guide the assistant through the task pattern, including which tools to use and in what order"#;

/// Core logic: gate check, LLM judge call, skill registration.
async fn evaluate_and_create(
    ctx: TurnContext,
    config: &LearningConfig,
    storage: Arc<StorageLayer>,
    registry: Arc<SkillRegistry>,
    llm: Arc<dyn LlmProvider>,
) -> Result<()> {
    if !passes_heuristic_gate(&ctx, config) {
        return Ok(());
    }

    // Format conversation history for the judge.
    let history_text = format_history_for_judge(&ctx.history);
    if history_text.is_empty() {
        return Ok(());
    }

    // Call the LLM as judge.
    let judge_history = vec![ChatHistoryMessage::Text {
        role: ChatRole::User,
        content: format!("Here is the conversation turn to evaluate:\n\n{history_text}"),
    }];
    let response = llm.chat(JUDGE_SYSTEM_PROMPT, &judge_history, &[]).await?;

    let text = match &response {
        LlmResponse::FinalAnswer(t, _) => t.trim(),
        LlmResponse::Thinking(t, _) => t.trim(),
        _ => return Ok(()), // ToolCalls response is unexpected here
    };
    let decision: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| anyhow::anyhow!("LLM judge returned invalid JSON: {e}\nRaw: {text}"))?;

    let should_create = decision
        .get("create")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !should_create {
        let reason = decision
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("no reason given");
        debug!(reason, "Skill learner: LLM judge declined skill creation");
        return Ok(());
    }

    let name = decision
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed-skill");
    let description = decision
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let body = decision.get("body").and_then(|v| v.as_str()).unwrap_or("");

    if name.is_empty() || body.is_empty() {
        debug!("Skill learner: LLM judge returned empty name or body, skipping");
        return Ok(());
    }

    // Handle name collision by appending numeric suffix.
    let final_name = resolve_name_collision(&registry, name).await;

    // Register the skill.
    match registry
        .create_user_skill(&final_name, description, body)
        .await
    {
        Ok(def) => {
            info!(
                skill_name = %final_name,
                source = "auto",
                "Skill learner: created new skill from turn"
            );
            // Record in conversation store for observability.
            // Use System role — Tool role requires a matching AssistantToolCalls
            // message and would cause providers to reject the request.
            let msg = assistant_core::types::conversation::Message::new(
                ctx.conversation_id,
                assistant_core::types::conversation::MessageRole::System,
                format!("Auto-created skill '{final_name}': {description}"),
            );
            let conv_store = storage.conversation_store();
            if let Err(e) = conv_store.save_message(&msg).await {
                warn!(error = %e, "Failed to record skill creation message");
            }
            let _ = def; // suppress unused warning
        }
        Err(e) => {
            warn!(skill_name = %final_name, error = %e, "Skill learner: failed to create skill");
        }
    }

    Ok(())
}

/// Resolve name collisions by appending `-2`, `-3`, etc.
async fn resolve_name_collision(registry: &SkillRegistry, base_name: &str) -> String {
    if registry.get(base_name).await.is_none() {
        return base_name.to_string();
    }

    for i in 2..=99 {
        let candidate = format!("{base_name}-{i}");
        if registry.get(&candidate).await.is_none() {
            return candidate;
        }
    }

    // Fallback: use UUID suffix (practically unreachable).
    format!("{base_name}-{}", Uuid::new_v4().as_simple())
}

/// Format chat history into a human-readable summary for the LLM judge.
fn format_history_for_judge(history: &[ChatHistoryMessage]) -> String {
    let mut parts = Vec::new();
    for msg in history {
        match msg {
            ChatHistoryMessage::Text { role, content } => match role {
                ChatRole::User => parts.push(format!("USER: {content}")),
                ChatRole::Assistant => parts.push(format!("ASSISTANT: {content}")),
                _ => {}
            },
            ChatHistoryMessage::MultimodalUser { .. } => {
                parts.push("USER: [multimodal content]".to_string());
            }
            ChatHistoryMessage::AssistantToolCalls(calls) => {
                for call in calls {
                    let params_preview = serde_json::to_string(&call.params)
                        .unwrap_or_default()
                        .chars()
                        .take(200)
                        .collect::<String>();
                    parts.push(format!("TOOL_CALL: {} {params_preview}", call.name));
                }
            }
            ChatHistoryMessage::ToolResult { name, content } => {
                let preview: String = content.chars().take(200).collect();
                parts.push(format!("TOOL_RESULT[{name}]: {preview}"));
            }
        }
    }
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heuristic_gate_rejects_low_tool_count() {
        let config = LearningConfig::default();
        let ctx = TurnContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".to_string(),
            tool_count: 1, // below default threshold of 3
            had_errors: false,
            active_skill: None,
            history: vec![],
        };
        assert!(!passes_heuristic_gate(&ctx, &config));
    }

    #[test]
    fn test_heuristic_gate_rejects_active_skill() {
        let config = LearningConfig::default();
        let ctx = TurnContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".to_string(),
            tool_count: 10,
            had_errors: false,
            active_skill: Some("existing-skill".to_string()),
            history: vec![],
        };
        assert!(!passes_heuristic_gate(&ctx, &config));
    }

    #[test]
    fn test_heuristic_gate_rejects_errors() {
        let config = LearningConfig::default();
        let ctx = TurnContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".to_string(),
            tool_count: 10,
            had_errors: true,
            active_skill: None,
            history: vec![],
        };
        assert!(!passes_heuristic_gate(&ctx, &config));
    }

    #[test]
    fn test_heuristic_gate_passes_valid_turn() {
        let config = LearningConfig::default();
        let ctx = TurnContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "test".to_string(),
            tool_count: 5,
            had_errors: false,
            active_skill: None,
            history: vec![],
        };
        assert!(passes_heuristic_gate(&ctx, &config));
    }

    #[tokio::test]
    async fn test_resolve_name_collision_no_conflict() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let name = resolve_name_collision(&registry, "new-skill").await;
        assert_eq!(name, "new-skill");
    }

    #[test]
    fn test_heuristic_gate_passes_subagent_turn() {
        let config = LearningConfig::default();
        // Subagent turns have no active_skill (they don't inherit from parent).
        let ctx = TurnContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "subagent-abc123".to_string(),
            tool_count: 10,
            had_errors: false,
            active_skill: None,
            history: vec![],
        };
        assert!(
            passes_heuristic_gate(&ctx, &config),
            "Subagent turns with enough tool calls should pass the heuristic gate"
        );
    }

    #[tokio::test]
    async fn test_evaluate_and_create_from_subagent_turn() {
        use assistant_core::{
            Capabilities, ChatHistoryMessage as CHM, LlmResponse, LlmResponseMeta, StreamChunk,
            ToolSpec, ToolSupport,
        };
        use async_trait::async_trait;
        use tokio::sync::mpsc;

        struct MockJudgeLlm;

        #[async_trait]
        impl LlmProvider for MockJudgeLlm {
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
                _system: &str,
                _history: &[CHM],
                _tools: &[ToolSpec],
            ) -> anyhow::Result<LlmResponse> {
                Ok(LlmResponse::FinalAnswer(
                    "{\"create\": true, \"name\": \"codebase-search\", \"description\": \"Search and summarize codebase patterns\", \"body\": \"# Codebase Search\\n\\n1. Glob for files\\n2. Grep for patterns\\n3. Summarize\"}".to_string(),
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
                _system: &str,
                _history: &[CHM],
                _tools: &[ToolSpec],
                _sink: Option<mpsc::Sender<StreamChunk>>,
            ) -> anyhow::Result<LlmResponse> {
                unimplemented!()
            }
        }

        let storage = Arc::new(
            assistant_storage::StorageLayer::new_in_memory()
                .await
                .unwrap(),
        );
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(MockJudgeLlm);

        // Simulate a subagent turn with rich tool-call history.
        let ctx = TurnContext {
            conversation_id: Uuid::new_v4(),
            agent_id: "subagent-test-abc".to_string(),
            tool_count: 5,
            had_errors: false,
            active_skill: None,
            history: vec![
                ChatHistoryMessage::Text {
                    role: ChatRole::User,
                    content: "Search the codebase for sidebar components".to_string(),
                },
                ChatHistoryMessage::Text {
                    role: ChatRole::Assistant,
                    content: "Found 5 sidebar-related files across the project.".to_string(),
                },
            ],
        };

        let config = LearningConfig::default();
        evaluate_and_create(ctx, &config, storage.clone(), registry.clone(), llm)
            .await
            .unwrap();

        // Verify the skill was created.
        let skill = registry.get("codebase-search").await;
        assert!(
            skill.is_some(),
            "Skill should have been auto-created from subagent turn"
        );
        let skill = skill.unwrap();
        assert_eq!(skill.description, "Search and summarize codebase patterns");
    }
}
