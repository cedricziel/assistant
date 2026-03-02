//! OpenTelemetry span helpers for the orchestrator.
//!
//! These are pure functions that create and enrich OTel spans for
//! LLM calls, tool executions, and conversation-level contexts.

use assistant_core::Interface;
use assistant_llm::{
    ChatHistoryMessage, ChatRole, LlmProvider, LlmResponse, LlmResponseMeta, ToolSpec,
};
use opentelemetry::{
    global,
    trace::{Span as _, TraceContextExt, Tracer as _},
    Context as OtelContext, KeyValue,
};
use uuid::Uuid;

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

pub(crate) fn start_tool_span(
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
pub(crate) fn start_llm_span(
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
pub(crate) fn finish_llm_span(
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
pub(crate) fn serialize_history_for_span(history: &[ChatHistoryMessage]) -> String {
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
