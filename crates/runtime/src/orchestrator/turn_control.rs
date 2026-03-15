//! Turn lifecycle control for the orchestrator.
//!
//! Handles `end_turn` evaluation and `FinalAnswer` processing when extension
//! tools are active (messaging interfaces like Slack/Mattermost).

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use assistant_core::{ExecutionContext, Interface, Message, ToolHandler};
use assistant_llm::ChatHistoryMessage;
use assistant_storage::conversations::ConversationStore;
use opentelemetry::trace::Span as _;
use opentelemetry::KeyValue;
use tracing::{info, warn};
use uuid::Uuid;

use super::{EndTurnOutcome, FinalAnswerOutcome, Orchestrator, TurnResult};

impl Orchestrator {
    /// Handle a `FinalAnswer` from the LLM when extension tools are active.
    ///
    /// Three paths:
    /// - **Already replied**: persist any non-empty wrap-up text -> `Done`.
    /// - **Empty answer, no reply yet**: warn -> `Retry`.
    /// - **Non-empty answer**: persist and optionally auto-post via a reply
    ///   extension tool -> `Done`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn handle_final_answer_with_extensions(
        replied: bool,
        text: &str,
        iteration: usize,
        base_turn: i64,
        conversation_id: Uuid,
        interface: &Interface,
        ext_map: &HashMap<String, Arc<dyn ToolHandler>>,
        conv_store: &ConversationStore,
    ) -> Result<FinalAnswerOutcome> {
        let turn_index = base_turn + iteration as i64 + 1;

        if replied {
            if !text.trim().is_empty() {
                let mut m = Message::assistant(conversation_id, text);
                m.turn = turn_index;
                if let Err(e) = conv_store.save_message(&m).await {
                    warn!("Failed to persist post-reply assistant message: {e}");
                }
            }
            return Ok(FinalAnswerOutcome::Done(TurnResult {
                answer: String::new(),
                attachments: Vec::new(),
            }));
        }

        if text.trim().is_empty() {
            warn!(
                iteration,
                "LLM returned empty final answer without a prior reply; retrying"
            );
            return Ok(FinalAnswerOutcome::Retry);
        }

        let mut m = Message::assistant(conversation_id, text);
        m.turn = turn_index;
        conv_store.save_message(&m).await?;

        // Collect candidates and sort deterministically so the chosen tool
        // doesn't depend on HashMap iteration order.
        // Priority: prefer "reply" over "post", prefer non-"blocks" variants,
        // alphabetical tiebreaker.
        let reply_entry = {
            let mut candidates: Vec<_> = ext_map
                .iter()
                .filter(|(name, _)| name.contains("reply") || name.contains("post"))
                .collect();
            candidates.sort_by(|(a, _), (b, _)| {
                let rank = |n: &str| -> u8 {
                    if n.contains("reply") && !n.contains("blocks") {
                        0
                    } else if n.contains("reply") {
                        1
                    } else {
                        2
                    }
                };
                rank(a).cmp(&rank(b)).then_with(|| a.cmp(b))
            });
            candidates.into_iter().next()
        };

        if let Some((reply_name, reply_handler)) = reply_entry {
            info!(
                iteration,
                tool = %reply_name,
                "LLM returned final answer; auto-posting via extension reply tool"
            );
            let schema = reply_handler.params_schema();
            let text_param = schema
                .get("required")
                .and_then(|r| r.as_array())
                .and_then(|r| if r.len() == 1 { r[0].as_str() } else { None })
                .filter(|name| matches!(*name, "text" | "content" | "message"));

            if let Some(param_name) = text_param {
                let mut params_map = HashMap::new();
                params_map.insert(
                    param_name.to_string(),
                    serde_json::Value::String(text.to_string()),
                );
                let ctx = ExecutionContext {
                    conversation_id,
                    turn: iteration as i64,
                    interface: interface.clone(),
                    interactive: false,
                    allowed_tools: None,
                    depth: 0,
                };
                if let Err(e) = reply_handler.run(params_map, &ctx).await {
                    warn!(tool = %reply_name, %e, "Auto-post via reply tool failed");
                }
            } else {
                warn!(
                    tool = %reply_name,
                    "Auto-post skipped: reply tool requires multiple or non-text params"
                );
            }
        } else {
            info!(
                iteration,
                "LLM returned final answer (no auto-post): no reply tool available"
            );
        }

        Ok(FinalAnswerOutcome::Done(TurnResult {
            answer: String::new(),
            attachments: Vec::new(),
        }))
    }

    /// Evaluate an `end_turn` tool call and return the appropriate outcome.
    ///
    /// Three possible paths:
    /// - **Deferred**: `end_turn` was called alongside real tool calls.
    /// - **Rejected**: a reply/react extension tool exists but was never called.
    /// - **Accepted**: the turn ends normally.
    ///
    /// In every case the helper records the OTel span, appends the tool result
    /// to `history`, and persists it to the database.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn handle_end_turn(
        has_real_calls: bool,
        replied: bool,
        ext_map: &HashMap<String, Arc<dyn ToolHandler>>,
        params: &serde_json::Value,
        iteration: usize,
        conversation_id: Uuid,
        turn_index: i64,
        otel_span: &mut opentelemetry::global::BoxedSpan,
        history: &mut Vec<ChatHistoryMessage>,
        conv_store: &ConversationStore,
    ) -> EndTurnOutcome {
        if has_real_calls {
            info!(
                iteration,
                "end_turn deferred (called alongside other tools)"
            );
            let msg = "end_turn deferred: processing other tool calls first";
            otel_span.set_attribute(KeyValue::new("tool_status", "deferred"));
            otel_span.set_attribute(KeyValue::new("tool_observation", msg.to_string()));
            crate::history::append_tool_result(history, "end_turn", msg);
            let tr = Self::make_tool_result_message(conversation_id, turn_index, "end_turn", msg);
            if let Err(e) = conv_store.save_message(&tr).await {
                warn!("Failed to persist deferred end_turn tool-result: {e}");
            }
            otel_span.end();
            return EndTurnOutcome::Deferred;
        }

        let reason = params
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("done");

        let has_reply_tool = ext_map
            .keys()
            .any(|n| n.contains("reply") || n.contains("post") || n.contains("react"));
        if !replied && has_reply_tool {
            warn!(
                iteration,
                reason, "end_turn rejected: reply tool available but no reply sent"
            );
            let msg = "end_turn rejected: you MUST call the `reply` tool \
                       before ending the turn. The user has not seen any \
                       response yet.";
            otel_span.set_attribute(KeyValue::new("tool_status", "rejected"));
            otel_span.set_attribute(KeyValue::new("tool_observation", msg.to_string()));
            crate::history::append_tool_result(history, "end_turn", msg);
            let tr = Self::make_tool_result_message(conversation_id, turn_index, "end_turn", msg);
            if let Err(e) = conv_store.save_message(&tr).await {
                warn!("Failed to persist rejected end_turn tool-result: {e}");
            }
            otel_span.end();
            return EndTurnOutcome::Rejected;
        }

        info!(iteration, reason, "end_turn called; stopping turn");
        let result_text = format!("end_turn: {reason}");
        otel_span.set_attribute(KeyValue::new("tool_status", "ok"));
        otel_span.set_attribute(KeyValue::new("tool_observation", result_text.clone()));
        crate::history::append_tool_result(history, "end_turn", &result_text);
        let tr =
            Self::make_tool_result_message(conversation_id, turn_index, "end_turn", &result_text);
        if let Err(e) = conv_store.save_message(&tr).await {
            warn!("Failed to persist end_turn tool-result: {e}");
        }
        otel_span.end();
        EndTurnOutcome::Accepted
    }
}
