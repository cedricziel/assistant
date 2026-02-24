//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, tool executor, and skill registry.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ExecutionContext, ExecutionTrace, Interface, MemoryLoader, Message, MessageRole, ToolHandler,
};
use assistant_llm::{ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse, ToolSpec};
use assistant_storage::{conversations::ConversationStore, StorageLayer};
use assistant_tool_executor::ToolExecutor;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

// ── Public types ──────────────────────────────────────────────────────────────

/// Callback trait for requesting user confirmation before executing a tool.
/// Typically implemented by the CLI interface.
pub trait ConfirmationCallback: Send + Sync {
    /// Return `true` if the user confirms execution of `tool_name` with
    /// `params`, or `false` to deny.
    fn confirm(&self, tool_name: &str, params: &serde_json::Value) -> bool;
}

/// The result of a single orchestrator turn.
pub struct TurnResult {
    /// The assistant's final answer to the user.
    pub answer: String,
    /// All tool execution traces collected during this turn.
    pub traces: Vec<ExecutionTrace>,
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

// ── Built-in extension tools ──────────────────────────────────────────────────

/// Build the `end_turn` ToolSpec that `run_turn_with_tools` always injects.
///
/// The tool carries no real handler — the orchestrator loop detects it by name
/// and exits cleanly.  Exposing it as a proper tool gives the LLM a first-class,
/// typed way to signal "I'm done" without having to return a plain FinalAnswer.
fn end_turn_spec() -> ToolSpec {
    ToolSpec {
        name: "end_turn".to_string(),
        description: "Signal that this turn is complete. Call this once you have sent your reply \
             (or decided no reply is needed). The `reason` field is optional and used for \
             logging only."
            .to_string(),
        params_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "reason": {
                    "type": "string",
                    "description": "Brief reason the turn is ending (e.g. \"replied\", \"no reply needed\"). Used for logging only."
                }
            }
        }),
        is_mutating: false,
        requires_confirmation: false,
    }
}

/// Drives the tool-calling loop for a single conversation turn.
///
/// Each call to [`run_turn`] performs the following high-level algorithm:
///
/// 1. Ensure a conversation row exists in SQLite.
/// 2. Persist the user message.
/// 3. Load all registered tool specs from the executor.
/// 4. Repeatedly call the LLM until it returns a `FinalAnswer` or the
///    iteration limit is reached.
/// 5. For each `ToolCall` response: check disabled-skills list, optionally
///    confirm with the user, execute the tool, record an [`ExecutionTrace`],
///    and append an `OBSERVATION` to the conversation history.
/// 6. Persist the final assistant message and return [`TurnResult`].
pub struct Orchestrator {
    llm: Arc<dyn LlmProvider>,
    storage: Arc<StorageLayer>,
    executor: Arc<ToolExecutor>,
    max_iterations: usize,
    disabled_skills: Vec<String>,
    trace_enabled: bool,
    confirmation_callback: Option<Arc<dyn ConfirmationCallback>>,
    /// Memory loader used to rebuild the system prompt at the start of every
    /// turn so that writes made by memory tools are reflected immediately.
    memory_loader: MemoryLoader,
}

