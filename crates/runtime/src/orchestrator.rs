//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, skill registry, and skill executor.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ExecutionContext, ExecutionTrace, Interface, MemoryLoader, Message, MessageRole, SkillDef,
    SkillSource, SkillTier,
};
use assistant_llm::{ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse};
use assistant_skills_executor::SkillExecutor;
use assistant_storage::{conversations::ConversationStore, registry::SkillRegistry, StorageLayer};
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

// ── Built-in extension tools ──────────────────────────────────────────────────

/// Build the `end_turn` SkillDef that `run_turn_with_tools` always injects.
///
/// The tool carries no real handler — the orchestrator loop detects it by name
/// and exits cleanly.  Exposing it as a proper tool gives the LLM a first-class,
/// typed way to signal "I'm done" without having to return a plain FinalAnswer.
fn end_turn_def() -> SkillDef {
    let mut metadata = HashMap::new();
    metadata.insert("tier".to_string(), "builtin".to_string());
    metadata.insert(
        "params".to_string(),
        r#"{"type":"object","properties":{"reason":{"type":"string","description":"Brief reason the turn is ending (e.g. \"replied\", \"no reply needed\"). Used for logging only."}}}"#
            .to_string(),
    );
    SkillDef {
        name: "end_turn".to_string(),
        description: "Signal that this turn is complete. Call this once you have sent your reply \
             (or decided no reply is needed). The `reason` field is optional and used for \
             logging only."
            .to_string(),
        license: None,
        compatibility: None,
        allowed_tools: vec![],
        metadata,
        body: String::new(),
        dir: std::path::PathBuf::new(),
        tier: SkillTier::Builtin,
        mutating: false,
        confirmation_required: false,
        source: SkillSource::Builtin,
    }
}

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
    llm: Arc<dyn LlmProvider>,
    storage: Arc<StorageLayer>,
    registry: Arc<SkillRegistry>,
    executor: Arc<SkillExecutor>,
    max_iterations: usize,
    disabled_skills: Vec<String>,
    trace_enabled: bool,
    confirmation_callback: Option<Arc<dyn ConfirmationCallback>>,
    /// Memory loader used to rebuild the system prompt at the start of every
    /// turn so that writes made by memory skills are reflected immediately.
    memory_loader: MemoryLoader,
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
        llm: Arc<dyn LlmProvider>,
        storage: Arc<StorageLayer>,
        registry: Arc<SkillRegistry>,
        executor: Arc<SkillExecutor>,
        config: &assistant_core::AssistantConfig,
    ) -> Self {
        let memory_loader = MemoryLoader::new(config);
        memory_loader.ensure_defaults();
        Self {
            llm,
            storage,
            registry,
            executor,
            max_iterations: config.llm.max_iterations,
            disabled_skills: config.skills.disabled.clone(),
            trace_enabled: config.mirror.trace_enabled,
            confirmation_callback: None,
            memory_loader,
        }
    }

    /// Attach a confirmation callback (used by the CLI interface).
    pub fn with_confirmation_callback(mut self, cb: Arc<dyn ConfirmationCallback>) -> Self {
        self.confirmation_callback = Some(cb);
        self
    }

    // ── Main entry point ──────────────────────────────────────────────────────

    /// Process one turn of the conversation with per-turn extension skills.
    ///
    /// Extension skills are injected by the calling interface (e.g. Slack,
    /// Mattermost) and are checked before the global skill registry.  They
    /// bypass the [`SafetyGate`] — the interface is responsible for vetting
    /// them before passing them in.
    ///
    /// Unlike [`run_turn`] / [`run_turn_streaming`], this method does **not**
    /// return the final answer; replies are expected to happen as side-effects
    /// of the extension tool calls (e.g. `slack-reply`).  If the LLM emits a
    /// `FinalAnswer` without calling a reply tool, it is persisted to the DB
    /// but not forwarded anywhere.
    ///
    /// # Parameters
    /// * `user_message` — the raw user input
    /// * `conversation_id` — the UUID of the conversation
    /// * `interface` — the originating interface
    /// * `extensions` — per-turn `(SkillDef, handler)` pairs; names must be
    ///   unique and must not collide with global skill names
    pub async fn run_turn_with_tools(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<(
            assistant_core::SkillDef,
            Arc<dyn assistant_core::SkillHandler>,
        )>,
    ) -> Result<()> {
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            extension_count = extensions.len(),
            "Starting turn with extension tools"
        );

        // Build extension lookup: name → (def, handler).
        let ext_map: HashMap<
            String,
            (
                assistant_core::SkillDef,
                Arc<dyn assistant_core::SkillHandler>,
            ),
        > = extensions
            .iter()
            .map(|(def, h)| (def.name.clone(), (def.clone(), h.clone())))
            .collect();
        // Always inject `end_turn` unless the caller already provided one.
        // It is handled directly in the dispatch loop below (no real handler).
        let et_def = end_turn_def();
        let mut ext_defs: Vec<assistant_core::SkillDef> =
            extensions.into_iter().map(|(def, _)| def).collect();
        if !ext_defs.iter().any(|d| d.name == "end_turn") && !ext_map.contains_key("end_turn") {
            ext_defs.push(et_def);
        }

        // 1-3. Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) =
            self.prepare_history(user_message, conversation_id).await?;

        // 4. Load global skills (including synthetic defs from self-describing
        //    handlers) and merge with extensions for LLM tool listing.
        //    Extension defs come first so the LLM sees them prominently.
        let global_skills = self.merge_skill_defs().await;
        let all_skill_refs: Vec<&assistant_core::SkillDef> =
            ext_defs.iter().chain(global_skills.iter()).collect();

        let base_system_prompt = self.memory_loader.load_system_prompt();
        // When extension tools are present, guide the LLM to use them.
        // We soft-encourage the reply tool rather than hard-requiring it; a
        // FinalAnswer fallback still exists for models that ignore instructions.
        // The one hard requirement is `end_turn` — call it to signal completion.
        let system_prompt = if ext_defs.is_empty() {
            base_system_prompt
        } else {
            let reply_tools: Vec<&str> = ext_defs
                .iter()
                .filter(|d| d.name.contains("reply") || d.name.contains("post"))
                .map(|d| d.name.as_str())
                .collect();
            let reply_hint = if reply_tools.is_empty() {
                String::new()
            } else {
                format!(
                    "To send a response to the user, prefer calling `{}`. ",
                    reply_tools.join("` or `")
                )
            };
            format!(
                "{base_system_prompt}\n\n---\n\n\
                You are operating inside a messaging interface. \
                {reply_hint}\
                When you have finished your turn — whether or not you sent a reply — \
                call `end_turn` to signal completion."
            )
        };

        let mut traces: Vec<ExecutionTrace> = Vec::new();
        // Whether `end_turn` was called explicitly this turn.
        let mut turn_ended = false;
        // Safety net: tracks whether a reply-capable extension tool was called,
        // so the FinalAnswer auto-post fallback does not double-post.
        let mut replied = false;

        // 6. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Extension-tools loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: false,
            };

            let response = self
                .llm
                .chat(&system_prompt, &history, &all_skill_refs)
                .await?;

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text) => {
                    // Persist the assistant message regardless.
                    let assistant_msg = {
                        let mut m = assistant_core::Message::assistant(conversation_id, &text);
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    // If a reply tool was already called during this turn (via the
                    // ToolCalls branch), the user has already received a message —
                    // skip the fallback to avoid double-posting.
                    if replied {
                        return Ok(());
                    }

                    // If a reply-capable extension tool exists, use it to forward
                    // the answer to the user.  This handles models that ignore the
                    // "always call a reply tool" instruction and emit a FinalAnswer
                    // instead.
                    //
                    // Prefer a plain "reply" tool (e.g. slack-reply) over structured
                    // variants (e.g. slack-reply-blocks) so the plain-text fallback
                    // reaches the user correctly.
                    let reply_entry = ext_map
                        .iter()
                        .find(|(name, _)| name.contains("reply") && !name.contains("blocks"))
                        .or_else(|| {
                            ext_map
                                .iter()
                                .find(|(name, _)| name.contains("reply") || name.contains("post"))
                        });

                    if let Some((reply_name, (reply_def, reply_handler))) = reply_entry {
                        info!(
                            iteration,
                            tool = %reply_name,
                            "LLM returned final answer; auto-posting via extension reply tool"
                        );
                        let mut params_map = HashMap::new();
                        // Use whichever parameter the reply tool expects for its text.
                        let text_param = if reply_def
                            .metadata
                            .get("params")
                            .map(|p| p.contains("\"text\""))
                            .unwrap_or(false)
                        {
                            "text"
                        } else {
                            "content"
                        };
                        params_map.insert(
                            text_param.to_string(),
                            serde_json::Value::String(text.clone()),
                        );
                        let ctx = ExecutionContext {
                            conversation_id,
                            turn: iteration as i64,
                            interface: interface.clone(),
                            interactive: false,
                        };
                        if let Err(e) = reply_handler.execute(reply_def, params_map, &ctx).await {
                            warn!(tool = %reply_name, %e, "Auto-post via reply tool failed");
                        }
                    } else {
                        info!(
                            iteration,
                            "LLM returned final answer (no auto-post): no reply tool available"
                        );
                    }

                    return Ok(());
                }

                // ── Tool calls ────────────────────────────────────────────────
                LlmResponse::ToolCalls(tool_call_items) => {
                    info!(
                        count = tool_call_items.len(),
                        iteration, "LLM requested skill execution(s)"
                    );

                    // Record the assistant's tool-call message in history *before*
                    // executing any tool.  This is required for correct Ollama
                    // multi-turn format: the assistant message with `tool_calls`
                    // must precede the corresponding `tool` result messages.
                    history.push(ChatHistoryMessage::AssistantToolCalls(
                        tool_call_items.clone(),
                    ));
                    let tc_msg = Self::make_tool_call_message(
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                    );
                    if let Err(e) = conv_store.save_message(&tc_msg).await {
                        warn!("Failed to persist tool-call message: {e}");
                    }

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;

                        // `end_turn` is handled directly — no real executor.
                        if name == "end_turn" {
                            let reason = params
                                .get("reason")
                                .and_then(|v| v.as_str())
                                .unwrap_or("done");
                            info!(iteration, reason, "end_turn called; stopping turn");

                            // Record a trace so end_turn decisions are visible in
                            // the execution history alongside other tool calls.
                            let mut trace = ExecutionTrace::new(
                                conversation_id,
                                iteration as i64,
                                "end_turn",
                                params.clone(),
                            );
                            trace = trace.with_success(format!("end_turn: {reason}"), 0);
                            if self.trace_enabled {
                                let trace_store = self.storage.trace_store();
                                if let Err(e) = trace_store.insert(&trace).await {
                                    warn!("Failed to persist end_turn trace: {e}");
                                }
                            }
                            traces.push(trace);

                            turn_ended = true;
                            break;
                        }

                        // Extension tools take priority and bypass the safety gate.
                        let (observation, trace_result) = if let Some((ext_def, handler)) =
                            ext_map.get(&name)
                        {
                            debug!(skill = %name, "Dispatching to extension handler");

                            let params_map: HashMap<String, serde_json::Value> =
                                if let serde_json::Value::Object(map) = &params {
                                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                } else {
                                    HashMap::new()
                                };

                            let start = std::time::Instant::now();
                            let exec_result = handler.execute(ext_def, params_map, &ctx).await;
                            let duration_ms = start.elapsed().as_millis() as i64;

                            let mut trace = ExecutionTrace::new(
                                conversation_id,
                                iteration as i64,
                                &name,
                                params.clone(),
                            );
                            let obs = match exec_result {
                                Ok(output) => {
                                    trace = trace.with_success(output.content.clone(), duration_ms);
                                    output.content
                                }
                                Err(err) => {
                                    warn!(skill = %name, %err, "Extension skill execution failed");
                                    let msg = err.to_string();
                                    trace = trace.with_error(msg.clone(), duration_ms);
                                    format!("Error executing '{name}': {msg}")
                                }
                            };
                            (obs, trace)
                        } else {
                            // Global registry path — look up def (registry first,
                            // then synthetic) and apply safety gate.
                            let skill_def = match self.registry.get(&name).await {
                                Some(def) => def,
                                None => match self.executor.get_synthetic_def(&name) {
                                    Some(def) => def,
                                    None => {
                                        let observation =
                                            format!("Skill '{}' not found in registry.", name);
                                        warn!(%observation);
                                        self.append_tool_result(&mut history, &name, &observation);
                                        continue;
                                    }
                                },
                            };

                            if let Err(reason) =
                                SafetyGate::check(&skill_def, &interface, &self.disabled_skills)
                            {
                                let observation = format!("Skill blocked: {reason}");
                                warn!(%observation);
                                self.append_tool_result(&mut history, &name, &observation);
                                continue;
                            }

                            // Confirmation gate — mirrors the gate in run_turn / run_turn_streaming.
                            if skill_def.confirmation_required && ctx.interactive {
                                if let Some(cb) = &self.confirmation_callback {
                                    if !cb.confirm(&name, &params) {
                                        let observation =
                                            format!("User denied execution of '{name}'.");
                                        info!(%observation);
                                        self.append_tool_result(&mut history, &name, &observation);
                                        continue;
                                    }
                                }
                            }

                            if matches!(skill_def.tier, SkillTier::Prompt) {
                                let sub_system = format!(
                                    "{}\n\n## Skill: {}\n\n{}",
                                    system_prompt, skill_def.name, skill_def.body
                                );
                                let sub_input = format_params_as_prompt(&name, &params);
                                let sub_history = vec![ChatHistoryMessage::Text {
                                    role: ChatRole::User,
                                    content: sub_input,
                                }];

                                let start = std::time::Instant::now();
                                let sub_result =
                                    self.llm.chat(&sub_system, &sub_history, &[]).await;
                                let duration_ms = start.elapsed().as_millis() as i64;

                                let observation = match sub_result {
                                    Ok(LlmResponse::FinalAnswer(text)) => text,
                                    Ok(LlmResponse::ToolCalls(calls)) => format!(
                                        "Prompt-skill sub-call returned unexpected tool calls: {}",
                                        calls
                                            .iter()
                                            .map(|c| c.name.as_str())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    ),
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
                                self.append_tool_result(&mut history, &name, &observation);
                                continue;
                            }

                            let params_map: HashMap<String, serde_json::Value> =
                                if let serde_json::Value::Object(map) = &params {
                                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                } else {
                                    HashMap::new()
                                };

                            let start = std::time::Instant::now();
                            let exec_result =
                                self.executor.execute(&skill_def, params_map, &ctx).await;
                            let duration_ms = start.elapsed().as_millis() as i64;

                            let mut trace = ExecutionTrace::new(
                                conversation_id,
                                iteration as i64,
                                &name,
                                params.clone(),
                            );
                            let obs = match exec_result {
                                Ok(output) => {
                                    debug!(skill = %name, duration_ms, "Skill execution completed");
                                    trace = trace.with_success(output.content.clone(), duration_ms);
                                    tool_result_content(&output.content, output.data.as_ref())
                                }
                                Err(err) => {
                                    warn!(skill = %name, %err, "Skill execution failed");
                                    let msg = err.to_string();
                                    trace = trace.with_error(msg.clone(), duration_ms);
                                    format!("Error executing '{name}': {msg}")
                                }
                            };
                            (obs, trace)
                        };

                        // Mark as replied if the dispatched tool is a reply/post
                        // extension, so the FinalAnswer fallback does not fire again.
                        if ext_map.contains_key(&name)
                            && (name.contains("reply") || name.contains("post"))
                        {
                            replied = true;
                        }

                        if self.trace_enabled {
                            let trace_store = self.storage.trace_store();
                            if let Err(e) = trace_store.insert(&trace_result).await {
                                warn!("Failed to persist execution trace: {e}");
                            }
                        }
                        traces.push(trace_result);
                        self.append_tool_result(&mut history, &name, &observation);
                        let tr_msg = Self::make_tool_result_message(
                            conversation_id,
                            base_turn + iteration as i64 + 1,
                            &name,
                            &observation,
                        );
                        if let Err(e) = conv_store.save_message(&tr_msg).await {
                            warn!("Failed to persist tool-result message: {e}");
                        }
                    }

                    // Exit the turn if either the LLM called `end_turn` explicitly
                    // or a reply-capable tool was called (safety net for models that
                    // skip `end_turn` but do use the reply tool correctly).
                    if turn_ended || replied {
                        return Ok(());
                    }
                }

                // ── Intermediate thinking step ────────────────────────────────
                LlmResponse::Thinking(text) => {
                    debug!(iteration, "LLM emitted thinking step");
                    history.push(ChatHistoryMessage::Text {
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

        // 1-3. Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) =
            self.prepare_history(user_message, conversation_id).await?;

        // 4. Load all registered skills, merging synthetic defs from
        //    self-describing handlers.  Registry skills take precedence.
        let skill_defs = self.merge_skill_defs().await;
        let skill_refs: Vec<&assistant_core::SkillDef> = skill_defs.iter().collect();

        // 5. Build the system prompt fresh from disk so that any memory writes
        //    made earlier in this turn are immediately visible to the LLM.
        //    Skills are passed as a separate `tools` argument to the LLM client.
        let system_prompt = self.memory_loader.load_system_prompt();

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

            let response = self.llm.chat(&system_prompt, &history, &skill_refs).await?;

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text) => {
                    info!(iteration, "LLM returned final answer");

                    // Persist assistant message.
                    let assistant_msg = {
                        let mut m = Message::assistant(conversation_id, &text);
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    return Ok(TurnResult {
                        answer: text,
                        traces,
                    });
                }

                // ── Tool calls ────────────────────────────────────────────────
                LlmResponse::ToolCalls(tool_call_items) => {
                    info!(
                        count = tool_call_items.len(),
                        iteration, "LLM requested skill execution(s)"
                    );

                    // Record the assistant's tool-call message in history *before*
                    // executing any tool.  Required for correct Ollama multi-turn format.
                    history.push(ChatHistoryMessage::AssistantToolCalls(
                        tool_call_items.clone(),
                    ));
                    let tc_msg = Self::make_tool_call_message(
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                    );
                    if let Err(e) = conv_store.save_message(&tc_msg).await {
                        warn!("Failed to persist tool-call message: {e}");
                    }

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;

                        // Look up the skill definition (registry first, then synthetic).
                        let skill_def = match self.registry.get(&name).await {
                            Some(def) => def,
                            None => match self.executor.get_synthetic_def(&name) {
                                Some(def) => def,
                                None => {
                                    let observation =
                                        format!("Skill '{}' not found in registry.", name);
                                    warn!(%observation);
                                    self.append_tool_result(&mut history, &name, &observation);
                                    continue;
                                }
                            },
                        };

                        // Safety gate.
                        if let Err(reason) =
                            SafetyGate::check(&skill_def, &interface, &self.disabled_skills)
                        {
                            let observation = format!("Skill blocked: {reason}");
                            warn!(%observation);
                            self.append_tool_result(&mut history, &name, &observation);
                            continue;
                        }

                        // Confirmation gate (for mutating / confirmation-required skills).
                        if skill_def.confirmation_required && ctx.interactive {
                            if let Some(cb) = &self.confirmation_callback {
                                if !cb.confirm(&name, &params) {
                                    let observation = format!("User denied execution of '{name}'.");
                                    info!(%observation);
                                    self.append_tool_result(&mut history, &name, &observation);
                                    continue;
                                }
                            }
                        }

                        // For prompt-tier skills, invoke a sub-LLM call instead of the executor.
                        // The SKILL.md body becomes the system prompt; params are formatted as user input.
                        if matches!(skill_def.tier, SkillTier::Prompt) {
                            debug!(skill = %name, "Prompt-tier skill: running sub-LLM call");

                            let sub_system = format!(
                                "{}\n\n## Skill: {}\n\n{}",
                                system_prompt, skill_def.name, skill_def.body
                            );
                            let sub_input = format_params_as_prompt(&name, &params);
                            let sub_history = vec![assistant_llm::ChatHistoryMessage::Text {
                                role: assistant_llm::ChatRole::User,
                                content: sub_input,
                            }];

                            let start = std::time::Instant::now();
                            let sub_result = self.llm.chat(&sub_system, &sub_history, &[]).await;
                            let duration_ms = start.elapsed().as_millis() as i64;

                            let observation = match sub_result {
                                Ok(LlmResponse::FinalAnswer(text)) => text,
                                Ok(LlmResponse::ToolCalls(calls)) => format!(
                                    "Prompt-skill sub-call returned unexpected tool calls: {}",
                                    calls
                                        .iter()
                                        .map(|c| c.name.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
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

                            self.append_tool_result(&mut history, &name, &observation);
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
                                tool_result_content(&output.content, output.data.as_ref())
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

                        // Append OBSERVATION to history and persist as a tool-result row.
                        self.append_tool_result(&mut history, &name, &observation);
                        let tr_msg = Self::make_tool_result_message(
                            conversation_id,
                            base_turn + iteration as i64 + 1,
                            &name,
                            &observation,
                        );
                        if let Err(e) = conv_store.save_message(&tr_msg).await {
                            warn!("Failed to persist tool-result message: {e}");
                        }
                    }
                }

                // ── Intermediate thinking step ────────────────────────────────
                LlmResponse::Thinking(text) => {
                    debug!(iteration, "LLM emitted thinking step");
                    history.push(ChatHistoryMessage::Text {
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

        // Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) =
            self.prepare_history(user_message, conversation_id).await?;

        let skill_defs = self.merge_skill_defs().await;
        let skill_refs: Vec<&assistant_core::SkillDef> = skill_defs.iter().collect();

        let system_prompt = self.memory_loader.load_system_prompt();

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
                    &system_prompt,
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
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    return Ok(TurnResult {
                        answer: text,
                        traces,
                    });
                }

                LlmResponse::ToolCalls(tool_call_items) => {
                    info!(
                        count = tool_call_items.len(),
                        iteration, "Streaming LLM requested skill execution(s)"
                    );

                    // Record the assistant's tool-call message in history *before*
                    // executing any tool.  Required for correct Ollama multi-turn format.
                    history.push(ChatHistoryMessage::AssistantToolCalls(
                        tool_call_items.clone(),
                    ));
                    let tc_msg = Self::make_tool_call_message(
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                    );
                    if let Err(e) = conv_store.save_message(&tc_msg).await {
                        warn!("Failed to persist tool-call message: {e}");
                    }

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;

                        let skill_def = match self.registry.get(&name).await {
                            Some(def) => def,
                            None => match self.executor.get_synthetic_def(&name) {
                                Some(def) => def,
                                None => {
                                    let observation =
                                        format!("Skill '{}' not found in registry.", name);
                                    warn!(%observation);
                                    self.append_tool_result(&mut history, &name, &observation);
                                    continue;
                                }
                            },
                        };

                        if let Err(reason) =
                            SafetyGate::check(&skill_def, &interface, &self.disabled_skills)
                        {
                            let observation = format!("Skill blocked: {reason}");
                            warn!(%observation);
                            self.append_tool_result(&mut history, &name, &observation);
                            continue;
                        }

                        if skill_def.confirmation_required && ctx.interactive {
                            if let Some(cb) = &self.confirmation_callback {
                                if !cb.confirm(&name, &params) {
                                    let observation = format!("User denied execution of '{name}'.");
                                    info!(%observation);
                                    self.append_tool_result(&mut history, &name, &observation);
                                    continue;
                                }
                            }
                        }

                        if matches!(skill_def.tier, SkillTier::Prompt) {
                            let sub_system = format!(
                                "{}\n\n## Skill: {}\n\n{}",
                                system_prompt, skill_def.name, skill_def.body
                            );
                            let sub_input = format_params_as_prompt(&name, &params);
                            let sub_history = vec![assistant_llm::ChatHistoryMessage::Text {
                                role: assistant_llm::ChatRole::User,
                                content: sub_input,
                            }];

                            let start = std::time::Instant::now();
                            let sub_result = self.llm.chat(&sub_system, &sub_history, &[]).await;
                            let duration_ms = start.elapsed().as_millis() as i64;

                            let observation = match sub_result {
                                Ok(LlmResponse::FinalAnswer(text)) => text,
                                Ok(LlmResponse::ToolCalls(calls)) => format!(
                                    "Prompt-skill sub-call returned unexpected tool calls: {}",
                                    calls
                                        .iter()
                                        .map(|c| c.name.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
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

                            self.append_tool_result(&mut history, &name, &observation);
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
                                tool_result_content(&output.content, output.data.as_ref())
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
                        self.append_tool_result(&mut history, &name, &observation);
                        let tr_msg = Self::make_tool_result_message(
                            conversation_id,
                            base_turn + iteration as i64 + 1,
                            &name,
                            &observation,
                        );
                        if let Err(e) = conv_store.save_message(&tr_msg).await {
                            warn!("Failed to persist tool-result message: {e}");
                        }
                    }
                }

                LlmResponse::Thinking(text) => {
                    debug!(iteration, "Streaming LLM emitted thinking step");
                    history.push(ChatHistoryMessage::Text {
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

    /// Prepare conversation history for a new turn.
    ///
    /// 1. Creates the conversation row if it does not exist.
    /// 2. Loads all prior messages.
    /// 3. Persists the incoming user message.
    /// 4. Builds an LLM-ready [`ChatHistoryMessage`] list (user + assistant)
    ///    with the new user message appended.
    ///
    /// Returns `(conv_store, history, base_turn)` where `base_turn` is the
    /// turn index of the user message just saved (used to assign monotonically
    /// increasing turn numbers to later assistant messages).
    async fn prepare_history(
        &self,
        user_message: &str,
        conversation_id: Uuid,
    ) -> Result<(ConversationStore, Vec<ChatHistoryMessage>, i64)> {
        let conv_store = self.storage.conversation_store();
        conv_store
            .create_conversation_with_id(conversation_id, None)
            .await?;

        let prior = conv_store.load_history(conversation_id).await?;
        let base_turn = prior.len() as i64;

        let user_msg = {
            let mut m = Message::user(conversation_id, user_message);
            m.turn = base_turn;
            m
        };
        conv_store.save_message(&user_msg).await?;

        let mut history: Vec<ChatHistoryMessage> = prior
            .into_iter()
            .filter_map(|m| match m.role {
                MessageRole::User => Some(ChatHistoryMessage::Text {
                    role: ChatRole::User,
                    content: m.content,
                }),
                MessageRole::Assistant => {
                    // If this assistant message carried tool calls, reconstruct
                    // the AssistantToolCalls variant so the LLM sees its own
                    // decisions in subsequent turns.
                    if let Some(tc_json) = m.tool_calls_json {
                        if let Ok(items) =
                            serde_json::from_str::<Vec<assistant_llm::ToolCallItem>>(&tc_json)
                        {
                            if !items.is_empty() {
                                return Some(ChatHistoryMessage::AssistantToolCalls(items));
                            }
                        }
                    }
                    Some(ChatHistoryMessage::Text {
                        role: ChatRole::Assistant,
                        content: m.content,
                    })
                }
                MessageRole::Tool => Some(ChatHistoryMessage::ToolResult {
                    name: m.skill_name.unwrap_or_default(),
                    content: m.content,
                }),
                _ => None,
            })
            .collect();
        history.push(ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: user_message.to_string(),
        });

        Ok((conv_store, history, base_turn))
    }

    /// Append a tool result message to the chat history.
    ///
    /// The result is added as a `ToolResult` variant so the LLM can
    /// recognise it as skill output.
    fn append_tool_result(&self, history: &mut Vec<ChatHistoryMessage>, name: &str, content: &str) {
        history.push(ChatHistoryMessage::ToolResult {
            name: name.to_string(),
            content: content.to_string(),
        });
    }

    /// Build a `Message` row for a turn where the LLM requested tool calls.
    ///
    /// Persisting this row ensures `prepare_history` can reconstruct the
    /// `AssistantToolCalls` variant on subsequent turns.
    fn make_tool_call_message(
        conversation_id: Uuid,
        turn: i64,
        items: &[assistant_llm::ToolCallItem],
    ) -> Message {
        let mut m = Message::assistant(conversation_id, "");
        m.turn = turn;
        m.tool_calls_json = serde_json::to_string(items).ok();
        m
    }

    /// Build a `Message` row for a tool-result observation.
    fn make_tool_result_message(
        conversation_id: Uuid,
        turn: i64,
        skill_name: &str,
        observation: &str,
    ) -> Message {
        let mut m = Message::new(conversation_id, MessageRole::Tool, observation);
        m.turn = turn;
        m.skill_name = Some(skill_name.to_string());
        m
    }

    /// Merge registry skills with synthetic defs from self-describing handlers.
    /// Registry/SKILL.md skills take precedence on name collision.
    async fn merge_skill_defs(&self) -> Vec<assistant_core::SkillDef> {
        let mut merged = self.registry.list().await;
        let registry_names: std::collections::HashSet<String> =
            merged.iter().map(|s| s.name.clone()).collect();

        for def in self.executor.synthetic_skill_defs() {
            if !registry_names.contains(&def.name) {
                merged.push(def);
            }
        }
        merged.sort_by(|a, b| a.name.cmp(&b.name));
        merged
    }
}

// ── Module-level helpers ───────────────────────────────────────────────────────

/// Build the tool result content from a skill output.
///
/// When `data` is present (structured JSON from a `ToolHandler`), the JSON is
/// returned directly so the model can parse it. Models are trained to handle
/// JSON tool results natively.
fn tool_result_content(content: &str, data: Option<&serde_json::Value>) -> String {
    if let Some(d) = data {
        if let Ok(json) = serde_json::to_string(d) {
            return json;
        }
    }
    content.to_string()
}

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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use assistant_core::{
        types::Interface, AssistantConfig, ExecutionContext, Message, SkillDef, SkillOutput,
        SkillSource, SkillTier,
    };
    use assistant_llm::{LlmClient, LlmClientConfig, LlmProvider};
    use assistant_skills_executor::SkillExecutor;
    use assistant_storage::{registry::SkillRegistry, StorageLayer};
    use async_trait::async_trait;
    use serde_json::{json, Value};
    use uuid::Uuid;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::Orchestrator;

    // ── Extension-tool test helpers ───────────────────────────────────────────

    /// A no-op reply handler: always succeeds with content "replied".
    struct NoopReplyHandler;

    #[async_trait]
    impl assistant_core::SkillHandler for NoopReplyHandler {
        fn skill_name(&self) -> &str {
            "test-reply"
        }

        async fn execute(
            &self,
            _def: &SkillDef,
            _params: HashMap<String, Value>,
            _ctx: &ExecutionContext,
        ) -> anyhow::Result<SkillOutput> {
            Ok(SkillOutput {
                content: "replied".to_string(),
                success: true,
                data: None,
            })
        }
    }

    /// Minimal SkillDef for the `test-reply` extension tool.
    fn test_reply_def() -> SkillDef {
        let mut metadata = HashMap::new();
        metadata.insert("tier".to_string(), "builtin".to_string());
        metadata.insert(
            "params".to_string(),
            r#"{"type":"object","properties":{"text":{"type":"string"}}}"#.to_string(),
        );
        SkillDef {
            name: "test-reply".to_string(),
            description: "Send a reply in tests.".to_string(),
            license: None,
            compatibility: None,
            allowed_tools: vec![],
            metadata,
            body: String::new(),
            dir: std::path::PathBuf::new(),
            tier: SkillTier::Builtin,
            mutating: false,
            confirmation_required: false,
            source: SkillSource::Builtin,
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Minimal Ollama final-answer response.
    fn ollama_answer(text: &str) -> Value {
        json!({
            "model": "test",
            "message": { "role": "assistant", "content": text },
            "done": true
        })
    }

    /// Mount a mock that returns a final answer for every POST /api/chat.
    async fn mount_answer(server: &MockServer, text: &str) {
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer(text)))
            .mount(server)
            .await;
    }

    /// Build an [`Orchestrator`] wired to `base_url` with a fresh in-memory DB.
    /// Returns the orchestrator and a handle to the storage so tests can seed data.
    async fn build(base_url: &str) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
        let mut config = AssistantConfig::default();
        config.memory.enabled = false; // disable FS writes in unit tests
        build_with_config(base_url, config).await
    }

    /// Like [`build`] but with a caller-supplied config (e.g. to enable memory
    /// with custom file paths).
    async fn build_with_config(
        base_url: &str,
        config: AssistantConfig,
    ) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let registry = Arc::new(SkillRegistry::new(storage.pool.clone()).await.unwrap());
        let llm: Arc<dyn LlmProvider> = Arc::new(
            LlmClient::new(LlmClientConfig {
                model: "test".to_string(),
                base_url: base_url.to_string(),
                timeout_secs: 10,
            })
            .unwrap(),
        );
        let executor = Arc::new(SkillExecutor::new(
            storage.clone(),
            llm.clone(),
            registry.clone(),
            Arc::new(config.clone()),
        ));
        let orch = Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            registry,
            executor,
            &config,
        ));
        (orch, storage)
    }

    /// Extract the `messages` array from an intercepted Ollama request body.
    fn messages_in(req: &wiremock::Request) -> Vec<Value> {
        let body: Value = serde_json::from_slice(&req.body).unwrap();
        body["messages"].as_array().cloned().unwrap_or_default()
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// First turn: the LLM receives only the system prompt + current user message.
    #[tokio::test]
    async fn first_turn_sends_only_current_message() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        let (orch, _) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        orch.run_turn("hello", conv_id, Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 1);

        let msgs = messages_in(&reqs[0]);
        // [system, user(hello)]
        assert_eq!(msgs.len(), 2, "expected [system, user], got {msgs:?}");
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "hello");
    }

    /// Second turn: prior user + assistant messages are prepended before the
    /// current user message.
    #[tokio::test]
    async fn second_turn_includes_prior_history() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        let (orch, _) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        orch.run_turn("first message", conv_id, Interface::Cli)
            .await
            .unwrap();
        orch.run_turn("second message", conv_id, Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 2);

        let msgs = messages_in(&reqs[1]);
        // [system, user(first), assistant(pong), user(second)]
        assert_eq!(msgs.len(), 4, "expected 4 messages on turn 2, got {msgs:?}");
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "first message");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[2]["content"], "pong");
        assert_eq!(msgs[3]["role"], "user");
        assert_eq!(msgs[3]["content"], "second message");
    }

    /// The current user message must appear exactly once — not duplicated by
    /// load_history returning the already-saved message.
    #[tokio::test]
    async fn current_message_not_duplicated() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        let (orch, _) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        orch.run_turn("turn one", conv_id, Interface::Cli)
            .await
            .unwrap();
        orch.run_turn("turn two", conv_id, Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        let msgs = messages_in(reqs.last().unwrap());

        let count = msgs
            .iter()
            .filter(|m| m["role"] == "user" && m["content"] == "turn two")
            .count();
        assert_eq!(
            count, 1,
            "current message must appear exactly once; found {count}"
        );
    }

    /// Manually seeded history (e.g. from Slack thread hydration) is included
    /// in the LLM payload on the first bot turn in that conversation.
    #[tokio::test]
    async fn seeded_history_included_in_llm_call() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        let (orch, storage) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        // Seed the conversation store directly (simulates Slack thread hydration).
        let conv_store = storage.conversation_store();
        conv_store
            .create_conversation_with_id(conv_id, Some("slack:C001:1234"))
            .await
            .unwrap();

        let mut seed_user = Message::user(conv_id, "seeded user message");
        seed_user.turn = 0;
        conv_store.save_message(&seed_user).await.unwrap();

        let mut seed_bot = Message::assistant(conv_id, "seeded bot reply");
        seed_bot.turn = 1;
        conv_store.save_message(&seed_bot).await.unwrap();

        orch.run_turn("follow-up", conv_id, Interface::Slack)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 1);

        let msgs = messages_in(&reqs[0]);
        // [system, seeded_user, seeded_bot, follow_up]
        assert_eq!(msgs.len(), 4, "expected 4 messages, got {msgs:?}");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "seeded user message");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[2]["content"], "seeded bot reply");
        assert_eq!(msgs[3]["role"], "user");
        assert_eq!(msgs[3]["content"], "follow-up");
    }

    /// Three-turn conversation accumulates history correctly across all turns.
    #[tokio::test]
    async fn three_turns_accumulate_history() {
        let server = MockServer::start().await;
        mount_answer(&server, "reply").await;

        let (orch, _) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        orch.run_turn("turn 1", conv_id, Interface::Cli)
            .await
            .unwrap();
        orch.run_turn("turn 2", conv_id, Interface::Cli)
            .await
            .unwrap();
        orch.run_turn("turn 3", conv_id, Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 3);

        // Third request: [system, u1, a1, u2, a2, u3]
        let msgs = messages_in(&reqs[2]);
        assert_eq!(msgs.len(), 6, "expected 6 messages on turn 3, got {msgs:?}");
        assert_eq!(msgs[1]["content"], "turn 1");
        assert_eq!(msgs[2]["content"], "reply");
        assert_eq!(msgs[3]["content"], "turn 2");
        assert_eq!(msgs[4]["content"], "reply");
        assert_eq!(msgs[5]["content"], "turn 3");
    }

    /// Different conversation IDs are fully isolated — no history bleed.
    #[tokio::test]
    async fn different_conversations_are_isolated() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        let (orch, _) = build(&server.uri()).await;
        let conv_a = Uuid::new_v4();
        let conv_b = Uuid::new_v4();

        orch.run_turn("conv-a message", conv_a, Interface::Cli)
            .await
            .unwrap();
        orch.run_turn("conv-b message", conv_b, Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();

        // conv-b's request must not contain conv-a's message.
        let msgs_b = messages_in(&reqs[1]);
        let bleed = msgs_b.iter().any(|m| m["content"] == "conv-a message");
        assert!(
            !bleed,
            "conv-a history must not appear in conv-b's LLM call"
        );
    }

    // ── Multiple tool-call tests ───────────────────────────────────────────────

    /// Build an Ollama response that contains `tool_calls` for the given skill names.
    fn ollama_tool_calls(names: &[&str]) -> Value {
        let calls: Vec<Value> = names
            .iter()
            .map(|n| json!({ "function": { "name": n, "arguments": {} } }))
            .collect();
        json!({
            "model": "test",
            "message": { "role": "assistant", "content": null, "tool_calls": calls },
            "done": true
        })
    }

    /// A single unknown tool call causes one observation and then one more LLM
    /// call that receives that observation in its messages.
    #[tokio::test]
    async fn single_tool_call_adds_observation_to_next_request() {
        let server = MockServer::start().await;

        // First call: one tool call for an unregistered skill.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["unknown-skill"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Second call: final answer.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        let result = orch
            .run_turn("go", Uuid::new_v4(), Interface::Cli)
            .await
            .unwrap();
        assert_eq!(result.answer, "done");

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 2, "expected exactly 2 LLM calls");

        // Second request must contain a tool observation mentioning the skill.
        let msgs = messages_in(&reqs[1]);
        let has_obs = msgs.iter().any(|m| {
            m["role"] == "tool"
                && m["content"]
                    .as_str()
                    .unwrap_or("")
                    .contains("unknown-skill")
        });
        assert!(
            has_obs,
            "second LLM call should contain the tool observation; msgs: {msgs:?}"
        );
    }

    /// Two tool calls returned in a single LLM response are both executed
    /// within the same iteration — exactly one additional LLM round-trip
    /// follows (not two).
    #[tokio::test]
    async fn two_tool_calls_handled_in_single_iteration() {
        let server = MockServer::start().await;

        // First call: two tool calls for unregistered skills.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(ollama_tool_calls(&["skill-a", "skill-b"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Second call: final answer.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        orch.run_turn("go", Uuid::new_v4(), Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            2,
            "two tool calls must be handled in ONE iteration — expected 2 LLM calls, got {}",
            reqs.len()
        );
    }

    /// After two simultaneous tool calls the second LLM request contains
    /// an observation for each call.
    #[tokio::test]
    async fn two_tool_calls_both_observations_sent_to_llm() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(ollama_tool_calls(&["skill-a", "skill-b"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        orch.run_turn("go", Uuid::new_v4(), Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        let msgs = messages_in(&reqs[1]);

        let tool_msgs: Vec<&Value> = msgs.iter().filter(|m| m["role"] == "tool").collect();
        assert_eq!(
            tool_msgs.len(),
            2,
            "expected 2 tool observation messages in second LLM call, got {}: {msgs:?}",
            tool_msgs.len()
        );

        let content_a = tool_msgs[0]["content"].as_str().unwrap_or("");
        let content_b = tool_msgs[1]["content"].as_str().unwrap_or("");
        assert!(
            content_a.contains("skill-a"),
            "first observation should mention skill-a; got: {content_a}"
        );
        assert!(
            content_b.contains("skill-b"),
            "second observation should mention skill-b; got: {content_b}"
        );
    }

    /// Three simultaneous tool calls are all handled within one iteration.
    #[tokio::test]
    async fn three_tool_calls_handled_in_single_iteration() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["s1", "s2", "s3"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        orch.run_turn("go", Uuid::new_v4(), Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            2,
            "three tool calls must collapse into one iteration"
        );

        let msgs = messages_in(&reqs[1]);
        let tool_count = msgs.iter().filter(|m| m["role"] == "tool").count();
        assert_eq!(
            tool_count, 3,
            "expected 3 tool observations; msgs: {msgs:?}"
        );
    }

    /// When tool calls are present, the second LLM request must contain an
    /// assistant message with a `tool_calls` array — not a plain text message.
    /// This validates the Ollama multi-turn tool-calling wire format.
    #[tokio::test]
    async fn tool_call_assistant_message_in_history() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["my-skill"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        orch.run_turn("go", Uuid::new_v4(), Interface::Cli)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        let msgs = messages_in(&reqs[1]);

        // There must be exactly one assistant message that carries `tool_calls`.
        let assistant_with_calls: Vec<&Value> = msgs
            .iter()
            .filter(|m| m["role"] == "assistant" && m["tool_calls"].is_array())
            .collect();
        assert_eq!(
            assistant_with_calls.len(),
            1,
            "expected one assistant/tool_calls message in second request; msgs: {msgs:?}"
        );

        // The tool_calls array must name the skill.
        let tc = &assistant_with_calls[0]["tool_calls"][0]["function"]["name"];
        assert_eq!(tc, "my-skill");
    }

    // ── run_turn_with_tools history tests ────────────────────────────────────

    /// `run_turn_with_tools` must persist the assistant's tool-call message and
    /// the tool result to the database so that a subsequent turn sends the full
    /// conversation history to the LLM — including those entries.
    ///
    /// Sequence verified:
    ///   Turn 1 LLM request:  [system, user("msg1")]
    ///   Turn 1 LLM response: tool_calls = [test-reply]
    ///   → handler runs; replied=true; turn 1 exits
    ///   Turn 2 LLM request:  [system, user("msg1"),
    ///                          assistant{tool_calls:[test-reply]},
    ///                          tool{name:test-reply, content:"replied"},
    ///                          user("msg2")]
    #[tokio::test]
    async fn run_turn_with_tools_preserves_full_history_across_turns() {
        let server = MockServer::start().await;

        // Turn 1: LLM calls test-reply → replied=true → turn ends.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["test-reply"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Turn 2: LLM calls end_turn → turn ends.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["end_turn"])),
            )
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        let ext: Vec<(SkillDef, Arc<dyn assistant_core::SkillHandler>)> =
            vec![(test_reply_def(), Arc::new(NoopReplyHandler))];

        // Turn 1.
        orch.run_turn_with_tools("msg1", conv_id, Interface::Slack, ext.clone())
            .await
            .unwrap();

        // Turn 2.
        orch.run_turn_with_tools("msg2", conv_id, Interface::Slack, ext)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            2,
            "expected exactly 2 LLM calls, got {}",
            reqs.len()
        );

        // Inspect turn 2's request.
        let msgs = messages_in(&reqs[1]);
        // [system, user(msg1), assistant{tool_calls}, tool(test-reply), user(msg2)]
        assert_eq!(
            msgs.len(),
            5,
            "turn 2 should carry 5 messages (system + 3 from turn-1 history + user(msg2)); got {msgs:?}"
        );

        assert_eq!(msgs[0]["role"], "system");

        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "msg1");

        // The assistant message from turn 1 must be a tool-call message, not
        // plain text — this is the Ollama multi-turn wire format requirement.
        assert_eq!(msgs[2]["role"], "assistant");
        assert!(
            msgs[2]["tool_calls"].is_array(),
            "turn-1 assistant message must carry tool_calls, not plain content; msgs: {msgs:?}"
        );
        assert_eq!(
            msgs[2]["tool_calls"][0]["function"]["name"], "test-reply",
            "tool call must name test-reply; msgs: {msgs:?}"
        );

        // The tool result from turn 1.
        assert_eq!(msgs[3]["role"], "tool");
        let obs = msgs[3]["content"].as_str().unwrap_or("");
        assert!(
            obs.contains("replied"),
            "tool observation must echo the handler output; got: {obs}"
        );

        // Current user turn.
        assert_eq!(msgs[4]["role"], "user");
        assert_eq!(msgs[4]["content"], "msg2");
    }

    /// When `run_turn_with_tools` is used across THREE turns, all prior
    /// tool-call/result pairs from previous turns appear in the third turn's
    /// LLM request.
    #[tokio::test]
    async fn run_turn_with_tools_accumulates_history_over_three_turns() {
        let server = MockServer::start().await;

        // Turns 1 and 2 each call test-reply; turn 3 calls end_turn.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["test-reply"])),
            )
            .up_to_n_times(2)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["end_turn"])),
            )
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        let ext: Vec<(SkillDef, Arc<dyn assistant_core::SkillHandler>)> =
            vec![(test_reply_def(), Arc::new(NoopReplyHandler))];

        orch.run_turn_with_tools("turn1", conv_id, Interface::Slack, ext.clone())
            .await
            .unwrap();
        orch.run_turn_with_tools("turn2", conv_id, Interface::Slack, ext.clone())
            .await
            .unwrap();
        orch.run_turn_with_tools("turn3", conv_id, Interface::Slack, ext)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 3);

        // Third turn: [system,
        //   user(turn1), assistant{tool_calls}, tool(test-reply),
        //   user(turn2), assistant{tool_calls}, tool(test-reply),
        //   user(turn3)]
        let msgs = messages_in(&reqs[2]);
        assert_eq!(
            msgs.len(),
            8,
            "turn 3 should carry 8 messages; got {msgs:?}"
        );

        // Two assistant tool-call messages should be present (one per prior turn).
        let tc_count = msgs
            .iter()
            .filter(|m| m["role"] == "assistant" && m["tool_calls"].is_array())
            .count();
        assert_eq!(
            tc_count, 2,
            "expected 2 assistant tool-call messages in turn-3 history; msgs: {msgs:?}"
        );

        // Two tool result messages should be present.
        let tr_count = msgs.iter().filter(|m| m["role"] == "tool").count();
        assert_eq!(
            tr_count, 2,
            "expected 2 tool-result messages in turn-3 history; msgs: {msgs:?}"
        );

        // The last message must be the current turn's user message.
        assert_eq!(msgs.last().unwrap()["role"], "user");
        assert_eq!(msgs.last().unwrap()["content"], "turn3");
    }

    // ── Memory-reload tests ───────────────────────────────────────────────────

    /// Regression: the system prompt must be reloaded from disk at the start of
    /// every turn so that memory-skill writes (soul-update, memory-patch, …) are
    /// visible to the LLM in subsequent turns.
    ///
    /// Concretely: if USER.md changes between turn 1 and turn 2, the system
    /// prompt sent for turn 2 must contain the new content.
    #[tokio::test]
    async fn system_prompt_reflects_memory_file_changes_between_turns() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        // Create an isolated temp directory with a single USER.md file.
        let dir = std::env::temp_dir().join(format!("assistant-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let user_md = dir.join("USER.md");
        std::fs::write(&user_md, "# User\n- **Name:** Alice\n").unwrap();

        // Build an orchestrator with memory enabled and custom file paths.
        // SOUL.md / IDENTITY.md / MEMORY.md are intentionally absent — they are
        // skipped silently by load_system_prompt().
        let mut config = AssistantConfig::default();
        config.memory.enabled = true;
        config.memory.user_path = Some(user_md.to_str().unwrap().to_string());
        config.memory.soul_path = Some(dir.join("SOUL.md").to_str().unwrap().to_string());
        config.memory.identity_path = Some(dir.join("IDENTITY.md").to_str().unwrap().to_string());
        config.memory.memory_path = Some(dir.join("MEMORY.md").to_str().unwrap().to_string());
        config.memory.notes_dir = Some(dir.to_str().unwrap().to_string());

        let (orch, _) = build_with_config(&server.uri(), config).await;
        let conv_id = Uuid::new_v4();

        // Turn 1 — USER.md still says "Alice".
        orch.run_turn("hello", conv_id, Interface::Cli)
            .await
            .unwrap();

        // Simulate a memory-skill write between turns.
        std::fs::write(&user_md, "# User\n- **Name:** Bob\n").unwrap();

        // Turn 2 — must pick up the updated USER.md.
        orch.run_turn("hello again", conv_id, Interface::Cli)
            .await
            .unwrap();

        // Clean up before assertions so a panic doesn't leave stale files.
        std::fs::remove_dir_all(&dir).ok();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 2);

        let prompt1 = messages_in(&reqs[0])[0]["content"]
            .as_str()
            .unwrap()
            .to_string();
        let prompt2 = messages_in(&reqs[1])[0]["content"]
            .as_str()
            .unwrap()
            .to_string();

        assert!(
            prompt1.contains("Alice"),
            "turn 1 system prompt should contain 'Alice'; got:\n{prompt1}"
        );
        assert!(
            prompt2.contains("Bob"),
            "turn 2 system prompt should contain updated name 'Bob'; got:\n{prompt2}"
        );
        assert!(
            !prompt2.contains("Alice"),
            "turn 2 system prompt must not contain stale name 'Alice'; got:\n{prompt2}"
        );
    }
}
