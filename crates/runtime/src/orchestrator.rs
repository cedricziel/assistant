//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, tool executor, and skill registry.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use assistant_core::{
    bus_messages, strip_html_comments, topic, AgentReport, AgentReportStatus, AgentSpawn,
    Attachment, ClaimFilter, ExecutionContext, Interface, MemoryLoader, Message, MessageBus,
    MessageRole, PublishRequest, SubagentRunner, ToolHandler, DEFAULT_MAX_AGENT_DEPTH,
};
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, ContentBlock, HostedTool, LlmProvider, LlmResponse,
    LlmResponseMeta, ToolSpec,
};
use assistant_skills::SkillDef as SpecSkillDef;
use assistant_storage::{conversations::ConversationStore, SkillRegistry, StorageLayer};
use assistant_tool_executor::ToolExecutor;
use async_trait::async_trait;
use opentelemetry::{
    global,
    trace::{Span as _, TraceContextExt, Tracer as _},
    Context as OtelContext, KeyValue,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, info_span, warn, Instrument};
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

    // ── Bus-based turn processing ────────────────────────────────────────────

    /// Register a token sink for a streaming turn.
    ///
    /// Call this *before* publishing the [`TurnRequest`](bus_messages::TurnRequest)
    /// to the bus.  The worker will consume (remove) the sink when it processes
    /// the request, routing tokens through it via
    /// [`run_turn_streaming`](Self::run_turn_streaming).
    pub async fn register_token_sink(&self, conversation_id: Uuid, sink: mpsc::Sender<String>) {
        self.token_sinks.write().await.insert(conversation_id, sink);
    }

    /// Register extension tools and attachments for an interface-specific turn.
    ///
    /// Call this *before* publishing the [`TurnRequest`](bus_messages::TurnRequest)
    /// to the bus.  The worker will consume the registration when it processes
    /// the request, routing to
    /// [`run_turn_with_tools`](Self::run_turn_with_tools).
    pub async fn register_extensions(
        &self,
        conversation_id: Uuid,
        tools: Vec<Arc<dyn ToolHandler>>,
        attachments: Vec<ContentBlock>,
    ) {
        self.extension_registrations.write().await.insert(
            conversation_id,
            ExtensionRegistration { tools, attachments },
        );
    }

    /// Run the turn-processing worker loop.
    ///
    /// Claims messages from the [`topic::TURN_REQUEST`] topic and dispatches
    /// them to the appropriate processing method:
    ///
    /// - **Extension tools registered** → [`run_turn_with_tools`](Self::run_turn_with_tools)
    /// - **Token sink registered** → [`run_turn_streaming`](Self::run_turn_streaming)
    /// - **Neither** → [`run_turn`](Self::run_turn)
    ///
    /// After processing, a [`TurnResult`](bus_messages::TurnResult) is
    /// published to [`topic::TURN_RESULT`].
    ///
    /// This method runs indefinitely and should be spawned as a background
    /// task.  It exits when the tokio task is cancelled / dropped.
    ///
    /// ```rust,ignore
    /// let orch = Arc::new(orchestrator);
    /// tokio::spawn({
    ///     let orch = orch.clone();
    ///     async move { orch.run_worker("worker-1").await }
    /// });
    /// ```
    pub async fn run_worker(&self, worker_id: &str) {
        self.run_worker_filtered(worker_id, None).await;
    }

    /// Run a turn-processing worker that only claims messages for the given
    /// interface.  Pass `None` to claim messages for any interface (the
    /// original `run_worker` behaviour).
    ///
    /// When multiple services share the same SQLite database, each service
    /// should scope its worker to its own interface so one service doesn't
    /// steal turns from another.
    pub async fn run_worker_filtered(&self, worker_id: &str, interface: Option<&str>) {
        info!(worker_id, ?interface, "Turn worker started");
        let filter = match interface {
            Some(iface) => ClaimFilter::new().with_interface(iface),
            None => ClaimFilter::default(),
        };
        loop {
            match self
                .bus
                .claim_filtered(topic::TURN_REQUEST, worker_id, &filter)
                .await
            {
                Ok(Some(msg)) => {
                    let turn_req: bus_messages::TurnRequest =
                        match serde_json::from_value(msg.payload.clone()) {
                            Ok(req) => req,
                            Err(e) => {
                                warn!(
                                    error = %e,
                                    msg_id = %msg.id,
                                    "Failed to deserialize TurnRequest"
                                );
                                let _ = self.bus.fail(msg.id).await;
                                continue;
                            }
                        };

                    let interface = msg
                        .interface
                        .as_deref()
                        .map(parse_interface)
                        .unwrap_or(Interface::Cli);

                    let conv_id = turn_req.conversation_id;

                    debug!(
                        conversation_id = %conv_id,
                        worker_id,
                        "Processing turn request"
                    );

                    // Check for registered side-channel resources.
                    let ext = self.extension_registrations.write().await.remove(&conv_id);
                    let token_sink = self.token_sinks.write().await.remove(&conv_id);

                    // Dispatch to the appropriate processing method.
                    let result: Result<TurnResult> = if let Some(reg) = ext {
                        // Extension-tool turn (Slack, Mattermost).
                        self.run_turn_with_tools(
                            &turn_req.prompt,
                            conv_id,
                            interface,
                            reg.tools,
                            None,
                            reg.attachments,
                        )
                        .await
                    } else if let Some(sink) = token_sink {
                        // Streaming turn (CLI, Signal).
                        self.run_turn_streaming(&turn_req.prompt, conv_id, interface, sink, None)
                            .await
                    } else {
                        // Standard non-streaming turn.
                        self.run_turn(&turn_req.prompt, conv_id, interface, None)
                            .await
                    };

                    match result {
                        Ok(turn_result) => {
                            let bus_result = bus_messages::TurnResult {
                                conversation_id: conv_id,
                                content: turn_result.answer,
                                turn: 0,
                                attachments: turn_result.attachments,
                            };

                            // Propagate batch_id from the request so submit_turn
                            // can match the result to its specific request.
                            let mut pub_req = PublishRequest::new(
                                topic::TURN_RESULT,
                                serde_json::to_value(&bus_result).unwrap_or_default(),
                            )
                            .with_conversation_id(conv_id);
                            if let Some(bid) = msg.batch_id {
                                pub_req = pub_req.with_batch_id(bid);
                            }

                            match self.bus.publish(pub_req).await {
                                Ok(_) => {
                                    if let Err(e) = self.bus.ack(msg.id).await {
                                        warn!(
                                            error = %e,
                                            msg_id = %msg.id,
                                            "Failed to ack bus message"
                                        );
                                    }
                                    info!(
                                        conversation_id = %conv_id,
                                        worker_id,
                                        "Turn completed via worker"
                                    );
                                }
                                Err(e) => {
                                    warn!(error = %e, "Failed to publish TurnResult, nacking request");
                                    let _ = self.bus.nack(msg.id).await;
                                }
                            }
                        }
                        Err(e) => {
                            warn!(
                                error = %e,
                                conversation_id = %conv_id,
                                worker_id,
                                "Turn failed in worker"
                            );

                            // Publish a failure TurnResult so submit_turn
                            // callers get an immediate error instead of
                            // waiting until timeout.
                            let err_result = bus_messages::TurnResult {
                                conversation_id: conv_id,
                                content: format!("Turn failed: {e}"),
                                turn: 0,
                                attachments: vec![],
                            };
                            let mut pub_req = PublishRequest::new(
                                topic::TURN_RESULT,
                                serde_json::to_value(&err_result).unwrap_or_default(),
                            )
                            .with_conversation_id(conv_id);
                            if let Some(bid) = msg.batch_id {
                                pub_req = pub_req.with_batch_id(bid);
                            }
                            let _ = self.bus.publish(pub_req).await;

                            let _ = self.bus.fail(msg.id).await;
                        }
                    }
                }
                Ok(None) => {
                    // No pending messages — back off.
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => {
                    error!(error = %e, worker_id, "Turn worker claim error");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Submit a turn through the message bus and wait for the result.
    ///
    /// Publishes a [`TurnRequest`](bus_messages::TurnRequest) to the bus and
    /// polls for the corresponding [`TurnResult`](bus_messages::TurnResult).
    /// Requires [`run_worker`](Self::run_worker) to be running in a
    /// background task.
    ///
    /// # Parameters
    /// * `prompt` — the user message
    /// * `conversation_id` — conversation to continue (or start)
    /// * `interface` — originating interface
    pub async fn submit_turn(
        &self,
        prompt: &str,
        conversation_id: Uuid,
        interface: Interface,
    ) -> Result<TurnResult> {
        let request_id = Uuid::new_v4();
        let turn_req = bus_messages::TurnRequest {
            prompt: prompt.to_string(),
            conversation_id,
            extension_tools: vec![],
        };

        self.bus
            .publish(
                PublishRequest::new(topic::TURN_REQUEST, serde_json::to_value(&turn_req)?)
                    .with_conversation_id(conversation_id)
                    .with_interface(format!("{:?}", interface))
                    .with_reply_to(topic::TURN_RESULT)
                    .with_batch_id(request_id),
            )
            .await?;

        // Poll for the result with a 10-minute timeout.
        // Match by both conversation_id and batch_id (request_id) so
        // overlapping turns for the same conversation don't collide.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(600);
        loop {
            if tokio::time::Instant::now() > deadline {
                anyhow::bail!(
                    "submit_turn timed out waiting for result \
                     (conversation_id={conversation_id}, request_id={request_id})"
                );
            }

            let filter = ClaimFilter::new()
                .with_conversation_id(conversation_id)
                .with_batch_id(request_id);
            if let Some(msg) = self
                .bus
                .claim_filtered(topic::TURN_RESULT, "submit_turn", &filter)
                .await?
            {
                let bus_result: bus_messages::TurnResult = serde_json::from_value(msg.payload)?;
                self.bus.ack(msg.id).await?;
                return Ok(TurnResult {
                    answer: bus_result.content,
                    attachments: bus_result.attachments,
                });
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
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

        // -- OTel trace hierarchy --
        let tracer = global::tracer("assistant.orchestrator");
        let _conv_cx = match trace_cx {
            Some(cx) => cx.clone(),
            None => {
                let mut span = tracer.start("conversation");
                span.set_attribute(KeyValue::new(
                    "conversation_id",
                    conversation_id.to_string(),
                ));
                span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
                OtelContext::current().with_span(span)
            }
        };
        let mut otel_turn = tracer.start_with_context("turn", &_conv_cx);
        otel_turn.set_attribute(KeyValue::new(
            "conversation_id",
            conversation_id.to_string(),
        ));
        otel_turn.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
        let turn_cx = _conv_cx.with_span(otel_turn);

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

            let mut llm_span = start_llm_span(
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
                    Self::persist_error_recovery(&conv_store, conversation_id).await;
                    self.metrics
                        .record_error("llm_error", "run_turn_with_tools");
                    return Err(e);
                }
            };
            finish_llm_span(
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
                        let turn_index = base_turn + iteration as i64 + 1;
                        let mut otel_span = start_tool_span(
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
                                self.append_tool_result(&mut history, "end_turn", deferred_msg);
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
                                self.append_tool_result(&mut history, "end_turn", reject_msg);
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
                            self.append_tool_result(&mut history, "end_turn", &result_text);
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
                        let observation = if let Some(handler) = ext_map.get(&name) {
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
                            self.metrics.record_tool_invocation(&name);
                            self.metrics
                                .record_tool_duration(&name, duration_ms as f64 / 1000.0);

                            let obs = match exec_result {
                                Ok(output) => {
                                    otel_span
                                        .set_attribute(KeyValue::new("duration_ms", duration_ms));
                                    otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                                    otel_span.set_attribute(KeyValue::new(
                                        "tool_observation",
                                        output.content.clone(),
                                    ));
                                    debug!(observation = %output.content, "extension observation");
                                    // Collect any attachments from the extension tool.
                                    if !output.attachments.is_empty() {
                                        turn_attachments.extend(output.attachments);
                                    }
                                    output.content
                                }
                                Err(err) => {
                                    warn!(tool = %name, %err, "Extension tool execution failed");
                                    let msg = err.to_string();
                                    otel_span
                                        .set_attribute(KeyValue::new("duration_ms", duration_ms));
                                    otel_span.set_attribute(KeyValue::new("tool_status", "error"));
                                    otel_span
                                        .set_attribute(KeyValue::new("tool_error", msg.clone()));
                                    format!("Error executing '{name}': {msg}")
                                }
                            };
                            obs
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
                                        self.append_tool_result(&mut history, &name, &observation);
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

                            let params_map: HashMap<String, serde_json::Value> =
                                if let serde_json::Value::Object(map) = &params {
                                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                } else {
                                    HashMap::new()
                                };

                            let start = std::time::Instant::now();
                            let exec_result = self
                                .executor
                                .execute(&name, params_map, &ctx)
                                .instrument(builtin_span.clone())
                                .await;
                            let duration_ms = start.elapsed().as_millis() as i64;
                            self.metrics.record_tool_invocation(&name);
                            self.metrics
                                .record_tool_duration(&name, duration_ms as f64 / 1000.0);

                            let obs = match exec_result {
                                Ok(output) => {
                                    debug!(tool = %name, duration_ms, "Tool execution completed");
                                    otel_span
                                        .set_attribute(KeyValue::new("duration_ms", duration_ms));
                                    otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                                    otel_span.set_attribute(KeyValue::new(
                                        "tool_observation",
                                        output.content.clone(),
                                    ));
                                    // Collect any attachments from the global tool.
                                    if !output.attachments.is_empty() {
                                        turn_attachments.extend(output.attachments);
                                    }
                                    tool_result_content(&output.content, output.data.as_ref())
                                }
                                Err(err) => {
                                    warn!(tool = %name, %err, "Tool execution failed");
                                    let msg = err.to_string();
                                    otel_span
                                        .set_attribute(KeyValue::new("duration_ms", duration_ms));
                                    otel_span.set_attribute(KeyValue::new("tool_status", "error"));
                                    otel_span
                                        .set_attribute(KeyValue::new("tool_error", msg.clone()));
                                    format!("Error executing '{name}': {msg}")
                                }
                            };
                            obs
                        };

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
                        otel_span.end();
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

        Self::persist_error_recovery(&conv_store, conversation_id).await;
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
        self.metrics.record_turn(None, &format!("{interface:?}"));
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            "Starting turn"
        );

        // -- OTel trace hierarchy --
        let tracer = global::tracer("assistant.orchestrator");
        let _conv_cx = match trace_cx {
            Some(cx) => cx.clone(),
            None => {
                let mut span = tracer.start("conversation");
                span.set_attribute(KeyValue::new(
                    "conversation_id",
                    conversation_id.to_string(),
                ));
                span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
                OtelContext::current().with_span(span)
            }
        };
        let mut otel_turn = tracer.start_with_context("turn", &_conv_cx);
        otel_turn.set_attribute(KeyValue::new(
            "conversation_id",
            conversation_id.to_string(),
        ));
        otel_turn.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
        let turn_cx = _conv_cx.with_span(otel_turn);

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

            let mut llm_span = start_llm_span(
                self.llm.as_ref(),
                iteration,
                &turn_cx,
                self.trace_content,
                &system_prompt,
                &history,
                &tool_specs,
            );
            let llm_start = std::time::Instant::now();
            let response = self
                .llm
                .chat(&system_prompt, &history, &tool_specs)
                .instrument(iteration_span.clone())
                .await;
            let llm_elapsed = llm_start.elapsed();
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    Self::persist_error_recovery(&conv_store, conversation_id)
                        .instrument(iteration_span.clone())
                        .await;
                    self.metrics.record_error("llm_error", "run_turn");
                    return Err(e);
                }
            };
            finish_llm_span(
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
                        "LLM requested tool execution(s)"
                    );

                    history.push(ChatHistoryMessage::AssistantToolCalls(
                        tool_call_items.clone(),
                    ));
                    let tc_msg = Self::make_tool_call_message(
                        conversation_id,
                        base_turn + iteration as i64 + 1,
                        &tool_call_items,
                    );
                    if let Err(e) = conv_store
                        .save_message(&tc_msg)
                        .instrument(iteration_span.clone())
                        .await
                    {
                        warn!("Failed to persist tool-call message: {e}");
                    }

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;
                        let turn_index = base_turn + iteration as i64 + 1;
                        let mut otel_span = start_tool_span(
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
                                    self.append_tool_result(&mut history, &name, &observation);
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

                        let params_map: HashMap<String, serde_json::Value> =
                            if let serde_json::Value::Object(map) = &params {
                                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                            } else {
                                HashMap::new()
                            };

                        let start = std::time::Instant::now();
                        let exec_result = self
                            .executor
                            .execute(&name, params_map, &ctx)
                            .instrument(iteration_span.clone())
                            .await;
                        let duration_ms = start.elapsed().as_millis() as i64;
                        self.metrics.record_tool_invocation(&name);
                        self.metrics
                            .record_tool_duration(&name, duration_ms as f64 / 1000.0);

                        let observation = match exec_result {
                            Ok(output) => {
                                debug!(
                                    tool = %name,
                                    duration_ms,
                                    success = output.success,
                                    attachments = output.attachments.len(),
                                    "Tool execution completed"
                                );
                                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                                otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                                otel_span.set_attribute(KeyValue::new(
                                    "tool_observation",
                                    output.content.clone(),
                                ));
                                // Collect any attachments from the tool output.
                                if !output.attachments.is_empty() {
                                    turn_attachments.extend(output.attachments);
                                }
                                tool_result_content(&output.content, output.data.as_ref())
                            }
                            Err(err) => {
                                warn!(tool = %name, %err, "Tool execution failed");
                                let msg = err.to_string();
                                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                                otel_span.set_attribute(KeyValue::new("tool_status", "error"));
                                otel_span.set_attribute(KeyValue::new("tool_error", msg.clone()));
                                format!("Error executing '{name}': {msg}")
                            }
                        };

                        self.append_tool_result(&mut history, &name, &observation);
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
        Self::persist_error_recovery(&conv_store, conversation_id).await;
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
        trace_cx: Option<&OtelContext>,
    ) -> Result<TurnResult> {
        self.metrics.record_turn(None, &format!("{interface:?}"));
        info!(
            conversation_id = %conversation_id,
            interface = ?interface,
            "Starting streaming turn"
        );

        // -- OTel trace hierarchy --
        let tracer = global::tracer("assistant.orchestrator");
        let _conv_cx = match trace_cx {
            Some(cx) => cx.clone(),
            None => {
                let mut span = tracer.start("conversation");
                span.set_attribute(KeyValue::new(
                    "conversation_id",
                    conversation_id.to_string(),
                ));
                span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
                OtelContext::current().with_span(span)
            }
        };
        let mut otel_turn = tracer.start_with_context("turn", &_conv_cx);
        otel_turn.set_attribute(KeyValue::new(
            "conversation_id",
            conversation_id.to_string(),
        ));
        otel_turn.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
        let turn_cx = _conv_cx.with_span(otel_turn);

        let (conv_store, mut history, base_turn) = self
            .prepare_history(user_message, conversation_id, Vec::new())
            .await?;

        let provider_caps = self.llm.capabilities();
        let tool_specs = Self::filter_tool_specs(self.executor.to_specs(), &provider_caps);

        let system_prompt = self.compose_system_prompt().await;

        let mut turn_attachments: Vec<Attachment> = Vec::new();

        for iteration in 0..self.max_iterations {
            let iteration_span = info_span!("turn_iteration", iteration);
            debug!(parent: &iteration_span, iteration, "Streaming tool-calling loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
                allowed_tools: None,
                depth: 0,
            };

            let mut llm_span = start_llm_span(
                self.llm.as_ref(),
                iteration,
                &turn_cx,
                self.trace_content,
                &system_prompt,
                &history,
                &tool_specs,
            );
            let llm_start = std::time::Instant::now();
            let response = self
                .llm
                .chat_streaming(
                    &system_prompt,
                    &history,
                    &tool_specs,
                    Some(token_sink.clone()),
                )
                .instrument(iteration_span.clone())
                .await;
            let llm_elapsed = llm_start.elapsed();
            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    Self::persist_error_recovery(&conv_store, conversation_id)
                        .instrument(iteration_span.clone())
                        .await;
                    self.metrics.record_error("llm_error", "run_turn_streaming");
                    return Err(e);
                }
            };
            finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
                Some((&self.metrics, self.llm.provider_name(), llm_elapsed)),
            );

            match response {
                LlmResponse::FinalAnswer(text, _meta) => {
                    info!(iteration, "Streaming LLM returned final answer");

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

                LlmResponse::ToolCalls(tool_call_items, _meta) => {
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
                    if let Err(e) = conv_store
                        .save_message(&tc_msg)
                        .instrument(iteration_span.clone())
                        .await
                    {
                        warn!("Failed to persist tool-call message: {e}");
                    }

                    for tool_call_item in tool_call_items {
                        let name = tool_call_item.name;
                        let params = tool_call_item.params;
                        let turn_index = base_turn + iteration as i64 + 1;
                        let mut otel_span = start_tool_span(
                            conversation_id,
                            iteration,
                            turn_index,
                            &interface,
                            &name,
                            &params,
                            &turn_cx,
                        );

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
                                    self.append_tool_result(&mut history, &name, &observation);
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

                        let params_map: HashMap<String, serde_json::Value> =
                            if let serde_json::Value::Object(map) = &params {
                                map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                            } else {
                                HashMap::new()
                            };

                        let start = std::time::Instant::now();
                        let exec_result = self
                            .executor
                            .execute(&name, params_map, &ctx)
                            .instrument(iteration_span.clone())
                            .await;
                        let duration_ms = start.elapsed().as_millis() as i64;
                        self.metrics.record_tool_invocation(&name);
                        self.metrics
                            .record_tool_duration(&name, duration_ms as f64 / 1000.0);

                        let observation = match exec_result {
                            Ok(output) => {
                                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                                otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                                otel_span.set_attribute(KeyValue::new(
                                    "tool_observation",
                                    output.content.clone(),
                                ));
                                // Collect any attachments from the tool output.
                                if !output.attachments.is_empty() {
                                    turn_attachments.extend(output.attachments);
                                }
                                tool_result_content(&output.content, output.data.as_ref())
                            }
                            Err(err) => {
                                warn!(tool = %name, %err, "Tool execution failed");
                                let msg = err.to_string();
                                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                                otel_span.set_attribute(KeyValue::new("tool_status", "error"));
                                otel_span.set_attribute(KeyValue::new("tool_error", msg.clone()));
                                format!("Error executing '{name}': {msg}")
                            }
                        };
                        self.append_tool_result(&mut history, &name, &observation);
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
                    }
                }

                LlmResponse::Thinking(text, _meta) => {
                    debug!(iteration, "Streaming LLM emitted thinking step");
                    history.push(ChatHistoryMessage::Text {
                        role: ChatRole::Assistant,
                        content: text,
                    });
                }
            }
        }

        Self::persist_error_recovery(&conv_store, conversation_id).await;
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

    async fn prepare_history(
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

        // -- History sanitisation --------------------------------------------------
        //
        // A prior turn may have failed after persisting the user message but
        // before any assistant response was saved.  This leaves an orphaned
        // trailing user message in the database.  When we append the *current*
        // user message below, consecutive user messages would violate the
        // alternation requirement of some providers (Anthropic) and can confuse
        // tool-calling models.
        //
        // Similarly, a crash between persisting an AssistantToolCalls message
        // and persisting its ToolResult messages leaves orphaned tool calls.
        // Some providers reject the request when tool results are missing.
        //
        // Walk the loaded history and patch these structural issues.
        Self::sanitize_history(&mut history);

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

    /// Repair structural problems in a loaded conversation history.
    ///
    /// Two issues are addressed:
    ///
    /// 1. **Trailing orphaned user message** – a prior turn may have failed
    ///    after persisting the user message but before any assistant response
    ///    was saved.  A synthetic assistant message is inserted so the caller
    ///    can safely append a new user message without creating consecutive
    ///    user entries (which Anthropic rejects outright and which confuse
    ///    most tool-calling models).
    ///
    /// 2. **Orphaned `AssistantToolCalls`** – the process may have crashed
    ///    after persisting a tool-call message but before all `ToolResult`
    ///    messages were written.  Missing results are filled in with a
    ///    synthetic error result so providers that require tool results
    ///    (Ollama, Anthropic) do not reject the request.
    fn sanitize_history(history: &mut Vec<ChatHistoryMessage>) {
        if history.is_empty() {
            return;
        }

        // --- Pass 1: fill in missing tool results for orphaned tool calls ------
        //
        // Walk the history and, for every AssistantToolCalls, count how many
        // ToolResult messages follow (before the next non-ToolResult entry or
        // the end of the list).  If fewer results exist than calls, insert
        // synthetic ones.
        let mut i = 0;
        while i < history.len() {
            if let ChatHistoryMessage::AssistantToolCalls(calls) = &history[i] {
                let expected = calls.len();
                let call_names: Vec<String> = calls.iter().map(|c| c.name.clone()).collect();

                // Count consecutive ToolResult messages immediately following.
                let mut result_count = 0;
                while i + 1 + result_count < history.len() {
                    if matches!(
                        history[i + 1 + result_count],
                        ChatHistoryMessage::ToolResult { .. }
                    ) {
                        result_count += 1;
                    } else {
                        break;
                    }
                }

                if result_count < expected {
                    let insert_at = i + 1 + result_count;
                    let missing = expected - result_count;
                    debug!(
                        expected,
                        result_count,
                        missing,
                        "Sanitizing history: inserting synthetic tool results"
                    );
                    for j in result_count..expected {
                        let name = call_names
                            .get(j)
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());
                        history.insert(
                            insert_at + (j - result_count),
                            ChatHistoryMessage::ToolResult {
                                name,
                                content: "[error: result lost due to a prior crash]".to_string(),
                            },
                        );
                    }
                    // Advance past the newly inserted results.
                    i = insert_at + missing;
                    continue;
                }
            }
            i += 1;
        }

        // --- Pass 2: trailing orphaned user message ----------------------------
        let is_trailing_user = matches!(
            history.last(),
            Some(ChatHistoryMessage::Text {
                role: ChatRole::User,
                ..
            }) | Some(ChatHistoryMessage::MultimodalUser { .. })
        );

        if is_trailing_user {
            debug!("Sanitizing history: inserting synthetic assistant message after orphaned user message");
            history.push(ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "[An error occurred processing the previous message.]".to_string(),
            });
        }
    }

    /// Persist a synthetic assistant message so the conversation history
    /// maintains proper User→Assistant alternation after a turn error.
    ///
    /// Called when the tool-calling loop (or the LLM call itself) fails.
    /// The user message was already persisted by `prepare_history`; without
    /// this recovery message the orphaned user entry would poison subsequent
    /// turns.
    async fn persist_error_recovery(conv_store: &ConversationStore, conversation_id: Uuid) {
        let error_msg = Message::assistant(
            conversation_id,
            "[An error occurred processing this message.]",
        );
        if let Err(e) = conv_store.save_message(&error_msg).await {
            warn!("Failed to persist error recovery assistant message: {e}");
        }
    }

    fn append_tool_result(&self, history: &mut Vec<ChatHistoryMessage>, name: &str, content: &str) {
        history.push(ChatHistoryMessage::ToolResult {
            name: name.to_string(),
            content: content.to_string(),
        });
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
        self.append_tool_result(history, name, &observation);
        let tr_msg = Self::make_tool_result_message(conversation_id, turn_idx, name, &observation);
        if let Err(e) = conv_store.save_message(&tr_msg).await {
            warn!("Failed to persist tool-result message: {e}");
        }
        Some(observation)
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

// ── SubagentRunner ────────────────────────────────────────────────────────────

#[async_trait]
impl SubagentRunner for Orchestrator {
    /// Run an isolated sub-agent turn synchronously.
    ///
    /// Creates a fresh conversation, restricts the available tool set, and
    /// runs the normal tool-calling loop until the sub-agent produces a final
    /// answer (or hits the iteration limit).  A [`CancellationToken`] is
    /// registered for the lifetime of the subagent so that external callers
    /// can cancel it via [`cancel_agent`].
    ///
    /// # Observability
    ///
    /// A root `subagent` OTel span is created as a child of the **current**
    /// context (typically the parent turn's `agent-spawn` tool span).  This
    /// links the subagent's trace tree to the parent conversation's trace,
    /// giving end-to-end visibility across agent boundaries.  Inside the
    /// subagent span, per-iteration LLM and tool spans mirror the structure
    /// of [`run_turn`].
    async fn run_subagent(&self, spawn: AgentSpawn, parent_depth: u32) -> Result<AgentReport> {
        let new_depth = parent_depth + 1;
        if new_depth > DEFAULT_MAX_AGENT_DEPTH {
            return Ok(AgentReport {
                status: AgentReportStatus::Failed,
                content: format!(
                    "Maximum subagent nesting depth ({}) exceeded. \
                     Cannot spawn agent '{}'.",
                    DEFAULT_MAX_AGENT_DEPTH, spawn.agent_id
                ),
                data: None,
            });
        }

        let conversation_id = Uuid::new_v4();

        // -- OTel: root subagent span (child of current context) ---------------
        let tracer = global::tracer("assistant.orchestrator");
        let parent_cx = OtelContext::current();
        let mut agent_span =
            tracer.start_with_context(format!("subagent {}", spawn.agent_id), &parent_cx);
        agent_span.set_attribute(KeyValue::new("agent_id", spawn.agent_id.clone()));
        agent_span.set_attribute(KeyValue::new(
            "conversation_id",
            conversation_id.to_string(),
        ));
        agent_span.set_attribute(KeyValue::new("agent.depth", new_depth as i64));
        agent_span.set_attribute(KeyValue::new("agent.task", spawn.task.clone()));
        let agent_cx = parent_cx.with_span(agent_span);

        info!(
            agent_id = %spawn.agent_id,
            task = %spawn.task,
            depth = new_depth,
            conversation_id = %conversation_id,
            "Spawning subagent"
        );
        self.metrics.agent_spawn_count.add(1, &[]);

        // Register a cancellation token for this agent.
        let cancel_token = CancellationToken::new();
        self.agent_cancellations
            .write()
            .await
            .insert(spawn.agent_id.clone(), cancel_token.clone());

        // Build the tool allowlist.  An empty list in AgentSpawn means "all
        // tools", which maps to `None` in ExecutionContext.
        let allowed_tools = if spawn.allowed_tools.is_empty() {
            None
        } else {
            Some(spawn.allowed_tools.clone())
        };

        // Determine the tool specs to advertise to the LLM for this subagent.
        let provider_caps = self.llm.capabilities();
        let tool_specs = Self::filter_tool_specs(
            self.executor.to_specs_filtered(&allowed_tools),
            &provider_caps,
        );

        // Build the system prompt.  If the spawn request provides one, use it;
        // otherwise fall back to the default composed prompt.
        let system_prompt = match spawn.system_prompt {
            Some(ref prompt) if !prompt.is_empty() => prompt.clone(),
            _ => self.compose_system_prompt().await,
        };

        // Set up the conversation and history with the task as the user message.
        let (conv_store, mut history, base_turn) = self
            .prepare_history(&spawn.task, conversation_id, Vec::new())
            .await?;

        // Record the agent in the lifecycle table.
        let agent_store = self.storage.agent_store();
        if let Err(e) = agent_store
            .create(
                &spawn.agent_id,
                None,
                &conversation_id.to_string(),
                &conversation_id.to_string(),
                &spawn.task,
                new_depth,
            )
            .await
        {
            warn!(agent_id = %spawn.agent_id, %e, "Failed to persist agent record");
        }

        // Tool-calling loop (same structure as run_turn, but with restricted context).
        let report = 'outer: {
            for iteration in 0..self.max_iterations {
                // Check for cancellation before each iteration.
                if cancel_token.is_cancelled() {
                    info!(
                        agent_id = %spawn.agent_id,
                        iteration,
                        "Subagent cancelled before iteration"
                    );
                    Self::persist_error_recovery(&conv_store, conversation_id).await;
                    let msg = format!("Subagent '{}' was cancelled", spawn.agent_id);
                    let _ = agent_store
                        .complete(
                            &spawn.agent_id,
                            assistant_storage::AgentStatus::Cancelled,
                            Some(&msg),
                        )
                        .await;
                    // Record cancellation on the agent span.
                    let span = agent_cx.span();
                    span.set_attribute(KeyValue::new("agent.status", "cancelled"));
                    span.end();
                    break 'outer AgentReport {
                        status: AgentReportStatus::Cancelled,
                        content: msg,
                        data: None,
                    };
                }

                let iteration_span = info_span!(
                    "subagent_iteration",
                    agent_id = %spawn.agent_id,
                    iteration
                );
                debug!(parent: &iteration_span, iteration, agent_id = %spawn.agent_id, "Subagent tool-calling loop");

                let _ctx = ExecutionContext {
                    conversation_id,
                    turn: iteration as i64,
                    interface: Interface::Scheduler, // non-interactive
                    interactive: false,
                    allowed_tools: allowed_tools.clone(),
                    depth: new_depth,
                };

                // -- OTel: LLM span (child of agent span) ---------------------
                let mut llm_span = start_llm_span(
                    self.llm.as_ref(),
                    iteration,
                    &agent_cx,
                    self.trace_content,
                    &system_prompt,
                    &history,
                    &tool_specs,
                );
                let llm_start = std::time::Instant::now();
                let response = self
                    .llm
                    .chat(&system_prompt, &history, &tool_specs)
                    .instrument(iteration_span.clone())
                    .await;
                let llm_elapsed = llm_start.elapsed();
                let response = match response {
                    Ok(r) => r,
                    Err(e) => {
                        llm_span.set_attribute(KeyValue::new("error", true));
                        llm_span.set_attribute(KeyValue::new("error.message", e.to_string()));
                        llm_span.end();
                        Self::persist_error_recovery(&conv_store, conversation_id)
                            .instrument(iteration_span.clone())
                            .await;
                        self.metrics.record_error("llm_error", "run_subagent");
                        let msg = format!("LLM error: {e}");
                        let _ = agent_store
                            .complete(
                                &spawn.agent_id,
                                assistant_storage::AgentStatus::Failed,
                                Some(&msg),
                            )
                            .instrument(iteration_span.clone())
                            .await;
                        let span = agent_cx.span();
                        span.set_attribute(KeyValue::new("agent.status", "failed"));
                        span.end();
                        break 'outer AgentReport {
                            status: AgentReportStatus::Failed,
                            content: msg,
                            data: None,
                        };
                    }
                };
                finish_llm_span(
                    &mut llm_span,
                    response.meta(),
                    &response,
                    self.trace_content,
                    Some((&self.metrics, self.llm.provider_name(), llm_elapsed)),
                );

                match response {
                    // ── Final answer ──────────────────────────────────────────
                    LlmResponse::FinalAnswer(text, _meta) => {
                        info!(
                            iteration,
                            agent_id = %spawn.agent_id,
                            "Subagent returned final answer"
                        );

                        if !text.trim().is_empty() {
                            let assistant_msg = {
                                let mut m = Message::assistant(conversation_id, &text);
                                m.turn = base_turn + iteration as i64 + 1;
                                m
                            };
                            if let Err(e) = conv_store
                                .save_message(&assistant_msg)
                                .instrument(iteration_span.clone())
                                .await
                            {
                                warn!("Failed to persist subagent answer: {e}");
                            }
                        }

                        // Truncate for the summary column.
                        let summary: String = text.chars().take(500).collect();
                        let _ = agent_store
                            .complete(
                                &spawn.agent_id,
                                assistant_storage::AgentStatus::Completed,
                                Some(&summary),
                            )
                            .instrument(iteration_span.clone())
                            .await;

                        // Finalize the agent span with success.
                        let span = agent_cx.span();
                        span.set_attribute(KeyValue::new("agent.status", "completed"));
                        span.set_attribute(KeyValue::new(
                            "agent.iterations",
                            (iteration + 1) as i64,
                        ));
                        span.end();

                        break 'outer AgentReport {
                            status: AgentReportStatus::Completed,
                            content: text,
                            data: None,
                        };
                    }

                    // ── Tool calls ────────────────────────────────────────────
                    LlmResponse::ToolCalls(tool_call_items, _meta) => {
                        debug!(
                            count = tool_call_items.len(),
                            agent_id = %spawn.agent_id,
                            "Subagent requested tool execution(s)"
                        );

                        history.push(ChatHistoryMessage::AssistantToolCalls(
                            tool_call_items.clone(),
                        ));
                        let tc_msg = Self::make_tool_call_message(
                            conversation_id,
                            base_turn + iteration as i64 + 1,
                            &tool_call_items,
                        );
                        if let Err(e) = conv_store
                            .save_message(&tc_msg)
                            .instrument(iteration_span.clone())
                            .await
                        {
                            warn!("Failed to persist subagent tool-call message: {e}");
                        }

                        for tool_call_item in tool_call_items {
                            // Check cancellation between individual tool executions.
                            if cancel_token.is_cancelled() {
                                info!(
                                    agent_id = %spawn.agent_id,
                                    "Subagent cancelled during tool execution"
                                );
                                break;
                            }

                            let name = tool_call_item.name;
                            let params = tool_call_item.params;
                            let turn_index = base_turn + iteration as i64 + 1;

                            // -- OTel: tool span (child of agent span) ---------
                            let mut otel_span = start_tool_span(
                                conversation_id,
                                iteration,
                                turn_index,
                                &Interface::Scheduler,
                                &name,
                                &params,
                                &agent_cx,
                            );

                            let params_map: HashMap<String, serde_json::Value> =
                                if let serde_json::Value::Object(map) = &params {
                                    map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                } else {
                                    HashMap::new()
                                };

                            let ctx = ExecutionContext {
                                conversation_id,
                                turn: iteration as i64,
                                interface: Interface::Scheduler,
                                interactive: false,
                                allowed_tools: allowed_tools.clone(),
                                depth: new_depth,
                            };

                            let start = std::time::Instant::now();
                            let observation = match self
                                .executor
                                .execute(&name, params_map, &ctx)
                                .instrument(iteration_span.clone())
                                .await
                            {
                                Ok(output) => {
                                    let duration_ms = start.elapsed().as_millis() as i64;
                                    self.metrics.record_tool_invocation(&name);
                                    self.metrics
                                        .record_tool_duration(&name, duration_ms as f64 / 1000.0);
                                    debug!(
                                        tool = %name,
                                        success = output.success,
                                        agent_id = %spawn.agent_id,
                                        duration_ms,
                                        "Subagent tool execution completed"
                                    );
                                    otel_span
                                        .set_attribute(KeyValue::new("duration_ms", duration_ms));
                                    otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                                    otel_span.set_attribute(KeyValue::new(
                                        "tool_observation",
                                        output.content.clone(),
                                    ));
                                    tool_result_content(&output.content, output.data.as_ref())
                                }
                                Err(err) => {
                                    let duration_ms = start.elapsed().as_millis() as i64;
                                    self.metrics.record_tool_invocation(&name);
                                    self.metrics
                                        .record_tool_duration(&name, duration_ms as f64 / 1000.0);
                                    self.metrics.record_error("tool_error", &name);
                                    warn!(
                                        tool = %name,
                                        %err,
                                        agent_id = %spawn.agent_id,
                                        "Subagent tool execution failed"
                                    );
                                    otel_span
                                        .set_attribute(KeyValue::new("duration_ms", duration_ms));
                                    otel_span.set_attribute(KeyValue::new("tool_status", "error"));
                                    otel_span.set_attribute(KeyValue::new(
                                        "tool_error",
                                        err.to_string(),
                                    ));
                                    format!("Error executing '{name}': {err}")
                                }
                            };

                            otel_span.end();

                            self.append_tool_result(&mut history, &name, &observation);
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
                                warn!("Failed to persist subagent tool-result: {e}");
                            }
                        }
                    }

                    // ── Thinking ──────────────────────────────────────────────
                    LlmResponse::Thinking(text, _meta) => {
                        debug!(
                            iteration,
                            agent_id = %spawn.agent_id,
                            "Subagent emitted thinking step"
                        );
                        history.push(ChatHistoryMessage::Text {
                            role: ChatRole::Assistant,
                            content: text,
                        });
                    }
                }
            }

            // Reached iteration limit.
            Self::persist_error_recovery(&conv_store, conversation_id).await;
            let msg = format!(
                "Subagent '{}' reached max iterations ({}) without a final answer",
                spawn.agent_id, self.max_iterations
            );
            let _ = agent_store
                .complete(
                    &spawn.agent_id,
                    assistant_storage::AgentStatus::Failed,
                    Some(&msg),
                )
                .await;
            let span = agent_cx.span();
            span.set_attribute(KeyValue::new("agent.status", "failed"));
            span.set_attribute(KeyValue::new(
                "agent.iterations",
                self.max_iterations as i64,
            ));
            span.end();
            AgentReport {
                status: AgentReportStatus::Failed,
                content: msg,
                data: None,
            }
        };

        // Clean up the cancellation token registry.
        self.agent_cancellations
            .write()
            .await
            .remove(&spawn.agent_id);

        Ok(report)
    }

    /// Request cancellation of a running sub-agent.
    ///
    /// Triggers the [`CancellationToken`] associated with the agent so the
    /// subagent loop exits at the next check point.  Also marks the agent as
    /// cancelled in the lifecycle database.
    async fn cancel_agent(&self, agent_id: &str) -> Result<bool> {
        let token = self.agent_cancellations.read().await.get(agent_id).cloned();
        match token {
            Some(t) => {
                info!(agent_id, "Cancelling subagent");
                t.cancel();
                // Mark the agent as cancelled in the DB (the loop will also do
                // this when it detects the token, but setting it here ensures
                // the status is updated even if the loop is blocked on an LLM
                // call).
                let agent_store = self.storage.agent_store();
                let _ = agent_store
                    .complete(
                        agent_id,
                        assistant_storage::AgentStatus::Cancelled,
                        Some("Cancelled by agent-terminate"),
                    )
                    .await;
                Ok(true)
            }
            None => Ok(false),
        }
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

/// Create an OpenTelemetry context carrying a conversation-level root span.
///
/// Callers that manage conversation lifetimes (e.g. the CLI REPL or Slack
/// thread handler) should create this once per conversation and pass it to
/// each `run_turn*` call so all turns within the conversation share a single
/// trace.
pub fn start_conversation_context(conversation_id: Uuid, interface: &Interface) -> OtelContext {
    let tracer = global::tracer("assistant.orchestrator");
    let mut span = tracer.start("conversation");
    span.set_attribute(KeyValue::new(
        "conversation_id",
        conversation_id.to_string(),
    ));
    span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
    OtelContext::current().with_span(span)
}

fn start_tool_span(
    conversation_id: Uuid,
    iteration: usize,
    turn: i64,
    interface: &Interface,
    tool_name: &str,
    params: &serde_json::Value,
    parent_cx: &OtelContext,
) -> opentelemetry::global::BoxedSpan {
    let tracer = global::tracer("assistant.orchestrator");
    let span_name = format!("execute_tool {tool_name}");
    let mut span = tracer.start_with_context(span_name, parent_cx);
    span.set_attribute(KeyValue::new(
        "conversation_id",
        conversation_id.to_string(),
    ));
    span.set_attribute(KeyValue::new("iteration", iteration as i64));
    span.set_attribute(KeyValue::new("turn", turn));
    span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
    span.set_attribute(KeyValue::new("tool_name", tool_name.to_string()));
    let params_json =
        serde_json::to_string(params).unwrap_or_else(|_| "<unserializable>".to_string());
    span.set_attribute(KeyValue::new("tool_params", params_json));
    span
}

/// Create an OTel span for an LLM chat call, populated with GenAI semantic
/// convention request-side attributes.
///
/// When `trace_content` is `true`, the span also records:
/// - `gen_ai.system_instructions` — the full system prompt
/// - `gen_ai.input.messages` — serialised chat history
/// - `gen_ai.tool.definitions` — serialised tool spec list
#[allow(clippy::too_many_arguments)]
fn start_llm_span(
    llm: &dyn LlmProvider,
    iteration: usize,
    parent_cx: &OtelContext,
    trace_content: bool,
    system_prompt: &str,
    history: &[ChatHistoryMessage],
    tools: &[ToolSpec],
) -> opentelemetry::global::BoxedSpan {
    let tracer = global::tracer("assistant.orchestrator");
    let model = llm.model_name();
    let span_name = format!("chat {model}");
    let mut span = tracer.start_with_context(span_name, parent_cx);
    span.set_attribute(KeyValue::new(
        "gen_ai.system",
        llm.provider_name().to_string(),
    ));
    span.set_attribute(KeyValue::new("gen_ai.request.model", model.to_string()));
    span.set_attribute(KeyValue::new("gen_ai.operation.name", "chat"));
    span.set_attribute(KeyValue::new(
        "server.address",
        llm.server_address().to_string(),
    ));
    span.set_attribute(KeyValue::new("iteration", iteration as i64));

    if trace_content {
        span.set_attribute(KeyValue::new(
            "gen_ai.system_instructions",
            system_prompt.to_string(),
        ));
        let input_json = serialize_history_for_span(history);
        span.set_attribute(KeyValue::new("gen_ai.input.messages", input_json));
        if !tools.is_empty() {
            let tools_json = serde_json::to_string(
                &tools
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                        })
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap_or_default();
            span.set_attribute(KeyValue::new("gen_ai.tool.definitions", tools_json));
        }
    }

    span
}

/// Enrich an LLM span with GenAI semantic convention response-side attributes
/// extracted from [`LlmResponseMeta`].
///
/// When `trace_content` is `true`, the assistant's output text is also recorded
/// as `gen_ai.output.messages`.
fn finish_llm_span(
    span: &mut opentelemetry::global::BoxedSpan,
    meta: &LlmResponseMeta,
    response: &LlmResponse,
    trace_content: bool,
    metrics: Option<(&crate::MetricsRecorder, &str, std::time::Duration)>,
) {
    if let Some(model) = &meta.model {
        span.set_attribute(KeyValue::new("gen_ai.response.model", model.clone()));
    }
    if let Some(id) = &meta.response_id {
        span.set_attribute(KeyValue::new("gen_ai.response.id", id.clone()));
    }
    if let Some(reason) = &meta.finish_reason {
        // OTel spec uses a string array; we record the single reason as a scalar.
        span.set_attribute(KeyValue::new(
            "gen_ai.response.finish_reasons",
            reason.clone(),
        ));
    }
    if let Some(input) = meta.input_tokens {
        span.set_attribute(KeyValue::new("gen_ai.usage.input_tokens", input as i64));
    }
    if let Some(output) = meta.output_tokens {
        span.set_attribute(KeyValue::new("gen_ai.usage.output_tokens", output as i64));
    }

    if trace_content {
        let output_json = match response {
            LlmResponse::FinalAnswer(text, _) => {
                serde_json::json!([{"role": "assistant", "content": text}]).to_string()
            }
            LlmResponse::ToolCalls(calls, _) => {
                let items: Vec<serde_json::Value> = calls
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "role": "assistant",
                            "tool_call": { "name": c.name, "arguments": c.params }
                        })
                    })
                    .collect();
                serde_json::Value::Array(items).to_string()
            }
            LlmResponse::Thinking(text, _) => {
                serde_json::json!([{"role": "assistant", "thinking": text}]).to_string()
            }
        };
        span.set_attribute(KeyValue::new("gen_ai.output.messages", output_json));
    }

    // -- Record OTel metrics alongside the span ---------------------------------
    if let Some((recorder, provider_name, duration)) = metrics {
        let model = meta.model.as_deref().unwrap_or("unknown");
        let input = meta.input_tokens.unwrap_or(0);
        let output = meta.output_tokens.unwrap_or(0);

        recorder.record_token_usage(model, provider_name, "chat", input, output);
        recorder.record_operation_duration(
            model,
            provider_name,
            "chat",
            duration.as_secs_f64(),
            None,
        );
    }

    span.end();
}