impl Orchestrator {
    /// Create a new orchestrator.
    ///
    /// # Parameters
    /// * `llm` — the LLM client (Ollama wrapper)
    /// * `storage` — the SQLite storage layer
    /// * `executor` — tool executor (dispatches to all registered ToolHandlers)
    /// * `config` — assistant configuration (controls iteration limit, disabled
    ///   skills, and trace logging)
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        storage: Arc<StorageLayer>,
        executor: Arc<ToolExecutor>,
        config: &assistant_core::AssistantConfig,
    ) -> Self {
        let memory_loader = MemoryLoader::new(config);
        memory_loader.ensure_defaults();
        Self {
            llm,
            storage,
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

    /// Process one turn of the conversation with per-turn extension tools.
    ///
    /// Extension tools are injected by the calling interface (e.g. Slack,
    /// Mattermost) and are checked before the global tool executor.  They
    /// bypass the disabled-skills list — the interface is responsible for vetting
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
    /// * `extensions` — per-turn `Arc<dyn ToolHandler>` pairs; names must be
    ///   unique and must not collide with global tool names
    pub async fn run_turn_with_tools(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<Arc<dyn ToolHandler>>,
    ) -> Result<()> {
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            extension_count = extensions.len(),
            "Starting turn with extension tools"
        );

        // Build extension lookup: name → handler.
        let ext_map: HashMap<String, Arc<dyn ToolHandler>> = extensions
            .iter()
            .map(|h| (h.name().to_string(), h.clone()))
            .collect();

        // Build extension ToolSpecs for LLM listing.
        let mut ext_specs: Vec<ToolSpec> = extensions
            .iter()
            .map(|h| ToolSpec {
                name: h.name().to_string(),
                description: h.description().to_string(),
                params_schema: h.params_schema(),
                is_mutating: h.is_mutating(),
                requires_confirmation: h.requires_confirmation(),
            })
            .collect();

        // Always inject `end_turn` unless the caller already provided one.
        if !ext_specs.iter().any(|s| s.name == "end_turn") && !ext_map.contains_key("end_turn") {
            ext_specs.push(end_turn_spec());
        }

        // 1-3. Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) =
            self.prepare_history(user_message, conversation_id).await?;

        // 4. Load global tool specs and merge with extensions for LLM tool listing.
        //    Extension specs come first so the LLM sees them prominently.
        let global_specs = self.executor.to_specs();
        let all_specs: Vec<ToolSpec> = ext_specs
            .iter()
            .cloned()
            .chain(global_specs.into_iter())
            .collect();

        let base_system_prompt = self.memory_loader.load_system_prompt();
        // When extension tools are present, guide the LLM to use them.
        let system_prompt = if ext_specs.is_empty() {
            base_system_prompt
        } else {
            let reply_tools: Vec<&str> = ext_specs
                .iter()
                .filter(|s| s.name.contains("reply") || s.name.contains("post"))
                .map(|s| s.name.as_str())
                .collect();
            let react_tools: Vec<&str> = ext_specs
                .iter()
                .filter(|s| s.name.contains("react"))
                .map(|s| s.name.as_str())
                .collect();
            let ack_instruction = match (!reply_tools.is_empty(), !react_tools.is_empty()) {
                (true, true) => format!(
                    "Before calling `end_turn` you MUST always acknowledge the user's message: \
                     use `{}` for text responses, or `{}` for brief emoji acknowledgements \
                     (e.g. `thumbsup`, `white_check_mark`). ",
                    reply_tools.join("` or `"),
                    react_tools.join("` or `")
                ),
                (true, false) => format!(
                    "Before calling `end_turn` you MUST always reply using `{}`. ",
                    reply_tools.join("` or `")
                ),
                (false, true) => format!(
                    "Before calling `end_turn` you MUST always acknowledge using `{}`. ",
                    react_tools.join("` or `")
                ),
                (false, false) => String::new(),
            };
            format!(
                "{base_system_prompt}\n\n---\n\n\
                You are operating inside a messaging interface. \
                {ack_instruction}\
                When you have finished, call `end_turn` to signal completion."
            )
        };

        let mut traces: Vec<ExecutionTrace> = Vec::new();
        let mut turn_ended = false;
        let mut replied = false;

        // 5. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Extension-tools loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: false,
            };

            let response = self.llm.chat(&system_prompt, &history, &all_specs).await?;

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text) => {
                    let assistant_msg = {
                        let mut m = assistant_core::Message::assistant(conversation_id, &text);
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    if replied {
                        return Ok(());
                    }

                    // Don't attempt to auto-post an empty answer — this causes
                    // messaging APIs (e.g. Slack) to reject with "no_text".
                    // This can happen with thinking models (e.g. qwen3) when the
                    // model produces a reasoning block but no visible reply text.
                    if text.trim().is_empty() {
                        warn!(
                            iteration,
                            "LLM returned empty final answer; skipping auto-post"
                        );
                        return Ok(());
                    }

                    // If a reply-capable extension tool exists, use it to forward
                    // the answer to the user.
                    let reply_entry = ext_map
                        .iter()
                        .find(|(name, _)| name.contains("reply") && !name.contains("blocks"))
                        .or_else(|| {
                            ext_map
                                .iter()
                                .find(|(name, _)| name.contains("reply") || name.contains("post"))
                        });

                    if let Some((reply_name, reply_handler)) = reply_entry {
                        info!(
                            iteration,
                            tool = %reply_name,
                            "LLM returned final answer; auto-posting via extension reply tool"
                        );
                        let mut params_map = HashMap::new();
                        // Determine which single text parameter the reply tool expects.
                        // Only auto-post when there is exactly one required field and it
                        // is a recognised text-like name; skip otherwise to avoid silent
                        // failures with multi-param or non-text reply tools.
                        let schema = reply_handler.params_schema();
                        let text_param = schema
                            .get("required")
                            .and_then(|r| r.as_array())
                            .and_then(|r| if r.len() == 1 { r[0].as_str() } else { None })
                            .filter(|name| matches!(*name, "text" | "content" | "message"));
                        let Some(text_param) = text_param else {
                            warn!(
                                tool = %reply_name,
                                "Auto-post skipped: reply tool requires multiple or non-text params"
                            );
                            return Ok(());
                        };
                        params_map.insert(
                            text_param.to_string(),
                            serde_json::Value::String(text.clone()),
                        );
                        let ctx2 = ExecutionContext {
                            conversation_id,
                            turn: iteration as i64,
                            interface: interface.clone(),
                            interactive: false,
                        };
                        if let Err(e) = reply_handler.run(params_map, &ctx2).await {
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
                        iteration, "LLM requested tool execution(s)"
                    );

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

                    let has_real_calls = tool_call_items.iter().any(|t| t.name != "end_turn");

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;

                        if name == "end_turn" {
                            if has_real_calls {
                                info!(
                                    iteration,
                                    "end_turn deferred (called alongside other tools)"
                                );
                                let deferred_msg =
                                    "end_turn deferred: processing other tool calls first";
                                self.append_tool_result(&mut history, "end_turn", deferred_msg);
                                let tr_msg = Self::make_tool_result_message(
                                    conversation_id,
                                    base_turn + iteration as i64 + 1,
                                    "end_turn",
                                    deferred_msg,
                                );
                                if let Err(e) = conv_store.save_message(&tr_msg).await {
                                    warn!("Failed to persist deferred end_turn tool-result: {e}");
                                }
                                continue;
                            }

                            let reason = params
                                .get("reason")
                                .and_then(|v| v.as_str())
                                .unwrap_or("done");
                            info!(iteration, reason, "end_turn called; stopping turn");

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
                        let (observation, trace_result) = if let Some(handler) = ext_map.get(&name)
                        {
                            debug!(tool = %name, "Dispatching to extension handler");

                            let params_map: HashMap<String, serde_json::Value> =
                                if let serde_json::Value::Object(map) = &params {
                                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                } else {
                                    HashMap::new()
                                };

                            let start = std::time::Instant::now();
                            let exec_result = handler.run(params_map, &ctx).await;
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
                                    warn!(tool = %name, %err, "Extension tool execution failed");
                                    let msg = err.to_string();
                                    trace = trace.with_error(msg.clone(), duration_ms);
                                    format!("Error executing '{name}': {msg}")
                                }
                            };
                            (obs, trace)
                        } else {
                            // Global executor path.
                            if self
                                .reject_if_disabled(
                                    &name,
                                    &mut history,
                                    &conv_store,
                                    conversation_id,
                                    base_turn + iteration as i64 + 1,
                                )
                                .await
                            {
                                continue;
                            }

                            // Confirmation gate.
                            let requires_confirm = self
                                .executor
                                .list_tools()
                                .iter()
                                .find(|h| h.name() == name)
                                .map(|h| h.requires_confirmation())
                                .unwrap_or(false);

                            if requires_confirm && ctx.interactive {
                                if let Some(cb) = &self.confirmation_callback {
                                    if !cb.confirm(&name, &params) {
                                        let observation =
                                            format!("User denied execution of '{name}'.");
                                        info!(%observation);
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
                                        continue;
                                    }
                                }
                            }

                            let params_map: HashMap<String, serde_json::Value> =
                                if let serde_json::Value::Object(map) = &params {
                                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                } else {
                                    HashMap::new()
                                };

                            let start = std::time::Instant::now();
                            let exec_result = self.executor.execute(&name, params_map, &ctx).await;
                            let duration_ms = start.elapsed().as_millis() as i64;

                            let mut trace = ExecutionTrace::new(
                                conversation_id,
                                iteration as i64,
                                &name,
                                params.clone(),
                            );
                            let obs = match exec_result {
                                Ok(output) => {
                                    debug!(tool = %name, duration_ms, "Tool execution completed");
                                    trace = trace.with_success(output.content.clone(), duration_ms);
                                    tool_result_content(&output.content, output.data.as_ref())
                                }
                                Err(err) => {
                                    warn!(tool = %name, %err, "Tool execution failed");
                                    let msg = err.to_string();
                                    trace = trace.with_error(msg.clone(), duration_ms);
                                    format!("Error executing '{name}': {msg}")
                                }
                            };
                            (obs, trace)
                        };

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

        // 4. Load all registered tool specs.
        let tool_specs = self.executor.to_specs();

        // 5. Build the system prompt fresh from disk.
        let system_prompt = self.memory_loader.load_system_prompt();

        let mut traces: Vec<ExecutionTrace> = Vec::new();

        // 6. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Tool-calling loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
            };

            let response = self.llm.chat(&system_prompt, &history, &tool_specs).await?;

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text) => {
                    info!(iteration, "LLM returned final answer");

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
                        iteration, "LLM requested tool execution(s)"
                    );

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

                        // Disabled-tools gate.
                        if self
                            .reject_if_disabled(
                                &name,
                                &mut history,
                                &conv_store,
                                conversation_id,
                                base_turn + iteration as i64 + 1,
                            )
                            .await
                        {
                            continue;
                        }

                        // Confirmation gate.
                        let requires_confirm = self
                            .executor
                            .list_tools()
                            .iter()
                            .find(|h| h.name() == name)
                            .map(|h| h.requires_confirmation())
                            .unwrap_or(false);

                        if requires_confirm && ctx.interactive {
                            if let Some(cb) = &self.confirmation_callback {
                                if !cb.confirm(&name, &params) {
                                    let observation = format!("User denied execution of '{name}'.");
                                    info!(%observation);
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
                                    continue;
                                }
                            }
                        }

                        let params_map: HashMap<String, serde_json::Value> =
                            if let serde_json::Value::Object(map) = &params {
                                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                            } else {
                                HashMap::new()
                            };

                        let start = std::time::Instant::now();
                        let exec_result = self.executor.execute(&name, params_map, &ctx).await;
                        let duration_ms = start.elapsed().as_millis() as i64;

                        let mut trace = ExecutionTrace::new(
                            conversation_id,
                            iteration as i64,
                            &name,
                            params.clone(),
                        );

                        let observation = match exec_result {
                            Ok(output) => {
                                debug!(
                                    tool = %name,
                                    duration_ms,
                                    success = output.success,
                                    "Tool execution completed"
                                );
                                trace = trace.with_success(output.content.clone(), duration_ms);
                                tool_result_content(&output.content, output.data.as_ref())
                            }
                            Err(err) => {
                                warn!(tool = %name, %err, "Tool execution failed");
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

        let (conv_store, mut history, base_turn) =
            self.prepare_history(user_message, conversation_id).await?;

        let tool_specs = self.executor.to_specs();

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

            let response = self
                .llm
                .chat_streaming(
                    &system_prompt,
                    &history,
                    &tool_specs,
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
                        iteration, "Streaming LLM requested tool execution(s)"
                    );

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

                        if self
                            .reject_if_disabled(
                                &name,
                                &mut history,
                                &conv_store,
                                conversation_id,
                                base_turn + iteration as i64 + 1,
                            )
                            .await
                        {
                            continue;
                        }

                        let requires_confirm = self
                            .executor
                            .list_tools()
                            .iter()
                            .find(|h| h.name() == name)
                            .map(|h| h.requires_confirmation())
                            .unwrap_or(false);

                        if requires_confirm && ctx.interactive {
                            if let Some(cb) = &self.confirmation_callback {
                                if !cb.confirm(&name, &params) {
                                    let observation = format!("User denied execution of '{name}'.");
                                    info!(%observation);
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
                                    continue;
                                }
                            }
                        }

                        let params_map: HashMap<String, serde_json::Value> =
                            if let serde_json::Value::Object(map) = &params {
                                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                            } else {
                                HashMap::new()
                            };

                        let start = std::time::Instant::now();
                        let exec_result = self.executor.execute(&name, params_map, &ctx).await;
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
                                warn!(tool = %name, %err, "Tool execution failed");
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

    fn append_tool_result(&self, history: &mut Vec<ChatHistoryMessage>, name: &str, content: &str) {
        history.push(ChatHistoryMessage::ToolResult {
            name: name.to_string(),
            content: content.to_string(),
        });
    }

    async fn reject_if_disabled(
        &self,
        name: &str,
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_idx: i64,
    ) -> bool {
        if !self.disabled_skills.iter().any(|s| s == name) {
            return false;
        }
        let observation = format!("Tool '{name}' is disabled by configuration.");
        warn!(%observation);
        self.append_tool_result(history, name, &observation);
        let tr_msg = Self::make_tool_result_message(conversation_id, turn_idx, name, &observation);
        if let Err(e) = conv_store.save_message(&tr_msg).await {
            warn!("Failed to persist tool-result message: {e}");
        }
        true
    }

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

    fn make_tool_result_message(
        conversation_id: Uuid,
        turn: i64,
        tool_name: &str,
        observation: &str,
    ) -> Message {
        let mut m = Message::new(conversation_id, MessageRole::Tool, observation);
        m.turn = turn;
        m.skill_name = Some(tool_name.to_string());
        m
    }
}

// ── Module-level helpers ───────────────────────────────────────────────────────

/// Build the tool result content from a tool output.
///
/// Always returns the human-readable `content` string so the LLM receives
/// a consistent, formatted observation. The structured `data` field is for
/// downstream callers that need machine-readable output; it is not sent to
/// the model directly.
fn tool_result_content(content: &str, _data: Option<&serde_json::Value>) -> String {
    content.to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assistant_core::{types::Interface, AssistantConfig};
    use assistant_llm::{LlmClient, LlmClientConfig, LlmProvider};
    use assistant_storage::{registry::SkillRegistry, StorageLayer};
    use assistant_tool_executor::ToolExecutor;
    use serde_json::{json, Value};
    use uuid::Uuid;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use super::Orchestrator;

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
    async fn build(base_url: &str) -> (Arc<Orchestrator>, Arc<StorageLayer>) {
        let mut config = AssistantConfig::default();
        config.memory.enabled = false;
        build_with_config(base_url, config).await
    }

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
        let executor = Arc::new(ToolExecutor::new(
            storage.clone(),
            llm.clone(),
            registry.clone(),
            Arc::new(config.clone()),
        ));
        let orch = Arc::new(Orchestrator::new(llm, storage.clone(), executor, &config));
        (orch, storage)
    }

    /// Extract the `messages` array from an intercepted Ollama request body.
    fn messages_in(req: &wiremock::Request) -> Vec<Value> {
        let body: Value = serde_json::from_slice(&req.body).unwrap();
        body["messages"].as_array().cloned().unwrap_or_default()
    }

    // ── Tests ─────────────────────────────────────────────────────────────────

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
        assert_eq!(msgs.len(), 2, "expected [system, user], got {msgs:?}");
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "hello");
    }

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
        assert_eq!(msgs.len(), 4, "expected 4 messages on turn 2, got {msgs:?}");
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "first message");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[2]["content"], "pong");
        assert_eq!(msgs[3]["role"], "user");
        assert_eq!(msgs[3]["content"], "second message");
    }

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

    #[tokio::test]
    async fn seeded_history_included_in_llm_call() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        let (orch, storage) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        let conv_store = storage.conversation_store();
        conv_store
            .create_conversation_with_id(conv_id, Some("slack:C001:1234"))
            .await
            .unwrap();

        let mut seed_user = assistant_core::Message::user(conv_id, "seeded user message");
        seed_user.turn = 0;
        conv_store.save_message(&seed_user).await.unwrap();

        let mut seed_bot = assistant_core::Message::assistant(conv_id, "seeded bot reply");
        seed_bot.turn = 1;
        conv_store.save_message(&seed_bot).await.unwrap();

        orch.run_turn("follow-up", conv_id, Interface::Slack)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 1);

        let msgs = messages_in(&reqs[0]);
        assert_eq!(msgs.len(), 4, "expected 4 messages, got {msgs:?}");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[1]["content"], "seeded user message");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[2]["content"], "seeded bot reply");
        assert_eq!(msgs[3]["role"], "user");
        assert_eq!(msgs[3]["content"], "follow-up");
    }

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

        let msgs = messages_in(&reqs[2]);
        assert_eq!(msgs.len(), 6, "expected 6 messages on turn 3, got {msgs:?}");
        assert_eq!(msgs[1]["content"], "turn 1");
        assert_eq!(msgs[2]["content"], "reply");
        assert_eq!(msgs[3]["content"], "turn 2");
        assert_eq!(msgs[4]["content"], "reply");
        assert_eq!(msgs[5]["content"], "turn 3");
    }

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

        let msgs_b = messages_in(&reqs[1]);
        let bleed = msgs_b.iter().any(|m| m["content"] == "conv-a message");
        assert!(
            !bleed,
            "conv-a history must not appear in conv-b's LLM call"
        );
    }

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

    #[tokio::test]
    async fn single_tool_call_adds_observation_to_next_request() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["unknown-skill"])),
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
        let result = orch
            .run_turn("go", Uuid::new_v4(), Interface::Cli)
            .await
            .unwrap();
        assert_eq!(result.answer, "done");

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 2, "expected exactly 2 LLM calls");

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

    #[tokio::test]
    async fn two_tool_calls_handled_in_single_iteration() {
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
        assert_eq!(
            reqs.len(),
            2,
            "two tool calls must be handled in ONE iteration — expected 2 LLM calls, got {}",
            reqs.len()
        );
    }

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
            "three tool calls must be handled in ONE iteration"
        );
    }
}
