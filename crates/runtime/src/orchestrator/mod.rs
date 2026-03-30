//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, tool executor, and skill registry.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    context::agent_base_dir, strip_html_comments, Attachment, ExecutionContext, Interface,
    MemoryLoader, Message, MessageBus, ToolHandler,
};
use assistant_llm::{
    ChatHistoryMessage, ChatRole, ContentBlock, LlmProvider, LlmResponse, ToolSpec,
};
use assistant_storage::{conversations::ConversationStore, SkillRegistry, StorageLayer};
use assistant_tool_executor::ToolExecutor;
use opentelemetry::{
    global,
    trace::{Span as _, Status as OtelStatus, TraceContextExt, Tracer as _},
    Context as OtelContext, KeyValue,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, info_span, warn, Instrument};
use uuid::Uuid;

// ── Submodules ────────────────────────────────────────────────────────────────

mod dispatch;
mod prompt;
mod turn_control;

mod subagent;
mod worker;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

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
    /// File attachments collected from tool outputs during the turn.
    ///
    /// Interfaces should deliver these to the user (e.g. save to disk in the
    /// CLI, upload in Slack/Mattermost).
    pub attachments: Vec<Attachment>,
}

// ── Internal enums ────────────────────────────────────────────────────────────

/// Outcome of processing an `end_turn` tool call.
pub(crate) enum EndTurnOutcome {
    /// `end_turn` was called alongside real tool calls — deferred.
    Deferred,
    /// `end_turn` rejected because a reply tool is available but not yet used.
    Rejected,
    /// `end_turn` accepted; the turn is complete.
    Accepted,
}

/// Outcome of processing a `FinalAnswer` from the LLM when extension tools
/// are active.
pub(crate) enum FinalAnswerOutcome {
    Done(TurnResult),
    Retry,
}

/// Outcome of dispatching a single global (executor) tool call.
pub(crate) enum DispatchOutcome {
    /// The user denied execution via the confirmation gate.
    Denied,
    /// The tool was executed and its result finalized.
    Executed,
}

/// Per-conversation extension tool registration consumed by the worker.
///
/// Interfaces (Slack, Mattermost) register their per-turn tools and
/// attachments before publishing a `TurnRequest` to the bus.  The worker
/// removes the registration when processing the request.
#[derive(Clone)]
pub(crate) struct ExtensionRegistration {
    pub(crate) tools: Vec<Arc<dyn ToolHandler>>,
    pub(crate) attachments: Vec<ContentBlock>,
}

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

/// Parse an interface string back to the [`Interface`] enum.
///
/// Matches the `Debug` format that the codebase uses for serialisation
/// (e.g. `"Cli"`, `"Slack"`).  Falls back to [`Interface::Cli`] for
/// unknown values.
pub(crate) fn parse_interface(s: &str) -> Interface {
    match s.to_lowercase().as_str() {
        "cli" => Interface::Cli,
        "signal" => Interface::Signal,
        "mcp" => Interface::Mcp,
        "slack" => Interface::Slack,
        "mattermost" => Interface::Mattermost,
        "nextcloud" => Interface::Nextcloud,
        "matrix" => Interface::Matrix,
        "web" => Interface::Web,
        "scheduler" => Interface::Scheduler,
        _ => Interface::Cli,
    }
}

// ── Orchestrator ──────────────────────────────────────────────────────────────

