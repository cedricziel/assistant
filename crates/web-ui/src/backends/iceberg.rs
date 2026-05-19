//! Iceberg query backends that scan Parquet files directly from the warehouse.
//!
//! The Iceberg exporter uses an in-memory catalog whose metadata does not
//! survive process restarts.  Rather than standing up a persistent catalog
//! service, these backends walk the warehouse directory, read every `.parquet`
//! file they find for the relevant table, and aggregate the results in memory.
//!
//! This works well for single-user deployments; for high-volume setups a
//! persistent catalog (e.g. Nessie) and pushdown predicates are recommended.

use assistant_core::clock::{Clock, SystemClock};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use arrow_array::cast::AsArray;
use arrow_array::types::{Float64Type, Int32Type, Int64Type, TimestampMicrosecondType};
use arrow_array::{Array, RecordBatch};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde_json::Value;
use uuid::Uuid;

use assistant_core::types::observability::IcebergConfig;
use assistant_storage::{
    LogStats, MetricsSummary, ModelTokenUsage, RecordedLog, RecordedSpan, SkillStatsProvider,
    TimeSeriesPoint, ToolUsageStats, TraceFilter, TraceStats, TraceSummary,
};

use super::{LogBackend, MetricsBackend, TraceBackend};

// -- helpers ------------------------------------------------------------------

/// Resolve the warehouse root, expanding `~` if present.
fn warehouse_path(config: &IcebergConfig) -> PathBuf {
    let raw = config
        .warehouse
        .clone()
        .unwrap_or_else(|| "~/.assistant/iceberg".to_string());
    if let Some(rest) = raw.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(raw)
}

/// Collect every `.parquet` file under `dir` recursively.
fn collect_parquet_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_parquet_files(&path));
            } else if path.extension().and_then(|e| e.to_str()) == Some("parquet") {
                files.push(path);
            }
        }
    }
    // Most-recently modified files first so we can short-circuit if desired.
    files.sort_by(|a, b| {
        let mt = |p: &PathBuf| p.metadata().and_then(|m| m.modified()).ok();
        mt(b).cmp(&mt(a))
    });
    files
}

/// Read all `RecordBatch`es from a single Parquet file.
fn read_parquet(path: &Path) -> Result<Vec<RecordBatch>> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("opening parquet file {}", path.display()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .with_context(|| format!("building parquet reader for {}", path.display()))?;
    let reader = builder.build()?;
    reader
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("reading parquet batches: {e}"))
}

/// Extract a nullable `&str` value at row `i` from a string column named `col`.
fn str_col<'a>(batch: &'a RecordBatch, col: &str, i: usize) -> Option<&'a str> {
    let idx = batch.schema().index_of(col).ok()?;
    let arr = batch.column(idx).as_string_opt::<i32>()?;
    if arr.is_null(i) {
        None
    } else {
        Some(arr.value(i))
    }
}

/// Extract a non-nullable `&str` value at row `i` from a string column.
fn str_col_req<'a>(batch: &'a RecordBatch, col: &str, i: usize) -> &'a str {
    str_col(batch, col, i).unwrap_or("")
}

/// Extract a nullable `i64` microsecond timestamp (UTC) at row `i`.
fn ts_col(batch: &RecordBatch, col: &str, i: usize) -> Option<DateTime<Utc>> {
    let idx = batch.schema().index_of(col).ok()?;
    let arr = batch
        .column(idx)
        .as_primitive_opt::<TimestampMicrosecondType>()?;
    if arr.is_null(i) {
        return None;
    }
    let us = arr.value(i);
    Utc.timestamp_micros(us).single()
}

/// Extract a nullable `i64` at row `i`.
fn i64_col(batch: &RecordBatch, col: &str, i: usize) -> Option<i64> {
    let idx = batch.schema().index_of(col).ok()?;
    let arr = batch.column(idx).as_primitive_opt::<Int64Type>()?;
    if arr.is_null(i) {
        None
    } else {
        Some(arr.value(i))
    }
}

/// Extract a nullable `i32` at row `i`.
fn i32_col(batch: &RecordBatch, col: &str, i: usize) -> Option<i32> {
    let idx = batch.schema().index_of(col).ok()?;
    let arr = batch.column(idx).as_primitive_opt::<Int32Type>()?;
    if arr.is_null(i) {
        None
    } else {
        Some(arr.value(i))
    }
}

