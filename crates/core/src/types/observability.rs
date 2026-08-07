//! Observability / telemetry configuration.
//!
//! The assistant keeps a small, local-only telemetry store (SQLite) to power
//! the built-in trace/log/metric viewers. Everything beyond that — retention,
//! aggregation, dashboards, alerting — belongs to a real observability stack
//! reached over OTLP. See `docs/opentelemetry.md`.

use serde::{Deserialize, Serialize};

/// Which local OTel exporter backend to use for spans, logs, and metrics.
///
/// OTLP export is always additive and is controlled separately via
/// `OTEL_EXPORTER_OTLP_*` environment variables.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OtelExporter {
    /// Write to the local SQLite database (default).
    #[default]
    Sqlite,
    /// Disable the local exporter (OTLP only, or no telemetry at all).
    None,
}

/// Top-level observability configuration.
///
/// Configured via `[observability]` in `config.toml`.
/// The legacy `[self_improvement]` / `[mirror]` section is still accepted for
/// backwards compatibility; an explicit `[observability]` block always takes
/// precedence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Which local exporter backend to activate. Default: `sqlite`.
    #[serde(default)]
    pub exporter: OtelExporter,
    /// When `true`, LLM span events include full message content
    /// (`gen_ai.input.messages`, `gen_ai.output.messages`, etc.).
    /// Off by default because content may contain PII.
    #[serde(default)]
    pub trace_content: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            exporter: OtelExporter::Sqlite,
            trace_content: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_sqlite_exporter() {
        let cfg: ObservabilityConfig = toml::from_str("").expect("empty section parses");
        assert!(
            matches!(cfg.exporter, OtelExporter::Sqlite),
            "default local exporter must be sqlite, got {:?}",
            cfg.exporter
        );
        assert!(!cfg.trace_content, "trace_content must default to false");
    }

    #[test]
    fn accepts_none_exporter() {
        let cfg: ObservabilityConfig =
            toml::from_str(r#"exporter = "none""#).expect(r#"exporter = "none" parses"#);
        assert!(
            matches!(cfg.exporter, OtelExporter::None),
            "expected None, got {:?}",
            cfg.exporter
        );
    }

    /// The Iceberg backend was removed — its config values must be rejected
    /// rather than silently falling back to SQLite.
    #[test]
    fn rejects_removed_iceberg_exporter() {
        for value in ["iceberg", "both"] {
            let parsed: Result<ObservabilityConfig, _> =
                toml::from_str(&format!(r#"exporter = "{value}""#));
            assert!(
                parsed.is_err(),
                "exporter = \"{value}\" must be rejected after the Iceberg removal"
            );
        }
    }

    /// A stray `[observability.iceberg]` table left over in an old config file
    /// must not abort startup.
    #[test]
    fn ignores_leftover_iceberg_table() {
        let cfg: ObservabilityConfig = toml::from_str(
            r#"
exporter = "sqlite"

[iceberg]
warehouse = "/tmp/wh"
namespace = "assistant"
"#,
        )
        .expect("leftover [iceberg] table must be ignored, not fatal");
        assert!(matches!(cfg.exporter, OtelExporter::Sqlite));
    }
}
