//! OpenTelemetry span helpers for the orchestrator.
//!
//! These are pure functions that create and enrich OTel spans for
//! LLM calls, tool executions, and conversation-level contexts.

use std::collections::HashMap;

use assistant_core::{
    ChatHistoryMessage, ChatRole, ContentBlock, Interface, LlmProvider, LlmResponse,
    LlmResponseMeta, ToolSpec,
};
use opentelemetry::{
    Array, Context as OtelContext, KeyValue, StringValue, Value, global,
    propagation::{Extractor, Injector},
    trace::{Span as _, SpanKind, TraceContextExt, Tracer as _},
};
use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_CONVERSATION_ID, GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MODEL,
    GEN_AI_RESPONSE_FINISH_REASONS, GEN_AI_RESPONSE_ID, GEN_AI_RESPONSE_MODEL,
    GEN_AI_USAGE_INPUT_TOKENS, GEN_AI_USAGE_OUTPUT_TOKENS, SERVER_ADDRESS,
};
use uuid::Uuid;

const GEN_AI_PROVIDER_NAME: &str = "gen_ai.provider.name";
const GEN_AI_SYSTEM_INSTRUCTIONS: &str = "gen_ai.system_instructions";
const GEN_AI_INPUT_MESSAGES: &str = "gen_ai.input.messages";
const GEN_AI_TOOL_DEFINITIONS: &str = "gen_ai.tool.definitions";
const GEN_AI_OUTPUT_MESSAGES: &str = "gen_ai.output.messages";

/// Create an OpenTelemetry context carrying a conversation-level root span.
///
/// Callers that manage conversation lifetimes (e.g. the CLI REPL or Slack
/// thread handler) should create this once per conversation and pass it to
/// each `run_turn*` call so all turns within the conversation share a single
/// trace.
pub fn start_conversation_context(conversation_id: Uuid, interface: &Interface) -> OtelContext {
    let tracer = global::tracer("assistant.orchestrator");
    let span_name = "conversation";
    let mut span = tracer.start(span_name);
    span.set_attribute(KeyValue::new("span.name", span_name));
    span.set_attribute(KeyValue::new(
        "conversation_id",
        conversation_id.to_string(),
    ));
    span.set_attribute(KeyValue::new(
        GEN_AI_CONVERSATION_ID,
        conversation_id.to_string(),
    ));
    span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
    OtelContext::current().with_span(span)
}

/// Create an OTel root span for an inbound interface event.
///
/// This is used at interface/webhook boundaries so downstream turn spans can be
/// linked through async bus handoffs via propagated trace context.
pub fn start_interface_root_context(
    interface: &Interface,
    operation: &str,
    conversation_id: Option<Uuid>,
) -> OtelContext {
    let tracer = global::tracer("assistant.interface");
    let span_name = format!("interface.{operation}");
    let mut span = tracer.start(span_name.clone());
    span.set_attribute(KeyValue::new("span.name", span_name));
    span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
    span.set_attribute(KeyValue::new("operation", operation.to_string()));
    if let Some(conversation_id) = conversation_id {
        span.set_attribute(KeyValue::new(
            "conversation_id",
            conversation_id.to_string(),
        ));
        span.set_attribute(KeyValue::new(
            GEN_AI_CONVERSATION_ID,
            conversation_id.to_string(),
        ));
    }
    OtelContext::current().with_span(span)
}

/// Inject W3C trace context into a `traceparent` header value.
pub fn traceparent_from_context(cx: &OtelContext) -> Option<String> {
    let mut carrier = HashMap::new();
    global::get_text_map_propagator(|prop| {
        prop.inject_context(cx, &mut HeaderInjector(&mut carrier));
    });
    carrier.get("traceparent").cloned()
}

/// Rebuild an OpenTelemetry context from a W3C `traceparent` value.
pub fn context_from_traceparent(traceparent: &str) -> OtelContext {
    global::get_text_map_propagator(|prop| {
        prop.extract(&HeaderExtractor {
            traceparent: Some(traceparent),
        })
    })
}