/// Drives the tool-calling loop for a single conversation turn.
///
/// Each call to [`run_turn`] performs the following high-level algorithm:
///
/// 1. Ensure a conversation row exists in SQLite.
/// 2. Persist the user message.
/// 3. Load all registered tool specs from the executor.
/// 4. Repeatedly call the LLM until it returns a `FinalAnswer` or the
///    iteration limit is reached.
/// 5. For each `ToolCall` response: optionally confirm with the user,
///    execute the tool, emit an OpenTelemetry span, and append an
///    `OBSERVATION` to the conversation history.
/// 6. Persist the final assistant message and return [`TurnResult`].
pub struct Orchestrator {
    /// The LLM provider used for chat and embeddings.
    pub llm: Arc<dyn LlmProvider>,
    pub(crate) storage: Arc<StorageLayer>,
    pub(crate) executor: Arc<ToolExecutor>,
    pub(crate) registry: Arc<SkillRegistry>,
    /// Durable message bus for decoupled inter-component communication.
    pub(crate) bus: Arc<dyn MessageBus>,
    pub(crate) max_iterations: usize,
    pub(crate) confirmation_callback: Option<Arc<dyn ConfirmationCallback>>,
    /// Memory loader used to rebuild the system prompt at the start of every
    /// turn so that writes made by memory tools are reflected immediately.
    pub(crate) memory_loader: MemoryLoader,
    /// When true, record full message content on LLM spans (PII-sensitive).
    pub(crate) trace_content: bool,
    /// Per-conversation token sinks for streaming turns dispatched through
    /// the bus.  Consumed (removed) by the worker when processing.
    pub(crate) token_sinks: tokio::sync::RwLock<HashMap<Uuid, mpsc::Sender<String>>>,
    /// Per-conversation extension tool registrations for interface-specific
    /// turns dispatched through the bus.  Consumed by the worker.
    pub(crate) extension_registrations: tokio::sync::RwLock<HashMap<Uuid, ExtensionRegistration>>,
    /// Cancellation tokens for running subagents, keyed by agent ID.
    pub(crate) agent_cancellations: tokio::sync::RwLock<HashMap<String, CancellationToken>>,
    /// OTel metric instruments for GenAI and operational metrics.
    pub(crate) metrics: crate::MetricsRecorder,
    /// Active assistant agent ID for memory/workspace conversation scoping.
    pub(crate) agent_id: String,
    /// Context compaction configuration.
    pub(crate) compaction_config: assistant_core::CompactionConfig,
}

impl Orchestrator {
    /// Create a new orchestrator.
    pub fn new(
        llm: Arc<dyn LlmProvider>,
        storage: Arc<StorageLayer>,
        executor: Arc<ToolExecutor>,
        registry: Arc<SkillRegistry>,
        bus: Arc<dyn MessageBus>,
        config: &assistant_core::AssistantConfig,
    ) -> Self {
        let memory_loader = MemoryLoader::new(config);
        memory_loader.ensure_defaults();
        Self {
            llm,
            storage,
            executor,
            registry,
            bus,
            max_iterations: config.llm.max_iterations,
            confirmation_callback: None,
            memory_loader,
            trace_content: config.observability.trace_content,
            token_sinks: tokio::sync::RwLock::new(HashMap::new()),
            extension_registrations: tokio::sync::RwLock::new(HashMap::new()),
            agent_cancellations: tokio::sync::RwLock::new(HashMap::new()),
            metrics: crate::MetricsRecorder::new(),
            agent_id: config.agent.id.clone(),
            compaction_config: config.compaction.clone(),
        }
    }

    /// Return a reference to the message bus.
    pub fn bus(&self) -> &Arc<dyn MessageBus> {
        &self.bus
    }

    /// Attach a confirmation callback (used by the CLI interface).
    pub fn with_confirmation_callback(mut self, cb: Arc<dyn ConfirmationCallback>) -> Self {
        self.confirmation_callback = Some(cb);
        self
    }

    /// Return the path to this persona's HEARTBEAT.md.
    ///
    /// Always derived from `agent_id` so it stays consistent even if the
    /// `MemoryLoader` was constructed from a config that didn't explicitly set
    /// `heartbeat_path` (e.g. in tests or when `apply_agent_context` wasn't
    /// called).
    pub fn heartbeat_path(&self) -> std::path::PathBuf {
        agent_base_dir(&self.agent_id).join("HEARTBEAT.md")
    }

    /// Return the path to BOOT.md (per-session startup hook).
    pub fn boot_path(&self) -> &Path {
        self.memory_loader.boot_path()
    }

