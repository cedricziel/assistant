//! Tool dispatch and result finalization for the orchestrator.
//!
//! Contains the permission gate, tool execution through the global executor,
//! result recording (metrics, OTel, persistence), and tool-call history helpers.

use assistant_core::{
    Attachment, Capabilities, ChatHistoryMessage, ChatRole, HostedTool, ProviderState,
    ToolCallItem, ToolSpec,
    permission::{Outcome, PermissionRequest},
    types::conversation::ExecutionContext,
    types::conversation::Interface,
    types::conversation::Message,
    types::conversation::MessageRole,
};
use assistant_storage::AttachmentStore as _AttachmentStoreTrait;
use assistant_storage::conversations::ConversationStore;
use assistant_storage::{SkillExecution, SkillOutcome};
use assistant_tool_executor::ToolOverlay;
use opentelemetry::KeyValue;
use opentelemetry::trace::Span as _;
use tokio::sync::mpsc;
use tracing::{Instrument, debug, info, warn};
use uuid::Uuid;

use super::{
    ApprovalOptions, ConfirmationAnswer, ConfirmationRequest, DispatchOutcome, Orchestrator,
    OrchestratorEvent, value_to_params_map,
};
use crate::history::TurnHistory;
use crate::webhook_dispatch;

/// Call metadata persisted on a tool row next to the observation.
#[derive(Clone, Copy)]
pub(crate) struct ToolResultMeta<'a> {
    /// Provider-supplied id of the call this row answers.
    pub tool_call_id: Option<&'a str>,
    /// `ok`, `error` or `denied`.
    pub status: &'a str,
    /// Capped structured render data; client-only.
    pub data: Option<&'a serde_json::Value>,
}

impl ToolResultMeta<'_> {
    /// Metadata for rows that carry neither an id nor data (`end_turn`).
    pub(crate) const fn ok() -> Self {
        Self {
            tool_call_id: None,
            status: "ok",
            data: None,
        }
    }
}

/// Result of [`Orchestrator::finalize_tool_result`].
///
/// Carries a flag indicating whether the tool execution failed, so the
/// caller can update the per-turn `had_errors` accumulator. The
/// observation string is recorded into history and persisted internally
/// by `finalize_tool_result`, so callers do not need it back.
pub(crate) struct FinalizedTool {
    pub had_error: bool,
}

impl Orchestrator {
    /// Whether a turn's client can render rich UI such as an A2UI surface.
    ///
    /// Only an interactive web turn at depth 0 can: `quick-message` and voice
    /// are `Interface::Web` but not interactive, a CLI turn is interactive but
    /// has no renderer, and subagents have no user attached.
    pub(crate) fn rich_client_turn(interface: &Interface, interactive: bool, depth: u32) -> bool {
        interactive && *interface == Interface::Web && depth == 0
    }

    /// Filter tool specs based on provider capabilities and the turn's client.
    ///
    /// Suppresses builtin tools when the provider offers an equivalent
    /// hosted tool (e.g. suppress `web-search` when the provider has
    /// native web search), and drops `requires_rich_client` tools unless
    /// `rich_client` is set (see [`Self::rich_client_turn`]).
    pub(crate) fn filter_tool_specs(
        specs: Vec<ToolSpec>,
        caps: &Capabilities,
        rich_client: bool,
    ) -> Vec<ToolSpec> {
        specs
            .into_iter()
            .filter(|spec| {
                (rich_client || !spec.requires_rich_client)
                    && !Self::tool_suppressed_by_caps(spec, caps)
            })
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
        meta: ToolResultMeta<'_>,
    ) -> Message {
        let mut m = Message::new(conversation_id, MessageRole::Tool, observation);
        m.turn = turn;
        m.skill_name = Some(tool_name.to_string());
        m.tool_call_id = meta.tool_call_id.map(str::to_string);
        m.tool_status = Some(meta.status.to_string());
        m.tool_data_json = meta.data.and_then(|data| serde_json::to_string(data).ok());
        m
    }