/// Create a bus interaction span (`produce` or `consume`) under `parent_cx`.
pub fn start_bus_span(
    span_kind: SpanKind,
    topic: &str,
    conversation_id: Option<Uuid>,
    parent_cx: &OtelContext,
) -> opentelemetry::global::BoxedSpan {
    let tracer = global::tracer("assistant.bus");
    let operation = match span_kind {
        SpanKind::Producer => "produce",
        SpanKind::Consumer => "consume",
        SpanKind::Client => "client",
        SpanKind::Server => "server",
        SpanKind::Internal => "internal",
    };
    let span_name = format!("bus.{operation} {topic}");
    let mut span = tracer.build_with_context(
        tracer.span_builder(span_name.clone()).with_kind(span_kind),
        parent_cx,
    );
    span.set_attribute(KeyValue::new("span.name", span_name));
    span.set_attribute(KeyValue::new("messaging.system", "assistant.bus"));
    span.set_attribute(KeyValue::new("messaging.operation", operation.to_string()));
    span.set_attribute(KeyValue::new(
        "messaging.destination.name",
        topic.to_string(),
    ));
    if let Some(conversation_id) = conversation_id {
        span.set_attribute(KeyValue::new(
            "conversation_id",
            conversation_id.to_string(),
        ));
        span.set_attribute(KeyValue::new(
            GEN_AI_CONVERSATION_ID,
            conversation_id.to_string(),
        ));
    }
    span
}

struct HeaderInjector<'a>(&'a mut HashMap<String, String>);

impl Injector for HeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_ascii_lowercase(), value);
    }
}

