//! OpenTelemetry span helpers for the orchestrator.
//!
//! These are pure functions that create and enrich OTel spans for
//! LLM calls, tool executions, and conversation-level contexts.
// The `gen_ai.*` constants from opentelemetry-semantic-conventions are deprecated
// pending migration to the new opentelemetry-semantic-conventions-genai crate.
// Allow their use until that crate is published on crates.io.
#![allow(deprecated)]

use std::collections::HashMap;

use assistant_core::{
    ChatHistoryMessage, ChatRole, ContentBlock, LlmProvider, LlmResponse, LlmResponseMeta,
    ToolSpec, types::conversation::Interface,
};
use opentelemetry::{
    Array, Context as OtelContext, KeyValue, StringValue, Value, global,
    propagation::{Extractor, Injector, TextMapPropagator as _},
    trace::{Span as _, SpanKind, TraceContextExt, Tracer as _},
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_CONVERSATION_ID, GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MODEL,
    GEN_AI_RESPONSE_FINISH_REASONS, GEN_AI_RESPONSE_ID, GEN_AI_RESPONSE_MODEL,
    GEN_AI_USAGE_CACHE_CREATION_INPUT_TOKENS, GEN_AI_USAGE_CACHE_READ_INPUT_TOKENS,
    GEN_AI_USAGE_INPUT_TOKENS, GEN_AI_USAGE_OUTPUT_TOKENS, GEN_AI_USAGE_REASONING_OUTPUT_TOKENS,
    SERVER_ADDRESS,
};
use uuid::Uuid;

const GEN_AI_PROVIDER_NAME: &str = "gen_ai.provider.name";
const GEN_AI_SYSTEM_INSTRUCTIONS: &str = "gen_ai.system_instructions";
const GEN_AI_INPUT_MESSAGES: &str = "gen_ai.input.messages";
const GEN_AI_TOOL_DEFINITIONS: &str = "gen_ai.tool.definitions";
const GEN_AI_OUTPUT_MESSAGES: &str = "gen_ai.output.messages";
/// No GenAI convention exists for cost yet.
const ASSISTANT_LLM_COST_USD: &str = "assistant.llm.cost_usd";

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

/// Format a span's context as a W3C `traceparent` value
/// (`00-{trace_id}-{span_id}-{flags}`).
///
/// Reads `span.span_context()` directly instead of going through the global
/// propagator, which is a no-op until OTLP export is enabled. Returns `None`
/// for an invalid context (a no-op span), so nothing is recorded when tracing
/// is off.
pub fn traceparent_from_span(span: &impl opentelemetry::trace::Span) -> Option<String> {
    let ctx = span.span_context();
    ctx.is_valid().then(|| {
        format!(
            "00-{}-{}-{:02x}",
            ctx.trace_id(),
            ctx.span_id(),
            ctx.trace_flags().to_u8()
        )
    })
}

// -- User feedback ------------------------------------------------------------

/// What a user feedback event reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackLabel {
    Positive,
    Negative,
    /// The user withdrew a previous vote.
    Cleared,
}

/// A user feedback event to export as `gen_ai.evaluation.result`.
///
/// Deliberately has no comment field: comment text must never reach an
/// exporter, only whether a comment exists.
#[derive(Debug, Clone)]
pub struct FeedbackTelemetry {
    pub label: FeedbackLabel,
    pub conversation_id: Uuid,
    pub agent_id: String,
    pub message_id: Uuid,
    pub has_comment: bool,
    /// W3C `traceparent` of the evaluated `chat {model}` span, if one was stored.
    pub traceparent: Option<String>,
}

