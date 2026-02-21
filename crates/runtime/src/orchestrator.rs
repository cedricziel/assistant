//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, skill registry, and skill executor.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ExecutionContext, ExecutionTrace, Interface, Message, MessageRole, SkillTier,
};
use assistant_llm::{ChatHistoryMessage, ChatRole, LlmClient, LlmResponse};
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{registry::SkillRegistry, StorageLayer};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::safety::SafetyGate;

// ── Public types ──────────────────────────────────────────────────────────────

/// Callback trait for requesting user confirmation before executing a skill.
/// Typically implemented by the CLI interface.
pub trait ConfirmationCallback: Send + Sync {
    /// Return `true` if the user confirms execution of `skill_name` with
    /// `params`, or `false` to deny.
    fn confirm(&self, skill_name: &str, params: &serde_json::Value) -> bool;
}

/// The result of a single orchestrator turn.
pub struct TurnResult {
    /// The assistant's final answer to the user.
    pub answer: String,
    /// All skill execution traces collected during this turn.
    pub traces: Vec<ExecutionTrace>,
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

/// Drives the tool-calling loop for a single conversation turn.
///
/// Each call to [`run_turn`] performs the following high-level algorithm:
///
/// 1. Ensure a conversation row exists in SQLite.
/// 2. Persist the user message.
/// 3. Load all registered skills from the registry.
/// 4. Repeatedly call the LLM until it returns a `FinalAnswer` or the
///    iteration limit is reached.
/// 5. For each `ToolCall` response: gate through [`SafetyGate`], optionally
///    confirm with the user, execute the skill, record an [`ExecutionTrace`],
///    and append an `OBSERVATION` to the conversation history.
/// 6. Persist the final assistant message and return [`TurnResult`].
pub struct Orchestrator {
    llm: Arc<LlmClient>,
    storage: Arc<StorageLayer>,
    registry: Arc<SkillRegistry>,
    executor: Arc<SkillExecutor>,
    max_iterations: usize,
    disabled_skills: Vec<String>,
    trace_enabled: bool,
    confirmation_callback: Option<Arc<dyn ConfirmationCallback>>,
}

impl Orchestrator {
    /// Create a new orchestrator.
    ///
    /// # Parameters
    /// * `llm` — the LLM client (Ollama wrapper)
    /// * `storage` — the SQLite storage layer
    /// * `registry` — skill registry (for skill lookups and listing)
    /// * `executor` — skill executor (dispatches to builtin / script / prompt
    ///   handlers)
    /// * `config` — assistant configuration (controls iteration limit, disabled
    ///   skills, and trace logging)
    pub fn new(
        llm: Arc<LlmClient>,
        storage: Arc<StorageLayer>,
        registry: Arc<SkillRegistry>,
        executor: Arc<SkillExecutor>,
        config: &assistant_core::AssistantConfig,
    ) -> Self {
        Self {
            llm,
            storage,
            registry,
            executor,
            max_iterations: config.llm.max_iterations,
            disabled_skills: config.skills.disabled.clone(),
            trace_enabled: config.mirror.trace_enabled,
            confirmation_callback: None,
        }
    }

    /// Attach a confirmation callback (used by the CLI interface).
    pub fn with_confirmation_callback(mut self, cb: Arc<dyn ConfirmationCallback>) -> Self {
        self.confirmation_callback = Some(cb);
        self
    }

    // ── Main entry point ──────────────────────────────────────────────────────

    /// Process one turn of the conversation.
    ///
    /// # Parameters
    /// * `user_message` — the raw user input
    /// * `conversation_id` — the UUID of the conversation; a new row is created
    ///   in SQLite automatically if one does not exist yet
    /// * `interface` — the interface that originated this request (affects
    ///   safety checks and whether confirmation prompts are allowed)
    pub async fn run_turn(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
    ) -> Result<TurnResult> {
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            "Starting turn"
        );

        // 1. Ensure conversation exists in SQLite.
        let conv_store = self.storage.conversation_store();
        conv_store
            .create_conversation_with_id(conversation_id, None)
            .await?;

        // 2. Load prior turns *before* saving the current message so that the
        //    current message does not appear twice in the LLM history.
        let prior = conv_store.load_history(conversation_id).await?;

        // 3. Persist the user message.
        let user_msg = {
            let mut m = Message::user(conversation_id, user_message);
            m.turn = 0;
            m
        };
        conv_store.save_message(&user_msg).await?;

        // 4. Load all registered skills.
        let skill_defs = self.registry.list().await;
        let skill_refs: Vec<&assistant_core::SkillDef> = skill_defs.iter().collect();

        // 5. Build the base system prompt.
        //    Skills are passed as a separate `tools` argument to the LLM client.
        let system_prompt = "You are a helpful AI assistant.";

