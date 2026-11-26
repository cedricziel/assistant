//! GenAI and operational metric instruments (OTel semantic conventions).
//!
//! Provides a [`MetricsRecorder`] that the orchestrator uses to record
//! observations against the global `MeterProvider`.
// The `gen_ai.*` constants from opentelemetry-semantic-conventions are deprecated
// pending migration to the new opentelemetry-semantic-conventions-genai crate.
// Allow their use until that crate is published on crates.io.
#![allow(deprecated)]

use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry::{KeyValue, global};
use opentelemetry_semantic_conventions::attribute::ERROR_TYPE;
use opentelemetry_semantic_conventions::attribute::{
    GEN_AI_OPERATION_NAME, GEN_AI_REQUEST_MODEL, GEN_AI_TOKEN_TYPE,
};
use opentelemetry_semantic_conventions::metric::{
    GEN_AI_CLIENT_OPERATION_DURATION, GEN_AI_CLIENT_TOKEN_USAGE,
};

const GEN_AI_PROVIDER_NAME: &str = "gen_ai.provider.name";

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
    // -- Operational metrics -------------------------------------------------
    /// `assistant.turn.count` — turns processed.
    pub turn_count: Counter<u64>,
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
    /// `assistant.submit.timeout_total` — submit_turn timeouts.
    pub submit_timeout_count: Counter<u64>,
    /// `assistant.submit.late_result_total` — results matched after timeout deadline.
    pub submit_late_result_count: Counter<u64>,
    /// `assistant.submit.result_unmatched_total` — result correlation mismatches.
    pub submit_result_unmatched_count: Counter<u64>,
    /// `assistant.slack.duplicate_event_total` — duplicate Slack events dropped.
    pub slack_duplicate_event_count: Counter<u64>,
    /// `assistant.feedback.count` — user feedback events (not current votes).
    pub feedback_count: Counter<u64>,
}

impl Default for MetricsRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRecorder {
    /// Create all instruments from the global meter provider.
    pub fn new() -> Self {
        Self::with_meter(&global::meter("assistant-runtime"))
    }

    /// Create all instruments from `meter`.
    pub fn with_meter(meter: &Meter) -> Self {
        Self {
            // -- GenAI semconv -----------------------------------------------
            token_usage: meter
                .f64_histogram(GEN_AI_CLIENT_TOKEN_USAGE)
                .with_description("Number of input and output tokens used")
                .with_unit("{token}")
                .build(),

            operation_duration: meter
                .f64_histogram(GEN_AI_CLIENT_OPERATION_DURATION)
                .with_description("GenAI operation duration")
                .with_unit("s")
                .build(),

            // -- Operational -------------------------------------------------
            turn_count: meter
                .u64_counter("assistant.turn.count")
                .with_description("Number of turns processed")
                .with_unit("{turn}")
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

            submit_timeout_count: meter
                .u64_counter("assistant.submit.timeout_total")
                .with_description("Number of submit_turn timeout outcomes")
                .with_unit("{timeout}")
                .build(),

            submit_late_result_count: meter
                .u64_counter("assistant.submit.late_result_total")
                .with_description("Number of submit_turn late result matches")
                .with_unit("{result}")
                .build(),

            submit_result_unmatched_count: meter
                .u64_counter("assistant.submit.result_unmatched_total")
                .with_description("Number of submit/result correlation mismatches")
                .with_unit("{result}")
                .build(),

            slack_duplicate_event_count: meter
                .u64_counter("assistant.slack.duplicate_event_total")
                .with_description("Number of duplicate Slack events dropped")
                .with_unit("{event}")
                .build(),

            feedback_count: meter
                .u64_counter("assistant.feedback.count")
                .with_description("Number of user feedback events on assistant replies")
                .with_unit("{event}")
                .build(),
        }
    }

    // -- Convenience recording methods ----------------------------------------

