//! Observability / telemetry configuration.
//!
//! The assistant exports traces, logs and metrics over OTLP only. Which
//! endpoint receives them is configured with the standard `OTEL_*`
//! environment variables, not here. See `docs/opentelemetry.md`.

use serde::{Deserialize, Serialize};

/// Top-level observability configuration.
///
/// Configured via `[observability]` in `config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// When `true`, LLM span events include full message content
    /// (`gen_ai.input.messages`, `gen_ai.output.messages`, etc.).
    /// Off by default because content may contain PII.
    #[serde(default)]
    pub trace_content: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_content_defaults_to_false() {
        let cfg: ObservabilityConfig = toml::from_str("").expect("empty section parses");
        assert!(!cfg.trace_content, "trace_content must default to false");
    }

    #[test]
    fn trace_content_can_be_enabled() {
        let cfg: ObservabilityConfig =
            toml::from_str("trace_content = true").expect("trace_content parses");
        assert!(cfg.trace_content);
    }

    /// The `exporter` key was removed; a config written for it must still load.
    #[test]
    fn leftover_exporter_key_is_ignored() {
        let cfg: ObservabilityConfig =
            toml::from_str("exporter = \"sqlite\"\ntrace_content = true")
                .expect("leftover exporter key must not abort startup");
        assert!(cfg.trace_content);
    }
}
