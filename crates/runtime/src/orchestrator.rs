//! Orchestrator — the main turn-processing loop that wires together the
//! LLM client, tool executor, and skill registry.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{
    ExecutionContext, Interface, MemoryLoader, Message, MessageRole, ToolHandler,
};
use assistant_llm::{
    Capabilities, ChatHistoryMessage, ChatRole, ContentBlock, HostedTool, LlmProvider, LlmResponse,
    LlmResponseMeta, ToolSpec,
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
use tracing::{debug, info, info_span, warn};
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
///    confirm with the user, execute the tool, emit an OpenTelemetry span,
///    and append an `OBSERVATION` to the conversation history.
/// 6. Persist the final assistant message and return [`TurnResult`].
pub struct Orchestrator {
    llm: Arc<dyn LlmProvider>,
    storage: Arc<StorageLayer>,
    executor: Arc<ToolExecutor>,
    registry: Arc<SkillRegistry>,
    max_iterations: usize,
    disabled_skills: Vec<String>,
    confirmation_callback: Option<Arc<dyn ConfirmationCallback>>,
    /// Memory loader used to rebuild the system prompt at the start of every
    /// turn so that writes made by memory tools are reflected immediately.
    memory_loader: MemoryLoader,
    /// When true, record full message content on LLM spans (PII-sensitive).
    trace_content: bool,
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
        config: &assistant_core::AssistantConfig,
    ) -> Self {
        let memory_loader = MemoryLoader::new(config);
        memory_loader.ensure_defaults();
        Self {
            llm,
            storage,
            executor,
            registry,
            max_iterations: config.llm.max_iterations,
            disabled_skills: config.skills.disabled.clone(),
            confirmation_callback: None,
            memory_loader,
            trace_content: config.mirror.trace_content,
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
    ) -> Result<()> {
        let turn_span = info_span!(
            "conversation_turn",
            %conversation_id,
            interface = ?interface,
            extension_tools = extensions.len()
        );
        let _turn_guard = turn_span.enter();
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

        // 5. Tool-calling loop.
        for iteration in 0..self.max_iterations {
            debug!(iteration, "Extension-tools loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: false,
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
            let response = self.llm.chat(&system_prompt, &history, &all_specs).await;
            let response = response?;
            finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
            );

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text, _meta) => {
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
                            let _builtin_guard = builtin_span.enter();
                            if let Some(reason) = self
                                .reject_if_disabled(
                                    &name,
                                    &mut history,
                                    &conv_store,
                                    conversation_id,
                                    turn_index,
                                )
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
                        return Ok(());
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
        for iteration in 0..self.max_iterations {
            let iteration_span = info_span!("turn_iteration", iteration);
            let _iteration_guard = iteration_span.enter();
            debug!(iteration, "Tool-calling loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
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
            let response = self.llm.chat(&system_prompt, &history, &tool_specs).await;
            let response = response?;
            finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
            );

            match response {
                // ── Final answer ──────────────────────────────────────────────
                LlmResponse::FinalAnswer(text, _meta) => {
                    info!(iteration, "LLM returned final answer");

                    let assistant_msg = {
                        let mut m = Message::assistant(conversation_id, &text);
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    return Ok(TurnResult { answer: text });
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
                    if let Err(e) = conv_store.save_message(&tc_msg).await {
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
                                    if let Err(e) = conv_store.save_message(&tr_msg).await {
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
                        let exec_result = self.executor.execute(&name, params_map, &ctx).await;
                        let duration_ms = start.elapsed().as_millis() as i64;

                        let observation = match exec_result {
                            Ok(output) => {
                                debug!(
                                    tool = %name,
                                    duration_ms,
                                    success = output.success,
                                    "Tool execution completed"
                                );
                                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                                otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                                otel_span.set_attribute(KeyValue::new(
                                    "tool_observation",
                                    output.content.clone(),
                                ));
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
                        if let Err(e) = conv_store.save_message(&tr_msg).await {
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

        for iteration in 0..self.max_iterations {
            let iteration_span = info_span!("turn_iteration", iteration);
            let _iteration_guard = iteration_span.enter();
            debug!(iteration, "Streaming tool-calling loop iteration");

            let ctx = ExecutionContext {
                conversation_id,
                turn: iteration as i64,
                interface: interface.clone(),
                interactive: matches!(interface, Interface::Cli),
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
            let response = self
                .llm
                .chat_streaming(
                    &system_prompt,
                    &history,
                    &tool_specs,
                    Some(token_sink.clone()),
                )
                .await;
            let response = response?;
            finish_llm_span(
                &mut llm_span,
                response.meta(),
                &response,
                self.trace_content,
            );

            match response {
                LlmResponse::FinalAnswer(text, _meta) => {
                    info!(iteration, "Streaming LLM returned final answer");

                    let assistant_msg = {
                        let mut m = Message::assistant(conversation_id, &text);
                        m.turn = base_turn + iteration as i64 + 1;
                        m
                    };
                    conv_store.save_message(&assistant_msg).await?;

                    return Ok(TurnResult { answer: text });
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
                    if let Err(e) = conv_store.save_message(&tc_msg).await {
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
                                    if let Err(e) = conv_store.save_message(&tr_msg).await {
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
                        let exec_result = self.executor.execute(&name, params_map, &ctx).await;
                        let duration_ms = start.elapsed().as_millis() as i64;

                        let observation = match exec_result {
                            Ok(output) => {
                                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                                otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                                otel_span.set_attribute(KeyValue::new(
                                    "tool_observation",
                                    output.content.clone(),
                                ));
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
                        if let Err(e) = conv_store.save_message(&tr_msg).await {
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
                                "size_bytes": data.len(),
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

    use assistant_core::{types::Interface, AssistantConfig};
    use assistant_llm::{ChatHistoryMessage, LlmClient, LlmClientConfig, LlmProvider};
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
        let orch = Arc::new(Orchestrator::new(
            llm,
            storage.clone(),
            executor,
            registry.clone(),
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
            json_str.contains("size_bytes"),
            "size_bytes placeholder should be present"
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
}