        // 6. Build LLM history from prior turns, then append the current message.
        let mut history: Vec<ChatHistoryMessage> = prior
            .into_iter()
            .filter_map(|m| match m.role {
                MessageRole::User => Some(ChatHistoryMessage {
                    role: ChatRole::User,
                    content: m.content,
                }),
                MessageRole::Assistant => Some(ChatHistoryMessage {
                    role: ChatRole::Assistant,
                    content: m.content,
                }),
                _ => None,
            })
            .collect();
        history.push(ChatHistoryMessage {
            role: ChatRole::User,
            content: user_message.to_string(),
        });

        let mut traces: Vec<ExecutionTrace> = Vec::new();

        // 7. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Tool-calling loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
            };

            let response = self.llm.chat(system_prompt, &history, &skill_refs).await?;

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text) => {
                    info!(iteration, "LLM returned final answer");

                    // Persist assistant message.
                    let assistant_msg = {
                        let mut m = Message::assistant(conversation_id, &text);
                        m.turn = iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    return Ok(TurnResult {
                        answer: text,
                        traces,
                    });
                }

                // ── Tool call ─────────────────────────────────────────────────
                LlmResponse::ToolCall { name, params } => {
                    info!(skill = %name, iteration, "LLM requested skill execution");

                    // Look up the skill definition.
                    let Some(skill_def) = self.registry.get(&name).await else {
                        let observation = format!("Skill '{}' not found in registry.", name);
                        warn!(%observation);
                        self.append_observation(&mut history, &observation, None);
                        continue;
                    };

                    // Safety gate.
                    if let Err(reason) =
                        SafetyGate::check(&skill_def, &interface, &self.disabled_skills)
                    {
                        let observation = format!("Skill blocked: {reason}");
                        warn!(%observation);
                        self.append_observation(&mut history, &observation, Some(&name));
                        continue;
                    }

                    // Confirmation gate (for mutating / confirmation-required skills).
                    if skill_def.confirmation_required && ctx.interactive {
                        if let Some(cb) = &self.confirmation_callback {
                            if !cb.confirm(&name, &params) {
                                let observation = format!("User denied execution of '{name}'.");
                                info!(%observation);
                                self.append_observation(&mut history, &observation, Some(&name));
                                continue;
                            }
                        }
                    }

                    // For prompt-tier skills, invoke a sub-LLM call instead of the executor.
                    // The SKILL.md body becomes the system prompt; params are formatted as user input.
                    if matches!(skill_def.tier, SkillTier::Prompt) {
                        debug!(skill = %name, "Prompt-tier skill: running sub-LLM call");

                        let sub_system = format!(
                            "You are a helpful AI assistant.\n\n## Skill: {}\n\n{}",
                            skill_def.name, skill_def.body
                        );
                        let sub_input = format_params_as_prompt(&name, &params);
                        let sub_history = vec![assistant_llm::ChatHistoryMessage {
                            role: assistant_llm::ChatRole::User,
                            content: sub_input,
                        }];

                        let start = std::time::Instant::now();
                        let sub_result = self.llm.chat(&sub_system, &sub_history, &[]).await;
                        let duration_ms = start.elapsed().as_millis() as i64;

                        let observation = match sub_result {
                            Ok(LlmResponse::FinalAnswer(text)) => text,
                            Ok(LlmResponse::ToolCall { name: n, .. }) => {
                                format!("Prompt-skill sub-call returned unexpected tool call: {n}")
                            }
                            Ok(LlmResponse::Thinking(text)) => text,
                            Err(err) => {
                                warn!(skill = %name, %err, "Prompt-tier sub-LLM call failed");
                                format!("Error running prompt skill '{name}': {err}")
                            }
                        };

                        let mut trace = ExecutionTrace::new(
                            conversation_id,
                            iteration as i64,
                            &name,
                            params.clone(),
                        );
                        trace = trace.with_success(observation.clone(), duration_ms);

                        if self.trace_enabled {
                            let trace_store = self.storage.trace_store();
                            if let Err(e) = trace_store.insert(&trace).await {
                                warn!("Failed to persist prompt-tier trace: {e}");
                            }
                        }
                        traces.push(trace);

                        self.append_observation(&mut history, &observation, Some(&name));
                        continue;
                    }

                    // Convert JSON params to the HashMap the executor expects.
                    let params_map: HashMap<String, serde_json::Value> =
                        if let serde_json::Value::Object(map) = &params {
                            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                        } else {
                            HashMap::new()
                        };

                    // Execute the skill and measure duration.
                    let start = std::time::Instant::now();
                    let exec_result = self.executor.execute(&skill_def, params_map, &ctx).await;
                    let duration_ms = start.elapsed().as_millis() as i64;

                    // Build execution trace.
                    let mut trace = ExecutionTrace::new(
                        conversation_id,
                        iteration as i64,
                        &name,
                        params.clone(),
                    );

                    let observation = match exec_result {
                        Ok(output) => {
                            debug!(
                                skill = %name,
                                duration_ms,
                                success = output.success,
                                "Skill execution completed"
                            );
                            trace = trace.with_success(output.content.clone(), duration_ms);
                            output.content
                        }
                        Err(err) => {
                            warn!(skill = %name, %err, "Skill execution failed");
                            let msg = err.to_string();
                            trace = trace.with_error(msg.clone(), duration_ms);
                            format!("Error executing '{name}': {msg}")
                        }
                    };

                    // Persist trace if enabled.
                    if self.trace_enabled {
                        let trace_store = self.storage.trace_store();
                        if let Err(e) = trace_store.insert(&trace).await {
                            warn!("Failed to persist execution trace: {e}");
                        }
                    }

                    traces.push(trace);

                    // Append OBSERVATION to history.
                    self.append_observation(&mut history, &observation, Some(&name));
                }

                // ── Intermediate thinking step ────────────────────────────────
                LlmResponse::Thinking(text) => {
                    debug!(iteration, "LLM emitted thinking step");
                    history.push(ChatHistoryMessage {
                        role: ChatRole::Assistant,
                        content: text,
                    });
                }
            }
        }

        // Reached iteration limit.
        anyhow::bail!(
            "Max iterations ({}) reached without a final answer",
            self.max_iterations
        );
    }

    /// Like [`run_turn`] but streams final-answer tokens through `token_sink`
    /// as they are generated.
    ///
    /// Tool-call and observation steps are silent — only the tokens that make
    /// up the final answer are forwarded.  The complete answer is also
    /// returned in [`TurnResult`] so callers can persist or process it.
    ///
    /// The `token_sink` channel is **not** closed by this method; callers
    /// should drop their own `Receiver` (or call `close()`) to signal
    /// completion.
    pub async fn run_turn_streaming(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        token_sink: mpsc::Sender<String>,
    ) -> Result<TurnResult> {
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            "Starting streaming turn"
        );

        let conv_store = self.storage.conversation_store();
        conv_store
            .create_conversation_with_id(conversation_id, None)
            .await?;

        // Load prior turns before saving the current message to avoid duplication.
        let prior = conv_store.load_history(conversation_id).await?;

        let user_msg = {
            let mut m = Message::user(conversation_id, user_message);
            m.turn = 0;
            m
        };
        conv_store.save_message(&user_msg).await?;

        let skill_defs = self.registry.list().await;
        let skill_refs: Vec<&assistant_core::SkillDef> = skill_defs.iter().collect();

        let system_prompt = "You are a helpful AI assistant.";

        let mut history: Vec<ChatHistoryMessage> = prior
            .into_iter()
            .filter_map(|m| match m.role {
                MessageRole::User => Some(ChatHistoryMessage {
                    role: ChatRole::User,
                    content: m.content,
                }),
                MessageRole::Assistant => Some(ChatHistoryMessage {
                    role: ChatRole::Assistant,
                    content: m.content,
                }),
                _ => None,
            })
            .collect();
        history.push(ChatHistoryMessage {
            role: ChatRole::User,
            content: user_message.to_string(),
        });

        let mut traces: Vec<ExecutionTrace> = Vec::new();

        for iteration in 0..self.max_iterations {
            debug!(iteration, "Streaming tool-calling loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
            };

            // Pass the token sink on every LLM call.  The LLM client forwards
            // tokens only when it determines the response is a final answer.
            let response = self
                .llm
                .chat_streaming(
                    system_prompt,
                    &history,
                    &skill_refs,
                    Some(token_sink.clone()),
                )
                .await?;

            match response {
                LlmResponse::FinalAnswer(text) => {
                    info!(iteration, "Streaming LLM returned final answer");

                    let assistant_msg = {
                        let mut m = Message::assistant(conversation_id, &text);
                        m.turn = iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    return Ok(TurnResult {
                        answer: text,
                        traces,
                    });
                }

                LlmResponse::ToolCall { name, params } => {
                    info!(skill = %name, iteration, "Streaming LLM requested skill execution");

                    let Some(skill_def) = self.registry.get(&name).await else {
                        let observation = format!("Skill '{}' not found in registry.", name);
                        warn!(%observation);
                        self.append_observation(&mut history, &observation, None);
                        continue;
                    };

                    if let Err(reason) =
                        SafetyGate::check(&skill_def, &interface, &self.disabled_skills)
                    {
                        let observation = format!("Skill blocked: {reason}");
                        warn!(%observation);
                        self.append_observation(&mut history, &observation, Some(&name));
                        continue;
                    }

                    if skill_def.confirmation_required && ctx.interactive {
                        if let Some(cb) = &self.confirmation_callback {
                            if !cb.confirm(&name, &params) {
                                let observation = format!("User denied execution of '{name}'.");
                                info!(%observation);
                                self.append_observation(&mut history, &observation, Some(&name));
                                continue;
                            }
                        }
                    }

                    if matches!(skill_def.tier, SkillTier::Prompt) {
                        let sub_system = format!(
                            "You are a helpful AI assistant.\n\n## Skill: {}\n\n{}",
                            skill_def.name, skill_def.body
                        );
                        let sub_input = format_params_as_prompt(&name, &params);
                        let sub_history = vec![assistant_llm::ChatHistoryMessage {
                            role: assistant_llm::ChatRole::User,
                            content: sub_input,
                        }];

                        let start = std::time::Instant::now();
                        let sub_result = self.llm.chat(&sub_system, &sub_history, &[]).await;
                        let duration_ms = start.elapsed().as_millis() as i64;

                        let observation = match sub_result {
                            Ok(LlmResponse::FinalAnswer(text)) => text,
                            Ok(LlmResponse::ToolCall { name: n, .. }) => {
                                format!("Prompt-skill sub-call returned unexpected tool call: {n}")
                            }
                            Ok(LlmResponse::Thinking(text)) => text,
                            Err(err) => {
                                warn!(skill = %name, %err, "Prompt-tier sub-LLM call failed");
                                format!("Error running prompt skill '{name}': {err}")
                            }
                        };

                        let mut trace = ExecutionTrace::new(
                            conversation_id,
                            iteration as i64,
                            &name,
                            params.clone(),
                        );
                        trace = trace.with_success(observation.clone(), duration_ms);

                        if self.trace_enabled {
                            let trace_store = self.storage.trace_store();
                            if let Err(e) = trace_store.insert(&trace).await {
                                warn!("Failed to persist prompt-tier trace: {e}");
                            }
                        }
                        traces.push(trace);

                        self.append_observation(&mut history, &observation, Some(&name));
                        continue;
                    }

                    let params_map: HashMap<String, serde_json::Value> =
                        if let serde_json::Value::Object(map) = &params {
                            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                        } else {
                            HashMap::new()
                        };

                    let start = std::time::Instant::now();
                    let exec_result = self.executor.execute(&skill_def, params_map, &ctx).await;
                    let duration_ms = start.elapsed().as_millis() as i64;

                    let mut trace = ExecutionTrace::new(
                        conversation_id,
                        iteration as i64,
                        &name,
                        params.clone(),
                    );

                    let observation = match exec_result {
                        Ok(output) => {
                            trace = trace.with_success(output.content.clone(), duration_ms);
                            output.content
                        }
                        Err(err) => {
                            warn!(skill = %name, %err, "Skill execution failed");
                            let msg = err.to_string();
                            trace = trace.with_error(msg.clone(), duration_ms);
                            format!("Error executing '{name}': {msg}")
                        }
                    };

                    if self.trace_enabled {
                        let trace_store = self.storage.trace_store();
                        if let Err(e) = trace_store.insert(&trace).await {
                            warn!("Failed to persist execution trace: {e}");
                        }
                    }

                    traces.push(trace);
                    self.append_observation(&mut history, &observation, Some(&name));
                }

                LlmResponse::Thinking(text) => {
                    debug!(iteration, "Streaming LLM emitted thinking step");
                    history.push(ChatHistoryMessage {
                        role: ChatRole::Assistant,
                        content: text,
                    });
                }
            }
        }

        anyhow::bail!(
            "Max iterations ({}) reached without a final answer",
            self.max_iterations
        );
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Append an observation message to the chat history.
    ///
    /// The observation is added as a `tool` role message so the LLM can
    /// recognise it as skill output.
    fn append_observation(
        &self,
        history: &mut Vec<ChatHistoryMessage>,
        observation: &str,
        skill_name: Option<&str>,
    ) {
        let content = if let Some(name) = skill_name {
            format!("OBSERVATION ({name}): {observation}")
        } else {
            format!("OBSERVATION: {observation}")
        };

        history.push(ChatHistoryMessage {
            role: ChatRole::Tool,
            content,
        });
    }
}

// ── Module-level helpers ───────────────────────────────────────────────────────

/// Format skill params as a human-readable prompt for sub-LLM calls (prompt-tier skills).
fn format_params_as_prompt(skill_name: &str, params: &serde_json::Value) -> String {
    if let serde_json::Value::Object(map) = params {
        if map.is_empty() {
            return format!("Execute the '{skill_name}' skill.");
        }
        let param_lines: Vec<String> = map
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                format!("- {k}: {val}")
            })
            .collect();
        format!(
            "Execute the '{skill_name}' skill with the following parameters:\n{}",
            param_lines.join("\n")
        )
    } else {
        format!("Execute the '{skill_name}' skill.")
    }
}
