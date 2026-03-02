//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, tool executor, and skill registry.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    strip_html_comments, Attachment, ExecutionContext, Interface, MemoryLoader, Message,
    MessageBus, MessageRole, ToolHandler,
};
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, ContentBlock, HostedTool, LlmProvider, LlmResponse,
    ToolSpec,
};
use assistant_skills::SkillDef as SpecSkillDef;
use assistant_storage::{conversations::ConversationStore, SkillRegistry, StorageLayer};
use assistant_tool_executor::ToolExecutor;
use opentelemetry::{
    global,
    trace::{Span as _, TraceContextExt, Tracer as _},
    Context as OtelContext, KeyValue,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, info_span, warn, Instrument};
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
    /// File attachments collected from tool outputs during the turn.
    ///
    /// Interfaces should deliver these to the user (e.g. save to disk in the
    /// CLI, upload in Slack/Mattermost).
    pub attachments: Vec<Attachment>,
}

/// Per-conversation extension tool registration consumed by the worker.
///
/// Interfaces (Slack, Mattermost) register their per-turn tools and
/// attachments before publishing a [`TurnRequest`](bus_messages::TurnRequest)
/// to the bus.  The worker removes the registration when processing the
/// request.
struct ExtensionRegistration {
    tools: Vec<Arc<dyn ToolHandler>>,
    attachments: Vec<ContentBlock>,
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

/// Parse an interface string back to the [`Interface`] enum.
///
/// Matches the `Debug` format that the codebase uses for serialisation
/// (e.g. `"Cli"`, `"Slack"`).  Falls back to [`Interface::Cli`] for
/// unknown values.
fn parse_interface(s: &str) -> Interface {
    match s.to_lowercase().as_str() {
        "cli" => Interface::Cli,
        "signal" => Interface::Signal,
        "mcp" => Interface::Mcp,
        "slack" => Interface::Slack,
        "mattermost" => Interface::Mattermost,
        "web" => Interface::Web,
        "scheduler" => Interface::Scheduler,
        _ => Interface::Cli,
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
///    confirm with the user, execute the tool, emit an OpenTelemetry span,
///    and append an `OBSERVATION` to the conversation history.
/// 6. Persist the final assistant message and return [`TurnResult`].
pub struct Orchestrator {
    llm: Arc<dyn LlmProvider>,
    storage: Arc<StorageLayer>,
    executor: Arc<ToolExecutor>,
    registry: Arc<SkillRegistry>,
    /// Durable message bus for decoupled inter-component communication.
    bus: Arc<dyn MessageBus>,
    max_iterations: usize,
    disabled_skills: Vec<String>,
    confirmation_callback: Option<Arc<dyn ConfirmationCallback>>,
    /// Memory loader used to rebuild the system prompt at the start of every
    /// turn so that writes made by memory tools are reflected immediately.
    memory_loader: MemoryLoader,
    /// When true, record full message content on LLM spans (PII-sensitive).
    trace_content: bool,
    /// Per-conversation token sinks for streaming turns dispatched through
    /// the bus.  Consumed (removed) by the worker when processing.
    token_sinks: tokio::sync::RwLock<HashMap<Uuid, mpsc::Sender<String>>>,
    /// Per-conversation extension tool registrations for interface-specific
    /// turns dispatched through the bus.  Consumed by the worker.
    extension_registrations: tokio::sync::RwLock<HashMap<Uuid, ExtensionRegistration>>,
    /// Cancellation tokens for running subagents, keyed by agent ID.
    /// Inserting a token when a subagent starts and removing it when it finishes
    /// allows external callers to cancel an in-progress subagent.
    agent_cancellations: tokio::sync::RwLock<HashMap<String, CancellationToken>>,
    /// OTel metric instruments for GenAI and operational metrics.
    metrics: crate::MetricsRecorder,
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
            disabled_skills: config.skills.disabled.clone(),
            confirmation_callback: None,
            memory_loader,
            trace_content: config.mirror.trace_content,
            token_sinks: tokio::sync::RwLock::new(HashMap::new()),
            extension_registrations: tokio::sync::RwLock::new(HashMap::new()),
            agent_cancellations: tokio::sync::RwLock::new(HashMap::new()),
            metrics: crate::MetricsRecorder::new(),
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

    /// Return the path to HEARTBEAT.md (used by the scheduler).
    pub fn heartbeat_path(&self) -> &Path {
        self.memory_loader.heartbeat_path()
    }

    /// Return the path to BOOT.md (per-session startup hook).
    pub fn boot_path(&self) -> &Path {
        self.memory_loader.boot_path()
    }

    /// Run the per-session startup hook (BOOT.md).
    ///
    /// Reads BOOT.md from the configured path.  If the file exists and contains
    /// non-comment, non-empty content, its text is submitted as a single silent
    /// turn through the message bus.  The result is logged but not displayed to
    /// the user — BOOT.md is infrastructure, not conversation.
    ///
    /// Requires [`run_worker`](Self::run_worker) to be running in a background
    /// task.
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

        let raw = std::fs::read_to_string(boot_path)
            .map_err(|e| anyhow::anyhow!("Failed to read BOOT.md: {e}"))?;

        // Strip HTML comments and whitespace — an empty/comment-only file is
        // treated as "no boot instructions".
        let stripped = strip_html_comments(&raw);
        if stripped.is_empty() {
            debug!("BOOT.md is empty or comment-only, skipping startup hook");
            return Ok(false);
        }

        info!(path = %boot_path.display(), "Running BOOT.md startup hook");
        match self
            .submit_turn(&stripped, conversation_id, interface)
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
    /// of the extension tool calls (e.g. `reply`).  If the LLM emits a
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

    async fn run_turn_with_tools_impl(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        extensions: Vec<Arc<dyn ToolHandler>>,
        trace_cx: Option<&OtelContext>,
        attachments: Vec<ContentBlock>,
    ) -> Result<TurnResult> {
        self.metrics.record_turn(None, &format!("{interface:?}"));
        info!("Starting turn with extension tools");

        let (_conv_cx, turn_cx) = setup_turn_trace(trace_cx, conversation_id, &interface);

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
        let (conv_store, mut history, base_turn) = self
            .prepare_history(user_message, conversation_id, attachments)
            .await?;

        // 4. Load global tool specs and merge with extensions for LLM tool listing.
        //    Extension specs come first so the LLM sees them prominently.
        //
        //    When a `reply` extension tool is present, suppress any global tools
        //    whose name contains "post" — those tools (e.g. `slack-post`) post to
        //    arbitrary channels without thread context and reliably confuse the LLM
        //    into replying to the channel root instead of the active thread.
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
        // When extension tools are present, guide the LLM to use them.
        let system_prompt = if ext_specs.is_empty() {
            base_system_prompt
        } else {
            // Separate reply tools by purpose so the LLM understands they are
            // alternatives, not complements — listing them all with "or" is
            // ambiguous and causes some models to call several at once.
            let plain_reply: Vec<&str> = ext_specs
                .iter()
                .filter(|s| {
                    (s.name.contains("reply") || s.name.contains("post"))
                        && !s.name.contains("block")
                })
                .map(|s| s.name.as_str())
                .collect();
            let block_reply: Vec<&str> = ext_specs
                .iter()
                .filter(|s| s.name.contains("block"))
                .map(|s| s.name.as_str())
                .collect();
            let react_tools: Vec<&str> = ext_specs
                .iter()
                .filter(|s| s.name.contains("react"))
                .map(|s| s.name.as_str())
                .collect();

            let has_reply = !plain_reply.is_empty() || !block_reply.is_empty();
            let has_react = !react_tools.is_empty();

            let ack_instruction = if has_reply && has_react {
                let plain_names = plain_reply.join("`, `");
                let block_names = block_reply.join("`, `");
                let react_names = react_tools.join("`, `");
                let block_clause = if !block_names.is_empty() {
                    format!(" or `{block_names}` for rich Block Kit layouts")
                } else {
                    String::new()
                };
                format!(
                    "Before calling `end_turn` you MUST send exactly one reply to the user.\n\
                     - Use `{plain_names}` for plain-text or mrkdwn responses{block_clause}.\n\
                     - Use `{react_names}` only for a brief emoji-only acknowledgement \
                       (e.g. `thumbsup`, `white_check_mark`) when no text is needed.\n\
                     Call at most ONE reply tool per turn — never call two reply tools \
                     or call the same tool twice.\n"
                )
            } else if has_reply {
                let plain_names = plain_reply.join("`, `");
                let block_names = block_reply.join("`, `");
                let block_clause = if !block_names.is_empty() {
                    format!(" or `{block_names}` for rich Block Kit layouts")
                } else {
                    String::new()
                };
                format!(
                    "Before calling `end_turn` you MUST reply to the user exactly once \
                     using `{plain_names}`{block_clause}. \
                     Never call a reply tool more than once per turn.\n"
                )
            } else if has_react {
                let react_names = react_tools.join("`, `");
                format!(
                    "Before calling `end_turn` you MUST acknowledge the user \
                     using `{react_names}` (exactly once).\n"
                )
            } else {
                String::new()
            };

            format!(
                "{base_system_prompt}\n\n---\n\n\
                You are operating inside a messaging interface. \
                {ack_instruction}\
                When you have finished all work, call `end_turn` to signal completion."
            )
        };

        let mut turn_ended = false;
        let mut replied = false;
        let mut turn_attachments: Vec<Attachment> = Vec::new();

        // 5. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Extension-tools loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: false,
                allowed_tools: None,
                depth: 0,
            };

            let mut llm_span = crate::otel_spans::start_llm_span(
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
                    // The user message was already persisted by prepare_history.
                    // Save a synthetic assistant message so the conversation
                    // keeps proper alternation and subsequent turns are not
                    // poisoned by an orphaned user message.
                    crate::history::persist_error_recovery(&conv_store, conversation_id).await;
                    self.metrics
                        .record_error("llm_error", "run_turn_with_tools");
                    return Err(e);
                }
            };
            crate::otel_spans::finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
                Some((&self.metrics, self.llm.provider_name(), llm_elapsed)),
            );

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text, _meta) => {
                    // When a reply was already posted via an extension tool,
                    // persist any non-empty wrap-up text and finish the turn.
                    if replied {
                        if !text.trim().is_empty() {
                            let assistant_msg = {
                                let mut m =
                                    assistant_core::Message::assistant(conversation_id, &text);
                                m.turn = base_turn + iteration as i64 + 1;
                                m
                            };
                            if let Err(e) = conv_store.save_message(&assistant_msg).await {
                                warn!("Failed to persist post-reply assistant message: {e}");
                            }
                        }
                        return Ok(TurnResult {
                            answer: String::new(),
                            attachments: turn_attachments,
                        });
                    }

                    // Empty final answer with no reply sent yet — the user would
                    // see nothing.  Don't persist the empty message (it pollutes
                    // history and can cause the model to repeat the pattern on
                    // subsequent turns) and loop to give the model another chance.
                    if text.trim().is_empty() {
                        warn!(
                            iteration,
                            "LLM returned empty final answer without a prior reply; retrying"
                        );
                        continue;
                    }

                    // Non-empty answer — persist to DB.
                    let assistant_msg = {
                        let mut m = assistant_core::Message::assistant(conversation_id, &text);
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

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
                            return Ok(TurnResult {
                                answer: String::new(),
                                attachments: turn_attachments,
                            });
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
                            allowed_tools: None,
                            depth: 0,
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

                    return Ok(TurnResult {
                        answer: String::new(),
                        attachments: turn_attachments,
                    });
                }

                // ── Tool calls ────────────────────────────────────────────────
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
                            if has_real_calls {
                                info!(
                                    iteration,
                                    "end_turn deferred (called alongside other tools)"
                                );
                                let deferred_msg =
                                    "end_turn deferred: processing other tool calls first";
                                otel_span.set_attribute(KeyValue::new("tool_status", "deferred"));
                                otel_span.set_attribute(KeyValue::new(
                                    "tool_observation",
                                    deferred_msg.to_string(),
                                ));
                                crate::history::append_tool_result(
                                    &mut history,
                                    "end_turn",
                                    deferred_msg,
                                );
                                let tr_msg = Self::make_tool_result_message(
                                    conversation_id,
                                    turn_index,
                                    "end_turn",
                                    deferred_msg,
                                );
                                if let Err(e) = conv_store.save_message(&tr_msg).await {
                                    warn!("Failed to persist deferred end_turn tool-result: {e}");
                                }
                                otel_span.end();
                                continue;
                            }

                            let reason = params
                                .get("reason")
                                .and_then(|v| v.as_str())
                                .unwrap_or("done");

                            // Guard: reject end_turn when a reply/react tool
                            // exists but the LLM never actually called one.
                            // This prevents the turn from silently completing
                            // without delivering a visible response to the
                            // user in messaging interfaces (e.g. Slack).
                            // Reactions count as valid acknowledgements per the
                            // system prompt.
                            let has_reply_tool = ext_map.keys().any(|n| {
                                n.contains("reply") || n.contains("post") || n.contains("react")
                            });
                            if !replied && has_reply_tool {
                                warn!(
                                    iteration,
                                    reason,
                                    "end_turn rejected: reply tool available but no reply sent"
                                );
                                let reject_msg =
                                    "end_turn rejected: you MUST call the `reply` tool \
                                     before ending the turn. The user has not seen any \
                                     response yet.";
                                otel_span.set_attribute(KeyValue::new("tool_status", "rejected"));
                                otel_span.set_attribute(KeyValue::new(
                                    "tool_observation",
                                    reject_msg.to_string(),
                                ));
                                crate::history::append_tool_result(
                                    &mut history,
                                    "end_turn",
                                    reject_msg,
                                );
                                let tr_msg = Self::make_tool_result_message(
                                    conversation_id,
                                    turn_index,
                                    "end_turn",
                                    reject_msg,
                                );
                                if let Err(e) = conv_store.save_message(&tr_msg).await {
                                    warn!("Failed to persist rejected end_turn tool-result: {e}");
                                }
                                otel_span.end();
                                continue;
                            }

                            info!(iteration, reason, "end_turn called; stopping turn");

                            let result_text = format!("end_turn: {reason}");
                            otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                            otel_span.set_attribute(KeyValue::new(
                                "tool_observation",
                                result_text.clone(),
                            ));
                            crate::history::append_tool_result(
                                &mut history,
                                "end_turn",
                                &result_text,
                            );
                            let tr_msg = Self::make_tool_result_message(
                                conversation_id,
                                turn_index,
                                "end_turn",
                                &result_text,
                            );
                            if let Err(e) = conv_store.save_message(&tr_msg).await {
                                warn!("Failed to persist end_turn tool-result: {e}");
                            }

                            turn_ended = true;
                            otel_span.end();
                            break;
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
                            if let Some(reason) = self
                                .reject_if_disabled(
                                    &name,
                                    &mut history,
                                    &conv_store,
                                    conversation_id,
                                    turn_index,
                                )
                                .instrument(builtin_span.clone())
                                .await
                            {
                                otel_span.set_attribute(KeyValue::new("tool_status", "blocked"));
                                otel_span.set_attribute(KeyValue::new("tool_error", reason));
                                otel_span.end();
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
                                        crate::history::append_tool_result(
                                            &mut history,
                                            &name,
                                            &observation,
                                        );
                                        let tr_msg = Self::make_tool_result_message(
                                            conversation_id,
                                            base_turn + iteration as i64 + 1,
                                            &name,
                                            &observation,
                                        );
                                        if let Err(e) = conv_store
                                            .save_message(&tr_msg)
                                            .instrument(builtin_span.clone())
                                            .await
                                        {
                                            warn!("Failed to persist tool-result message: {e}");
                                        }
                                        continue;
                                    }
                                }
                            }

                            let params_map = value_to_params_map(&params);
                            let start = std::time::Instant::now();
                            let exec_result = self
                                .executor
                                .execute(&name, params_map, &ctx)
                                .instrument(builtin_span.clone())
                                .await;
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
                        }

                        // Mark the turn as acknowledged if any posting, reply,
                        // or reaction tool was called — regardless of whether it
                        // is an extension tool or a global skill (e.g.
                        // `slack-post`).  Without this, calling a global posting
                        // skill leaves `replied=false` and the auto-post fallback
                        // fires on the next FinalAnswer, producing a second
                        // message in a different context (e.g. channel root vs.
                        // thread).  Reactions (e.g. `react`) count as valid
                        // acknowledgements per the system prompt.
                        if name.contains("reply") || name.contains("post") || name.contains("react")
                        {
                            replied = true;
                        }
                    }

                    if turn_ended || replied {
                        return Ok(TurnResult {
                            answer: String::new(),
                            attachments: turn_attachments,
                        });
                    }
                }

                // ── Intermediate thinking step ────────────────────────────────
                LlmResponse::Thinking(text, _meta) => {
                    debug!(iteration, "LLM emitted thinking step");
                    // Persist to DB so thinking is preserved, but the
                    // interface (Slack) will never display it directly.
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
        trace_cx: Option<&OtelContext>,
    ) -> Result<TurnResult> {
        self.run_turn_core(user_message, conversation_id, interface, None, trace_cx)
            .await
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

    /// Shared implementation for [`run_turn`] and [`run_turn_streaming`].
    ///
    /// When `token_sink` is `Some`, final-answer tokens are streamed via
    /// [`LlmProvider::chat_streaming`]; otherwise the non-streaming
    /// [`LlmProvider::chat`] is used.
    async fn run_turn_core(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        interface: Interface,
        token_sink: Option<mpsc::Sender<String>>,
        trace_cx: Option<&OtelContext>,
    ) -> Result<TurnResult> {
        let streaming = token_sink.is_some();
        self.metrics.record_turn(None, &format!("{interface:?}"));
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            streaming,
            "Starting turn"
        );

        let (_conv_cx, turn_cx) = setup_turn_trace(trace_cx, conversation_id, &interface);

        // 1-3. Set up conversation, load prior history, persist user message.
        let (conv_store, mut history, base_turn) = self
            .prepare_history(user_message, conversation_id, Vec::new())
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

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
                allowed_tools: None,
                depth: 0,
            };

            let mut llm_span = crate::otel_spans::start_llm_span(
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
                    self.metrics.record_error("llm_error", label);
                    return Err(e);
                }
            };
            crate::otel_spans::finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
                Some((&self.metrics, self.llm.provider_name(), llm_elapsed)),
            );

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text, _meta) => {
                    info!(iteration, "LLM returned final answer");

                    // Don't persist empty final answers — they pollute the
                    // conversation history and can confuse the model on
                    // subsequent turns.
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

                    return Ok(TurnResult {
                        answer: text,
                        attachments: turn_attachments,
                    });
                }

                // ── Tool calls ────────────────────────────────────────────────
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

                        // Disabled-tools gate.
                        if let Some(reason) = self
                            .reject_if_disabled(
                                &name,
                                &mut history,
                                &conv_store,
                                conversation_id,
                                turn_index,
                            )
                            .instrument(iteration_span.clone())
                            .await
                        {
                            otel_span.set_attribute(KeyValue::new("tool_status", "blocked"));
                            otel_span.set_attribute(KeyValue::new("tool_error", reason));
                            otel_span.end();
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
                                    otel_span.set_attribute(KeyValue::new("tool_status", "denied"));
                                    otel_span.set_attribute(KeyValue::new(
                                        "tool_error",
                                        observation.clone(),
                                    ));
                                    crate::history::append_tool_result(
                                        &mut history,
                                        &name,
                                        &observation,
                                    );
                                    let tr_msg = Self::make_tool_result_message(
                                        conversation_id,
                                        turn_index,
                                        &name,
                                        &observation,
                                    );
                                    if let Err(e) = conv_store
                                        .save_message(&tr_msg)
                                        .instrument(iteration_span.clone())
                                        .await
                                    {
                                        warn!("Failed to persist tool-result message: {e}");
                                    }
                                    otel_span.end();
                                    continue;
                                }
                            }
                        }

                        let params_map = value_to_params_map(&params);
                        let start = std::time::Instant::now();
                        let exec_result = self
                            .executor
                            .execute(&name, params_map, &ctx)
                            .instrument(iteration_span.clone())
                            .await;
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
                    }
                }

                // ── Intermediate thinking step ────────────────────────────────
                LlmResponse::Thinking(text, _meta) => {
                    debug!(iteration, "LLM emitted thinking step");
                    history.push(ChatHistoryMessage::Text {
                        role: ChatRole::Assistant,
                        content: text,
                    });
                }
            }
        }

        // Reached iteration limit.
        crate::history::persist_error_recovery(&conv_store, conversation_id).await;
        anyhow::bail!(
            "Max iterations ({}) reached without a final answer",
            self.max_iterations
        );
    }

    async fn compose_system_prompt(&self) -> String {
        let mut prompt = self.memory_loader.load_system_prompt();
        if let Some(skills_xml) = self.available_skills_xml().await {
            prompt.push_str("\n\n");
            prompt.push_str(&skills_xml);
        }
        prompt
    }

    async fn available_skills_xml(&self) -> Option<String> {
        let skills = self.registry.list().await;
        if skills.is_empty() {
            return None;
        }

        let mut buf = String::new();
        buf.push_str("<available_skills>\n");
        for skill in &skills {
            buf.push_str("  <skill>\n");
            buf.push_str(&format!("    <name>{}</name>\n", escape_xml(&skill.name)));
            buf.push_str(&format!(
                "    <description>{}</description>\n",
                escape_xml(&skill.description)
            ));
            if let Some(location) = skill_location_string(skill) {
                buf.push_str(&format!(
                    "    <location>{}</location>\n",
                    escape_xml(&location)
                ));
            }
            buf.push_str("  </skill>\n");
        }
        buf.push_str("</available_skills>");
        Some(buf)
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    pub(crate) async fn prepare_history(
        &self,
        user_message: &str,
        conversation_id: Uuid,
        attachments: Vec<ContentBlock>,
    ) -> Result<(ConversationStore, Vec<ChatHistoryMessage>, i64)> {
        let conv_store = self.storage.conversation_store();
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

        // Repair structural issues (orphaned messages, missing tool results).
        crate::history::sanitize_history(&mut history);

        // When attachments are present, emit a MultimodalUser message so
        // vision-capable providers receive the inline images.  Otherwise
        // fall back to the lightweight Text variant.
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

    fn filter_tool_specs(specs: Vec<ToolSpec>, caps: &Capabilities) -> Vec<ToolSpec> {
        specs
            .into_iter()
            .filter(|spec| !Self::tool_suppressed_by_caps(spec, caps))
            .collect()
    }

    fn tool_suppressed_by_caps(spec: &ToolSpec, caps: &Capabilities) -> bool {
        if caps.hosted_tools.contains(&HostedTool::WebSearch) && spec.name == "web-search" {
            return true;
        }
        if caps.hosted_tools.contains(&HostedTool::WebFetch) && spec.name == "web-fetch" {
            return true;
        }
        false
    }

    async fn reject_if_disabled(
        &self,
        name: &str,
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_idx: i64,
    ) -> Option<String> {
        if !self.disabled_skills.iter().any(|s| s == name) {
            return None;
        }
        let observation = format!("Tool '{name}' is disabled by configuration.");
        warn!(%observation);
        crate::history::append_tool_result(history, name, &observation);
        let tr_msg = Self::make_tool_result_message(conversation_id, turn_idx, name, &observation);
        if let Err(e) = conv_store.save_message(&tr_msg).await {
            warn!("Failed to persist tool-result message: {e}");
        }
        Some(observation)
    }

    /// Process a tool execution result: record metrics, set OTel span
    /// attributes, collect attachments, end the span, append to history,
    /// and persist the tool-result message to the database.
    ///
    /// Returns the observation string that was fed back to the LLM.
    ///
    /// This is the common post-execution step shared by all turn variants
    /// (extension-tools, core, and subagent).
    #[allow(clippy::too_many_arguments)]
    async fn finalize_tool_result(
        &self,
        tool_name: &str,
        exec_result: Result<assistant_core::ToolOutput>,
        elapsed: std::time::Duration,
        otel_span: &mut opentelemetry::global::BoxedSpan,
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        turn_attachments: &mut Vec<Attachment>,
    ) -> String {
        let duration_ms = elapsed.as_millis() as i64;
        self.metrics.record_tool_invocation(tool_name);
        self.metrics
            .record_tool_duration(tool_name, duration_ms as f64 / 1000.0);

        let observation = match exec_result {
            Ok(output) => {
                debug!(
                    tool = %tool_name,
                    duration_ms,
                    success = output.success,
                    "Tool execution completed"
                );
                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                otel_span.set_attribute(KeyValue::new("tool_observation", output.content.clone()));
                if !output.attachments.is_empty() {
                    turn_attachments.extend(output.attachments);
                }
                tool_result_content(&output.content, output.data.as_ref())
            }
            Err(err) => {
                warn!(tool = %tool_name, %err, "Tool execution failed");
                let msg = err.to_string();
                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                otel_span.set_attribute(KeyValue::new("tool_status", "error"));
                otel_span.set_attribute(KeyValue::new("tool_error", msg.clone()));
                format!("Error executing '{tool_name}': {msg}")
            }
        };

        otel_span.end();

        crate::history::append_tool_result(history, tool_name, &observation);
        let tr_msg =
            Self::make_tool_result_message(conversation_id, turn_index, tool_name, &observation);
        if let Err(e) = conv_store.save_message(&tr_msg).await {
            warn!("Failed to persist tool-result message: {e}");
        }

        observation
    }

    /// Record tool calls in the chat history and persist them to the database.
    ///
    /// This is the common pre-execution step shared by all three turn variants
    /// (extension-tools, core, and subagent).  It clones the items into the
    /// running history and saves a tool-call message to the conversation store.
    async fn persist_tool_calls(
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        tool_call_items: &[assistant_llm::ToolCallItem],
    ) {
        history.push(ChatHistoryMessage::AssistantToolCalls(
            tool_call_items.to_vec(),
        ));
        let tc_msg = Self::make_tool_call_message(conversation_id, turn_index, tool_call_items);
        if let Err(e) = conv_store.save_message(&tc_msg).await {
            warn!("Failed to persist tool-call message: {e}");
        }
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

mod subagent;

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

/// Build the tool result content from a tool output.
///
/// Always returns the human-readable `content` string so the LLM receives
/// a consistent, formatted observation. The structured `data` field is for
/// downstream callers that need machine-readable output; it is not sent to
/// the model directly.
fn tool_result_content(content: &str, _data: Option<&serde_json::Value>) -> String {
    content.to_string()
}

/// Convert a [`serde_json::Value`] (expected to be an Object) into the
/// `HashMap<String, Value>` that [`ToolHandler::run`] expects.
///
/// Non-object values produce an empty map — this matches the existing
/// behaviour at every call-site.
pub(crate) fn value_to_params_map(
    params: &serde_json::Value,
) -> HashMap<String, serde_json::Value> {
    if let serde_json::Value::Object(map) = params {
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    } else {
        HashMap::new()
    }
}

fn skill_location_string(skill: &SpecSkillDef) -> Option<String> {
    let path = skill.dir.join("SKILL.md");
    if path.exists() {
        Some(path.display().to_string())
    } else {
        None
    }
}

fn escape_xml(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

mod worker;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
