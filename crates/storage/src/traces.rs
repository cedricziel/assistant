//! Distributed trace storage backed by OpenTelemetry spans persisted in SQLite.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Summary statistics about a skill's past executions.
#[derive(Debug, Clone)]
pub struct TraceStats {
    pub skill_name: String,
    pub total: i64,
    pub success_count: i64,
    pub error_count: i64,
    pub avg_duration_ms: f64,
    /// Total prompt/input tokens consumed in the analysis window.
    pub total_input_tokens: i64,
    /// Total completion/output tokens consumed in the analysis window.
    pub total_output_tokens: i64,
    /// Up to 5 most-frequent error messages observed in the window.
    pub common_errors: Vec<String>,
}

/// Aggregated metadata describing a single distributed trace/tree.
#[derive(Debug, Clone)]
pub struct TraceSummary {
    pub trace_id: String,
    pub conversation_id: Option<Uuid>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub span_count: i64,
    pub tool_span_count: i64,
    pub error_count: i64,
    pub tool_names: Vec<String>,
    pub root_span_name: Option<String>,
    pub root_service_name: Option<String>,
    /// Originating interface extracted from root span attributes (e.g. `"Slack"`, `"Cli"`).
    pub interface: Option<String>,
    /// Whether a reply/slack-post tool was called during this trace.
    pub has_reply: bool,
}

/// Filter parameters for [`TraceStore::list_recent_traces_for_agent`].
///
/// All fields are optional; `None` means "no restriction on this dimension".
#[derive(Debug, Default)]
pub struct TraceFilter {
    /// Only include traces containing this tool name.
    pub skill: Option<String>,
    /// `"ok"` or `"error"` — applied in the caller after fetching.
    pub status: Option<String>,
    /// Restrict to a specific conversation UUID — applied in the caller after fetching.
    pub conversation: Option<Uuid>,
    /// Minimum trace duration in milliseconds — applied in the caller after fetching.
    pub min_duration_ms: Option<i64>,
    /// Filter by originating interface attribute.
    pub interface: Option<String>,
    /// Earliest span start time to include.
    pub since: Option<DateTime<Utc>>,
    /// Latest span start time to include.
    pub until: Option<DateTime<Utc>>,
}

/// A persisted OpenTelemetry span row enriched with tool metadata.
#[derive(Debug, Clone)]
pub struct RecordedSpan {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub service_name: Option<String>,
    pub conversation_id: Option<Uuid>,
    pub turn: Option<i64>,
    pub tool_name: Option<String>,
    pub tool_status: Option<String>,
    pub observation: Option<String>,
    pub error: Option<String>,
    pub duration_ms: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub attributes: Value,
    /// Prompt/input token count (from `gen_ai.usage.input_tokens`).
    pub input_tokens: Option<i64>,
    /// Completion/output token count (from `gen_ai.usage.output_tokens`).
    pub output_tokens: Option<i64>,
}

/// SQLite-backed store for execution traces.
pub struct TraceStore {
    pool: SqlitePool,
}