/// Extract a nullable `f64` at row `i`.
fn f64_col(batch: &RecordBatch, col: &str, i: usize) -> Option<f64> {
    let idx = batch.schema().index_of(col).ok()?;
    let arr = batch.column(idx).as_primitive_opt::<Float64Type>()?;
    if arr.is_null(i) {
        None
    } else {
        Some(arr.value(i))
    }
}

// -- IcebergTraceBackend ------------------------------------------------------

pub struct IcebergTraceBackend {
    config: IcebergConfig,
}

impl IcebergTraceBackend {
    pub fn new(config: IcebergConfig) -> Self {
        Self { config }
    }

    fn spans_data_dir(&self) -> PathBuf {
        warehouse_path(&self.config)
            .join(&self.config.namespace)
            .join("assistant_spans")
            .join("data")
    }

    fn load_all_batches(&self) -> Vec<RecordBatch> {
        let dir = self.spans_data_dir();
        let files = collect_parquet_files(&dir);
        let mut batches = Vec::new();
        for f in &files {
            match read_parquet(f) {
                Ok(b) => batches.extend(b),
                Err(e) => tracing::warn!("skipping span parquet file {}: {e}", f.display()),
            }
        }
        batches
    }
}

#[async_trait]
impl TraceBackend for IcebergTraceBackend {
    async fn list_recent_traces(
        &self,
        limit: i64,
        filter: &TraceFilter,
        _agent_id: &str,
    ) -> Result<Vec<TraceSummary>> {
        let batches = self.load_all_batches();

        // Per-trace accumulator.
        struct Acc {
            start: DateTime<Utc>,
            end: DateTime<Utc>,
            span_count: i64,
            tool_span_count: i64,
            error_count: i64,
            tool_names: std::collections::HashSet<String>,
            root_name: Option<String>,
            root_service: Option<String>,
            interface: Option<String>,
            has_reply: bool,
            conversation_id: Option<Uuid>,
        }

        let mut map: HashMap<String, Acc> = HashMap::new();

        for batch in &batches {
            for i in 0..batch.num_rows() {
                let trace_id = str_col_req(batch, "trace_id", i).to_string();
                let start = ts_col(batch, "start_time", i).unwrap_or_else(|| SystemClock.now());
                let end = ts_col(batch, "end_time", i).unwrap_or(start);

                // Apply time filters.
                if let Some(since) = filter.since
                    && start < since
                {
                    continue;
                }
                if let Some(until) = filter.until
                    && start > until
                {
                    continue;
                }

                let tool_name = str_col(batch, "tool_name", i).map(str::to_string);
                let tool_status = str_col(batch, "tool_status", i).map(str::to_string);
                let parent_span_id = str_col(batch, "parent_span_id", i);
                let is_root = parent_span_id.is_none();
                let service_name = str_col(batch, "service_name", i).map(str::to_string);
                let span_name = str_col(batch, "name", i).map(str::to_string);

                // Apply skill filter.
                if let Some(ref sk) = filter.skill
                    && tool_name.as_deref() != Some(sk.as_str())
                {
                    continue;
                }

                let attrs_str = str_col(batch, "attributes", i).unwrap_or("{}");
                let attrs: Value = serde_json::from_str(attrs_str).unwrap_or(Value::Null);
                let interface = attrs
                    .get("interface")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);

                // Apply interface filter.
                if let Some(ref iface) = filter.interface
                    && interface.as_deref() != Some(iface.as_str())
                {
                    continue;
                }

                let is_error = tool_status.as_deref() == Some("error");
                let is_reply = matches!(tool_name.as_deref(), Some("reply" | "slack-post"));
                let conversation_id =
                    str_col(batch, "conversation_id", i).and_then(|s| Uuid::parse_str(s).ok());

                let acc = map.entry(trace_id).or_insert_with(|| Acc {
                    start,
                    end,
                    span_count: 0,
                    tool_span_count: 0,
                    error_count: 0,
                    tool_names: std::collections::HashSet::new(),
                    root_name: None,
                    root_service: None,
                    interface: None,
                    has_reply: false,
                    conversation_id: None,
                });

                if start < acc.start {
                    acc.start = start;
                }
                if end > acc.end {
                    acc.end = end;
                }
                acc.span_count += 1;
                if tool_name.is_some() {
                    acc.tool_span_count += 1;
                    if let Some(ref tn) = tool_name {
                        acc.tool_names.insert(tn.clone());
                    }
                }
                if is_error {
                    acc.error_count += 1;
                }
                if is_reply {
                    acc.has_reply = true;
                }
                if is_root {
                    acc.root_name = span_name;
                    acc.root_service = service_name;
                    acc.interface = interface;
                }
                if acc.conversation_id.is_none() {
                    acc.conversation_id = conversation_id;
                }
            }
        }

