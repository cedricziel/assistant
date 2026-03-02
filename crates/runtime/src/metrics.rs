//! GenAI and operational metric instruments (OTel semantic conventions).
//!
//! Provides a [`MetricsRecorder`] that the orchestrator uses to record
//! observations against the global `MeterProvider`.

use opentelemetry::metrics::{Counter, Histogram};
use opentelemetry::{global, KeyValue};

/// Holds all metric instruments for recording GenAI and operational metrics.
///
/// Created once from the global meter provider and stored on the
/// [`Orchestrator`](crate::Orchestrator).
pub struct MetricsRecorder {
    // -- GenAI metrics (OTel semconv v1.40) ----------------------------------
    /// `gen_ai.client.token.usage` — histogram of input/output token counts.
    pub token_usage: Histogram<f64>,
    /// `gen_ai.client.operation.duration` — end-to-end LLM operation duration.
    pub operation_duration: Histogram<f64>,
    /// `gen_ai.server.time_to_first_token` — time to first token in streaming.
    pub time_to_first_token: Histogram<f64>,
    /// `gen_ai.server.time_per_output_token` — per-token decode latency.
    pub time_per_output_token: Histogram<f64>,

    // -- Operational metrics -------------------------------------------------
    /// `assistant.turn.count` — turns processed.
    pub turn_count: Counter<u64>,
    /// `assistant.turn.duration` — turn processing time.
    pub turn_duration: Histogram<f64>,
    /// `assistant.tool.invocations` — tool calls executed.
    pub tool_invocations: Counter<u64>,
    /// `assistant.tool.duration` — tool execution time.
    pub tool_duration: Histogram<f64>,
    /// `assistant.error.count` — errors encountered.
    pub error_count: Counter<u64>,
    /// `assistant.conversation.count` — conversations created.
    pub conversation_count: Counter<u64>,
    /// `assistant.agent.spawn.count` — sub-agents spawned.
    pub agent_spawn_count: Counter<u64>,
}

impl Default for MetricsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRecorder {
    /// Create all instruments from the global meter provider.
    pub fn new() -> Self {
        let meter = global::meter("assistant-runtime");

        Self {
            // -- GenAI semconv -----------------------------------------------
            token_usage: meter
                .f64_histogram("gen_ai.client.token.usage")
                .with_description("Number of input and output tokens used")
                .with_unit("{token}")
                .build(),

            operation_duration: meter
                .f64_histogram("gen_ai.client.operation.duration")
                .with_description("GenAI operation duration")
                .with_unit("s")
                .build(),

            time_to_first_token: meter
                .f64_histogram("gen_ai.server.time_to_first_token")
                .with_description("Time to generate first token")
                .with_unit("s")
                .build(),

            time_per_output_token: meter
                .f64_histogram("gen_ai.server.time_per_output_token")
                .with_description("Time per output token after the first")
                .with_unit("s")
                .build(),

            // -- Operational -------------------------------------------------
            turn_count: meter
                .u64_counter("assistant.turn.count")
                .with_description("Number of turns processed")
                .with_unit("{turn}")
                .build(),

            turn_duration: meter
                .f64_histogram("assistant.turn.duration")
                .with_description("Turn processing duration")
                .with_unit("s")
                .build(),

            tool_invocations: meter
                .u64_counter("assistant.tool.invocations")
                .with_description("Number of tool invocations")
                .with_unit("{invocation}")
                .build(),

            tool_duration: meter
                .f64_histogram("assistant.tool.duration")
                .with_description("Tool execution duration")
                .with_unit("s")
                .build(),

            error_count: meter
                .u64_counter("assistant.error.count")
                .with_description("Number of errors")
                .with_unit("{error}")
                .build(),

            conversation_count: meter
                .u64_counter("assistant.conversation.count")
                .with_description("Number of conversations created")
                .with_unit("{conversation}")
                .build(),

            agent_spawn_count: meter
                .u64_counter("assistant.agent.spawn.count")
                .with_description("Number of sub-agents spawned")
                .with_unit("{agent}")
                .build(),
        }
    }