impl TraceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Return the `limit` most-recent traces for the given skill name.
    pub async fn get_recent_for_skill(
        &self,
        skill_name: &str,
        limit: i64,
    ) -> Result<Vec<RecordedSpan>> {
        let rows = sqlx::query(
            "SELECT span_id, trace_id, parent_span_id, name, conversation_id, turn, \
                    service_name, \
                    tool_name, tool_status, tool_observation, tool_error, duration_ms, \
                    start_time, end_time, attributes, input_tokens, output_tokens \
             FROM distributed_traces \
             WHERE tool_name = ?1 \
             ORDER BY start_time DESC \
             LIMIT ?2",
        )
        .bind(skill_name)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_span).collect()
    }

    /// Return the `limit` most-recent traces across all skills.
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<RecordedSpan>> {
        let rows = sqlx::query(
            "SELECT span_id, trace_id, parent_span_id, name, conversation_id, turn, \
                    service_name, \
                    tool_name, tool_status, tool_observation, tool_error, duration_ms, \
                    start_time, end_time, attributes, input_tokens, output_tokens \
             FROM distributed_traces \
             WHERE tool_name IS NOT NULL \
             ORDER BY start_time DESC \
             LIMIT ?1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_span).collect()
    }

    /// Return metadata for the newest distributed traces, optionally filtered to
    /// those that include a particular tool span.
    pub async fn list_recent_traces(
        &self,
        limit: i64,
        skill_filter: Option<&str>,
    ) -> Result<Vec<TraceSummary>> {
        let rows = sqlx::query(
            "SELECT \
                trace_id, \
                MAX(conversation_id) AS conversation_id, \
                MIN(start_time) AS trace_start, \
                MAX(end_time) AS trace_end, \
                COUNT(*) AS span_count, \
                SUM(CASE WHEN tool_name IS NOT NULL THEN 1 ELSE 0 END) AS tool_span_count, \
                SUM(CASE WHEN tool_status = 'error' THEN 1 ELSE 0 END) AS error_count, \
                GROUP_CONCAT(DISTINCT CASE WHEN tool_name IS NULL THEN '' ELSE tool_name END) AS tool_names, \
                MAX(CASE WHEN parent_span_id IS NULL THEN name ELSE NULL END) AS root_span_name, \
                MAX(CASE WHEN parent_span_id IS NULL THEN service_name ELSE NULL END) AS root_service_name \
            FROM distributed_traces \
            GROUP BY trace_id \
            HAVING (?1 IS NULL) OR SUM(CASE WHEN tool_name = ?1 THEN 1 ELSE 0 END) > 0 \
            ORDER BY trace_start DESC \
            LIMIT ?2",
        )
        .bind(skill_filter)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let conv_raw: Option<String> = row.try_get("conversation_id").ok().flatten();
                let conversation_id = match conv_raw {
                    Some(ref raw) if !raw.is_empty() => Some(Uuid::parse_str(raw)?),
                    _ => None,
                };
                let start_time: DateTime<Utc> = row.get("trace_start");
                let end_time: DateTime<Utc> = row.get("trace_end");
                let span_count: i64 = row.get("span_count");
                let tool_span_count: i64 = row.get("tool_span_count");
                let error_count: i64 = row.get("error_count");
                let root_span_name = row
                    .try_get::<Option<String>, _>("root_span_name")
                    .ok()
                    .flatten();
                let root_service_name = row
                    .try_get::<Option<String>, _>("root_service_name")
                    .ok()
                    .flatten();
                let tool_concat = row
                    .try_get::<Option<String>, _>("tool_names")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                let tool_names = tool_concat
                    .split(',')
                    .filter_map(|name| {
                        let trimmed = name.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    })
                    .collect();
                Ok(TraceSummary {
                    trace_id: row.get("trace_id"),
                    conversation_id,
                    start_time,
                    end_time,
                    span_count,
                    tool_span_count,
                    error_count,
                    tool_names,
                    root_span_name,
                    root_service_name,
                    interface: None,
                    has_reply: false,
                })
            })
            .collect()
    }

    /// Return metadata for recent traces scoped to a specific assistant agent.
    pub async fn list_recent_traces_for_agent(
        &self,
        limit: i64,
        filter: &TraceFilter,
        agent_id: &str,
    ) -> Result<Vec<TraceSummary>> {
        let rows = sqlx::query(
            "SELECT \
                dt.trace_id AS trace_id, \
                MAX(dt.conversation_id) AS conversation_id, \
                MIN(dt.start_time) AS trace_start, \
                MAX(dt.end_time) AS trace_end, \
                COUNT(*) AS span_count, \
                SUM(CASE WHEN dt.tool_name IS NOT NULL THEN 1 ELSE 0 END) AS tool_span_count, \
                SUM(CASE WHEN dt.tool_status = 'error' THEN 1 ELSE 0 END) AS error_count, \
                GROUP_CONCAT(DISTINCT CASE WHEN dt.tool_name IS NULL THEN '' ELSE dt.tool_name END) AS tool_names, \
                MAX(CASE WHEN dt.parent_span_id IS NULL THEN dt.name ELSE NULL END) AS root_span_name, \
                MAX(CASE WHEN dt.parent_span_id IS NULL THEN dt.service_name ELSE NULL END) AS root_service_name, \
                MAX(CASE WHEN dt.parent_span_id IS NULL THEN json_extract(dt.attributes, '$.interface') ELSE NULL END) AS interface, \
                SUM(CASE WHEN dt.tool_name IN ('reply', 'slack-post') THEN 1 ELSE 0 END) > 0 AS has_reply \
             FROM distributed_traces dt \
             INNER JOIN conversations c ON c.id = dt.conversation_id \
             WHERE c.agent_id = ?1 \
             GROUP BY dt.trace_id \
             HAVING ((?2 IS NULL) OR SUM(CASE WHEN dt.tool_name = ?2 THEN 1 ELSE 0 END) > 0) \
               AND (?4 IS NULL OR MIN(CASE WHEN dt.parent_span_id IS NULL THEN dt.start_time ELSE NULL END) >= ?4) \
               AND (?5 IS NULL OR MAX(CASE WHEN dt.parent_span_id IS NULL THEN dt.start_time ELSE NULL END) <= ?5) \
               AND (?6 IS NULL OR MAX(CASE WHEN dt.parent_span_id IS NULL THEN json_extract(dt.attributes, '$.interface') ELSE NULL END) = ?6) \
               AND (?7 IS NULL OR MAX(dt.conversation_id) = ?7) \
               AND (?8 IS NULL OR (?8 = 'error' AND SUM(CASE WHEN dt.tool_status = 'error' THEN 1 ELSE 0 END) > 0) OR (?8 IN ('ok', 'success') AND SUM(CASE WHEN dt.tool_status = 'error' THEN 1 ELSE 0 END) = 0)) \
             ORDER BY trace_start DESC \
             LIMIT ?3",
        )
        .bind(agent_id)
        .bind(filter.skill.as_deref())
        .bind(limit)
        .bind(filter.since)
        .bind(filter.until)
        .bind(filter.interface.as_deref())
        .bind(filter.conversation.as_ref().map(|u| u.to_string()))
        .bind(filter.status.as_deref())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let conv_raw: Option<String> = row.try_get("conversation_id").ok().flatten();
                let conversation_id = match conv_raw {
                    Some(ref raw) if !raw.is_empty() => Some(Uuid::parse_str(raw)?),
                    _ => None,
                };
                let start_time: DateTime<Utc> = row.get("trace_start");
                let end_time: DateTime<Utc> = row.get("trace_end");
                let span_count: i64 = row.get("span_count");
                let tool_span_count: i64 = row.get("tool_span_count");
                let error_count: i64 = row.get("error_count");
                let root_span_name = row
                    .try_get::<Option<String>, _>("root_span_name")
                    .ok()
                    .flatten();
                let root_service_name = row
                    .try_get::<Option<String>, _>("root_service_name")
                    .ok()
                    .flatten();
                let tool_concat = row
                    .try_get::<Option<String>, _>("tool_names")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                let tool_names = tool_concat
                    .split(',')
                    .filter_map(|name| {
                        let trimmed = name.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    })
                    .collect();
                let interface = row.try_get::<Option<String>, _>("interface").ok().flatten();
                let has_reply: bool = row.try_get::<i64, _>("has_reply").unwrap_or(0) != 0;
                Ok(TraceSummary {
                    trace_id: row.get("trace_id"),
                    conversation_id,
                    start_time,
                    end_time,
                    span_count,
                    tool_span_count,
                    error_count,
                    tool_names,
                    root_span_name,
                    root_service_name,
                    interface,
                    has_reply,
                })
            })
            .collect()
    }

    /// Fetch every span belonging to a trace ordered by start time so the UI can
    /// render the full hierarchy/timeline.
    pub async fn get_trace(&self, trace_id: &str) -> Result<Vec<RecordedSpan>> {
        let rows = sqlx::query(
            "SELECT span_id, trace_id, parent_span_id, name, conversation_id, turn, \
                    service_name, \
                    tool_name, tool_status, tool_observation, tool_error, duration_ms, \
                    start_time, end_time, attributes, input_tokens, output_tokens \
             FROM distributed_traces \
             WHERE trace_id = ?1 \
             ORDER BY start_time ASC",
        )
        .bind(trace_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_span).collect()
    }

    /// Fetch every span for a trace, constrained to one assistant agent.
    pub async fn get_trace_for_agent(
        &self,
        trace_id: &str,
        agent_id: &str,
    ) -> Result<Vec<RecordedSpan>> {
        let rows = sqlx::query(
            "SELECT dt.span_id, dt.trace_id, dt.parent_span_id, dt.name, dt.conversation_id, dt.turn, \
                    dt.service_name, \
                    dt.tool_name, dt.tool_status, dt.tool_observation, dt.tool_error, dt.duration_ms, \
                    dt.start_time, dt.end_time, dt.attributes, dt.input_tokens, dt.output_tokens \
             FROM distributed_traces dt \
             INNER JOIN conversations c ON c.id = dt.conversation_id \
             WHERE dt.trace_id = ?1 AND c.agent_id = ?2 \
             ORDER BY dt.start_time ASC",
        )
        .bind(trace_id)
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(Self::row_to_span).collect()
    }

    /// List distinct skill names that have recorded traces.
    pub async fn list_skills(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            "SELECT DISTINCT tool_name \
             FROM distributed_traces \
             WHERE tool_name IS NOT NULL \
             ORDER BY tool_name",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|r| r.try_get::<Option<String>, _>("tool_name").ok().flatten())
            .collect())
    }

    /// Compute aggregate statistics over the most-recent `window` traces for a skill.
    pub async fn stats_for_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats> {
        // Aggregate over the newest `window` rows for this skill.
        let agg_row = sqlx::query(
            "WITH recent AS ( \
                SELECT tool_status, duration_ms, input_tokens, output_tokens \
                FROM distributed_traces \
                WHERE tool_name = ?1 \
                ORDER BY start_time DESC \
                LIMIT ?2 \
            ) \
            SELECT \
                COUNT(*)                                           AS total, \
                SUM(CASE WHEN tool_status = 'error' THEN 0 ELSE 1 END) AS success_count, \
                SUM(CASE WHEN tool_status = 'error' THEN 1 ELSE 0 END) AS error_count, \
                COALESCE(AVG(CAST(duration_ms AS REAL)), 0.0)      AS avg_duration_ms, \
                COALESCE(SUM(input_tokens), 0)                     AS total_input_tokens, \
                COALESCE(SUM(output_tokens), 0)                    AS total_output_tokens \
            FROM recent",
        )
        .bind(skill_name)
        .bind(window)
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = agg_row.try_get("total").unwrap_or(0);
        let success_count: i64 = agg_row.try_get("success_count").unwrap_or(0);
        let error_count: i64 = agg_row.try_get("error_count").unwrap_or(0);
        let avg_duration_ms: f64 = agg_row.try_get("avg_duration_ms").unwrap_or(0.0);
        let total_input_tokens: i64 = agg_row.try_get("total_input_tokens").unwrap_or(0);
        let total_output_tokens: i64 = agg_row.try_get("total_output_tokens").unwrap_or(0);

        // Collect the most common error strings (up to 5).
        let err_rows = sqlx::query(
            "WITH recent AS ( \
                SELECT tool_error \
                FROM distributed_traces \
                WHERE tool_name = ?1 \
                  AND tool_error IS NOT NULL \
                ORDER BY start_time DESC \
                LIMIT ?2 \
            ) \
            SELECT tool_error AS error \
            FROM recent \
            GROUP BY tool_error \
            ORDER BY COUNT(*) DESC \
            LIMIT 5",
        )
        .bind(skill_name)
        .bind(window)
        .fetch_all(&self.pool)
        .await?;

        let common_errors: Vec<String> = err_rows
            .into_iter()
            .filter_map(|r| r.try_get::<Option<String>, _>("error").ok().flatten())
            .collect();

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

    fn row_to_span(row: SqliteRow) -> Result<RecordedSpan> {
        let conv_raw: Option<String> = row.try_get("conversation_id").ok().flatten();
        let conversation_id = match conv_raw {
            Some(ref raw) if !raw.is_empty() => Some(Uuid::parse_str(raw)?),
            _ => None,
        };

        let attrs_str: String = row.get("attributes");
        let attributes: Value = serde_json::from_str(&attrs_str)?;

        Ok(RecordedSpan {
            span_id: row.get("span_id"),
            trace_id: row.get("trace_id"),
            parent_span_id: row
                .try_get::<Option<String>, _>("parent_span_id")
                .ok()
                .flatten(),
            name: row.get("name"),
            service_name: row
                .try_get::<Option<String>, _>("service_name")
                .ok()
                .flatten(),
            conversation_id,
            turn: row.try_get::<Option<i64>, _>("turn").ok().flatten(),
            tool_name: row.try_get::<Option<String>, _>("tool_name").ok().flatten(),
            tool_status: row
                .try_get::<Option<String>, _>("tool_status")
                .ok()
                .flatten(),
            observation: row
                .try_get::<Option<String>, _>("tool_observation")
                .ok()
                .flatten(),
            error: row
                .try_get::<Option<String>, _>("tool_error")
                .ok()
                .flatten(),
            duration_ms: row.get("duration_ms"),
            start_time: row.get("start_time"),
            end_time: row.get("end_time"),
            attributes,
            input_tokens: row.try_get::<Option<i64>, _>("input_tokens").ok().flatten(),
            output_tokens: row
                .try_get::<Option<i64>, _>("output_tokens")
                .ok()
                .flatten(),
        })
    }
}