        let mut summaries: Vec<TraceSummary> = map
            .into_iter()
            .filter_map(|(trace_id, acc)| {
                let duration_ms = (acc.end - acc.start).num_milliseconds();

                // Apply post-fetch filters.
                let status_ok = match filter.status.as_deref() {
                    Some("ok") => acc.error_count == 0,
                    Some("error") => acc.error_count > 0,
                    _ => true,
                };
                let duration_ok = filter.min_duration_ms.is_none_or(|min| duration_ms >= min);
                let conv_ok = filter
                    .conversation
                    .is_none_or(|c| acc.conversation_id == Some(c));

                if !status_ok || !duration_ok || !conv_ok {
                    return None;
                }

                Some(TraceSummary {
                    trace_id,
                    conversation_id: acc.conversation_id,
                    start_time: acc.start,
                    end_time: acc.end,
                    span_count: acc.span_count,
                    tool_span_count: acc.tool_span_count,
                    error_count: acc.error_count,
                    tool_names: acc.tool_names.into_iter().collect(),
                    root_span_name: acc.root_name,
                    root_service_name: acc.root_service,
                    interface: acc.interface,
                    has_reply: acc.has_reply,
                })
            })
            .collect();

        summaries.sort_by_key(|s| std::cmp::Reverse(s.start_time));
        summaries.truncate(limit as usize);
        Ok(summaries)
    }

    async fn get_trace(&self, trace_id: &str, _agent_id: &str) -> Result<Vec<RecordedSpan>> {
        let batches = self.load_all_batches();
        let mut spans = Vec::new();

        for batch in &batches {
            for i in 0..batch.num_rows() {
                if str_col_req(batch, "trace_id", i) != trace_id {
                    continue;
                }

                let start = ts_col(batch, "start_time", i).unwrap_or_else(|| SystemClock.now());
                let end = ts_col(batch, "end_time", i).unwrap_or(start);
                let attrs_str = str_col(batch, "attributes", i).unwrap_or("{}");
                let attributes: Value =
                    serde_json::from_str(attrs_str).unwrap_or(Value::Object(Default::default()));

                spans.push(RecordedSpan {
                    span_id: str_col_req(batch, "span_id", i).to_string(),
                    trace_id: trace_id.to_string(),
                    parent_span_id: str_col(batch, "parent_span_id", i).map(str::to_string),
                    name: str_col_req(batch, "name", i).to_string(),
                    service_name: str_col(batch, "service_name", i).map(str::to_string),
                    conversation_id: str_col(batch, "conversation_id", i)
                        .and_then(|s| Uuid::parse_str(s).ok()),
                    turn: None,
                    tool_name: str_col(batch, "tool_name", i).map(str::to_string),
                    tool_status: str_col(batch, "tool_status", i).map(str::to_string),
                    observation: None,
                    error: None,
                    duration_ms: i64_col(batch, "duration_ms", i).unwrap_or(0),
                    start_time: start,
                    end_time: end,
                    attributes,
                    input_tokens: i64_col(batch, "input_tokens", i),
                    output_tokens: i64_col(batch, "output_tokens", i),
                });
            }
        }

        spans.sort_by_key(|s| s.start_time);
        Ok(spans)
    }
}

// -- IcebergLogBackend --------------------------------------------------------

pub struct IcebergLogBackend {
    config: IcebergConfig,
}

impl IcebergLogBackend {
    pub fn new(config: IcebergConfig) -> Self {
        Self { config }
    }

    fn logs_data_dir(&self) -> PathBuf {
        warehouse_path(&self.config)
            .join(&self.config.namespace)
            .join("assistant_logs")
            .join("data")
    }

    fn load_all_batches(&self) -> Vec<RecordBatch> {
        let dir = self.logs_data_dir();
        let files = collect_parquet_files(&dir);
        let mut batches = Vec::new();
        for f in &files {
            match read_parquet(f) {
                Ok(b) => batches.extend(b),
                Err(e) => tracing::warn!("skipping log parquet file {}: {e}", f.display()),
            }
        }
        batches
    }
}

