//! Tool dispatch and result finalization for the orchestrator.
//!
//! Contains the confirmation gate, tool execution through the global executor,
//! result recording (metrics, OTel, persistence), and tool-call history helpers.

use std::sync::Arc;

use assistant_core::{Attachment, ExecutionContext, Message, MessageRole, ToolHandler};
use assistant_llm::{Capabilities, ChatHistoryMessage, HostedTool, ToolCallItem, ToolSpec};
use assistant_storage::conversations::ConversationStore;
use opentelemetry::KeyValue;
use opentelemetry::trace::Span as _;
use tokio::sync::mpsc;
use tracing::{Instrument, debug, info, warn};
use uuid::Uuid;

use super::{DispatchOutcome, Orchestrator, OrchestratorEvent, value_to_params_map};
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
    /// Maximum length for the tool result string included in SSE events and
    /// API responses.  Keeps payloads small while still giving clients useful
    /// context.
    const TOOL_RESULT_DISPLAY_LIMIT: usize = 512;

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn finalize_tool_result(
        &self,
        tool_name: &str,
        tool_arguments: Option<&serde_json::Value>,
        exec_result: anyhow::Result<assistant_core::ToolOutput>,
        elapsed: std::time::Duration,
        otel_span: &mut opentelemetry::global::BoxedSpan,
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        turn_attachments: &mut Vec<Attachment>,
        turn_attachment_ids: &mut Vec<Uuid>,
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

        // For the voice-response tool we need to extract the audio_id before
        // consuming the output below.
        let voice_audio_id: Option<String> = if tool_name == "voice-response" {
            if let Ok(ref output) = exec_result {
                output
                    .data
                    .as_ref()
                    .and_then(|d| d.get("audio_id"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        } else {
            None
        };

        let observation = match exec_result {
            Ok(output) => {
                debug!(tool = %tool_name, duration_ms, "Tool execution completed");
                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
                otel_span.set_attribute(KeyValue::new("tool_observation", output.content.clone()));
                if !output.attachments.is_empty() {
                    // Persist each tool-produced attachment to the store.
                    let store = self.storage.attachment_store();
                    for att in &output.attachments {
                        let meta = assistant_core::AttachmentMeta {
                            id: Uuid::new_v4(),
                            message_id: None,
                            conversation_id,
                            agent_id: self.agent_id.clone(),
                            filename: att.filename.clone(),
                            mime_type: att.mime_type.clone(),
                            size_bytes: att.data.len() as u64,
                            created_at: chrono::Utc::now(),
                        };
                        match store.store(&meta, &att.data).await {
                            Ok(()) => {
                                debug!(
                                    attachment_id = %meta.id,
                                    filename = %meta.filename,
                                    "Persisted outbound tool attachment"
                                );
                                turn_attachment_ids.push(meta.id);
                            }
                            Err(e) => {
                                warn!(
                                    filename = %att.filename,
                                    error = %e,
                                    "Failed to persist tool attachment"
                                );
                            }
                        }
                    }
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
            let truncated_result = if observation.len() > Self::TOOL_RESULT_DISPLAY_LIMIT {
                let mut s = observation[..Self::TOOL_RESULT_DISPLAY_LIMIT].to_string();
                s.push('…');
                Some(s)
            } else {
                Some(observation.clone())
            };
            let _ = sink
                .send(OrchestratorEvent::ToolResult {
                    tool_name: tool_name.to_string(),
                    status: tool_status.to_string(),
                    arguments: tool_arguments.cloned(),
                    result: truncated_result,
                })
                .await;

            // For the voice-response tool, emit a dedicated AudioReady event
            // so voice-enabled clients can auto-play the synthesised audio
            // without waiting for the full turn to finish.
            if let Some(ref audio_id) = voice_audio_id {
                let _ = sink
                    .send(OrchestratorEvent::AudioReady {
                        audio_id: audio_id.clone(),
                    })
                    .await;
            }
        }

        // When an AudioStore is available, retrieve the synthesised audio
        // blob and append it to the turn attachments so that channel
        // adapters (Matrix, Slack, Signal, …) can deliver it as a file.
        if let Some(ref audio_id_str) = voice_audio_id {
            if let Some(ref store) = self.audio_store {
                if let Ok(id) = Uuid::parse_str(audio_id_str) {
                    if let Some((data, mime_type)) = store.get(id).await {
                        let ext = match mime_type.as_str() {
                            "audio/mpeg" | "audio/mp3" => "mp3",
                            "audio/ogg" => "ogg",
                            "audio/wav" | "audio/x-wav" => "wav",
                            "audio/webm" => "webm",
                            "audio/flac" => "flac",
                            "audio/aac" => "aac",
                            "audio/mp4" | "audio/m4a" => "m4a",
                            "audio/opus" => "opus",
                            _ => "audio",
                        };
                        turn_attachments.push(Attachment {
                            filename: format!("voice-response.{ext}"),
                            mime_type,
                            data,
                        });
                    } else {
                        warn!(audio_id = %audio_id_str, "AudioStore entry expired or missing; skipping audio attachment");
                    }
                }
            }
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
        turn_attachment_ids: &mut Vec<Uuid>,
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

        if requires_confirm
            && ctx.interactive
            && let Some(cb) = &self.confirmation_callback
            && !cb.confirm(name, params)
        {
            let observation = format!("User denied execution of '{name}'.");
            info!(%observation);
            otel_span.set_attribute(KeyValue::new("tool_status", "denied"));
            otel_span.set_attribute(KeyValue::new("tool_error", observation.clone()));
            crate::history::append_tool_result(history, name, &observation);
            let tr_msg =
                Self::make_tool_result_message(conversation_id, turn_index, name, &observation);
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
                        arguments: Some(params.clone()),
                        result: Some(observation.clone()),
                    })
                    .await;
            }
            return DispatchOutcome::Denied;
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
            Some(params),
            exec_result,
            elapsed,
            otel_span,
            history,
            conv_store,
            conversation_id,
            turn_index,
            turn_attachments,
            turn_attachment_ids,
            event_sink,
        )
        .await;

        DispatchOutcome::Executed
    }
}