/// Trait for querying skill-scoped execution statistics from trace storage.
///
/// Implementations exist for both SQLite (`TraceStore`) and Iceberg backends.
#[async_trait::async_trait]
pub trait SkillStatsProvider: Send + Sync {
    /// Return aggregate stats for spans tagged with `active_skill = skill_name`
    /// over the most recent `window` executions.
    async fn stats_for_active_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats>;
}

#[async_trait::async_trait]
impl SkillStatsProvider for TraceStore {
    async fn stats_for_active_skill(&self, skill_name: &str, window: i64) -> Result<TraceStats> {
        let agg_row = sqlx::query(
            "WITH recent AS ( \
                SELECT tool_status, duration_ms, input_tokens, output_tokens \
                FROM distributed_traces \
                WHERE active_skill = ?1 \
                ORDER BY start_time DESC \
                LIMIT ?2 \
            ) \
            SELECT \
                COUNT(*)                                           AS total, \
                SUM(CASE WHEN tool_status = 'error' THEN 0 ELSE 1 END) AS success_count, \
                SUM(CASE WHEN tool_status = 'error' THEN 1 ELSE 0 END) AS error_count, \
                COALESCE(AVG(CAST(duration_ms AS REAL)), 0.0)      AS avg_duration_ms, \
                COALESCE(SUM(input_tokens), 0)                     AS total_input_tokens, \
                COALESCE(SUM(output_tokens), 0)                    AS total_output_tokens \
            FROM recent",
        )
        .bind(skill_name)
        .bind(window)
        .fetch_one(&self.pool)
        .await?;

        let total: i64 = agg_row.try_get("total").unwrap_or(0);
        let success_count: i64 = agg_row.try_get("success_count").unwrap_or(0);
        let error_count: i64 = agg_row.try_get("error_count").unwrap_or(0);
        let avg_duration_ms: f64 = agg_row.try_get("avg_duration_ms").unwrap_or(0.0);
        let total_input_tokens: i64 = agg_row.try_get("total_input_tokens").unwrap_or(0);
        let total_output_tokens: i64 = agg_row.try_get("total_output_tokens").unwrap_or(0);

        let err_rows = sqlx::query(
            "WITH recent AS ( \
                SELECT tool_error \
                FROM distributed_traces \
                WHERE active_skill = ?1 \
                  AND tool_error IS NOT NULL \
                ORDER BY start_time DESC \
                LIMIT ?2 \
            ) \
            SELECT tool_error AS error \
            FROM recent \
            GROUP BY tool_error \
            ORDER BY COUNT(*) DESC \
            LIMIT 5",
        )
        .bind(skill_name)
        .bind(window)
        .fetch_all(&self.pool)
        .await?;

        let common_errors: Vec<String> = err_rows
            .into_iter()
            .filter_map(|r| r.try_get::<Option<String>, _>("error").ok().flatten())
            .collect();

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageLayer;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_insert_and_query() {
        let storage = StorageLayer::new_in_memory().await.unwrap();

        // Insert a conversation row first to satisfy FK
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        insert_span(
            &storage.pool,
            conv_id,
            "web-fetch",
            "ok",
            Some("200 OK"),
            None,
            120,
        )
        .await;

        let recent = store.get_recent_for_skill("web-fetch", 10).await.unwrap();
        assert_eq!(recent.len(), 1);
        let span = &recent[0];
        assert_eq!(span.tool_name.as_deref(), Some("web-fetch"));
        assert_eq!(span.observation.as_deref(), Some("200 OK"));
        assert_eq!(span.duration_ms, 120);
    }

    #[tokio::test]
    async fn test_stats() {
        let storage = StorageLayer::new_in_memory().await.unwrap();

        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        insert_span(&storage.pool, conv_id, "bash", "ok", Some("ok"), None, 100).await;
        insert_span(&storage.pool, conv_id, "bash", "ok", Some("ok"), None, 100).await;
        insert_span(
            &storage.pool,
            conv_id,
            "bash",
            "error",
            None,
            Some("permission denied"),
            50,
        )
        .await;

        let stats = store.stats_for_skill("bash", 100).await.unwrap();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);
        assert!(!stats.common_errors.is_empty());
    }

