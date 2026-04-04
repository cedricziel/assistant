//! REST JSON API for distributed trace retrieval.
//!
//! ## Routes
//!
//! | Method | Path                  | Description                      |
//! |--------|-----------------------|----------------------------------|
//! | GET    | `/api/traces`         | List recent traces (filtered)    |
//! | GET    | `/api/traces/{id}`    | Get a single trace with spans    |

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::warn;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct TracesApiState {
    pub pool: SqlitePool,
}

// -- Response types ----------------------------------------------------------

/// A trace summary row.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TraceSummaryResponse {
    pub trace_id: String,
    pub persona_id: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: i64,
    pub skill_name: Option<String>,
    pub status: String,
    pub conversation_id: Option<String>,
}

/// A single span within a trace.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SpanEntryResponse {
    pub span_id: String,
    pub name: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: i64,
    pub attributes: serde_json::Value,
}

/// Full trace detail with span list.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TraceDetailResponse {
    pub trace_id: String,
    pub persona_id: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: i64,
    pub spans: Vec<SpanEntryResponse>,
}

// -- Query params ------------------------------------------------------------

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TracesQueryParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub skill: Option<String>,
    pub status: Option<String>,
    pub conversation: Option<String>,
}

// -- Router ------------------------------------------------------------------

pub fn traces_router() -> Router<TracesApiState> {
    Router::new()
        .route("/traces", get(list_traces))
        .route("/traces/{trace_id}", get(get_trace))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/traces` — list recent traces, newest first.
#[utoipa::path(
    get,
    path = "/api/traces",
    tag = "traces",
    params(TracesQueryParams),
    responses(
        (status = 200, description = "List of traces", body = Vec<TraceSummaryResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_traces(
    State(state): State<TracesApiState>,
    Query(params): Query<TracesQueryParams>,
) -> Response {
    let limit = params.limit.unwrap_or(50).min(200);
    let store = assistant_storage::traces::TraceStore::new(state.pool);

    let filter = assistant_storage::traces::TraceFilter {
        skill: params.skill.clone(),
        status: params.status.clone(),
        since: params.since,
        until: params.until,
        conversation: params
            .conversation
            .as_deref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok()),
        ..Default::default()
    };

    match store
        .list_recent_traces(limit, filter.skill.as_deref())
        .await
    {
        Ok(traces) => {
            let mut summaries: Vec<TraceSummaryResponse> = traces
                .into_iter()
                .filter(|t| {
                    // Apply post-fetch filters (status, conversation, since, until).
                    if let Some(ref status) = filter.status {
                        let trace_status = if t.error_count > 0 { "error" } else { "ok" };
                        if trace_status != status.as_str() {
                            return false;
                        }
                    }
                    if let Some(conv_id) = filter.conversation {
                        if t.conversation_id != Some(conv_id) {
                            return false;
                        }
                    }
                    if let Some(since) = filter.since {
                        if t.start_time < since {
                            return false;
                        }
                    }
                    if let Some(until) = filter.until {
                        if t.start_time > until {
                            return false;
                        }
                    }
                    true
                })
                .map(|t| {
                    let status = if t.error_count > 0 { "error" } else { "ok" }.to_string();
                    let duration_ms = t
                        .end_time
                        .signed_duration_since(t.start_time)
                        .num_milliseconds();
                    TraceSummaryResponse {
                        trace_id: t.trace_id,
                        persona_id: t.root_service_name.unwrap_or_default(),
                        start_time: t.start_time,
                        duration_ms,
                        skill_name: t.tool_names.into_iter().next(),
                        status,
                        conversation_id: t.conversation_id.map(|u| u.to_string()),
                    }
                })
                .collect();

            // Apply offset.
            let offset = params.offset.unwrap_or(0) as usize;
            if offset < summaries.len() {
                summaries = summaries.into_iter().skip(offset).collect();
            } else {
                summaries.clear();
            }

            Json(summaries).into_response()
        }
        Err(e) => {
            warn!("Failed to list traces: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list traces").into_response()
        }
    }
}

/// `GET /api/traces/{trace_id}` — get a single trace with span breakdown.
#[utoipa::path(
    get,
    path = "/api/traces/{trace_id}",
    tag = "traces",
    params(("trace_id" = String, Path, description = "Trace ID")),
    responses(
        (status = 200, description = "Trace detail with spans", body = TraceDetailResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Trace not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_trace(
    State(state): State<TracesApiState>,
    Path(trace_id): Path<String>,
) -> Response {
    let store = assistant_storage::traces::TraceStore::new(state.pool);

    match store.get_trace(&trace_id).await {
        Ok(spans) if spans.is_empty() => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Trace not found"})),
        )
            .into_response(),
        Ok(spans) => {
            let start_time = spans.iter().map(|s| s.start_time).min().unwrap_or_default();
            let end_time = spans.iter().map(|s| s.end_time).max().unwrap_or_default();
            let duration_ms = end_time
                .signed_duration_since(start_time)
                .num_milliseconds();
            let persona_id = spans
                .first()
                .and_then(|s| s.service_name.clone())
                .unwrap_or_default();

            let span_entries: Vec<SpanEntryResponse> = spans
                .into_iter()
                .map(|s| SpanEntryResponse {
                    span_id: s.span_id,
                    name: s.name,
                    start_time: s.start_time,
                    duration_ms: s.duration_ms,
                    attributes: s.attributes,
                })
                .collect();

            Json(TraceDetailResponse {
                trace_id: trace_id.clone(),
                persona_id,
                start_time,
                duration_ms,
                spans: span_entries,
            })
            .into_response()
        }
        Err(e) => {
            warn!("Failed to get trace {trace_id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use assistant_storage::StorageLayer;

    use super::{traces_router, TracesApiState};

    async fn test_state() -> (TracesApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let state = TracesApiState {
            pool: storage.pool.clone(),
        };
        (state, storage)
    }

    fn app(state: TracesApiState) -> axum::Router {
        traces_router().with_state(state)
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    #[tokio::test]
    async fn list_traces_empty_returns_empty_array() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/traces")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn get_trace_unknown_returns_404() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/traces/nonexistent-trace-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