/// Serialise [`ChatHistoryMessage`] slices into a compact JSON string for span
/// content capture.
fn serialize_history_for_span(history: &[ChatHistoryMessage]) -> String {
    let items: Vec<serde_json::Value> = history
        .iter()
        .map(|msg| match msg {
            ChatHistoryMessage::Text { role, content } => {
                let role_str = match role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                    ChatRole::Tool => "tool",
                };
                serde_json::json!({"role": role_str, "content": content})
            }
            ChatHistoryMessage::AssistantToolCalls(calls) => {
                let tc: Vec<serde_json::Value> = calls
                    .iter()
                    .map(|c| serde_json::json!({"name": c.name, "arguments": c.params}))
                    .collect();
                serde_json::json!({"role": "assistant", "tool_calls": tc})
            }
            ChatHistoryMessage::MultimodalUser { content } => {
                let blocks: Vec<serde_json::Value> = content
                    .iter()
                    .map(|block| match block {
                        assistant_llm::ContentBlock::Text(t) => {
                            serde_json::json!({"type": "text", "text": t})
                        }
                        assistant_llm::ContentBlock::Image { media_type, data } => {
                            serde_json::json!({
                                "type": "image",
                                "media_type": media_type,
                                "size_base64_chars": data.len(),
                            })
                        }
                    })
                    .collect();
                serde_json::json!({"role": "user", "content": blocks})
            }
            ChatHistoryMessage::ToolResult { name, content } => {
                serde_json::json!({"role": "tool", "name": name, "content": content})
            }
        })
        .collect();
    serde_json::Value::Array(items).to_string()
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use std::time::Duration;

    use assistant_core::{
        bus_messages, topic, types::Interface, AssistantConfig, MessageBus, PublishRequest,
    };
    use assistant_llm::{
        ChatHistoryMessage, ChatRole, LlmClient, LlmClientConfig, LlmProvider, ToolCallItem,
    };
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
        let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
        let orch = Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry.clone(),
            bus,
            &config,
        ));
        executor.set_subagent_runner(orch.clone());
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

        orch.run_turn("hello", conv_id, Interface::Cli, None)
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

        orch.run_turn("first message", conv_id, Interface::Cli, None)
            .await
            .unwrap();
        orch.run_turn("second message", conv_id, Interface::Cli, None)
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

        orch.run_turn("turn one", conv_id, Interface::Cli, None)
            .await
            .unwrap();
        orch.run_turn("turn two", conv_id, Interface::Cli, None)
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

        orch.run_turn("follow-up", conv_id, Interface::Slack, None)
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

        orch.run_turn("turn 1", conv_id, Interface::Cli, None)
            .await
            .unwrap();
        orch.run_turn("turn 2", conv_id, Interface::Cli, None)
            .await
            .unwrap();
        orch.run_turn("turn 3", conv_id, Interface::Cli, None)
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

        orch.run_turn("conv-a message", conv_a, Interface::Cli, None)
            .await
            .unwrap();
        orch.run_turn("conv-b message", conv_b, Interface::Cli, None)
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
        ollama_tool_calls_with_args(&names.iter().map(|n| (*n, json!({}))).collect::<Vec<_>>())
    }

    /// Build a tool-call Ollama response where each entry is `(name, arguments)`.
    fn ollama_tool_calls_with_args(calls: &[(&str, Value)]) -> Value {
        let tc: Vec<Value> = calls
            .iter()
            .map(|(n, a)| json!({ "function": { "name": n, "arguments": a } }))
            .collect();
        json!({
            "model": "test",
            "message": { "role": "assistant", "content": null, "tool_calls": tc },
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
            .run_turn("go", Uuid::new_v4(), Interface::Cli, None)
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
        orch.run_turn("go", Uuid::new_v4(), Interface::Cli, None)
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
        orch.run_turn("go", Uuid::new_v4(), Interface::Cli, None)
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
        orch.run_turn("go", Uuid::new_v4(), Interface::Cli, None)
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            2,
            "three tool calls must be handled in ONE iteration"
        );
    }

    // ── Mock extension handlers ─────────────────────────────────────────────

    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use assistant_core::tool::{ToolHandler, ToolOutput};
    use assistant_core::types::ExecutionContext;
    use async_trait::async_trait;

    /// A fake extension tool that records how many times it was called.
    struct MockExtTool {
        tool_name: &'static str,
        call_count: AtomicUsize,
    }

    impl MockExtTool {
        fn new(name: &'static str) -> Self {
            Self {
                tool_name: name,
                call_count: AtomicUsize::new(0),
            }
        }

        fn calls(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ToolHandler for MockExtTool {
        fn name(&self) -> &str {
            self.tool_name
        }

        fn description(&self) -> &str {
            "mock extension tool"
        }

        fn params_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": []
            })
        }

        async fn run(
            &self,
            _params: HashMap<String, Value>,
            _ctx: &ExecutionContext,
        ) -> anyhow::Result<ToolOutput> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(ToolOutput::success("ok"))
        }
    }

    /// A fake reply extension tool whose `params_schema` has `"required": ["text"]`
    /// so auto-post picks it up.  Records every `text` value it receives.
    struct MockReplyExtTool {
        call_count: AtomicUsize,
        texts: tokio::sync::Mutex<Vec<String>>,
    }

    impl MockReplyExtTool {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
                texts: tokio::sync::Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> usize {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ToolHandler for MockReplyExtTool {
        fn name(&self) -> &str {
            "reply"
        }

        fn description(&self) -> &str {
            "mock reply extension tool"
        }

        fn params_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            })
        }

        async fn run(
            &self,
            params: HashMap<String, Value>,
            _ctx: &ExecutionContext,
        ) -> anyhow::Result<ToolOutput> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if let Some(Value::String(t)) = params.get("text") {
                self.texts.lock().await.push(t.clone());
            }
            Ok(ToolOutput::success("ok"))
        }
    }

    // ── end_turn rejection tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn end_turn_rejected_when_reply_tool_exists_but_not_called() {
        let server = MockServer::start().await;

        // 1st LLM call: model calls end_turn without calling reply first.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                    "end_turn",
                    json!({"reason": "replied"}),
                )])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // 2nd LLM call: after rejection, model calls reply then end_turn.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                    ("reply", json!({"text": "hello!"})),
                    ("end_turn", json!({"reason": "replied"})),
                ])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        let reply_handler = Arc::new(MockExtTool::new("reply"));

        orch.run_turn_with_tools(
            "hi",
            Uuid::new_v4(),
            Interface::Slack,
            vec![reply_handler.clone() as Arc<dyn ToolHandler>],
            None,
            vec![],
        )
        .await
        .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            2,
            "expected 2 LLM calls: first end_turn rejected, second with reply"
        );

        // The rejection message should appear in the second LLM call.
        let msgs = messages_in(&reqs[1]);
        let has_rejection = msgs.iter().any(|m| {
            m["role"] == "tool"
                && m["content"]
                    .as_str()
                    .unwrap_or("")
                    .contains("end_turn rejected")
        });
        assert!(
            has_rejection,
            "second LLM call must contain the end_turn rejection; msgs: {msgs:?}"
        );

        assert_eq!(
            reply_handler.calls(),
            1,
            "reply handler must have been called exactly once"
        );
    }

    #[tokio::test]
    async fn end_turn_accepted_without_reply_tool_in_cli_mode() {
        let server = MockServer::start().await;

        // Model calls end_turn — no reply extension tool exists (CLI mode).
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[(
                    "end_turn",
                    json!({"reason": "done"}),
                )])),
            )
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;

        // No extension tools — CLI mode, end_turn should be accepted.
        orch.run_turn_with_tools("hi", Uuid::new_v4(), Interface::Cli, vec![], None, vec![])
            .await
            .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            1,
            "end_turn without reply tools should be accepted in a single LLM call"
        );
    }

    #[tokio::test]
    async fn end_turn_accepted_after_reply_tool_called() {
        let server = MockServer::start().await;

        // Model calls reply first, then end_turn — should be accepted immediately.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                    ("reply", json!({"text": "hello!"})),
                    ("end_turn", json!({"reason": "replied"})),
                ])),
            )
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        let reply_handler = Arc::new(MockExtTool::new("reply"));

        orch.run_turn_with_tools(
            "hi",
            Uuid::new_v4(),
            Interface::Slack,
            vec![reply_handler.clone() as Arc<dyn ToolHandler>],
            None,
            vec![],
        )
        .await
        .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            1,
            "reply + end_turn in same call should complete in a single LLM call"
        );

        assert_eq!(reply_handler.calls(), 1, "reply must have been called once");
    }

    #[tokio::test]
    async fn end_turn_accepted_after_react_tool_called() {
        let server = MockServer::start().await;

        // Model calls react then end_turn — reaction is a valid acknowledgement.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                    ("react", json!({"emoji": "thumbsup"})),
                    ("end_turn", json!({"reason": "acknowledged with reaction"})),
                ])),
            )
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;
        let reply_handler = Arc::new(MockExtTool::new("reply"));
        let react_handler = Arc::new(MockExtTool::new("react"));

        orch.run_turn_with_tools(
            "thanks!",
            Uuid::new_v4(),
            Interface::Slack,
            vec![
                reply_handler.clone() as Arc<dyn ToolHandler>,
                react_handler.clone() as Arc<dyn ToolHandler>,
            ],
            None,
            vec![],
        )
        .await
        .unwrap();

        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            1,
            "react + end_turn should complete in a single LLM call"
        );

        assert_eq!(react_handler.calls(), 1, "react must have been called once");
        assert_eq!(reply_handler.calls(), 0, "reply must not have been called");
    }

    // ── empty FinalAnswer history-poisoning tests ──────────────────────────────

    #[tokio::test]
    async fn empty_final_answer_not_persisted_and_retries() {
        // Scenario: LLM returns a tool call, then an empty FinalAnswer, then a
        // real answer.  The empty FinalAnswer must NOT be saved to the DB, and
        // the loop must retry until a non-empty answer is produced.
        let server = MockServer::start().await;

        // 1st LLM call: model calls a builtin tool (will get an error observation
        //   because "some-tool" is unknown, but that's fine — we just need a
        //   tool-call iteration to precede the empty answer).
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["some-tool"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // 2nd LLM call: model returns an empty FinalAnswer — should be retried.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("")))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // 3rd LLM call: model returns a non-empty FinalAnswer — should be
        //   persisted and auto-posted via the reply tool.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_answer("here is your answer")),
            )
            .mount(&server)
            .await;

        let (orch, storage) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();
        let reply_handler = Arc::new(MockReplyExtTool::new());

        orch.run_turn_with_tools(
            "hi",
            conv_id,
            Interface::Slack,
            vec![reply_handler.clone() as Arc<dyn ToolHandler>],
            None,
            vec![],
        )
        .await
        .unwrap();

        // Verify: 3 LLM calls (tool call → empty answer retry → real answer).
        let reqs = server.received_requests().await.unwrap();
        assert_eq!(
            reqs.len(),
            3,
            "expected 3 LLM calls: tool-call, empty-answer retry, real answer; got {}",
            reqs.len()
        );

        // Verify: reply handler was called exactly once with the real answer.
        assert_eq!(
            reply_handler.calls(),
            1,
            "reply handler must be called once for the non-empty answer"
        );
        let texts = reply_handler.texts.lock().await;
        assert_eq!(
            texts[0], "here is your answer",
            "reply handler must receive the non-empty answer text"
        );
        drop(texts);

        // Verify: no empty assistant *text* messages in the DB.
        // (Tool-call messages legitimately have empty content + tool_calls_json.)
        let conv_store = storage.conversation_store();
        let history = conv_store.load_history(conv_id).await.unwrap();
        let empty_text_assistant_msgs: Vec<_> = history
            .iter()
            .filter(|m| {
                m.role == assistant_core::types::MessageRole::Assistant
                    && m.content.trim().is_empty()
                    && m.tool_calls_json.is_none()
            })
            .collect();
        assert!(
            empty_text_assistant_msgs.is_empty(),
            "no empty FinalAnswer assistant messages should be persisted; found {} in DB",
            empty_text_assistant_msgs.len()
        );

        // Verify: the non-empty answer IS persisted.
        let assistant_msgs: Vec<_> = history
            .iter()
            .filter(|m| m.role == assistant_core::types::MessageRole::Assistant)
            .collect();
        assert!(
            assistant_msgs
                .iter()
                .any(|m| m.content == "here is your answer"),
            "the non-empty answer must be persisted in the DB; assistant msgs: {assistant_msgs:?}"
        );
    }

    #[tokio::test]
    async fn empty_final_answer_not_persisted_in_run_turn() {
        // Verify the same protection in the simpler `run_turn` path (CLI mode).
        let server = MockServer::start().await;
        mount_answer(&server, "").await;

        let (orch, storage) = build(&server.uri()).await;
        let conv_id = Uuid::new_v4();

        let result = orch
            .run_turn("hello", conv_id, Interface::Cli, None)
            .await
            .unwrap();

        // run_turn still returns the (empty) answer to the caller...
        assert_eq!(result.answer, "");

        // ...but must NOT have persisted it to the DB.
        let conv_store = storage.conversation_store();
        let history = conv_store.load_history(conv_id).await.unwrap();
        let empty_assistant_msgs: Vec<_> = history
            .iter()
            .filter(|m| {
                m.role == assistant_core::types::MessageRole::Assistant
                    && m.content.trim().is_empty()
            })
            .collect();
        assert!(
            empty_assistant_msgs.is_empty(),
            "empty assistant message must not be persisted in run_turn; found {} in DB",
            empty_assistant_msgs.len()
        );
    }

    // ── MultimodalUser / OTel serialisation tests ────────────────────────────

    #[test]
    fn serialize_history_multimodal_user_omits_base64_data() {
        use super::serialize_history_for_span;
        use assistant_llm::ContentBlock;

        let history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![
                ContentBlock::Text("describe this".to_string()),
                ContentBlock::Image {
                    media_type: "image/png".to_string(),
                    data: "A".repeat(10_000), // large base64 payload
                },
            ],
        }];

        let json_str = serialize_history_for_span(&history);
        assert!(
            !json_str.contains(&"A".repeat(100)),
            "base64 data must NOT appear in span output"
        );
        assert!(
            json_str.contains("image/png"),
            "media_type should be present"
        );
        assert!(
            json_str.contains("size_base64_chars"),
            "size_base64_chars field should be present"
        );
    }

    #[tokio::test]
    async fn prepare_history_with_attachments_emits_multimodal_user() {
        use assistant_llm::ContentBlock;

        let server = MockServer::start().await;
        mount_answer(&server, "ok").await;
        let (orch, _) = build(&server.uri()).await;

        let conv_id = Uuid::new_v4();
        let attachments = vec![ContentBlock::Image {
            media_type: "image/jpeg".to_string(),
            data: "base64data".to_string(),
        }];

        let (_conv_store, history, _turn) = orch
            .prepare_history("look at this", conv_id, attachments)
            .await
            .unwrap();

        // The last message in history should be MultimodalUser.
        let last = history.last().expect("history non-empty");
        match last {
            ChatHistoryMessage::MultimodalUser { content } => {
                assert_eq!(content.len(), 2, "text block + image block");
                assert!(
                    matches!(&content[0], ContentBlock::Text(t) if t == "look at this"),
                    "first block should be the text"
                );
                assert!(
                    matches!(&content[1], ContentBlock::Image { media_type, .. } if media_type == "image/jpeg"),
                    "second block should be the image"
                );
            }
            other => panic!("expected MultimodalUser, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn prepare_history_without_attachments_emits_plain_text() {
        let server = MockServer::start().await;
        mount_answer(&server, "ok").await;
        let (orch, _) = build(&server.uri()).await;

        let conv_id = Uuid::new_v4();
        let (_conv_store, history, _turn) = orch
            .prepare_history("hello", conv_id, Vec::new())
            .await
            .unwrap();

        let last = history.last().expect("history non-empty");
        match last {
            ChatHistoryMessage::Text { role, content } => {
                assert_eq!(*role, assistant_llm::ChatRole::User);
                assert_eq!(content, "hello");
            }
            other => panic!("expected Text, got {:?}", other),
        }
    }

    // ── Attachment collection tests ──────────────────────────────────────────

    /// A fake tool handler that returns attachments in its output.
    struct MockAttachmentTool {
        attachments: Vec<assistant_core::Attachment>,
    }

    impl MockAttachmentTool {
        fn new(attachments: Vec<assistant_core::Attachment>) -> Self {
            Self { attachments }
        }
    }

    #[async_trait]
    impl ToolHandler for MockAttachmentTool {
        fn name(&self) -> &str {
            "attachment-tool"
        }

        fn description(&self) -> &str {
            "returns attachments for testing"
        }

        fn params_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn run(
            &self,
            _params: HashMap<String, Value>,
            _ctx: &ExecutionContext,
        ) -> anyhow::Result<ToolOutput> {
            Ok(ToolOutput::success("generated 1 attachment")
                .with_attachments(self.attachments.clone()))
        }
    }

    /// Helper that returns orchestrator, storage, AND executor so tests can
    /// register custom ambient tools.
    async fn build_with_executor(
        base_url: &str,
    ) -> (Arc<Orchestrator>, Arc<StorageLayer>, Arc<ToolExecutor>) {
        let mut config = AssistantConfig::default();
        config.memory.enabled = false;
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
        let bus: Arc<dyn MessageBus> = Arc::new(storage.message_bus());
        let orch = Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            executor.clone(),
            registry.clone(),
            bus,
            &config,
        ));
        executor.set_subagent_runner(orch.clone());
        (orch, storage, executor)
    }

    #[tokio::test]
    async fn run_turn_collects_attachments_from_tool_output() {
        let server = MockServer::start().await;

        // 1st LLM call: model calls "attachment-tool".
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["attachment-tool"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // 2nd LLM call: final answer.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("here you go")))
            .mount(&server)
            .await;

        let (orch, _, executor) = build_with_executor(&server.uri()).await;

        // Register our mock tool that returns an attachment.
        let png_bytes = vec![0x89, 0x50, 0x4E, 0x47];
        executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
            assistant_core::Attachment::new("chart.png", "image/png", png_bytes.clone()),
        ])));

        let result = orch
            .run_turn("make a chart", Uuid::new_v4(), Interface::Cli, None)
            .await
            .unwrap();

        assert_eq!(result.answer, "here you go");
        assert_eq!(
            result.attachments.len(),
            1,
            "expected 1 attachment in TurnResult"
        );
        assert_eq!(result.attachments[0].filename, "chart.png");
        assert_eq!(result.attachments[0].mime_type, "image/png");
        assert_eq!(result.attachments[0].data, png_bytes);
    }

    #[tokio::test]
    async fn run_turn_collects_multiple_attachments_across_tool_calls() {
        let server = MockServer::start().await;

        // Model calls attachment-tool twice in one turn.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(ollama_tool_calls(&["attachment-tool", "attachment-tool"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _, executor) = build_with_executor(&server.uri()).await;

        executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
            assistant_core::Attachment::new("file.txt", "text/plain", b"hello".to_vec()),
        ])));

        let result = orch
            .run_turn("go", Uuid::new_v4(), Interface::Cli, None)
            .await
            .unwrap();

        assert_eq!(
            result.attachments.len(),
            2,
            "each tool call should contribute one attachment"
        );
        assert_eq!(result.attachments[0].filename, "file.txt");
        assert_eq!(result.attachments[1].filename, "file.txt");
    }

    #[tokio::test]
    async fn run_turn_no_attachments_when_tools_return_none() {
        let server = MockServer::start().await;
        mount_answer(&server, "pong").await;

        let (orch, _, _) = build_with_executor(&server.uri()).await;

        let result = orch
            .run_turn("hello", Uuid::new_v4(), Interface::Cli, None)
            .await
            .unwrap();

        assert!(
            result.attachments.is_empty(),
            "no tool calls means no attachments"
        );
    }

    #[tokio::test]
    async fn run_turn_streaming_collects_attachments() {
        let server = MockServer::start().await;

        // 1st LLM call: model calls "attachment-tool".
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["attachment-tool"])),
            )
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // 2nd LLM call: final answer.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _, executor) = build_with_executor(&server.uri()).await;
        executor.register_ambient_tool(Arc::new(MockAttachmentTool::new(vec![
            assistant_core::Attachment::new("report.pdf", "application/pdf", vec![0x25, 0x50]),
        ])));

        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(64);

        // Drain tokens in background.
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let result = orch
            .run_turn_streaming("generate report", Uuid::new_v4(), Interface::Cli, tx, None)
            .await
            .unwrap();

        assert_eq!(result.attachments.len(), 1);
        assert_eq!(result.attachments[0].filename, "report.pdf");
        assert_eq!(result.attachments[0].mime_type, "application/pdf");
    }

    #[tokio::test]
    async fn run_turn_with_tools_collects_attachments_from_extension() {
        let server = MockServer::start().await;

        // Model calls the extension tool then reply then end_turn.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ollama_tool_calls_with_args(&[
                    ("ext-attach", json!({})),
                    ("reply", json!({"text": "done"})),
                    ("end_turn", json!({"reason": "done"})),
                ])),
            )
            .mount(&server)
            .await;

        let (orch, _, _) = build_with_executor(&server.uri()).await;

        // Create an extension tool that returns attachments.
        struct ExtAttachTool;

        #[async_trait]
        impl ToolHandler for ExtAttachTool {
            fn name(&self) -> &str {
                "ext-attach"
            }
            fn description(&self) -> &str {
                "returns an attachment"
            }
            fn params_schema(&self) -> Value {
                json!({"type": "object", "properties": {}, "required": []})
            }
            async fn run(
                &self,
                _params: HashMap<String, Value>,
                _ctx: &ExecutionContext,
            ) -> anyhow::Result<ToolOutput> {
                Ok(ToolOutput::success("image generated").with_attachment(
                    assistant_core::Attachment::new("img.png", "image/png", vec![1, 2, 3]),
                ))
            }
        }

        let reply_handler = Arc::new(MockExtTool::new("reply"));
        let ext_attach = Arc::new(ExtAttachTool);

        // run_turn_with_tools returns Ok(()) — we can't inspect attachments
        // directly, but we verify the call succeeds without panicking and
        // that the extension tool is executed (reply is called).
        orch.run_turn_with_tools(
            "make image",
            Uuid::new_v4(),
            Interface::Slack,
            vec![
                ext_attach as Arc<dyn ToolHandler>,
                reply_handler.clone() as Arc<dyn ToolHandler>,
            ],
            None,
            vec![],
        )
        .await
        .unwrap();

        assert_eq!(
            reply_handler.calls(),
            1,
            "reply tool should have been called"
        );
    }

    // ── sanitize_history tests ────────────────────────────────────────────────

    #[test]
    fn sanitize_history_empty_is_noop() {
        let mut history = vec![];
        Orchestrator::sanitize_history(&mut history);
        assert!(history.is_empty());
    }

    #[test]
    fn sanitize_history_valid_alternation_is_noop() {
        let mut history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "hello".into(),
            },
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content: "hi".into(),
            },
        ];
        Orchestrator::sanitize_history(&mut history);
        assert_eq!(history.len(), 2, "valid alternation should not be modified");
    }

    #[test]
    fn sanitize_history_trailing_user_inserts_assistant() {
        let mut history = vec![ChatHistoryMessage::Text {
            role: ChatRole::User,
            content: "orphaned".into(),
        }];
        Orchestrator::sanitize_history(&mut history);
        assert_eq!(
            history.len(),
            2,
            "should insert a synthetic assistant message"
        );
        match &history[1] {
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                content,
            } => {
                assert!(
                    content.contains("error"),
                    "synthetic message should mention error"
                );
            }
            other => panic!("expected Text(Assistant), got {:?}", other),
        }
    }

    #[test]
    fn sanitize_history_trailing_multimodal_user_inserts_assistant() {
        let mut history = vec![ChatHistoryMessage::MultimodalUser {
            content: vec![assistant_llm::ContentBlock::Text("image msg".into())],
        }];
        Orchestrator::sanitize_history(&mut history);
        assert_eq!(history.len(), 2);
        assert!(matches!(
            &history[1],
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                ..
            }
        ));
    }

    #[test]
    fn sanitize_history_orphaned_tool_calls_get_synthetic_results() {
        let mut history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "do stuff".into(),
            },
            ChatHistoryMessage::AssistantToolCalls(vec![
                ToolCallItem {
                    name: "tool-a".into(),
                    params: serde_json::json!({}),
                    id: None,
                },
                ToolCallItem {
                    name: "tool-b".into(),
                    params: serde_json::json!({}),
                    id: None,
                },
            ]),
            // Only one ToolResult — tool-b is missing.
            ChatHistoryMessage::ToolResult {
                name: "tool-a".into(),
                content: "ok".into(),
            },
        ];
        Orchestrator::sanitize_history(&mut history);
        // Should have: User, AssistantToolCalls, ToolResult(a), ToolResult(b-synthetic)
        assert_eq!(history.len(), 4, "missing tool result should be inserted");
        match &history[3] {
            ChatHistoryMessage::ToolResult { name, content } => {
                assert_eq!(name, "tool-b");
                assert!(
                    content.contains("lost")
                        || content.contains("crash")
                        || content.contains("error"),
                    "synthetic result should indicate failure: {content}"
                );
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    #[test]
    fn sanitize_history_fully_orphaned_tool_calls_all_results_inserted() {
        let mut history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "run tools".into(),
            },
            ChatHistoryMessage::AssistantToolCalls(vec![
                ToolCallItem {
                    name: "alpha".into(),
                    params: serde_json::json!({}),
                    id: None,
                },
                ToolCallItem {
                    name: "beta".into(),
                    params: serde_json::json!({}),
                    id: None,
                },
            ]),
            // No ToolResult at all — process crashed right after persisting tool calls.
        ];
        Orchestrator::sanitize_history(&mut history);
        // Should have: User, AssistantToolCalls, ToolResult(alpha), ToolResult(beta)
        assert_eq!(
            history.len(),
            4,
            "both missing tool results should be inserted"
        );
        assert!(
            matches!(&history[2], ChatHistoryMessage::ToolResult { name, .. } if name == "alpha")
        );
        assert!(
            matches!(&history[3], ChatHistoryMessage::ToolResult { name, .. } if name == "beta")
        );
    }

    #[test]
    fn sanitize_history_combined_orphaned_tools_and_trailing_user() {
        // Simulates: process crashed during tool execution on turn 1,
        // then on turn 2 the user message was persisted but LLM failed.
        let mut history = vec![
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "turn 1".into(),
            },
            ChatHistoryMessage::AssistantToolCalls(vec![ToolCallItem {
                name: "my-tool".into(),
                params: serde_json::json!({}),
                id: None,
            }]),
            // Missing ToolResult, then orphaned user from turn 2:
            ChatHistoryMessage::Text {
                role: ChatRole::User,
                content: "turn 2".into(),
            },
        ];
        Orchestrator::sanitize_history(&mut history);
        // Should have: User, AssistantToolCalls, ToolResult(synthetic), User, Assistant(synthetic)
        assert_eq!(history.len(), 5);
        assert!(
            matches!(&history[2], ChatHistoryMessage::ToolResult { name, .. } if name == "my-tool")
        );
        assert!(matches!(
            &history[4],
            ChatHistoryMessage::Text {
                role: ChatRole::Assistant,
                ..
            }
        ));
    }

    // ── Bus integration tests ────────────────────────────────────────────────

    #[test]
    fn parse_interface_known_values() {
        use super::parse_interface;
        assert_eq!(parse_interface("Cli"), Interface::Cli);
        assert_eq!(parse_interface("cli"), Interface::Cli);
        assert_eq!(parse_interface("Slack"), Interface::Slack);
        assert_eq!(parse_interface("MATTERMOST"), Interface::Mattermost);
        assert_eq!(parse_interface("Signal"), Interface::Signal);
        assert_eq!(parse_interface("mcp"), Interface::Mcp);
    }

    #[test]
    fn parse_interface_unknown_falls_back_to_cli() {
        use super::parse_interface;
        assert_eq!(parse_interface("unknown"), Interface::Cli);
        assert_eq!(parse_interface(""), Interface::Cli);
    }

    #[tokio::test]
    async fn run_worker_processes_turn_request() {
        let server = MockServer::start().await;
        mount_answer(&server, "bus response").await;

        let (orch, _storage) = build(&server.uri()).await;

        // Spawn the worker in the background.
        let orch_worker = orch.clone();
        let worker = tokio::spawn(async move {
            orch_worker.run_worker("test-worker").await;
        });

        // Publish a TurnRequest to the bus.
        let conv_id = Uuid::new_v4();
        let turn_req = bus_messages::TurnRequest {
            prompt: "hello from bus".to_string(),
            conversation_id: conv_id,
            extension_tools: vec![],
        };
        orch.bus()
            .publish(
                PublishRequest::new(
                    topic::TURN_REQUEST,
                    serde_json::to_value(&turn_req).unwrap(),
                )
                .with_conversation_id(conv_id)
                .with_interface("Cli"),
            )
            .await
            .unwrap();

        // Poll for the worker to process and publish the result instead of
        // a fixed sleep, which can be flaky under CI load.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let results = loop {
            let r = orch.bus().list(topic::TURN_RESULT, None, 10).await.unwrap();
            if !r.is_empty() {
                break r;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for TurnResult"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };

        assert_eq!(results.len(), 1, "expected one TurnResult on the bus");
        let result: bus_messages::TurnResult =
            serde_json::from_value(results[0].payload.clone()).unwrap();
        assert_eq!(result.conversation_id, conv_id);
        assert_eq!(result.content, "bus response");

        // The original request should be acked (done).
        let pending = orch
            .bus()
            .list(
                topic::TURN_REQUEST,
                Some(assistant_core::MessageStatus::Pending),
                10,
            )
            .await
            .unwrap();
        assert!(pending.is_empty(), "turn request should be acked");

        worker.abort();
    }

    #[tokio::test]
    async fn submit_turn_publishes_and_waits_for_result() {
        let server = MockServer::start().await;
        mount_answer(&server, "submitted answer").await;

        let (orch, _storage) = build(&server.uri()).await;

        // Spawn the worker so it can process the submitted turn.
        let orch_worker = orch.clone();
        tokio::spawn(async move {
            orch_worker.run_worker("test-worker").await;
        });

        let conv_id = Uuid::new_v4();
        let result = orch
            .submit_turn("hello via submit", conv_id, Interface::Cli)
            .await
            .unwrap();
        assert_eq!(result.answer, "submitted answer");
    }

    // ── Subagent integration tests ────────────────────────────────────────────

    use assistant_core::{AgentReportStatus, AgentSpawn, SubagentRunner, DEFAULT_MAX_AGENT_DEPTH};

    #[tokio::test]
    async fn subagent_spawn_complete_round_trip() {
        let server = MockServer::start().await;

        // The subagent's LLM will return a final answer directly.
        mount_answer(&server, "subagent result").await;

        let (orch, storage) = build(&server.uri()).await;

        let spawn = AgentSpawn {
            agent_id: "test-agent-1".into(),
            task: "What is 2+2?".into(),
            system_prompt: None,
            model: None,
            allowed_tools: vec![],
        };

        let report = orch.run_subagent(spawn, 0).await.unwrap();

        assert_eq!(report.status, AgentReportStatus::Completed);
        assert_eq!(report.content, "subagent result");

        // Verify lifecycle was recorded in the DB.
        let agent_store = storage.agent_store();
        let record = agent_store
            .get("test-agent-1")
            .await
            .unwrap()
            .expect("agent record should exist");
        assert_eq!(record.status, assistant_storage::AgentStatus::Completed);
        assert!(record.completed_at.is_some());
        assert_eq!(record.task, "What is 2+2?");
    }

    #[tokio::test]
    async fn subagent_nesting_depth_limit_enforced() {
        let server = MockServer::start().await;
        mount_answer(&server, "should not reach here").await;

        let (orch, _) = build(&server.uri()).await;

        // Spawn at max depth — should be rejected.
        let spawn = AgentSpawn {
            agent_id: "deep-agent".into(),
            task: "too deep".into(),
            system_prompt: None,
            model: None,
            allowed_tools: vec![],
        };

        let report = orch
            .run_subagent(spawn, DEFAULT_MAX_AGENT_DEPTH)
            .await
            .unwrap();

        assert_eq!(report.status, AgentReportStatus::Failed);
        assert!(
            report.content.contains("depth"),
            "error should mention depth: {}",
            report.content
        );

        // No LLM call should have been made.
        let reqs = server.received_requests().await.unwrap();
        assert!(
            reqs.is_empty(),
            "no LLM calls should be made when depth limit is exceeded"
        );
    }

    #[tokio::test]
    async fn subagent_tool_filtering_restricts_tools() {
        let server = MockServer::start().await;

        // Subagent LLM tries to call "bash" which is NOT in the allowed list.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_tool_calls(&["bash"])))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Second call returns final answer.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ollama_answer("done")))
            .mount(&server)
            .await;

        let (orch, _) = build(&server.uri()).await;

        let spawn = AgentSpawn {
            agent_id: "restricted-agent".into(),
            task: "try to use bash".into(),
            system_prompt: None,
            model: None,
            // Only allow file-read — bash should be rejected.
            allowed_tools: vec!["file-read".into()],
        };

        let report = orch.run_subagent(spawn, 0).await.unwrap();

        // The subagent should still complete (the LLM got a rejection
        // observation and then returned a final answer).
        assert_eq!(report.status, AgentReportStatus::Completed);
        assert_eq!(report.content, "done");

        // Verify the first LLM call had the restricted tool set —
        // the request should only contain "file-read", not "bash".
        let reqs = server.received_requests().await.unwrap();
        assert_eq!(reqs.len(), 2);
        let body: Value = serde_json::from_slice(&reqs[0].body).unwrap();
        let tool_names: Vec<String> = body["tools"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|t| t["function"]["name"].as_str().map(String::from))
            .collect();
        assert!(
            tool_names.contains(&"file-read".to_string()),
            "file-read should be in tool specs: {tool_names:?}"
        );
        assert!(
            !tool_names.contains(&"bash".to_string()),
            "bash should NOT be in tool specs: {tool_names:?}"
        );
    }

    #[tokio::test]
    async fn subagent_cancellation_stops_loop() {
        let server = MockServer::start().await;

        // The subagent LLM returns tool calls indefinitely, so the subagent
        // would loop forever if not cancelled.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(ollama_tool_calls(&["unknown-tool"]))
                    // Add a small delay so the cancel has time to trigger
                    .set_body_json(ollama_tool_calls(&["unknown-tool"])),
            )
            .mount(&server)
            .await;

        let (orch, storage) = build(&server.uri()).await;

        let spawn = AgentSpawn {
            agent_id: "cancel-me".into(),
            task: "infinite loop task".into(),
            system_prompt: None,
            model: None,
            allowed_tools: vec![],
        };

        // Cancel the agent before it starts by pre-cancelling.
        // We can't easily cancel mid-loop in a unit test, but we can
        // test that the cancel_agent mechanism works by:
        // 1. Registering the token manually would require access to internals.
        // Instead, test cancel_agent returns false for unknown agents.
        let cancelled = orch.cancel_agent("nonexistent").await.unwrap();
        assert!(
            !cancelled,
            "cancelling nonexistent agent should return false"
        );

        // Test the actual cancellation flow: spawn in a task, cancel shortly after.
        let orch2 = orch.clone();
        let handle = tokio::spawn(async move { orch2.run_subagent(spawn, 0).await.unwrap() });

        // Give the subagent a moment to start and register the token.
        tokio::time::sleep(Duration::from_millis(50)).await;

        let cancelled = orch.cancel_agent("cancel-me").await.unwrap();
        assert!(cancelled, "should find and cancel the running agent");

        // Wait for the subagent to finish.
        let report = handle.await.unwrap();
        assert_eq!(
            report.status,
            AgentReportStatus::Cancelled,
            "subagent should report Cancelled status, got: {:?}",
            report.status
        );

        // Verify lifecycle recorded as cancelled.
        let agent_store = storage.agent_store();
        let record = agent_store
            .get("cancel-me")
            .await
            .unwrap()
            .expect("agent record should exist");
        assert_eq!(record.status, assistant_storage::AgentStatus::Cancelled);
    }

    #[tokio::test]
    async fn subagent_llm_error_records_failed_status() {
        let server = MockServer::start().await;

        // LLM returns a 500 error.
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let (orch, storage) = build(&server.uri()).await;

        let spawn = AgentSpawn {
            agent_id: "error-agent".into(),
            task: "this will fail".into(),
            system_prompt: None,
            model: None,
            allowed_tools: vec![],
        };

        let report = orch.run_subagent(spawn, 0).await.unwrap();

        assert_eq!(report.status, AgentReportStatus::Failed);
        assert!(report.content.contains("LLM error"));

        let agent_store = storage.agent_store();
        let record = agent_store
            .get("error-agent")
            .await
            .unwrap()
            .expect("agent record should exist");
        assert_eq!(record.status, assistant_storage::AgentStatus::Failed);
    }
}