    #[tokio::test]
    async fn test_trace_summaries_and_filters() {
        let storage = StorageLayer::new_in_memory().await.unwrap();

        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("analysis")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        insert_span(&storage.pool, conv_id, "bash", "ok", Some("ok"), None, 100).await;
        insert_span(
            &storage.pool,
            conv_id,
            "search",
            "error",
            None,
            Some("boom"),
            80,
        )
        .await;

        let all = store.list_recent_traces(10, None).await.unwrap();
        assert_eq!(all.len(), 2);

        let bash_only = store.list_recent_traces(10, Some("bash")).await.unwrap();
        assert_eq!(bash_only.len(), 1);
        assert_eq!(bash_only[0].tool_names, vec!["bash".to_string()]);

        let search_only = store.list_recent_traces(10, Some("search")).await.unwrap();
        assert_eq!(search_only.len(), 1);
        assert_eq!(search_only[0].error_count, 1);
    }

    #[tokio::test]
    async fn test_get_trace_retrieves_hierarchy() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.trace_store();

        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("trace")
            .execute(&storage.pool)
            .await
            .unwrap();

        let trace_id = Uuid::new_v4().to_string();
        let parent_id = Uuid::new_v4().to_string();
        let child_id = Uuid::new_v4().to_string();

        insert_custom_span(
            &storage.pool,
            &trace_id,
            &parent_id,
            None,
            conv_id,
            "root",
            "ok",
            None,
            None,
            42,
        )
        .await;
        insert_custom_span(
            &storage.pool,
            &trace_id,
            &child_id,
            Some(&parent_id),
            conv_id,
            "child",
            "error",
            Some("boom"),
            Some("fail"),
            10,
        )
        .await;