#[async_trait]
impl LogBackend for IcebergLogBackend {
    #[allow(clippy::too_many_arguments)]
    async fn list_recent_logs(
        &self,
        limit: i64,
        min_severity: Option<i32>,
        target_filter: Option<&str>,
        search: Option<&str>,
        trace_id_filter: Option<&str>,
        _conversation_id: Option<&str>,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
        _agent_id: &str,
    ) -> Result<Vec<RecordedLog>> {
        let batches = self.load_all_batches();
        let mut logs = Vec::new();

        for batch in &batches {
            for i in 0..batch.num_rows() {
                let ts = match ts_col(batch, "timestamp", i) {
                    Some(t) => t,
                    None => continue,
                };

                // Time range filter.
                if let Some(s) = since
                    && ts < s
                {
                    continue;
                }
                if let Some(u) = until
                    && ts > u
                {
                    continue;
                }

                let sev = i32_col(batch, "severity_number", i);
                if let Some(min) = min_severity
                    && sev.unwrap_or(0) < min
                {
                    continue;
                }

                let target = str_col(batch, "target", i);
                if let Some(tf) = target_filter
                    && target != Some(tf)
                {
                    continue;
                }

                let body = str_col(batch, "body", i);
                if let Some(q) = search
                    && !body.unwrap_or("").contains(q)
                {
                    continue;
                }

                let tid = str_col(batch, "trace_id", i);
                if let Some(f) = trace_id_filter
                    && tid != Some(f)
                {
                    continue;
                }

                let attrs_str = str_col(batch, "attributes", i).unwrap_or("{}");
                let attributes: Value =
                    serde_json::from_str(attrs_str).unwrap_or(Value::Object(Default::default()));

                logs.push(RecordedLog {
                    id: str_col_req(batch, "id", i).to_string(),
                    timestamp: ts,
                    observed_timestamp: None,
                    severity_number: sev,
                    severity_text: str_col(batch, "severity_text", i).map(str::to_string),
                    body: body.map(str::to_string),
                    trace_id: tid.map(str::to_string),
                    span_id: str_col(batch, "span_id", i).map(str::to_string),
                    span_name: None, // not stored in Iceberg logs table
                    service_name: str_col(batch, "service_name", i).map(str::to_string),
                    target: target.map(str::to_string),
                    attributes,
                });
            }
        }

        logs.sort_by_key(|l| std::cmp::Reverse(l.timestamp));
        logs.truncate(limit as usize);
        Ok(logs)
    }

    async fn get_log(&self, id: &str, _agent_id: &str) -> Result<Option<RecordedLog>> {
        let batches = self.load_all_batches();

        for batch in &batches {
            for i in 0..batch.num_rows() {
                if str_col_req(batch, "id", i) != id {
                    continue;
                }
                let ts = ts_col(batch, "timestamp", i).unwrap_or_else(|| SystemClock.now());
                let attrs_str = str_col(batch, "attributes", i).unwrap_or("{}");
                let attributes: Value =
                    serde_json::from_str(attrs_str).unwrap_or(Value::Object(Default::default()));

                return Ok(Some(RecordedLog {
                    id: id.to_string(),
                    timestamp: ts,
                    observed_timestamp: None,
                    severity_number: i32_col(batch, "severity_number", i),
                    severity_text: str_col(batch, "severity_text", i).map(str::to_string),
                    body: str_col(batch, "body", i).map(str::to_string),
                    trace_id: str_col(batch, "trace_id", i).map(str::to_string),
                    span_id: str_col(batch, "span_id", i).map(str::to_string),
                    span_name: None,
                    service_name: str_col(batch, "service_name", i).map(str::to_string),
                    target: str_col(batch, "target", i).map(str::to_string),
                    attributes,
                }));
            }
        }
        Ok(None)
    }

    async fn log_stats(&self, _agent_id: &str) -> Result<LogStats> {
        let batches = self.load_all_batches();
        let mut stats = LogStats::default();

        for batch in &batches {
            for i in 0..batch.num_rows() {
                let sev = i32_col(batch, "severity_number", i).unwrap_or(0);
                stats.total += 1;
                match sev {
                    1..=4 => stats.trace_count += 1,
                    5..=8 => stats.debug_count += 1,
                    9..=12 => stats.info_count += 1,
                    13..=16 => stats.warn_count += 1,
                    17..=20 => stats.error_count += 1,
                    _ => stats.fatal_count += 1,
                }
            }
        }
        Ok(stats)
    }

