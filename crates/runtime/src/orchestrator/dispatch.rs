//! Tool dispatch and result finalization for the orchestrator.
//!
//! Contains the confirmation gate, tool execution through the global executor,
//! result recording (metrics, OTel, persistence), and tool-call history helpers.

use std::sync::Arc;

use assistant_core::{Attachment, ExecutionContext, Message, MessageRole, ToolHandler};
use assistant_llm::{Capabilities, ChatHistoryMessage, HostedTool, ToolCallItem, ToolSpec};
use assistant_storage::conversations::ConversationStore;
use opentelemetry::trace::Span as _;
use opentelemetry::KeyValue;
use tokio::sync::mpsc;
use tracing::{debug, info, warn, Instrument};
use uuid::Uuid;

use super::{value_to_params_map, DispatchOutcome, Orchestrator, OrchestratorEvent};
use crate::webhook_dispatch;

impl Orchestrator {
    /// Filter tool specs based on provider capabilities.
    ///
    /// Suppresses builtin tools when the provider offers an equivalent
    /// hosted tool (e.g. suppress `web-search` when the provider has
    /// native web search).
    pub(crate) fn filter_tool_specs(specs: Vec<ToolSpec>, caps: &Capabilities) -> Vec<ToolSpec> {
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

    /// Build a tool-call assistant message for persistence.
    pub(crate) fn make_tool_call_message(
        conversation_id: Uuid,
        turn: i64,
        items: &[ToolCallItem],
    ) -> Message {
        let mut m = Message::assistant(conversation_id, "");
        m.turn = turn;
        m.tool_calls_json = serde_json::to_string(items).ok();
        m
    }

    /// Build a tool-result message for persistence.
    pub(crate) fn make_tool_result_message(
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

    /// Record tool calls in the chat history and persist them to the database.
    ///
    /// This is the common pre-execution step shared by all turn variants
    /// (extension-tools, core, and subagent).
    pub(crate) async fn persist_tool_calls(
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        tool_call_items: &[ToolCallItem],
    ) {
        history.push(ChatHistoryMessage::AssistantToolCalls(
            tool_call_items.to_vec(),
        ));
        let tc_msg = Self::make_tool_call_message(conversation_id, turn_index, tool_call_items);
        if let Err(e) = conv_store.save_message(&tc_msg).await {
            warn!("Failed to persist tool-call message: {e}");
        }
    }

    /// Process a tool execution result: record metrics, set OTel span
    /// attributes, collect attachments, end the span, append to history,
    /// and persist the tool-result message to the database.
    ///
    /// Returns the observation string that was fed back to the LLM.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn finalize_tool_result(
        &self,
        tool_name: &str,
        exec_result: anyhow::Result<assistant_core::ToolOutput>,
        elapsed: std::time::Duration,
        otel_span: &mut opentelemetry::global::BoxedSpan,
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        turn_attachments: &mut Vec<Attachment>,
        event_sink: Option<&mpsc::Sender<OrchestratorEvent>>,
    ) -> String {
        let duration_ms = elapsed.as_millis() as i64;
        let span_name = format!("execute_tool {tool_name}");
        self.metrics
            .record_tool_invocation(&self.agent_id, tool_name, &span_name);
        self.metrics.record_tool_duration(
            &self.agent_id,
            tool_name,
            &span_name,
            duration_ms as f64 / 1000.0,
        );

        let mut tool_status = "ok";

        let observation = match exec_result {
            Ok(output) => {
                debug!(tool = %tool_name, duration_ms, "Tool execution completed");
                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                otel_span.set_attribute(KeyValue::new("tool_observation", output.content.clone()));
                if !output.attachments.is_empty() {
                    turn_attachments.extend(output.attachments);
                }
                output.content.clone()
            }
            Err(err) => {
                warn!(tool = %tool_name, %err, "Tool execution failed");
                self.metrics
                    .record_error(&self.agent_id, "tool_error", tool_name);
                let msg = err.to_string();
                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                otel_span.set_attribute(KeyValue::new("tool_status", "error"));
                otel_span.set_attribute(KeyValue::new("tool_error", msg.clone()));
                tool_status = "error";
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

        let event_payload = serde_json::json!({
            "conversation_id": conversation_id,
            "turn": turn_index,
            "tool_name": tool_name,
            "status": tool_status,
            "observation": observation,
        });
        if let Err(e) = webhook_dispatch::dispatch_event(
            self.storage.as_ref(),
            &self.agent_id,
            assistant_core::topic::TOOL_RESULT,
            event_payload,
        )
        .await
        {
            warn!(tool = %tool_name, error = %e, "Failed to dispatch tool.result webhooks");
        }

        if let Some(sink) = event_sink {
            let _ = sink
                .send(OrchestratorEvent::ToolResult {
                    tool_name: tool_name.to_string(),
                    status: tool_status.to_string(),
                })
                .await;
        }

        observation
    }

    /// Dispatch a single tool call through the global executor, applying the
    /// confirmation gate when required.
    ///
    /// Checks whether the tool requires user confirmation, records the denial
    /// in OTel/history/DB when refused, and otherwise executes and finalizes
    /// the result.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn dispatch_global_tool(
        &self,
        name: &str,
        params: &serde_json::Value,
        ctx: &ExecutionContext,
        otel_span: &mut opentelemetry::global::BoxedSpan,
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        turn_attachments: &mut Vec<Attachment>,
        tool_handlers: &[Arc<dyn ToolHandler>],
        instrument_span: &tracing::Span,
        event_sink: Option<&mpsc::Sender<OrchestratorEvent>>,
    ) -> DispatchOutcome {
        // Confirmation gate.
        let requires_confirm = tool_handlers
            .iter()
            .find(|h| h.name() == name)
            .map(|h| h.requires_confirmation())
            .unwrap_or(false);

        if requires_confirm && ctx.interactive {
            if let Some(cb) = &self.confirmation_callback {
                if !cb.confirm(name, params) {
                    let observation = format!("User denied execution of '{name}'.");
                    info!(%observation);
                    otel_span.set_attribute(KeyValue::new("tool_status", "denied"));
                    otel_span.set_attribute(KeyValue::new("tool_error", observation.clone()));
                    crate::history::append_tool_result(history, name, &observation);
                    let tr_msg = Self::make_tool_result_message(
                        conversation_id,
                        turn_index,
                        name,
                        &observation,
                    );
                    if let Err(e) = conv_store
                        .save_message(&tr_msg)
                        .instrument(instrument_span.clone())
                        .await
                    {
                        warn!("Failed to persist tool-result message: {e}");
                    }
                    otel_span.end();
                    if let Some(sink) = event_sink {
                        let _ = sink
                            .send(OrchestratorEvent::ToolResult {
                                tool_name: name.to_string(),
                                status: "denied".to_string(),
                            })
                            .await;
                    }
                    return DispatchOutcome::Denied;
                }
            }
        }

        if let Some(sink) = event_sink {
            let _ = sink
                .send(OrchestratorEvent::Status(format!("Calling tool: {name}")))
                .await;
        }

        let params_map = value_to_params_map(params);

        let start = std::time::Instant::now();
        let exec_result = self
            .executor
            .execute(name, params_map, ctx)
            .instrument(instrument_span.clone())
            .await;
        let elapsed = start.elapsed();

        self.finalize_tool_result(
            name,
            exec_result,
            elapsed,
            otel_span,
            history,
            conv_store,
            conversation_id,
            turn_index,
            turn_attachments,
            event_sink,
        )
        .await;

        DispatchOutcome::Executed
    }
}
