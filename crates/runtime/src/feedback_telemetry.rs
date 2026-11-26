//! Telemetry for user feedback on assistant replies.
//!
//! The web API reports each stored or cleared vote to a
//! [`FeedbackTelemetrySink`]. The production sink exports a
//! `gen_ai.evaluation.result` log record and increments
//! `assistant.feedback.count`; tests substitute a recording sink.

use anyhow::Result;

use crate::metrics::MetricsRecorder;
use crate::otel_spans::{FeedbackLabel, FeedbackTelemetry, record_user_feedback};

/// Receives one event per stored or cleared vote.
///
/// A failing sink must never fail the request that triggered it; callers log
/// the error and carry on.
pub trait FeedbackTelemetrySink: Send + Sync {
    fn record(&self, feedback: &FeedbackTelemetry) -> Result<()>;
}

/// Exports feedback over OTLP: a span-parented log record plus a counter.
pub struct OtelFeedbackTelemetrySink {
    metrics: MetricsRecorder,
}

impl OtelFeedbackTelemetrySink {
    /// Build the sink on the global meter provider.
    pub fn new() -> Self {
        Self::with_metrics(MetricsRecorder::new())
    }

    pub fn with_metrics(metrics: MetricsRecorder) -> Self {
        Self { metrics }
    }
}

impl Default for OtelFeedbackTelemetrySink {
    fn default() -> Self {
        Self::new()
    }
}

impl FeedbackTelemetrySink for OtelFeedbackTelemetrySink {
    fn record(&self, feedback: &FeedbackTelemetry) -> Result<()> {
        record_user_feedback(feedback);
        let rating = match feedback.label {
            FeedbackLabel::Positive => "positive",
            FeedbackLabel::Negative => "negative",
            FeedbackLabel::Cleared => "cleared",
        };
        self.metrics.record_feedback(&feedback.agent_id, rating);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use opentelemetry::metrics::MeterProvider as _;
    use opentelemetry_sdk::metrics::SdkMeterProvider;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn otel_sink_records_without_error_on_a_local_meter() {
        let provider = SdkMeterProvider::builder().build();
        let sink = OtelFeedbackTelemetrySink::with_metrics(MetricsRecorder::with_meter(
            &provider.meter("test"),
        ));
        for label in [
            FeedbackLabel::Positive,
            FeedbackLabel::Negative,
            FeedbackLabel::Cleared,
        ] {
            let event = FeedbackTelemetry {
                label,
                conversation_id: Uuid::nil(),
                agent_id: "default".into(),
                message_id: Uuid::nil(),
                has_comment: false,
                traceparent: None,
            };
            assert!(sink.record(&event).is_ok(), "{label:?}");
        }
    }
}