    async fn list_targets(&self, _agent_id: &str) -> Result<Vec<String>> {
        let batches = self.load_all_batches();
        let mut targets: std::collections::HashSet<String> = Default::default();

        for batch in &batches {
            for i in 0..batch.num_rows() {
                if let Some(t) = str_col(batch, "target", i) {
                    targets.insert(t.to_string());
                }
            }
        }

        let mut v: Vec<String> = targets.into_iter().collect();
        v.sort();
        Ok(v)
    }
}

// -- SkillStatsProvider for Iceberg spans ------------------------------------

#[async_trait]
impl SkillStatsProvider for IcebergTraceBackend {
    async fn stats_for_active_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats> {
        let batches = self.load_all_batches();

        // Collect matching rows sorted by start_time desc, limited to `window`.
        struct Row {
            tool_status: Option<String>,
            duration_ms: Option<i64>,
            input_tokens: Option<i64>,
            output_tokens: Option<i64>,
            tool_error: Option<String>,
            start_us: i64,
        }

        let mut rows: Vec<Row> = Vec::new();
        for batch in &batches {
            for i in 0..batch.num_rows() {
                if str_col(batch, "active_skill", i) != Some(skill_name) {
                    continue;
                }
                let start_us = ts_col(batch, "start_time", i)
                    .map(|dt| dt.timestamp_micros())
                    .unwrap_or(0);
                rows.push(Row {
                    tool_status: str_col(batch, "tool_status", i).map(str::to_string),
                    duration_ms: i64_col(batch, "duration_ms", i),
                    input_tokens: i64_col(batch, "input_tokens", i),
                    output_tokens: i64_col(batch, "output_tokens", i),
                    tool_error: {
                        // Extract tool_error from the attributes JSON blob.
                        str_col(batch, "attributes", i).and_then(|json_str| {
                            serde_json::from_str::<Value>(json_str)
                                .ok()
                                .and_then(|v| v.get("tool_error")?.as_str().map(str::to_string))
                        })
                    },
                    start_us,
                });
            }
        }

        // Sort descending by time, take window.
        rows.sort_by_key(|r| std::cmp::Reverse(r.start_us));
        rows.truncate(window as usize);

        let total = rows.len() as i64;
        let mut success_count = 0i64;
        let mut error_count = 0i64;
        let mut total_duration = 0.0f64;
        let mut total_input_tokens = 0i64;
        let mut total_output_tokens = 0i64;
        let mut error_counts: HashMap<String, usize> = HashMap::new();

        for row in &rows {
            if row.tool_status.as_deref() == Some("error") {
                error_count += 1;
                if let Some(ref err) = row.tool_error {
                    *error_counts.entry(err.clone()).or_default() += 1;
                }
            } else {
                success_count += 1;
            }
            total_duration += row.duration_ms.unwrap_or(0) as f64;
            total_input_tokens += row.input_tokens.unwrap_or(0);
            total_output_tokens += row.output_tokens.unwrap_or(0);
        }

        let avg_duration_ms = if total > 0 {
            total_duration / total as f64
        } else {
            0.0
        };

        let mut common_errors: Vec<(String, usize)> = error_counts.into_iter().collect();
        common_errors.sort_by_key(|e| std::cmp::Reverse(e.1));
        let common_errors: Vec<String> =
            common_errors.into_iter().take(5).map(|(e, _)| e).collect();

        Ok(TraceStats {
            skill_name: skill_name.to_string(),
            total,
            success_count,
            error_count,
            avg_duration_ms,
            total_input_tokens,
            total_output_tokens,
            common_errors,
        })
    }
}

// -- IcebergMetricsBackend ----------------------------------------------------

// OTel semantic convention metric names.
const GEN_AI_TOKEN_USAGE: &str = "gen_ai.client.token.usage";
const GEN_AI_OPERATION_DURATION: &str = "gen_ai.client.operation.duration";
const ASSISTANT_TURN_COUNT: &str = "assistant.turn.count";
const ASSISTANT_TOOL_INVOCATIONS: &str = "assistant.tool.invocations";
const ASSISTANT_ERROR_COUNT: &str = "assistant.error.count";
const GEN_AI_TOKEN_TYPE: &str = "gen_ai.token.type";

pub struct IcebergMetricsBackend {
    config: IcebergConfig,
}