    /// Run the per-session startup hook (BOOT.md).
    ///
    /// Reads BOOT.md from the configured path.  If the file exists and contains
    /// non-comment, non-empty content, its text is submitted as a single silent
    /// turn through the message bus.
    ///
    /// Call this once per session, before the first interactive turn.  Returns
    /// `Ok(true)` if a boot turn was executed, `Ok(false)` if skipped.
    pub async fn run_boot(
        &self,
        conversation_id: uuid::Uuid,
        interface: Interface,
    ) -> Result<bool> {
        let boot_path = self.memory_loader.boot_path();
        if !boot_path.exists() {
            debug!("No BOOT.md found, skipping startup hook");
            return Ok(false);
        }

        let raw = tokio::fs::read_to_string(boot_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read BOOT.md: {e}"))?;

        let stripped = strip_html_comments(&raw);
        if stripped.is_empty() {
            debug!("BOOT.md is empty or comment-only, skipping startup hook");
            return Ok(false);
        }

        info!(path = %boot_path.display(), "Running BOOT.md startup hook");
        match self
            .submit_turn(&stripped, conversation_id, interface, None)
            .await
        {
            Ok(turn) => {
                info!(
                    answer_len = turn.answer.len(),
                    "BOOT.md startup hook completed"
                );
                Ok(true)
            }
            Err(e) => {
                warn!(error = %e, "BOOT.md startup hook failed");
                Err(e)
            }
        }
    }

    // ── Turn entry points ─────────────────────────────────────────────────────

    /// Process one turn of the conversation with per-turn extension tools.
    ///
    /// Extension tools are injected by the calling interface (e.g. Slack,
    /// Mattermost) and are checked before the global tool executor.
    pub async fn run_turn_with_tools(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<Arc<dyn ToolHandler>>,
        trace_cx: Option<&OtelContext>,
        attachments: Vec<ContentBlock>,
    ) -> Result<TurnResult> {
        let turn_span = info_span!(
            "conversation_turn",
            %conversation_id,
            interface = ?interface,
            extension_tools = extensions.len()
        );
        self.run_turn_with_tools_impl(
            user_message,
            conversation_id,
            interface,
            extensions,
            trace_cx,
            attachments,
        )
        .instrument(turn_span)
        .await
    }

    /// Process one turn of the conversation.
    pub async fn run_turn(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        trace_cx: Option<&OtelContext>,
    ) -> Result<TurnResult> {
        self.run_turn_core(user_message, conversation_id, interface, None, trace_cx)
            .await
    }

    /// Like [`run_turn`] but streams final-answer tokens through `token_sink`.
    pub async fn run_turn_streaming(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        token_sink: mpsc::Sender<String>,
        trace_cx: Option<&OtelContext>,
    ) -> Result<TurnResult> {
        self.run_turn_core(
            user_message,
            conversation_id,
            interface,
            Some(token_sink),
            trace_cx,
        )
        .await
    }

    // ── Turn implementations ──────────────────────────────────────────────────

    async fn run_turn_with_tools_impl(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<Arc<dyn ToolHandler>>,
        trace_cx: Option<&OtelContext>,
        attachments: Vec<ContentBlock>,
    ) -> Result<TurnResult> {
        self.metrics
            .record_turn(&self.agent_id, None, &format!("{interface:?}"));
        info!("Starting turn with extension tools");

        let (_conv_cx, turn_cx) = setup_turn_trace(trace_cx, conversation_id, &interface);

        // Build extension lookup: name -> handler.
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
        let (conv_store, mut history, base_turn) = self
            .prepare_history(user_message, conversation_id, attachments, &self.agent_id)
            .await?;

        // 4. Load global tool specs and merge with extensions.
        let has_reply_ext = ext_specs.iter().any(|s| s.name.contains("reply"));
        let provider_caps = self.llm.capabilities();
        let global_specs = Self::filter_tool_specs(self.executor.to_specs(), &provider_caps);
        let all_specs: Vec<ToolSpec> = ext_specs
            .iter()
            .cloned()
            .chain(
                global_specs
                    .into_iter()
                    .filter(|s| !has_reply_ext || !s.name.contains("post")),
            )
            .collect();

        let base_system_prompt = self.compose_system_prompt().await;
        let system_prompt = Self::build_extension_system_prompt(&base_system_prompt, &ext_specs);

        let mut turn_ended = false;
        let mut replied = false;
        let mut turn_attachments: Vec<Attachment> = Vec::new();

        // 5. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Extension-tools loop iteration");

            // Pre-call compaction: compact before sending to the LLM so we
            // never exceed the context window on the call itself.  Use the
            // token estimator as a fallback when we have no prior metadata.
            {
                let estimated = crate::compaction::estimate_tokens(&history);
                if crate::compaction::should_compact(estimated, &self.compaction_config) {
                    crate::compaction::maybe_compact(
                        &mut history,
                        &self.llm,
                        &self.compaction_config,
                        Some((&conv_store, conversation_id)),
                    )
                    .await;
                }
            }

            let ctx = ExecutionContext {
                conversation_id,
                agent_id: self.agent_id.clone(),
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: false,
                allowed_tools: None,
                depth: 0,
            };

            let mut llm_span = crate::otel_spans::start_llm_span(
                conversation_id,
                self.llm.as_ref(),
                iteration,
                &turn_cx,
                self.trace_content,
                &system_prompt,
                &history,
                &all_specs,
            );
            let llm_start = std::time::Instant::now();
            let response = self.llm.chat(&system_prompt, &history, &all_specs).await;
            let llm_elapsed = llm_start.elapsed();
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    crate::history::persist_error_recovery(&conv_store, conversation_id).await;
                    self.metrics
                        .record_error(&self.agent_id, "llm_error", "run_turn_with_tools");
                    turn_cx.span().set_status(OtelStatus::Error {
                        description: std::borrow::Cow::Owned(e.to_string()),
                    });
                    return Err(e);
                }
            };
            crate::otel_spans::finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
                Some((
                    &self.metrics,
                    &self.agent_id,
                    self.llm.provider_name(),
                    llm_elapsed,
                )),
            );

            // Post-call compaction: use the accurate token count reported by
            // the provider as a secondary signal to catch any cases the pre-call
            // estimator missed.
            if let Some(tokens) = response.meta().input_tokens {
                if crate::compaction::should_compact(tokens, &self.compaction_config) {
                    crate::compaction::maybe_compact(
                        &mut history,
                        &self.llm,
                        &self.compaction_config,
                        Some((&conv_store, conversation_id)),
                    )
                    .await;
                }
            }

            match response {
                LlmResponse::FinalAnswer(text, _meta) => {
                    let outcome = Self::handle_final_answer_with_extensions(
                        replied,
                        &text,
                        iteration,
                        base_turn,
                        conversation_id,
                        &self.agent_id,
                        &interface,
                        &ext_map,
                        &conv_store,
                    )
                    .await?;
                    match outcome {
                        FinalAnswerOutcome::Done(mut result) => {
                            result.attachments = turn_attachments;
                            return Ok(result);
                        }
                        FinalAnswerOutcome::Retry => continue,
                    }
                }

                LlmResponse::ToolCalls(tool_call_items, _meta) => {
                    info!(
                        count = tool_call_items.len(),
                        iteration, "LLM requested tool execution(s)"
                    );

                    Self::persist_tool_calls(
                        &mut history,
                        &conv_store,
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                    )
                    .await;

                    let has_real_calls = tool_call_items.iter().any(|t| t.name != "end_turn");
                    let tool_handlers = self.executor.list_tools();

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;
                        let turn_index = base_turn + iteration as i64 + 1;
                        let mut otel_span = crate::otel_spans::start_tool_span(
                            conversation_id,
                            iteration,
                            turn_index,
                            &interface,
                            &name,
                            &params,
                            &turn_cx,
                        );

                        if name == "end_turn" {
                            let outcome = Self::handle_end_turn(
                                has_real_calls,
                                replied,
                                &ext_map,
                                &params,
                                iteration,
                                conversation_id,
                                turn_index,
                                &mut otel_span,
                                &mut history,
                                &conv_store,
                            )
                            .await;
                            match outcome {
                                EndTurnOutcome::Deferred | EndTurnOutcome::Rejected => continue,
                                EndTurnOutcome::Accepted => {
                                    turn_ended = true;
                                    break;
                                }
                            }
                        }

                        // Extension tools take priority and bypass the safety gate.
                        if let Some(handler) = ext_map.get(&name) {
                            debug!(tool = %name, "Dispatching to extension handler");

                            let params_map = value_to_params_map(&params);

                            let start = std::time::Instant::now();
                            let exec_result = handler.run(params_map, &ctx).await;
                            let elapsed = start.elapsed();

                            self.finalize_tool_result(
                                &name,
                                exec_result,
                                elapsed,
                                &mut otel_span,
                                &mut history,
                                &conv_store,
                                conversation_id,
                                turn_index,
                                &mut turn_attachments,
                            )
                            .await;
                        } else {
                            // Global executor path.
                            let builtin_span = info_span!(
                                "tool_handler",
                                tool = %name,
                                source = "builtin"
                            );
                            let outcome = self
                                .dispatch_global_tool(
                                    &name,
                                    &params,
                                    &ctx,
                                    &mut otel_span,
                                    &mut history,
                                    &conv_store,
                                    conversation_id,
                                    turn_index,
                                    &mut turn_attachments,
                                    &tool_handlers,
                                    &builtin_span,
                                )
                                .await;
                            if matches!(outcome, DispatchOutcome::Denied) {
                                continue;
                            }
                        }

                        if name.contains("reply") || name.contains("post") || name.contains("react")
                        {
                            replied = true;
                        }
                    }

                    if turn_ended || replied {
                        crate::conversation_indexer::spawn_index(
                            conversation_id,
                            self.agent_id.clone(),
                            Arc::clone(&self.storage),
                            Arc::clone(&self.llm),
                        );
                        return Ok(TurnResult {
                            answer: String::new(),
                            attachments: turn_attachments,
                        });
                    }
                }

                LlmResponse::Thinking(text, _meta) => {
                    debug!(iteration, "LLM emitted thinking step");
                    let thinking_msg = {
                        let mut m = assistant_core::Message::assistant(
                            conversation_id,
                            format!("<think>{text}</think>"),
                        );
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    if let Err(e) = conv_store.save_message(&thinking_msg).await {
                        warn!("Failed to persist thinking step: {e}");
                    }
                    history.push(ChatHistoryMessage::Text {
                        role: ChatRole::Assistant,
                        content: text,
                    });
                }
            }
        }

        crate::history::persist_error_recovery(&conv_store, conversation_id).await;
        self.metrics
            .record_error(&self.agent_id, "max_iterations", "run_turn_with_tools");
        turn_cx.span().set_status(OtelStatus::Error {
            description: std::borrow::Cow::Owned(format!(
                "Max iterations ({}) reached without a final answer",
                self.max_iterations
            )),
        });
        anyhow::bail!(
            "Max iterations ({}) reached without a final answer",
            self.max_iterations
        );
    }

    /// Shared implementation for [`run_turn`] and [`run_turn_streaming`].
    async fn run_turn_core(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        token_sink: Option<mpsc::Sender<String>>,
        trace_cx: Option<&OtelContext>,
    ) -> Result<TurnResult> {
        let streaming = token_sink.is_some();
        self.metrics
            .record_turn(&self.agent_id, None, &format!("{interface:?}"));
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            streaming,
            "Starting turn"
        );

        let (_conv_cx, turn_cx) = setup_turn_trace(trace_cx, conversation_id, &interface);

        // 1-3. Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) = self
            .prepare_history(user_message, conversation_id, Vec::new(), &self.agent_id)
            .await?;

        // 4. Load all registered tool specs.
        let provider_caps = self.llm.capabilities();
        let tool_specs = Self::filter_tool_specs(self.executor.to_specs(), &provider_caps);

        // 5. Build the system prompt fresh from disk.
        let system_prompt = self.compose_system_prompt().await;

        // 6. Tool-calling loop.
        let mut turn_attachments: Vec<Attachment> = Vec::new();

        for iteration in 0..self.max_iterations {
            let iteration_span = info_span!("turn_iteration", iteration);
            debug!(parent: &iteration_span, iteration, "Tool-calling loop iteration");

            // Pre-call compaction using the token estimator.
            {
                let estimated = crate::compaction::estimate_tokens(&history);
                if crate::compaction::should_compact(estimated, &self.compaction_config) {
                    crate::compaction::maybe_compact(
                        &mut history,
                        &self.llm,
                        &self.compaction_config,
                        Some((&conv_store, conversation_id)),
                    )
                    .await;
                }
            }

            let ctx = ExecutionContext {
                conversation_id,
                agent_id: self.agent_id.clone(),
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
                allowed_tools: None,
                depth: 0,
            };

            let mut llm_span = crate::otel_spans::start_llm_span(
                conversation_id,
                self.llm.as_ref(),
                iteration,
                &turn_cx,
                self.trace_content,
                &system_prompt,
                &history,
                &tool_specs,
            );
            let llm_start = std::time::Instant::now();
            let response = if let Some(ref sink) = token_sink {
                self.llm
                    .chat_streaming(&system_prompt, &history, &tool_specs, Some(sink.clone()))
                    .instrument(iteration_span.clone())
                    .await
            } else {
                self.llm
                    .chat(&system_prompt, &history, &tool_specs)
                    .instrument(iteration_span.clone())
                    .await
            };
            let llm_elapsed = llm_start.elapsed();
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    crate::history::persist_error_recovery(&conv_store, conversation_id)
                        .instrument(iteration_span.clone())
                        .await;
                    let label = if streaming {
                        "run_turn_streaming"
                    } else {
                        "run_turn"
                    };
                    self.metrics
                        .record_error(&self.agent_id, "llm_error", label);
                    turn_cx.span().set_status(OtelStatus::Error {
                        description: std::borrow::Cow::Owned(e.to_string()),
                    });
                    return Err(e);
                }
            };
            crate::otel_spans::finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
                Some((
                    &self.metrics,
                    &self.agent_id,
                    self.llm.provider_name(),
                    llm_elapsed,
                )),
            );

            // Post-call compaction using the accurate provider-reported token count.
            if let Some(tokens) = response.meta().input_tokens {
                if crate::compaction::should_compact(tokens, &self.compaction_config) {
                    crate::compaction::maybe_compact(
                        &mut history,
                        &self.llm,
                        &self.compaction_config,
                        Some((&conv_store, conversation_id)),
                    )
                    .await;
                }
            }

            match response {
                LlmResponse::FinalAnswer(text, _meta) => {
                    info!(iteration, "LLM returned final answer");

                    if !text.trim().is_empty() {
                        let assistant_msg = {
                            let mut m = Message::assistant(conversation_id, &text);
                            m.turn = base_turn + iteration as i64 + 1;
                            m
                        };
                        conv_store
                            .save_message(&assistant_msg)
                            .instrument(iteration_span.clone())
                            .await?;
                    }

                    crate::conversation_indexer::spawn_index(
                        conversation_id,
                        self.agent_id.clone(),
                        Arc::clone(&self.storage),
                        Arc::clone(&self.llm),
                    );
                    return Ok(TurnResult {
                        answer: text,
                        attachments: turn_attachments,
                    });
                }

                LlmResponse::ToolCalls(tool_call_items, _meta) => {
                    info!(
                        count = tool_call_items.len(),
                        iteration, "LLM requested tool execution(s)"
                    );

                    Self::persist_tool_calls(
                        &mut history,
                        &conv_store,
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                    )
                    .instrument(iteration_span.clone())
                    .await;

                    let tool_handlers = self.executor.list_tools();

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;
                        let turn_index = base_turn + iteration as i64 + 1;
                        let mut otel_span = crate::otel_spans::start_tool_span(
                            conversation_id,
                            iteration,
                            turn_index,
                            &interface,
                            &name,
                            &params,
                            &turn_cx,
                        );

                        let outcome = self
                            .dispatch_global_tool(
                                &name,
                                &params,
                                &ctx,
                                &mut otel_span,
                                &mut history,
                                &conv_store,
                                conversation_id,
                                turn_index,
                                &mut turn_attachments,
                                &tool_handlers,
                                &iteration_span,
                            )
                            .await;
                        if matches!(outcome, DispatchOutcome::Denied) {
                            continue;
                        }
                    }
                }

                LlmResponse::Thinking(text, _meta) => {
                    debug!(iteration, "LLM emitted thinking step");
                    let thinking_msg = {
                        let mut m =
                            Message::assistant(conversation_id, format!("<think>{text}</think>"));
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    if let Err(e) = conv_store.save_message(&thinking_msg).await {
                        warn!("Failed to persist thinking step: {e}");
                    }
                    history.push(ChatHistoryMessage::Text {
                        role: ChatRole::Assistant,
                        content: text,
                    });
                }
            }
        }

        // Reached iteration limit.
        crate::history::persist_error_recovery(&conv_store, conversation_id).await;
        let label = if streaming {
            "run_turn_streaming"
        } else {
            "run_turn"
        };
        self.metrics
            .record_error(&self.agent_id, "max_iterations", label);
        turn_cx.span().set_status(OtelStatus::Error {
            description: std::borrow::Cow::Owned(format!(
                "Max iterations ({}) reached without a final answer",
                self.max_iterations
            )),
        });
        anyhow::bail!(
            "Max iterations ({}) reached without a final answer",
            self.max_iterations
        );
    }

    // ── History setup ─────────────────────────────────────────────────────────

    pub(crate) async fn prepare_history(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        attachments: Vec<ContentBlock>,
        agent_id: &str,
    ) -> Result<(ConversationStore, Vec<ChatHistoryMessage>, i64)> {
        let conv_store = self.storage.conversation_store_for_agent(agent_id);
        conv_store
            .create_conversation_with_id(conversation_id, None)
            .await?;

        let prior = conv_store.load_history(conversation_id).await?;
        let base_turn = prior.len() as i64;

        if base_turn == 0 {
            self.metrics.conversation_count.add(1, &[]);
        }

        let user_msg = {
            let mut m = Message::user(conversation_id, user_message);
            m.turn = base_turn;
            m
        };
        conv_store.save_message(&user_msg).await?;

        let mut history = crate::history::messages_to_chat_history(prior);
        crate::history::sanitize_history(&mut history);

        if attachments.is_empty() {
            history.push(ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: user_message.to_string(),
            });
        } else {
            let mut blocks = vec![ContentBlock::Text(user_message.to_string())];
            blocks.extend(attachments);
            history.push(ChatHistoryMessage::MultimodalUser { content: blocks });
        }

        Ok((conv_store, history, base_turn))
    }
}