        let spans = store.get_trace(&trace_id).await.unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].span_id, parent_id);
        assert_eq!(spans[1].parent_span_id.as_deref(), Some(parent_id.as_str()));
    }

    async fn insert_span(
        pool: &SqlitePool,
        conversation_id: Uuid,
        tool_name: &str,
        status: &str,
        observation: Option<&str>,
        error: Option<&str>,
        duration_ms: i64,
    ) {
        let span_id = Uuid::new_v4().to_string();
        let trace_id = Uuid::new_v4().to_string();
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(duration_ms.max(0));
        let attrs = json!({
            "conversation_id": conversation_id.to_string(),
            "tool_name": tool_name,
            "tool_status": status,
        });

        sqlx::query(
            "INSERT INTO distributed_traces \
                (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                 tool_status, tool_observation, tool_error, duration_ms, start_time, end_time, attributes) \
             VALUES (?1, ?2, NULL, 'tool_execution', ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        )
        .bind(span_id)
        .bind(trace_id)
        .bind(conversation_id.to_string())
        .bind(tool_name)
        .bind(status)
        .bind(observation)
        .bind(error)
        .bind(duration_ms)
        .bind(start)
        .bind(end)
        .bind(attrs.to_string())
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_custom_span(
        pool: &SqlitePool,
        trace_id: &str,
        span_id: &str,
        parent_span_id: Option<&str>,
        conversation_id: Uuid,
        tool_name: &str,
        status: &str,
        observation: Option<&str>,
        error: Option<&str>,
        duration_ms: i64,
    ) {
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(duration_ms.max(0));
        let attrs = json!({
            "conversation_id": conversation_id.to_string(),
            "tool_name": tool_name,
            "tool_status": status,
        });

        sqlx::query(
            "INSERT INTO distributed_traces \
                (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                 tool_status, tool_observation, tool_error, duration_ms, start_time, end_time, attributes) \
             VALUES (?1, ?2, ?3, 'tool_execution', ?4, 0, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"
        )
        .bind(span_id)
        .bind(trace_id)
        .bind(parent_span_id)
        .bind(conversation_id.to_string())
        .bind(tool_name)
        .bind(status)
        .bind(observation)
        .bind(error)
        .bind(duration_ms)
        .bind(start)
        .bind(end)
        .bind(attrs.to_string())
        .execute(pool)
        .await
        .unwrap();
    }

    /// Insert a root span (no parent) with a custom attributes JSON blob.
    async fn insert_root_span_with_attrs(
        pool: &SqlitePool,
        conversation_id: Uuid,
        tool_name: &str,
        attrs: serde_json::Value,
    ) {
        let span_id = Uuid::new_v4().to_string();
        let trace_id = Uuid::new_v4().to_string();
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(10);

        sqlx::query(
            "INSERT INTO distributed_traces \
                (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                 tool_status, tool_observation, tool_error, duration_ms, start_time, end_time, attributes) \
             VALUES (?1, ?2, NULL, 'turn', ?3, 0, ?4, 'ok', NULL, NULL, 10, ?5, ?6, ?7)",
        )
        .bind(span_id)
        .bind(trace_id)
        .bind(conversation_id.to_string())
        .bind(tool_name)
        .bind(start)
        .bind(end)
        .bind(attrs.to_string())
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_has_reply_true_when_reply_tool_called() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("reply-test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        // Insert a span with tool_name = 'reply'
        insert_span(
            &storage.pool,
            conv_id,
            "reply",
            "ok",
            Some("sent"),
            None,
            50,
        )
        .await;

        let filter = TraceFilter::default();
        let summaries = store
            .list_recent_traces_for_agent(10, &filter, "default")
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1, "expected one trace summary");
        assert!(
            summaries[0].has_reply,
            "has_reply should be true when reply tool was called"
        );
    }

    #[tokio::test]
    async fn test_has_reply_false_when_no_reply_tool() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("no-reply-test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        // Insert only a non-reply tool span
        insert_span(
            &storage.pool,
            conv_id,
            "list-tasks",
            "ok",
            Some("[]"),
            None,
            80,
        )
        .await;

        let filter = TraceFilter::default();
        let summaries = store
            .list_recent_traces_for_agent(10, &filter, "default")
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1, "expected one trace summary");
        assert!(
            !summaries[0].has_reply,
            "has_reply should be false when no reply tool was called"
        );
    }

    #[tokio::test]
    async fn test_interface_extracted_from_root_span_attributes() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("iface-test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        // Insert a root span (parent_span_id IS NULL) with interface in attributes
        let attrs =
            serde_json::json!({ "interface": "Slack", "conversation_id": conv_id.to_string() });
        insert_root_span_with_attrs(&storage.pool, conv_id, "turn", attrs).await;

        let filter = TraceFilter::default();
        let summaries = store
            .list_recent_traces_for_agent(10, &filter, "default")
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1, "expected one trace summary");
        assert_eq!(
            summaries[0].interface.as_deref(),
            Some("Slack"),
            "interface should be extracted from root span JSON attributes"
        );
    }

    /// A trace with a root span (interface=Slack) plus two child tool spans must
    /// report span_count=3 after an interface filter — proving the HAVING-clause
    /// fix does not pre-filter child spans before aggregation.
    #[tokio::test]
    async fn test_interface_filter_preserves_span_count() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("span-count-test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        // Build a single trace: root span + 2 child tool spans
        let trace_id = Uuid::new_v4().to_string();
        let root_id = Uuid::new_v4().to_string();
        let child1_id = Uuid::new_v4().to_string();
        let child2_id = Uuid::new_v4().to_string();

        // Root span — carries the interface attribute
        sqlx::query(
            "INSERT INTO distributed_traces \
                (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                 tool_status, tool_observation, tool_error, duration_ms, start_time, end_time, attributes) \
             VALUES (?1, ?2, NULL, 'turn', ?3, 0, NULL, 'ok', NULL, NULL, 100, ?4, ?5, ?6)",
        )
        .bind(&root_id)
        .bind(&trace_id)
        .bind(conv_id.to_string())
        .bind(Utc::now())
        .bind(Utc::now() + chrono::Duration::milliseconds(100))
        .bind(json!({"interface": "Slack"}).to_string())
        .execute(&storage.pool)
        .await
        .unwrap();

        // Two child tool spans — no interface attribute
        for cid in [&child1_id, &child2_id] {
            sqlx::query(
                "INSERT INTO distributed_traces \
                    (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                     tool_status, tool_observation, tool_error, duration_ms, start_time, end_time, attributes) \
                 VALUES (?1, ?2, ?3, 'tool', ?4, 0, 'list-tasks', 'ok', '[]', NULL, 20, ?5, ?6, ?7)",
            )
            .bind(cid)
            .bind(&trace_id)
            .bind(&root_id)
            .bind(conv_id.to_string())
            .bind(Utc::now())
            .bind(Utc::now() + chrono::Duration::milliseconds(20))
            .bind("{}")
            .execute(&storage.pool)
            .await
            .unwrap();
        }

        let filter = TraceFilter {
            interface: Some("Slack".to_string()),
            ..TraceFilter::default()
        };
        let results = store
            .list_recent_traces_for_agent(10, &filter, "default")
            .await
            .unwrap();
        assert_eq!(results.len(), 1, "should return the Slack trace");
        assert_eq!(
            results[0].span_count, 3,
            "span_count must include root + both child spans, not just the root"
        );
    }

    /// Conversation filter applied via SQL should scope traces to the given
    /// conversation UUID and not rely on a post-query retain().
    #[tokio::test]
    async fn test_conversation_filter_scopes_traces() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.trace_store();

        // Two separate conversations
        let conv_a = Uuid::new_v4();
        let conv_b = Uuid::new_v4();
        for (cid, title) in [(&conv_a, "conv-a"), (&conv_b, "conv-b")] {
            sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
                .bind(cid.to_string())
                .bind(title)
                .execute(&storage.pool)
                .await
                .unwrap();
        }

        // One trace per conversation
        insert_span(&storage.pool, conv_a, "bash", "ok", Some("ok"), None, 50).await;
        insert_span(
            &storage.pool,
            conv_b,
            "web-fetch",
            "ok",
            Some("200"),
            None,
            30,
        )
        .await;

        // Filter by conv_a
        let filter = TraceFilter {
            conversation: Some(conv_a),
            ..TraceFilter::default()
        };
        let results = store
            .list_recent_traces_for_agent(10, &filter, "default")
            .await
            .unwrap();
        assert_eq!(results.len(), 1, "should return only conv_a traces");
        assert_eq!(
            results[0].conversation_id,
            Some(conv_a),
            "returned trace must belong to conv_a"
        );
    }

    /// Status filter `"error"` applied via SQL should return only traces that
    /// contain at least one span with `tool_status = 'error'`.
    #[tokio::test]
    async fn test_status_filter_error_returns_only_error_traces() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let store = storage.trace_store();

        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("status-filter-test")
            .execute(&storage.pool)
            .await
            .unwrap();

        // One successful trace, one error trace
        insert_span(&storage.pool, conv_id, "bash", "ok", Some("ok"), None, 50).await;
        insert_span(
            &storage.pool,
            conv_id,
            "bash",
            "error",
            None,
            Some("permission denied"),
            30,
        )
        .await;

        let filter = TraceFilter {
            status: Some("error".to_string()),
            ..TraceFilter::default()
        };
        let results = store
            .list_recent_traces_for_agent(10, &filter, "default")
            .await
            .unwrap();
        assert_eq!(
            results.len(),
            1,
            "status=error should return only error traces"
        );
        assert!(
            results[0].error_count > 0,
            "returned trace must have error_count > 0"
        );
    }

    #[tokio::test]
    async fn test_interface_filter_scopes_results() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("iface-filter")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        // Slack root span
        insert_root_span_with_attrs(
            &storage.pool,
            conv_id,
            "turn",
            serde_json::json!({ "interface": "Slack" }),
        )
        .await;
        // Cli root span
        insert_root_span_with_attrs(
            &storage.pool,
            conv_id,
            "turn",
            serde_json::json!({ "interface": "Cli" }),
        )
        .await;

        let filter = TraceFilter {
            interface: Some("Slack".to_string()),
            ..TraceFilter::default()
        };
        let slack_only = store
            .list_recent_traces_for_agent(10, &filter, "default")
            .await
            .unwrap();
        assert_eq!(
            slack_only.len(),
            1,
            "interface filter should return only Slack traces"
        );
        assert_eq!(slack_only[0].interface.as_deref(), Some("Slack"));
    }

    /// Helper that inserts a span with an active_skill tag.
    async fn insert_span_with_skill(
        pool: &SqlitePool,
        conversation_id: Uuid,
        tool_name: &str,
        active_skill: &str,
        status: &str,
        error: Option<&str>,
        duration_ms: i64,
    ) {
        let span_id = Uuid::new_v4().to_string();
        let trace_id = Uuid::new_v4().to_string();
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(duration_ms.max(0));
        let attrs = json!({
            "conversation_id": conversation_id.to_string(),
            "tool_name": tool_name,
            "tool_status": status,
            "active_skill": active_skill,
        });

        sqlx::query(
            "INSERT INTO distributed_traces \
                (span_id, trace_id, parent_span_id, name, conversation_id, turn, tool_name, \
                 active_skill, tool_status, tool_observation, tool_error, duration_ms, \
                 start_time, end_time, attributes) \
             VALUES (?1, ?2, NULL, 'tool_execution', ?3, 0, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(span_id)
        .bind(trace_id)
        .bind(conversation_id.to_string())
        .bind(tool_name)
        .bind(active_skill)
        .bind(status)
        .bind(error)
        .bind(duration_ms)
        .bind(start)
        .bind(end)
        .bind(attrs.to_string())
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_stats_for_active_skill() {
        let storage = StorageLayer::new_in_memory().await.unwrap();
        let conv_id = Uuid::new_v4();
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?1, ?2)")
            .bind(conv_id.to_string())
            .bind("test")
            .execute(&storage.pool)
            .await
            .unwrap();

        let store = storage.trace_store();

        // Insert spans for the "code-review" skill using different tools.
        insert_span_with_skill(
            &storage.pool,
            conv_id,
            "bash",
            "code-review",
            "ok",
            None,
            100,
        )
        .await;
        insert_span_with_skill(
            &storage.pool,
            conv_id,
            "file-read",
            "code-review",
            "ok",
            None,
            50,
        )
        .await;
        insert_span_with_skill(
            &storage.pool,
            conv_id,
            "bash",
            "code-review",
            "error",
            Some("exit 1"),
            200,
        )
        .await;

        // Insert a span for a different skill — should not be counted.
        insert_span_with_skill(&storage.pool, conv_id, "bash", "deploy", "ok", None, 10).await;

        let stats = store
            .stats_for_active_skill("code-review", 100)
            .await
            .unwrap();
        assert_eq!(stats.total, 3, "should count only code-review spans");
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.error_count, 1);
        assert!(!stats.common_errors.is_empty());
    }
}
