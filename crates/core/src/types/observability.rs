//! Observability / telemetry configuration: OTel exporter selection and
//! Iceberg-specific settings.

use serde::{Deserialize, Serialize};

/// Which local OTel exporter backend(s) to use for spans, logs, and metrics.
///
/// OTLP export is always additive and is controlled separately via
/// `OTEL_EXPORTER_OTLP_*` environment variables.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OtelExporter {
    /// Write to the local SQLite database (default — backwards-compatible).
    #[default]
    Sqlite,
    /// Write to Apache Iceberg tables in Parquet format.
    Iceberg,
    /// Write to both SQLite and Iceberg simultaneously.
    Both,
    /// Disable all local exporters (OTLP only, or no local storage).
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
    /// Which local exporter backend(s) to activate. Default: `sqlite`.
    #[serde(default)]
    pub exporter: OtelExporter,
    /// When `true`, LLM span events include full message content
    /// (`gen_ai.input.messages`, `gen_ai.output.messages`, etc.).
    /// Off by default because content may contain PII.
    #[serde(default)]
    pub trace_content: bool,
    /// Iceberg-specific settings.  Used when `exporter` is `"iceberg"` or `"both"`.
    #[serde(default)]
    pub iceberg: IcebergConfig,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            exporter: OtelExporter::Sqlite,
            trace_content: false,
            iceberg: IcebergConfig::default(),
        }
    }
}

/// Iceberg exporter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergConfig {
    /// Warehouse root path where Parquet data files are written.
    /// Default: `~/.assistant/iceberg`
    pub warehouse: Option<String>,
    /// Iceberg namespace for the three tables (`assistant_spans`,
    /// `assistant_logs`, `assistant_metric_points`).
    /// Default: `"assistant"`
    #[serde(default = "IcebergConfig::default_namespace")]
    pub namespace: String,
    /// Time-based partition granularity applied to all three tables.
    /// Default: `day`.
    #[serde(default)]
    pub partition: PartitionGranularity,
    /// REST catalog URI (e.g. `http://localhost:8181` for a local Nessie or
    /// Polaris instance).  When absent, an in-memory catalog backed by the
    /// filesystem `FileIO` is used instead.
    pub catalog_uri: Option<String>,
}

impl IcebergConfig {
    fn default_namespace() -> String {
        "assistant".to_string()
    }
}

impl Default for IcebergConfig {
    fn default() -> Self {
        Self {
            warehouse: None,
            namespace: Self::default_namespace(),
            partition: PartitionGranularity::default(),
            catalog_uri: None,
        }
    }
}

/// Time-based partition granularity for Iceberg tables.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartitionGranularity {
    /// No partitioning.
    None,
    /// Partition by year.
    Year,
    /// Partition by month.
    Month,
    /// Partition by day (default).
    #[default]
    Day,
    /// Partition by hour.
    Hour,
}
