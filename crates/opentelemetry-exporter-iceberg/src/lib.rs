//! Apache Iceberg OpenTelemetry exporter for spans, logs, and metrics.
//!
//! Writes OTel signals to three Iceberg tables backed by Parquet files:
//!
//! | Table | Signal |
//! |---|---|
//! | `assistant_spans` | Traces / spans |
//! | `assistant_logs` | Log records |
//! | `assistant_metric_points` | Metric data points |
//!
//! # Configuration
//!
//! All configuration is provided via [`IcebergConfig`] from `assistant-core`:
//!
//! ```toml
//! [observability]
//! exporter = "iceberg"        # or "both" to run alongside SQLite
//!
//! [observability.iceberg]
//! warehouse = "~/.assistant/iceberg"
//! namespace = "assistant"
//! partition = "day"           # day | hour | month | year | none
//! # catalog_uri = "http://localhost:8181"
//! ```
//!
//! # Catalog
//!
//! When `catalog_uri` is not set an in-memory catalog backed by the local
//! filesystem `FileIO` is used (catalog metadata is not persisted across
//! restarts, but Parquet data files are). Set `catalog_uri` to the URI of a
//! Nessie, Polaris, or other Iceberg REST catalog for fully persistent
//! metadata.

use anyhow::Result;
use assistant_core::IcebergConfig;

pub mod catalog;
mod log;
mod metric;
mod partition;
mod schema;
mod span;

pub use log::IcebergLogExporter;
pub use metric::IcebergMetricExporter;
pub use span::IcebergSpanExporter;

/// Build all three Iceberg exporters from a single [`IcebergConfig`].
///
/// Returns `(span_exporter, log_exporter, metric_exporter)`.
///
/// This function creates the Iceberg namespace and tables if they do not
/// already exist.  It must be called inside a Tokio runtime.
pub async fn build_exporters(
    config: IcebergConfig,
) -> Result<(
    IcebergSpanExporter,
    IcebergLogExporter,
    IcebergMetricExporter,
)> {
    let span_exp = IcebergSpanExporter::new(config.clone()).await?;
    let log_exp = IcebergLogExporter::new(config.clone()).await?;
    let metric_exp = IcebergMetricExporter::new(config).await?;
    Ok((span_exp, log_exp, metric_exp))
}
