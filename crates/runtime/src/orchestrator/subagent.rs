//! Subagent runner implementation for the orchestrator.
//!
//! Contains the [`SubagentRunner`] trait implementation that allows the
//! orchestrator to spawn isolated sub-agent conversations with restricted
//! tool sets and independent cancellation tokens.

use anyhow::Result;
use assistant_core::{
    AgentReport, AgentReportStatus, AgentSpawn, ExecutionContext, Interface, Message,
    SubagentRunner, DEFAULT_MAX_AGENT_DEPTH,
};
use assistant_llm::{ChatHistoryMessage, ChatRole, LlmResponse};
use async_trait::async_trait;
use opentelemetry::{
    global,
    trace::{Span as _, TraceContextExt, Tracer as _},
    Context as OtelContext, KeyValue,
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, info_span, warn, Instrument};
use uuid::Uuid;

use super::{value_to_params_map, Orchestrator};

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
                    crate::history::persist_error_recovery(&conv_store, conversation_id).await;
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

                // -- OTel: LLM span (child of agent span) ---------------------
                let mut llm_span = crate::otel_spans::start_llm_span(
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
                        crate::history::persist_error_recovery(&conv_store, conversation_id)
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
                crate::otel_spans::finish_llm_span(
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
                            let mut otel_span = crate::otel_spans::start_tool_span(
                                conversation_id,
                                iteration,
                                turn_index,
                                &Interface::Scheduler,
                                &name,
                                &params,
                                &agent_cx,
                            );

                            let params_map = value_to_params_map(&params);

                            let ctx = ExecutionContext {
                                conversation_id,
                                turn: iteration as i64,
                                interface: Interface::Scheduler,
                                interactive: false,
                                allowed_tools: allowed_tools.clone(),
                                depth: new_depth,
                            };

                            let start = std::time::Instant::now();
                            let exec_result = self
                                .executor
                                .execute(&name, params_map, &ctx)
                                .instrument(iteration_span.clone())
                                .await;
                            let elapsed = start.elapsed();

                            // Subagent does not surface attachments to
                            // the parent — pass a scratch vector.
                            let mut scratch_attachments = Vec::new();
                            self.finalize_tool_result(
                                &name,
                                exec_result,
                                elapsed,
                                &mut otel_span,
                                &mut history,
                                &conv_store,
                                conversation_id,
                                turn_index,
                                &mut scratch_attachments,
                            )
                            .await;
                        }
                    }

                    // ── Thinking ──────────────────────────────────────────────
                    LlmResponse::Thinking(text, _meta) => {
                        debug!(
                            iteration,
                            agent_id = %spawn.agent_id,
                            "Subagent emitted thinking step"
                        );
                        // Persist to DB so thinking is preserved for trace
                        // inspection, matching the behavior of the other two
                        // turn variants.
                        let thinking_msg = {
                            let mut m = Message::assistant(
                                conversation_id,
                                format!("<think>{text}</think>"),
                            );
                            m.turn = base_turn + iteration as i64 + 1;
                            m
                        };
                        if let Err(e) = conv_store.save_message(&thinking_msg).await {
                            warn!("Failed to persist subagent thinking step: {e}");
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