/// Export a `gen_ai.evaluation.result` log record for user feedback.
///
/// The record is parented to the evaluated span when `traceparent` parses:
/// the stored value is parsed with a local [`TraceContextPropagator`] (the
/// global one is a no-op unless OTLP export is enabled) and attached while the
/// event is emitted, which is when the SDK logger reads the trace context.
/// `cleared` carries no `score.value`, so averaging backends do not count a
/// withdrawn vote as a neutral one.
pub fn record_user_feedback(feedback: &FeedbackTelemetry) {
    let _guard = feedback.traceparent.as_deref().and_then(|tp| {
        let cx = TraceContextPropagator::new().extract(&HeaderExtractor {
            traceparent: Some(tp),
        });
        cx.span().span_context().is_valid().then(|| cx.attach())
    });
    let (label, score) = match feedback.label {
        FeedbackLabel::Positive => ("positive", Some(1.0_f64)),
        FeedbackLabel::Negative => ("negative", Some(-1.0_f64)),
        FeedbackLabel::Cleared => ("cleared", None),
    };
    tracing::event!(
        name: "gen_ai.evaluation.result",
        target: "gen_ai.evaluation.result",
        tracing::Level::INFO,
        { gen_ai.evaluation.name = "user_feedback",
        gen_ai.evaluation.score.label = label,
        gen_ai.evaluation.score.value = score,
        gen_ai.conversation.id = %feedback.conversation_id,
        gen_ai.agent.id = feedback.agent_id.as_str(),
        assistant.message.id = %feedback.message_id,
        assistant.feedback.has_comment = feedback.has_comment }
    );
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
    for (key, value) in [
        (
            GEN_AI_USAGE_CACHE_READ_INPUT_TOKENS,
            meta.cached_input_tokens,
        ),
        (
            GEN_AI_USAGE_CACHE_CREATION_INPUT_TOKENS,
            meta.cache_write_input_tokens,
        ),
        (GEN_AI_USAGE_REASONING_OUTPUT_TOKENS, meta.reasoning_tokens),
    ] {
        if let Some(tokens) = value {
            span.set_attribute(KeyValue::new(key, tokens as i64));
        }
    }
    if let Some(cost) = meta.cost_usd {
        span.set_attribute(KeyValue::new(ASSISTANT_LLM_COST_USD, cost));
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
            ChatHistoryMessage::AssistantToolCalls { items: calls, .. } => {
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

    /// Serializes tests that install the global tracer provider, which
    /// otherwise race and export each other's spans.
    static GLOBAL_TRACER_PROVIDER: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    /// Finish an LLM span on a local provider and return its attributes.
    fn finished_llm_span_attrs(meta: LlmResponseMeta) -> HashMap<String, opentelemetry::Value> {
        use opentelemetry::trace::{Tracer, TracerProvider};

        let exporter = CollectingExporter::default();
        let spans = exporter.spans.clone();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        let tracer = opentelemetry::global::BoxedTracer::new(Box::new(provider.tracer("test")));
        let mut span = tracer.start("chat");
        let response = LlmResponse::FinalAnswer("hi".into(), meta.clone());
        finish_llm_span(&mut span, &meta, &response, false, None);
        let _ = provider.force_flush();

        let collected = spans.lock().unwrap();
        collected[0]
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str().to_string(), kv.value.clone()))
            .collect()
    }

    #[test]
    fn traceparent_from_span_formats_a_sampled_span_context() {
        use opentelemetry::trace::{Tracer, TracerProvider};

        let provider = SdkTracerProvider::builder().build();
        let tracer = provider.tracer("test");
        let span = tracer.start("chat");
        let ctx = span.span_context().clone();

        let expected = format!("00-{}-{}-01", ctx.trace_id(), ctx.span_id());
        assert_eq!(traceparent_from_span(&span), Some(expected));
    }

    #[test]
    fn traceparent_from_span_is_none_for_a_noop_span() {
        let span = opentelemetry::trace::noop::NoopSpan::DEFAULT;
        assert_eq!(traceparent_from_span(&span), None);
    }

    #[test]
    fn llm_span_records_cache_reasoning_and_cost() {
        let attrs = finished_llm_span_attrs(LlmResponseMeta {
            input_tokens: Some(10_339),
            output_tokens: Some(60),
            cached_input_tokens: Some(10_318),
            cache_write_input_tokens: Some(21),
            reasoning_tokens: Some(40),
            cost_usd: Some(0.0021),
            ..Default::default()
        });
        assert_eq!(
            attrs.get("gen_ai.usage.cache_read.input_tokens"),
            Some(&Value::I64(10_318))
        );
        assert_eq!(
            attrs.get("gen_ai.usage.cache_creation.input_tokens"),
            Some(&Value::I64(21))
        );
        assert_eq!(
            attrs.get("gen_ai.usage.reasoning.output_tokens"),
            Some(&Value::I64(40))
        );
        assert_eq!(
            attrs.get("assistant.llm.cost_usd"),
            Some(&Value::F64(0.0021))
        );
    }

    #[test]
    fn llm_span_omits_unknown_usage_details() {
        let attrs = finished_llm_span_attrs(LlmResponseMeta {
            input_tokens: Some(5),
            output_tokens: Some(1),
            ..Default::default()
        });
        for key in [
            "gen_ai.usage.cache_read.input_tokens",
            "gen_ai.usage.cache_creation.input_tokens",
            "gen_ai.usage.reasoning.output_tokens",
            "assistant.llm.cost_usd",
        ] {
            assert!(
                !attrs.contains_key(key),
                "{key} must be absent when unknown"
            );
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
        let _global = GLOBAL_TRACER_PROVIDER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        opentelemetry::global::set_tracer_provider(provider.clone());

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
        let _global = GLOBAL_TRACER_PROVIDER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        opentelemetry::global::set_tracer_provider(provider.clone());

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
    // -- record_user_feedback ---------------------------------------------------

    const TRACEPARENT: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    fn feedback(label: FeedbackLabel, comment: bool, tp: Option<&str>) -> FeedbackTelemetry {
        FeedbackTelemetry {
            label,
            conversation_id: Uuid::nil(),
            agent_id: "default".into(),
            message_id: Uuid::from_u128(7),
            has_comment: comment,
            traceparent: tp.map(str::to_owned),
        }
    }

    /// Emit `feedback` through a thread-scoped log pipeline and return the
    /// exported records.
    fn exported_logs(
        feedback: &FeedbackTelemetry,
    ) -> Vec<opentelemetry_sdk::logs::in_memory_exporter::LogDataWithResource> {
        use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
        use opentelemetry_sdk::logs::{InMemoryLogExporter, SdkLoggerProvider};
        use tracing_subscriber::layer::SubscriberExt as _;

        let exporter = InMemoryLogExporter::default();
        let provider = SdkLoggerProvider::builder()
            .with_simple_exporter(exporter.clone())
            .build();
        let subscriber =
            tracing_subscriber::registry().with(OpenTelemetryTracingBridge::new(&provider));
        tracing::subscriber::with_default(subscriber, || record_user_feedback(feedback));
        exporter.get_emitted_logs().unwrap_or_default()
    }

    fn log_attrs(
        log: &opentelemetry_sdk::logs::in_memory_exporter::LogDataWithResource,
    ) -> HashMap<String, opentelemetry::logs::AnyValue> {
        log.record
            .attributes_iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn negative_feedback_is_exported_in_the_stored_trace() {
        let logs = exported_logs(&feedback(FeedbackLabel::Negative, false, Some(TRACEPARENT)));
        assert_eq!(logs.len(), 1, "one record per event");
        let record = &logs[0].record;
        assert_eq!(record.event_name(), Some("gen_ai.evaluation.result"));
        let tc = record.trace_context().expect("trace context");
        assert_eq!(
            tc.trace_id.to_string(),
            "4bf92f3577b34da6a3ce929d0e0e4736",
            "trace id"
        );
        assert_eq!(tc.span_id.to_string(), "00f067aa0ba902b7", "span id");

        let attrs = log_attrs(&logs[0]);
        let text = |k: &str| match attrs.get(k) {
            Some(opentelemetry::logs::AnyValue::String(s)) => Some(s.to_string()),
            _ => None,
        };
        assert_eq!(
            text("gen_ai.evaluation.name").as_deref(),
            Some("user_feedback")
        );
        assert_eq!(
            text("gen_ai.evaluation.score.label").as_deref(),
            Some("negative")
        );
        assert!(
            matches!(
                attrs.get("gen_ai.evaluation.score.value"),
                Some(opentelemetry::logs::AnyValue::Double(v)) if *v == -1.0
            ),
            "score.value must be -1.0, got {:?}",
            attrs.get("gen_ai.evaluation.score.value")
        );
        assert_eq!(
            text("gen_ai.conversation.id").as_deref(),
            Some(Uuid::nil().to_string().as_str())
        );
        assert_eq!(text("gen_ai.agent.id").as_deref(), Some("default"));
        assert_eq!(
            text("assistant.message.id").as_deref(),
            Some(Uuid::from_u128(7).to_string().as_str())
        );
        assert!(matches!(
            attrs.get("assistant.feedback.has_comment"),
            Some(opentelemetry::logs::AnyValue::Boolean(false))
        ));
        assert!(!attrs.contains_key("gen_ai.evaluation.explanation"));
    }

    #[test]
    fn positive_feedback_scores_plus_one() {
        let logs = exported_logs(&feedback(FeedbackLabel::Positive, false, None));
        let attrs = log_attrs(&logs[0]);
        assert!(matches!(
            attrs.get("gen_ai.evaluation.score.value"),
            Some(opentelemetry::logs::AnyValue::Double(v)) if *v == 1.0
        ));
    }

    #[test]
    fn has_comment_is_reported_and_no_text_is_exported() {
        for label in [FeedbackLabel::Positive, FeedbackLabel::Negative] {
            let logs = exported_logs(&feedback(label, true, Some(TRACEPARENT)));
            let attrs = log_attrs(&logs[0]);
            assert!(matches!(
                attrs.get("assistant.feedback.has_comment"),
                Some(opentelemetry::logs::AnyValue::Boolean(true))
            ));
            assert!(!attrs.contains_key("gen_ai.evaluation.explanation"));
        }
    }

    #[test]
    fn cleared_has_a_label_but_no_score_value() {
        let logs = exported_logs(&feedback(FeedbackLabel::Cleared, false, Some(TRACEPARENT)));
        assert_eq!(logs.len(), 1);
        let attrs = log_attrs(&logs[0]);
        assert!(matches!(
            attrs.get("gen_ai.evaluation.score.label"),
            Some(opentelemetry::logs::AnyValue::String(s)) if s.as_str() == "cleared"
        ));
        assert!(!attrs.contains_key("gen_ai.evaluation.score.value"));
    }

    #[test]
    fn missing_or_malformed_traceparent_exports_without_trace_context() {
        for tp in [None, Some("garbage")] {
            let logs = exported_logs(&feedback(FeedbackLabel::Positive, false, tp));
            assert_eq!(logs.len(), 1, "event still emitted for {tp:?}");
            assert!(logs[0].record.trace_context().is_none(), "for {tp:?}");
            assert!(log_attrs(&logs[0]).contains_key("assistant.message.id"));
        }
    }
}