    /// Record token usage for an LLM call (separate input/output observations
    /// per the GenAI semconv).
    pub fn record_token_usage(
        &self,
        agent_id: &str,
        model: &str,
        provider: &str,
        operation: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) {
        let common = [
            KeyValue::new(GEN_AI_REQUEST_MODEL, model.to_string()),
            KeyValue::new(GEN_AI_PROVIDER_NAME, provider.to_string()),
            KeyValue::new(GEN_AI_OPERATION_NAME, operation.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];

        let mut input_attrs = common.to_vec();
        input_attrs.push(KeyValue::new(GEN_AI_TOKEN_TYPE, "input"));
        self.token_usage.record(input_tokens as f64, &input_attrs);

        let mut output_attrs = common.to_vec();
        output_attrs.push(KeyValue::new(GEN_AI_TOKEN_TYPE, "output"));
        self.token_usage.record(output_tokens as f64, &output_attrs);
    }

    /// Record LLM operation duration.
    pub fn record_operation_duration(
        &self,
        agent_id: &str,
        model: &str,
        provider: &str,
        operation: &str,
        duration_s: f64,
        error_type: Option<&str>,
    ) {
        let mut attrs = vec![
            KeyValue::new(GEN_AI_REQUEST_MODEL, model.to_string()),
            KeyValue::new(GEN_AI_PROVIDER_NAME, provider.to_string()),
            KeyValue::new(GEN_AI_OPERATION_NAME, operation.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        if let Some(err) = error_type {
            attrs.push(KeyValue::new(ERROR_TYPE, err.to_string()));
        }
        self.operation_duration.record(duration_s, &attrs);
    }

    /// Record a turn start.
    pub fn record_turn(&self, agent_id: &str, skill: Option<&str>, interface: &str) {
        let mut attrs = vec![
            KeyValue::new("interface", interface.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        if let Some(s) = skill {
            attrs.push(KeyValue::new("skill", s.to_string()));
        }
        self.turn_count.add(1, &attrs);
    }

    /// Record a tool invocation.
    pub fn record_tool_invocation(&self, agent_id: &str, tool_name: &str, span_name: &str) {
        let attrs = [
            KeyValue::new("tool.name", tool_name.to_string()),
            KeyValue::new("span.name", span_name.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        self.tool_invocations.add(1, &attrs);
    }

    /// Record tool execution duration.
    pub fn record_tool_duration(
        &self,
        agent_id: &str,
        tool_name: &str,
        span_name: &str,
        duration_s: f64,
    ) {
        let attrs = [
            KeyValue::new("tool.name", tool_name.to_string()),
            KeyValue::new("span.name", span_name.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        self.tool_duration.record(duration_s, &attrs);
    }

    /// Record an error.
    pub fn record_error(&self, agent_id: &str, error_type: &str, source: &str) {
        let attrs = [
            KeyValue::new(ERROR_TYPE, error_type.to_string()),
            KeyValue::new("source", source.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        self.error_count.add(1, &attrs);
    }

    /// Record a submit_turn timeout.
    pub fn record_submit_timeout(&self, agent_id: &str, interface: &str) {
        let attrs = [
            KeyValue::new("interface", interface.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        self.submit_timeout_count.add(1, &attrs);
    }

    /// Record a submit_turn late result match.
    pub fn record_submit_late_result(&self, agent_id: &str, interface: &str) {
        let attrs = [
            KeyValue::new("interface", interface.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        self.submit_late_result_count.add(1, &attrs);
    }

    /// Record a submit/result correlation mismatch.
    pub fn record_submit_result_unmatched(&self, agent_id: &str, interface: &str, reason: &str) {
        let attrs = [
            KeyValue::new("interface", interface.to_string()),
            KeyValue::new("reason", reason.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        self.submit_result_unmatched_count.add(1, &attrs);
    }

    /// Record a duplicate Slack event that was deduplicated by the runner.
    pub fn record_slack_duplicate_event(&self, agent_id: &str, event_kind: &str) {
        let attrs = [
            KeyValue::new("interface", "Slack"),
            KeyValue::new("event.kind", event_kind.to_string()),
            KeyValue::new("agent.id", agent_id.to_string()),
        ];
        self.slack_duplicate_event_count.add(1, &attrs);
    }

    /// Record a user feedback event. `rating` is `positive`, `negative` or
    /// `cleared`.
    pub fn record_feedback(&self, agent_id: &str, rating: &str) {
        let attrs = [
            KeyValue::new("rating", rating.to_string()),
            KeyValue::new("agent_id", agent_id.to_string()),
        ];
        self.feedback_count.add(1, &attrs);
    }
}

#[cfg(test)]
mod tests {
    use opentelemetry::metrics::MeterProvider as _;
    use opentelemetry_sdk::metrics::data::{AggregatedMetrics, MetricData};
    use opentelemetry_sdk::metrics::{InMemoryMetricExporter, PeriodicReader, SdkMeterProvider};

    use super::*;

    #[test]
    fn record_feedback_counts_events_by_rating_and_agent() {
        let exporter = InMemoryMetricExporter::default();
        let provider = SdkMeterProvider::builder()
            .with_reader(PeriodicReader::builder(exporter.clone()).build())
            .build();
        let recorder = MetricsRecorder::with_meter(&provider.meter("test"));

        recorder.record_feedback("default", "negative");
        recorder.record_feedback("default", "negative");
        recorder.record_feedback("default", "cleared");
        provider.force_flush().expect("flush");

        let mut points = Vec::new();
        for rm in exporter.get_finished_metrics().expect("metrics") {
            for sm in rm.scope_metrics() {
                for m in sm
                    .metrics()
                    .filter(|m| m.name() == "assistant.feedback.count")
                {
                    if let AggregatedMetrics::U64(MetricData::Sum(sum)) = m.data() {
                        for dp in sum.data_points() {
                            let attr = |k: &str| {
                                dp.attributes()
                                    .find(|kv| kv.key.as_str() == k)
                                    .map(|kv| kv.value.to_string())
                            };
                            points.push((attr("rating"), attr("agent_id"), dp.value()));
                        }
                    }
                }
            }
        }
        points.sort();
        assert_eq!(
            points,
            vec![
                (Some("cleared".into()), Some("default".into()), 1),
                (Some("negative".into()), Some("default".into()), 2),
            ]
        );
    }
}