    /// Record tool calls in the chat history and persist them to the database.
    ///
    /// This is the common pre-execution step shared by all turn variants
    /// (extension-tools, core, and subagent).
    pub(crate) async fn persist_tool_calls(
        history: &mut TurnHistory,
        conv_store: &dyn ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        tool_call_items: &[ToolCallItem],
        provider_state: Option<&ProviderState>,
    ) {
        let mut tc_msg = Self::make_tool_call_message(conversation_id, turn_index, tool_call_items);
        tc_msg.provider_state_json = provider_state.and_then(|s| serde_json::to_string(s).ok());
        history.push(
            ChatHistoryMessage::AssistantToolCalls {
                items: tool_call_items.to_vec(),
                provider_state: provider_state.cloned(),
            },
            Some(tc_msg.id),
        );
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

    /// Maximum serialized size of the structured `data` carried on a
    /// `ToolResult` event. Larger payloads are dropped whole: cutting JSON
    /// generically would produce half-valid shapes.
    pub(crate) const TOOL_RESULT_DATA_LIMIT_BYTES: usize = 32 * 1024;

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn finalize_tool_result(
        &self,
        tool_name: &str,
        tool_arguments: Option<&serde_json::Value>,
        exec_result: anyhow::Result<assistant_core::ToolOutput>,
        elapsed: std::time::Duration,
        otel_span: &mut opentelemetry::global::BoxedSpan,
        history: &mut TurnHistory,
        conv_store: &dyn ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        turn_attachments: &mut Vec<Attachment>,
        turn_attachment_ids: &mut Vec<Uuid>,
        active_skill: &mut Option<String>,
        event_sink: Option<&mpsc::Sender<OrchestratorEvent>>,
        tool_call_id: Option<&str>,
    ) -> FinalizedTool {
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

        // Calls made while a skill is active count as executions of it; the
        // `load-skill` call that activates a skill does not.
        let skill_in_use = active_skill.clone();
        let failure = match &exec_result {
            Ok(output) if output.success => None,
            Ok(output) => Some(output.content.clone()),
            Err(err) => Some(err.to_string()),
        };

        // Track the active skill when load-skill is called successfully.
        if tool_name == "load-skill"
            && let Some(args) = tool_arguments
            && let Some(name) = args.get("name").and_then(|v| v.as_str())
        {
            *active_skill = Some(name.to_string());
        }

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

        let mut had_error = false;
        let mut data = None;
        let observation = match exec_result {
            Ok(mut output) => {
                debug!(tool = %tool_name, duration_ms, "Tool execution completed");
                // Structured render data goes to the event only: take it before
                // anything else sees the output so it cannot leak into the LLM
                // history, the persisted observation or the webhook payload.
                data = Self::cap_tool_data(tool_name, output.data.take());
                // A tool-level failure (`ToolOutput::error`, non-zero bash
                // exit) is an error even though the call itself returned `Ok`.
                if !output.success {
                    tool_status = "error";
                }
                otel_span.set_attribute(KeyValue::new("duration_ms", duration_ms));
                otel_span.set_attribute(KeyValue::new("tool_status", tool_status));
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
                            created_at: self.clock.now(),
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
                had_error = true;
                format!("Error executing '{tool_name}': {msg}")
            }
        };

        otel_span.end();

        if let Some(skill_name) = skill_in_use {
            self.record_skill_execution(
                skill_name,
                tool_name,
                conversation_id,
                failure,
                duration_ms,
            )
            .await;
        }

        let tr_msg = Self::make_tool_result_message(
            conversation_id,
            turn_index,
            tool_name,
            &observation,
            ToolResultMeta {
                tool_call_id,
                status: tool_status,
                data: data.as_ref(),
            },
        );
        crate::history::append_tool_result(history, tool_name, &observation, Some(tr_msg.id));
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
                let cut = observation.floor_char_boundary(Self::TOOL_RESULT_DISPLAY_LIMIT);
                let mut s = observation[..cut].to_string();
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
                    tool_call_id: tool_call_id.map(|s| s.to_string()),
                    data,
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
        if let Some(ref audio_id_str) = voice_audio_id
            && let Some(ref store) = self.audio_store
            && let Ok(id) = Uuid::parse_str(audio_id_str)
        {
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

        FinalizedTool { had_error }
    }

    /// Drop `data` whole when its serialized form exceeds
    /// [`Self::TOOL_RESULT_DATA_LIMIT_BYTES`].
    fn cap_tool_data(
        tool_name: &str,
        data: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        let value = data?;
        let size = value.to_string().len();
        if size > Self::TOOL_RESULT_DATA_LIMIT_BYTES {
            warn!(
                tool = %tool_name,
                size,
                limit = Self::TOOL_RESULT_DATA_LIMIT_BYTES,
                "Dropping tool result data over the size cap"
            );
            return None;
        }
        Some(value)
    }

    /// Append one tool call to the skill execution log.
    async fn record_skill_execution(
        &self,
        skill_name: String,
        tool_name: &str,
        conversation_id: Uuid,
        failure: Option<String>,
        duration_ms: i64,
    ) {
        let execution = SkillExecution {
            skill_name,
            agent_id: self.agent_id.clone(),
            conversation_id,
            tool_name: tool_name.to_string(),
            outcome: if failure.is_some() {
                SkillOutcome::Error
            } else {
                SkillOutcome::Success
            },
            error: failure,
            duration_ms,
            created_at: self.clock.now(),
        };
        if let Err(e) = self.skill_executions.record(&execution).await {
            warn!(skill = %execution.skill_name, error = %e, "Failed to record skill execution");
        }
    }

    /// Run the permission pipeline for one tool call.
    ///
    /// `Ok(())` means the call may execute. `Err(observation)` carries the
    /// text to return to the LLM in place of a tool result. Undecided calls
    /// defer to the confirmation callback on interactive turns and are denied
    /// on every other turn.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn authorize_tool_call(
        &self,
        name: &str,
        params: &serde_json::Value,
        ctx: &ExecutionContext,
        requires_confirmation: bool,
        task_context: Option<&str>,
        event_sink: Option<&mpsc::Sender<OrchestratorEvent>>,
        tool_call_id: Option<&str>,
    ) -> Result<(), String> {
        let resolution = self
            .permission_policy
            .evaluate(&PermissionRequest {
                tool_name: name,
                params,
                ctx,
                requires_confirmation,
                task_context,
            })
            .await;

        if resolution.outcome == Outcome::Ask
            && ctx.interactive
            && let Some(cb) = &self.confirmation_callback
        {
            let answer = cb
                .confirm(ConfirmationRequest {
                    tool_name: name,
                    params,
                    ctx,
                    tool_call_id,
                    reason: &resolution.decision.reason,
                    options: ApprovalOptions::once_only(),
                    event_sink,
                })
                .await;
            return match answer {
                ConfirmationAnswer::AllowOnce | ConfirmationAnswer::AllowAlways => Ok(()),
                ConfirmationAnswer::RejectOnce
                | ConfirmationAnswer::RejectAlways
                | ConfirmationAnswer::Cancelled => {
                    Err(format!("User denied execution of '{name}'."))
                }
                ConfirmationAnswer::Expired => Err(format!(
                    "The approval request for '{name}' expired before the user answered, so it was not run."
                )),
            };
        }
        resolution.refusal(name).map_or(Ok(()), Err)
    }

    /// Dispatch a single tool call through the global executor, applying the
    /// permission gate.
    ///
    /// Runs the permission pipeline, records a denial
    /// in OTel/history/DB when refused, and otherwise executes and finalizes
    /// the result.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn dispatch_global_tool(
        &self,
        name: &str,
        params: &serde_json::Value,
        ctx: &ExecutionContext,
        otel_span: &mut opentelemetry::global::BoxedSpan,
        history: &mut TurnHistory,
        conv_store: &dyn ConversationStore,
        conversation_id: Uuid,
        turn_index: i64,
        turn_attachments: &mut Vec<Attachment>,
        turn_attachment_ids: &mut Vec<Uuid>,
        overlay: &ToolOverlay,
        active_skill: &mut Option<String>,
        instrument_span: &tracing::Span,
        event_sink: Option<&mpsc::Sender<OrchestratorEvent>>,
        tool_call_id: Option<&str>,
    ) -> DispatchOutcome {
        let requires_confirm = self
            .executor
            .resolve(overlay, name)
            .is_some_and(|h| h.requires_confirmation());

        let authorization = self
            .authorize_tool_call(
                name,
                params,
                ctx,
                requires_confirm,
                task_context(history),
                event_sink,
                tool_call_id,
            )
            .await;
        if let Err(observation) = authorization {
            info!(%observation);
            otel_span.set_attribute(KeyValue::new("tool_status", "denied"));
            otel_span.set_attribute(KeyValue::new("tool_error", observation.clone()));
            let tr_msg = Self::make_tool_result_message(
                conversation_id,
                turn_index,
                name,
                &observation,
                ToolResultMeta {
                    tool_call_id,
                    status: "denied",
                    data: None,
                },
            );
            crate::history::append_tool_result(history, name, &observation, Some(tr_msg.id));
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
                        tool_call_id: tool_call_id.map(|s| s.to_string()),
                        data: None,
                    })
                    .await;
            }
            return DispatchOutcome::Denied;
        }

        if let Some(sink) = event_sink {
            let _ = sink
                .send(OrchestratorEvent::Status {
                    message: format!("Calling tool: {name}"),
                    tool_call_id: tool_call_id.map(|s| s.to_string()),
                })
                .await;
        }

        let params_map = value_to_params_map(params);

        let start = std::time::Instant::now();
        let exec_result = self
            .executor
            .execute_with(overlay, name, params_map, ctx)
            .instrument(instrument_span.clone())
            .await;
        let elapsed = start.elapsed();

        let result = self
            .finalize_tool_result(
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
                active_skill,
                event_sink,
                tool_call_id,
            )
            .await;

        DispatchOutcome::Executed {
            had_error: result.had_error,
        }
    }
}

/// The user's most recent message, truncated: the task the permission
/// pipeline weighs a tool call against.
pub(crate) fn task_context(history: &[ChatHistoryMessage]) -> Option<&str> {
    const MAX_CHARS: usize = 1_000;
    history.iter().rev().find_map(|m| match m {
        ChatHistoryMessage::Text {
            role: ChatRole::User,
            content,
        } => Some(truncate_chars(content, MAX_CHARS)),
        _ => None,
    })
}

fn truncate_chars(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}