impl IcebergMetricsBackend {
    pub fn new(config: IcebergConfig) -> Self {
        Self { config }
    }

    fn metrics_data_dir(&self) -> PathBuf {
        warehouse_path(&self.config)
            .join(&self.config.namespace)
            .join("assistant_metric_points")
            .join("data")
    }

    fn load_all_batches(&self) -> Vec<RecordBatch> {
        let dir = self.metrics_data_dir();
        let files = collect_parquet_files(&dir);
        let mut batches = Vec::new();
        for f in &files {
            match read_parquet(f) {
                Ok(b) => batches.extend(b),
                Err(e) => tracing::warn!("skipping metric parquet file {}: {e}", f.display()),
            }
        }
        batches
    }
}

/// A parsed metric row from Parquet.
struct MetricRow {
    metric_name: String,
    recorded_at: DateTime<Utc>,
    value: Option<f64>,
    count: Option<i64>,
    sum: Option<f64>,
    model: Option<String>,
    attributes: Value,
}

/// Parse all metric rows from batches, filtering by window and agent.
fn parse_metric_rows(batches: &[RecordBatch], window_hours: i64, agent_id: &str) -> Vec<MetricRow> {
    let cutoff = SystemClock.now() - chrono::Duration::hours(window_hours);
    let mut rows = Vec::new();

    for batch in batches {
        for i in 0..batch.num_rows() {
            let recorded_at = match ts_col(batch, "recorded_at", i) {
                Some(t) => t,
                None => continue,
            };
            if recorded_at < cutoff {
                continue;
            }

            // Filter by agent_id when provided (the Parquet schema has a
            // dedicated `agent_id` column written by the Iceberg exporter).
            if !agent_id.is_empty() {
                let row_agent = str_col(batch, "agent_id", i).unwrap_or("");
                if row_agent != agent_id {
                    continue;
                }
            }

            let attrs_str = str_col(batch, "attributes", i).unwrap_or("{}");
            let attributes: Value =
                serde_json::from_str(attrs_str).unwrap_or(Value::Object(Default::default()));

            rows.push(MetricRow {
                metric_name: str_col_req(batch, "metric_name", i).to_string(),
                recorded_at,
                value: f64_col(batch, "value", i),
                count: i64_col(batch, "count", i),
                sum: f64_col(batch, "sum", i),
                model: str_col(batch, "model", i).map(str::to_string),
                attributes,
            });
        }
    }
    rows
}