    // -- Convenience recording methods ----------------------------------------

    /// Record token usage for an LLM call (separate input/output observations
    /// per the GenAI semconv).
    pub fn record_token_usage(
        &self,
        model: &str,
        provider: &str,
        operation: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) {
        let common = [
            KeyValue::new("gen_ai.request.model", model.to_string()),
            KeyValue::new("gen_ai.provider.name", provider.to_string()),
            KeyValue::new("gen_ai.operation.name", operation.to_string()),
        ];

        let mut input_attrs = common.to_vec();
        input_attrs.push(KeyValue::new("gen_ai.token.type", "input"));
        self.token_usage.record(input_tokens as f64, &input_attrs);

        let mut output_attrs = common.to_vec();
        output_attrs.push(KeyValue::new("gen_ai.token.type", "output"));
        self.token_usage.record(output_tokens as f64, &output_attrs);
    }

    /// Record LLM operation duration.
    pub fn record_operation_duration(
        &self,
        model: &str,
        provider: &str,
        operation: &str,
        duration_s: f64,
        error_type: Option<&str>,
    ) {
        let mut attrs = vec![
            KeyValue::new("gen_ai.request.model", model.to_string()),
            KeyValue::new("gen_ai.provider.name", provider.to_string()),
            KeyValue::new("gen_ai.operation.name", operation.to_string()),
        ];
        if let Some(err) = error_type {
            attrs.push(KeyValue::new("error.type", err.to_string()));
        }
        self.operation_duration.record(duration_s, &attrs);
    }

    /// Record a turn start.
    pub fn record_turn(&self, skill: Option<&str>, interface: &str) {
        let mut attrs = vec![KeyValue::new("interface", interface.to_string())];
        if let Some(s) = skill {
            attrs.push(KeyValue::new("skill", s.to_string()));
        }
        self.turn_count.add(1, &attrs);
    }

    /// Record turn duration.
    pub fn record_turn_duration(&self, duration_s: f64, skill: Option<&str>, interface: &str) {
        let mut attrs = vec![KeyValue::new("interface", interface.to_string())];
        if let Some(s) = skill {
            attrs.push(KeyValue::new("skill", s.to_string()));
        }
        self.turn_duration.record(duration_s, &attrs);
    }

    /// Record a tool invocation.
    pub fn record_tool_invocation(&self, tool_name: &str) {
        let attrs = [KeyValue::new("tool.name", tool_name.to_string())];
        self.tool_invocations.add(1, &attrs);
    }

    /// Record tool execution duration.
    pub fn record_tool_duration(&self, tool_name: &str, duration_s: f64) {
        let attrs = [KeyValue::new("tool.name", tool_name.to_string())];
        self.tool_duration.record(duration_s, &attrs);
    }

    /// Record an error.
    pub fn record_error(&self, error_type: &str, source: &str) {
        let attrs = [
            KeyValue::new("error.type", error_type.to_string()),
            KeyValue::new("source", source.to_string()),
        ];
        self.error_count.add(1, &attrs);
    }

    /// Record time-to-first-token for a streaming response.
    pub fn record_ttft(&self, model: &str, provider: &str, ttft_s: f64) {
        let attrs = [
            KeyValue::new("gen_ai.request.model", model.to_string()),
            KeyValue::new("gen_ai.provider.name", provider.to_string()),
        ];
        self.time_to_first_token.record(ttft_s, &attrs);
    }

    /// Record per-token output latency.
    pub fn record_time_per_token(&self, model: &str, provider: &str, per_token_s: f64) {
        let attrs = [
            KeyValue::new("gen_ai.request.model", model.to_string()),
            KeyValue::new("gen_ai.provider.name", provider.to_string()),
        ];
        self.time_per_output_token.record(per_token_s, &attrs);
    }
}