struct HeaderExtractor<'a> {
    traceparent: Option<&'a str>,
}

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        if key.eq_ignore_ascii_case("traceparent") {
            self.traceparent
        } else {
            None
        }
    }

    fn keys(&self) -> Vec<&str> {
        if self.traceparent.is_some() {
            vec!["traceparent"]
        } else {
            vec![]
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn start_tool_span(
    conversation_id: Uuid,
    iteration: usize,
    turn: i64,
    interface: &Interface,
    tool_name: &str,
    params: &serde_json::Value,
    parent_cx: &OtelContext,
    active_skill: Option<&str>,
) -> opentelemetry::global::BoxedSpan {
    let tracer = global::tracer("assistant.orchestrator");
    let span_name = format!("execute_tool {tool_name}");
    let mut span = tracer.start_with_context(span_name.clone(), parent_cx);
    span.set_attribute(KeyValue::new("span.name", span_name));
    span.set_attribute(KeyValue::new(
        "conversation_id",
        conversation_id.to_string(),
    ));
    span.set_attribute(KeyValue::new(
        GEN_AI_CONVERSATION_ID,
        conversation_id.to_string(),
    ));
    span.set_attribute(KeyValue::new("iteration", iteration as i64));
    span.set_attribute(KeyValue::new("turn", turn));
    span.set_attribute(KeyValue::new("interface", format!("{:?}", interface)));
    span.set_attribute(KeyValue::new("tool_name", tool_name.to_string()));
    let params_json =
        serde_json::to_string(params).unwrap_or_else(|_| "<unserializable>".to_string());
    span.set_attribute(KeyValue::new("tool_params", params_json));
    if let Some(skill) = active_skill {
        span.set_attribute(KeyValue::new("active_skill", skill.to_string()));
    }
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
    conversation_id: Uuid,
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
    let mut span = tracer.start_with_context(span_name.clone(), parent_cx);
    span.set_attribute(KeyValue::new("span.name", span_name));
    span.set_attribute(KeyValue::new(
        GEN_AI_PROVIDER_NAME,
        llm.provider_name().to_string(),
    ));
    span.set_attribute(KeyValue::new(GEN_AI_REQUEST_MODEL, model.to_string()));
    span.set_attribute(KeyValue::new(GEN_AI_OPERATION_NAME, "chat"));
    span.set_attribute(KeyValue::new(
        GEN_AI_CONVERSATION_ID,
        conversation_id.to_string(),
    ));
    span.set_attribute(KeyValue::new(
        SERVER_ADDRESS,
        llm.server_address().to_string(),
    ));
    span.set_attribute(KeyValue::new("iteration", iteration as i64));

    if trace_content {
        span.set_attribute(KeyValue::new(
            GEN_AI_SYSTEM_INSTRUCTIONS,
            system_prompt.to_string(),
        ));
        let input_json = serialize_history_for_span(history);
        span.set_attribute(KeyValue::new(GEN_AI_INPUT_MESSAGES, input_json));
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
            span.set_attribute(KeyValue::new(GEN_AI_TOOL_DEFINITIONS, tools_json));
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
    metrics: Option<(&crate::MetricsRecorder, &str, &str, std::time::Duration)>,
) {
    if let Some(model) = &meta.model {
        span.set_attribute(KeyValue::new(GEN_AI_RESPONSE_MODEL, model.clone()));
    }
    if let Some(id) = &meta.response_id {
        span.set_attribute(KeyValue::new(GEN_AI_RESPONSE_ID, id.clone()));
    }
    if let Some(reason) = &meta.finish_reason {
        span.set_attribute(KeyValue::new(
            GEN_AI_RESPONSE_FINISH_REASONS,
            Value::Array(Array::String(vec![StringValue::from(reason.clone())])),
        ));
    }
    if let Some(input) = meta.input_tokens {
        span.set_attribute(KeyValue::new(GEN_AI_USAGE_INPUT_TOKENS, input as i64));
    }
    if let Some(output) = meta.output_tokens {
        span.set_attribute(KeyValue::new(GEN_AI_USAGE_OUTPUT_TOKENS, output as i64));
    }

    if trace_content {
        let output_json = match response {
            LlmResponse::FinalAnswer(text, _) => {
                serde_json::json!([{"role": "assistant", "content": text}]).to_string()
            }
            LlmResponse::ToolCalls(resp) => {
                let items: Vec<serde_json::Value> = resp
                    .items
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
        span.set_attribute(KeyValue::new(GEN_AI_OUTPUT_MESSAGES, output_json));
    }

    // -- Record OTel metrics alongside the span ---------------------------------
    if let Some((recorder, agent_id, provider_name, duration)) = metrics {
        let model = meta.model.as_deref().unwrap_or("unknown");
        let input = meta.input_tokens.unwrap_or(0);
        let output = meta.output_tokens.unwrap_or(0);

        recorder.record_token_usage(agent_id, model, provider_name, "chat", input, output);
        recorder.record_operation_duration(
            agent_id,
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
                        ContentBlock::Text(t) => {
                            serde_json::json!({"type": "text", "text": t})
                        }
                        ContentBlock::Image { media_type, data } => {
                            serde_json::json!({
                                "type": "image",
                                "media_type": media_type,
                                "size_base64_chars": data.len(),
                            })
                        }
                        ContentBlock::Document { media_type, data } => {
                            serde_json::json!({
                                "type": "document",
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

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry_sdk::trace::{SdkTracerProvider, SpanExporter};

    /// A minimal in-memory exporter that collects exported spans.
    #[derive(Debug, Clone, Default)]
    struct CollectingExporter {
        spans: std::sync::Arc<std::sync::Mutex<Vec<opentelemetry_sdk::trace::SpanData>>>,
    }

    impl SpanExporter for CollectingExporter {
        async fn export(
            &self,
            batch: Vec<opentelemetry_sdk::trace::SpanData>,
        ) -> opentelemetry_sdk::error::OTelSdkResult {
            self.spans.lock().unwrap().extend(batch);
            Ok(())
        }
    }

    /// Verify that `start_tool_span` sets the `active_skill` attribute when provided.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_start_tool_span_sets_active_skill() {
        let exporter = CollectingExporter::default();
        let spans = exporter.spans.clone();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        // start_tool_span uses global::tracer(), so we must set the global provider.
        let _prev = opentelemetry::global::set_tracer_provider(provider.clone());

        let conversation_id = Uuid::new_v4();
        let params = serde_json::json!({"name": "my-skill"});
        let parent_cx = OtelContext::current();

        let mut span = start_tool_span(
            conversation_id,
            0,
            1,
            &Interface::Cli,
            "load-skill",
            &params,
            &parent_cx,
            Some("my-skill"),
        );
        span.end();

        let _ = provider.force_flush();

        let collected = spans.lock().unwrap();
        let tool_span = collected
            .iter()
            .find(|s| s.name.contains("execute_tool"))
            .expect("should have an execute_tool span");

        let attrs: HashMap<&str, &str> = tool_span
            .attributes
            .iter()
            .filter_map(|kv| {
                if let Value::String(ref s) = kv.value {
                    Some((kv.key.as_str(), s.as_str()))
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(
            attrs.get("active_skill"),
            Some(&"my-skill"),
            "active_skill attribute should be set"
        );
        assert_eq!(attrs.get("tool_name"), Some(&"load-skill"));
    }

    /// Verify that `start_tool_span` does NOT set `active_skill` when None.
    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_start_tool_span_no_active_skill() {
        let exporter = CollectingExporter::default();
        let spans = exporter.spans.clone();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        let _prev = opentelemetry::global::set_tracer_provider(provider.clone());

        let conversation_id = Uuid::new_v4();
        let params = serde_json::json!({"query": "hello"});
        let parent_cx = OtelContext::current();

        let mut span = start_tool_span(
            conversation_id,
            0,
            1,
            &Interface::Cli,
            "web-search",
            &params,
            &parent_cx,
            None,
        );
        span.end();

        let _ = provider.force_flush();

        let collected = spans.lock().unwrap();
        let tool_span = collected
            .iter()
            .find(|s| s.name.contains("execute_tool"))
            .expect("should have an execute_tool span");

        let has_active_skill = tool_span
            .attributes
            .iter()
            .any(|kv| kv.key.as_str() == "active_skill");

        assert!(
            !has_active_skill,
            "active_skill attribute should not be present when None"
        );
    }
}