/// Format a timestamp into a bucket string for a given bucket width in minutes.
///
/// Uses epoch-based rounding so bucket widths > 60 minutes (e.g. 180, 360)
/// produce correct multi-hour buckets.
fn bucket_key(ts: &DateTime<Utc>, bucket_minutes: i64) -> String {
    let bucket_seconds = bucket_minutes.max(1) * 60;
    let bucketed = (ts.timestamp() / bucket_seconds) * bucket_seconds;
    Utc.timestamp_opt(bucketed, 0)
        .single()
        .unwrap_or(*ts)
        .format("%Y-%m-%dT%H:%M:00Z")
        .to_string()
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn warehouse_path_uses_explicit_value_when_provided() {
        let mut cfg = IcebergConfig::default();
        cfg.warehouse = Some("/custom/path".to_string());
        assert_eq!(warehouse_path(&cfg), PathBuf::from("/custom/path"));
    }

    #[test]
    fn warehouse_path_expands_tilde_prefix() {
        let mut cfg = IcebergConfig::default();
        cfg.warehouse = Some("~/warehouse".to_string());
        let p = warehouse_path(&cfg);
        // Should NOT still contain a literal tilde once home is known.
        if let Some(home) = dirs::home_dir() {
            assert_eq!(p, home.join("warehouse"));
        } else {
            // No home → falls back to literal path.
            assert_eq!(p, PathBuf::from("~/warehouse"));
        }
    }

    #[test]
    fn warehouse_path_uses_default_when_unset() {
        let cfg = IcebergConfig::default();
        let p = warehouse_path(&cfg);
        // Default is "~/.assistant/iceberg" — expanded if home exists.
        if let Some(home) = dirs::home_dir() {
            assert_eq!(p, home.join(".assistant/iceberg"));
        } else {
            assert_eq!(p, PathBuf::from("~/.assistant/iceberg"));
        }
    }

    #[test]
    fn bucket_key_rounds_to_60_minute_buckets_by_default() {
        // 2026-05-19T08:37:12Z with bucket_minutes=60 → 2026-05-19T08:00:00Z
        let ts: DateTime<Utc> = "2026-05-19T08:37:12Z".parse().unwrap();
        assert_eq!(bucket_key(&ts, 60), "2026-05-19T08:00:00Z");
    }

    #[test]
    fn bucket_key_rounds_to_15_minute_buckets() {
        let ts: DateTime<Utc> = "2026-05-19T08:47:30Z".parse().unwrap();
        // 8:47 falls in the 8:45-9:00 bucket.
        assert_eq!(bucket_key(&ts, 15), "2026-05-19T08:45:00Z");
    }

    #[test]
    fn bucket_key_handles_zero_and_negative_bucket_minutes_safely() {
        let ts: DateTime<Utc> = "2026-05-19T08:00:30Z".parse().unwrap();
        // .max(1) clamps to 1-minute bucket.
        assert_eq!(bucket_key(&ts, 0), "2026-05-19T08:00:00Z");
        assert_eq!(bucket_key(&ts, -5), "2026-05-19T08:00:00Z");
    }

    #[test]
    fn collect_parquet_files_returns_empty_for_missing_dir() {
        let p = std::path::Path::new("/no/such/dir/anywhere");
        assert!(collect_parquet_files(p).is_empty());
    }

    #[test]
    fn collect_parquet_files_ignores_non_parquet_extensions() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("data.txt"), b"hi").unwrap();
        std::fs::write(tmp.path().join("data.parquet"), b"\x00\x00").unwrap();
        let files = collect_parquet_files(tmp.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("data.parquet"));
    }

    #[test]
    fn collect_parquet_files_recurses_into_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        std::fs::write(tmp.path().join("a/x.parquet"), b"\x00").unwrap();
        std::fs::write(tmp.path().join("a/b/y.parquet"), b"\x00").unwrap();
        std::fs::write(tmp.path().join("a/b/c/z.parquet"), b"\x00").unwrap();
        let files = collect_parquet_files(tmp.path());
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn read_parquet_errors_on_garbage_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.parquet");
        std::fs::write(&path, b"not parquet data").unwrap();
        assert!(read_parquet(&path).is_err());
    }
}

#[async_trait]
impl MetricsBackend for IcebergMetricsBackend {
    async fn summary(&self, window_hours: i64, agent_id: &str) -> Result<MetricsSummary> {
        let batches = self.load_all_batches();
        let rows = parse_metric_rows(&batches, window_hours, agent_id);

        let mut token_in: f64 = 0.0;
        let mut token_out: f64 = 0.0;
        let mut requests: f64 = 0.0;
        let mut tools: f64 = 0.0;
        let mut errors: f64 = 0.0;
        let mut duration_sum: f64 = 0.0;
        let mut duration_count: f64 = 0.0;
        let mut models: std::collections::HashSet<String> = Default::default();

        for row in &rows {
            match row.metric_name.as_str() {
                n if n == GEN_AI_TOKEN_USAGE => {
                    let token_type = row
                        .attributes
                        .get(GEN_AI_TOKEN_TYPE)
                        .and_then(|v| v.as_str());
                    let s = row.sum.unwrap_or(0.0);
                    match token_type {
                        Some("input") => token_in += s,
                        Some("output") => token_out += s,
                        _ => {}
                    }
                    if let Some(ref m) = row.model {
                        models.insert(m.clone());
                    }
                }
                n if n == ASSISTANT_TURN_COUNT => {
                    requests += row.value.unwrap_or(0.0);
                }
                n if n == ASSISTANT_TOOL_INVOCATIONS => {
                    tools += row.value.unwrap_or(0.0);
                }
                n if n == ASSISTANT_ERROR_COUNT => {
                    errors += row.value.unwrap_or(0.0);
                }
                n if n == GEN_AI_OPERATION_DURATION => {
                    duration_sum += row.sum.unwrap_or(0.0);
                    duration_count += row.count.unwrap_or(0) as f64;
                }
                _ => {}
            }
        }

        let avg_duration_s = if duration_count > 0.0 {
            duration_sum / duration_count
        } else {
            0.0
        };

        let mut unique_models: Vec<String> = models.into_iter().collect();
        unique_models.sort();

        Ok(MetricsSummary {
            total_tokens_in: token_in as i64,
            total_tokens_out: token_out as i64,
            total_requests: requests as i64,
            total_tool_invocations: tools as i64,
            avg_duration_s,
            error_count: errors as i64,
            unique_models,
        })
    }

