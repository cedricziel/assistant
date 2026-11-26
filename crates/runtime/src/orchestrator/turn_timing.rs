//! Turn timing measured by [`Orchestrator::run_turn_core`](super::Orchestrator).
//!
//! All instants come from the orchestrator's injected
//! [`Clock`](assistant_core::clock::Clock), so tests can drive them with a
//! `FakeClock`.

use std::time::{Duration, Instant};

use assistant_core::TurnTiming;

/// Instants observed while one LLM call streams.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CallMarks {
    /// First chunk of any kind, thinking included.
    pub first_chunk: Option<Instant>,
    /// First visible answer token.
    pub first_text: Option<Instant>,
}

/// Accumulates timing across the LLM calls of one turn.
#[derive(Debug)]
pub(super) struct TurnTimer {
    start: Instant,
    first_text: Option<Instant>,
    decode: Duration,
    /// `None` once any call has not reported its output tokens.
    output_tokens: Option<u64>,
}

impl TurnTimer {
    pub(super) fn new(start: Instant) -> Self {
        Self {
            start,
            first_text: None,
            decode: Duration::ZERO,
            output_tokens: Some(0),
        }
    }

    /// Fold in one finished LLM call.
    ///
    /// Decode time runs from the call's first chunk, or from its start when
    /// it produced none (non-streaming, or tool calls only), to its end.
    pub(super) fn record_call(
        &mut self,
        call_start: Instant,
        call_end: Instant,
        marks: CallMarks,
        output_tokens: Option<u64>,
    ) {
        let decode_from = marks.first_chunk.unwrap_or(call_start);
        self.decode += call_end.saturating_duration_since(decode_from);
        if self.first_text.is_none() {
            self.first_text = marks.first_text;
        }
        self.output_tokens = match (self.output_tokens, output_tokens) {
            (Some(sum), Some(n)) => Some(sum.saturating_add(n)),
            _ => None,
        };
    }

    /// The turn's timing, with `now` as the end of the turn.
    pub(super) fn finish(&self, now: Instant) -> TurnTiming {
        TurnTiming {
            time_to_first_token_ms: self
                .first_text
                .map(|t| millis(t.saturating_duration_since(self.start))),
            total_ms: millis(now.saturating_duration_since(self.start)),
            output_tokens: self.output_tokens,
            decode_ms: self.output_tokens.map(|_| millis(self.decode)),
        }
    }
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}