// ── Module-level helpers ───────────────────────────────────────────────────────

/// Set up the two-level OTel trace hierarchy used by every turn variant.
///
/// Returns `(conv_cx, turn_cx)`.  The caller **must** keep `conv_cx` alive
/// (bind it to `_conv_cx`) so the conversation span is not dropped early.
fn setup_turn_trace(
    trace_cx: Option<&OtelContext>,
    conversation_id: Uuid,
    interface: &Interface,
) -> (OtelContext, OtelContext) {
    let tracer = global::tracer("assistant.orchestrator");
    let conv_cx = match trace_cx {
        Some(cx) => cx.clone(),
        None => {
            let mut span = tracer.start("conversation");
            span.set_attribute(KeyValue::new(
                "conversation_id",
                conversation_id.to_string(),
            ));
            span.set_attribute(KeyValue::new("interface", format!("{interface:?}")));
            OtelContext::current().with_span(span)
        }
    };
    let mut otel_turn = tracer.start_with_context("turn", &conv_cx);
    otel_turn.set_attribute(KeyValue::new(
        "conversation_id",
        conversation_id.to_string(),
    ));
    otel_turn.set_attribute(KeyValue::new("interface", format!("{interface:?}")));
    let turn_cx = conv_cx.with_span(otel_turn);
    (conv_cx, turn_cx)
}

/// Convert a [`serde_json::Value`] to a flat `HashMap<String, Value>`.
///
/// If the value is an `Object`, its entries are cloned into the map.
/// Any other variant (or `Null`) yields an empty map.
pub(crate) fn value_to_params_map(value: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    if let serde_json::Value::Object(map) = value {
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        HashMap::new()
    }
}