    async fn token_usage_over_time(
        &self,
        window_hours: i64,
        bucket_minutes: i64,
        agent_id: &str,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let batches = self.load_all_batches();
        let rows = parse_metric_rows(&batches, window_hours, agent_id);

        let mut buckets: std::collections::BTreeMap<String, f64> = Default::default();
        for row in &rows {
            if row.metric_name != GEN_AI_TOKEN_USAGE {
                continue;
            }
            let key = bucket_key(&row.recorded_at, bucket_minutes);
            *buckets.entry(key).or_default() += row.sum.unwrap_or(0.0);
        }

        Ok(buckets
            .into_iter()
            .map(|(bucket, value)| TimeSeriesPoint { bucket, value })
            .collect())
    }

    async fn model_comparison(
        &self,
        window_hours: i64,
        agent_id: &str,
    ) -> Result<Vec<ModelTokenUsage>> {
        let batches = self.load_all_batches();
        let rows = parse_metric_rows(&batches, window_hours, agent_id);

        struct Acc {
            input: f64,
            output: f64,
            count: f64,
        }

        let mut map: HashMap<String, Acc> = HashMap::new();
        for row in &rows {
            if row.metric_name != GEN_AI_TOKEN_USAGE {
                continue;
            }
            let model = row.model.clone().unwrap_or_else(|| "unknown".to_string());
            let token_type = row
                .attributes
                .get(GEN_AI_TOKEN_TYPE)
                .and_then(|v| v.as_str());
            let acc = map.entry(model).or_insert(Acc {
                input: 0.0,
                output: 0.0,
                count: 0.0,
            });
            match token_type {
                Some("input") => {
                    acc.input += row.sum.unwrap_or(0.0);
                    // Only count requests from input rows to avoid double-counting
                    // (each LLM call emits both an input and output token row).
                    acc.count += row.count.unwrap_or(0) as f64;
                }
                Some("output") => acc.output += row.sum.unwrap_or(0.0),
                _ => {}
            }
        }

        let mut result: Vec<ModelTokenUsage> = map
            .into_iter()
            .map(|(model, acc)| ModelTokenUsage {
                model,
                input_tokens: acc.input as i64,
                output_tokens: acc.output as i64,
                request_count: acc.count as i64,
                avg_duration_s: 0.0,
            })
            .collect();

        result.sort_by(|a, b| {
            (b.input_tokens + b.output_tokens).cmp(&(a.input_tokens + a.output_tokens))
        });
        Ok(result)
    }

    async fn tool_usage(&self, window_hours: i64, agent_id: &str) -> Result<Vec<ToolUsageStats>> {
        let batches = self.load_all_batches();
        let rows = parse_metric_rows(&batches, window_hours, agent_id);

        let mut map: HashMap<String, f64> = HashMap::new();
        for row in &rows {
            if row.metric_name != ASSISTANT_TOOL_INVOCATIONS {
                continue;
            }
            let tool_name = row
                .attributes
                .get("span.name")
                .or_else(|| row.attributes.get("tool.name"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            *map.entry(tool_name).or_default() += row.value.unwrap_or(0.0);
        }

        let mut result: Vec<ToolUsageStats> = map
            .into_iter()
            .map(|(tool_name, invocations)| ToolUsageStats {
                tool_name,
                invocations: invocations as i64,
                avg_duration_s: 0.0,
            })
            .collect();

        result.sort_by_key(|t| std::cmp::Reverse(t.invocations));
        Ok(result)
    }

    async fn request_rate(
        &self,
        window_hours: i64,
        bucket_minutes: i64,
        agent_id: &str,
    ) -> Result<Vec<TimeSeriesPoint>> {
        let batches = self.load_all_batches();
        let rows = parse_metric_rows(&batches, window_hours, agent_id);

        let mut buckets: std::collections::BTreeMap<String, f64> = Default::default();
        for row in &rows {
            if row.metric_name != ASSISTANT_TURN_COUNT {
                continue;
            }
            let key = bucket_key(&row.recorded_at, bucket_minutes);
            *buckets.entry(key).or_default() += row.value.unwrap_or(0.0);
        }

        Ok(buckets
            .into_iter()
            .map(|(bucket, value)| TimeSeriesPoint { bucket, value })
            .collect())
    }
}
