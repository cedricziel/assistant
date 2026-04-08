//! REST JSON API for usage analytics.
//!
//! ## Routes
//!
//! | Method | Path            | Description              |
//! |--------|-----------------|--------------------------|
//! | GET    | `/api/analytics`| Aggregated usage summary |

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::warn;

use assistant_storage::MetricsStore;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct AnalyticsApiState {
    pub pool: SqlitePool,
    pub agent_id: Arc<RwLock<String>>,
}

// -- Response types ----------------------------------------------------------

/// Aggregated analytics summary for a time window.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AnalyticsSummaryResponse {
    /// The requested window in hours.
    pub window_hours: i64,
    pub total_tokens_in: i64,
    pub total_tokens_out: i64,
    pub total_requests: i64,
    pub total_tool_invocations: i64,
    pub avg_duration_s: f64,
    pub error_count: i64,
    pub unique_models: Vec<String>,
    pub models: Vec<ModelUsageResponse>,
    pub tools: Vec<ToolUsageResponse>,
    pub token_series: Vec<TimeSeriesResponse>,
    pub request_series: Vec<TimeSeriesResponse>,
}

/// Per-model token usage breakdown.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ModelUsageResponse {
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: i64,
    pub avg_duration_s: f64,
}

/// Tool invocation statistics.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ToolUsageResponse {
    pub tool_name: String,
    pub invocations: i64,
    pub avg_duration_s: f64,
}

/// A single point in a time series.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TimeSeriesResponse {
    pub bucket: String,
    pub value: f64,
}

// -- Query params ------------------------------------------------------------

/// Query parameters for the analytics endpoint.
#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct AnalyticsQueryParams {
    /// Window in hours. Valid values: 1, 6, 24, 72, 168. Defaults to 24.
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub window: Option<i64>,
}

// -- Helpers -----------------------------------------------------------------

/// Validate and return a canonical window value. Returns 24 for unknown values.
fn canonical_window(window: Option<i64>) -> i64 {
    match window {
        Some(w) if [1, 6, 24, 72, 168].contains(&w) => w,
        _ => 24,
    }
}

/// Choose a bucket width (in minutes) appropriate for the window size.
fn bucket_minutes(window_hours: i64) -> i64 {
    match window_hours {
        1 => 5,
        6 => 15,
        24 => 60,
        72 => 180,
        _ => 360,
    }
}

// -- Router ------------------------------------------------------------------

pub fn analytics_api_router() -> Router<AnalyticsApiState> {
    Router::new().route("/analytics", get(get_analytics))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/analytics` — get aggregated usage analytics for a time window.
#[utoipa::path(
    get,
    path = "/api/analytics",
    tag = "analytics",
    params(AnalyticsQueryParams),
    responses(
        (status = 200, description = "Analytics summary", body = AnalyticsSummaryResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_analytics(
    State(state): State<AnalyticsApiState>,
    Query(params): Query<AnalyticsQueryParams>,
) -> Response {
    let window_hours = canonical_window(params.window);
    let bucket_mins = bucket_minutes(window_hours);

    let agent_id = state.agent_id.read().await.clone();
    let store = MetricsStore::for_agent(state.pool, &agent_id);

    let (summary_res, models_res, tools_res, token_series_res, request_series_res) = tokio::join!(
        store.summary(window_hours),
        store.model_comparison(window_hours),
        store.tool_usage(window_hours),
        store.token_usage_over_time(window_hours, bucket_mins),
        store.request_rate(window_hours, bucket_mins),
    );

    let summary = match summary_res {
        Ok(s) => s,
        Err(e) => {
            warn!("Failed to fetch analytics summary: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch analytics",
            )
                .into_response();
        }
    };

    let models: Vec<ModelUsageResponse> = models_res
        .unwrap_or_default()
        .into_iter()
        .map(|m| ModelUsageResponse {
            model: m.model,
            input_tokens: m.input_tokens,
            output_tokens: m.output_tokens,
            request_count: m.request_count,
            avg_duration_s: m.avg_duration_s,
        })
        .collect();

    let tools: Vec<ToolUsageResponse> = tools_res
        .unwrap_or_default()
        .into_iter()
        .map(|t| ToolUsageResponse {
            tool_name: t.tool_name,
            invocations: t.invocations,
            avg_duration_s: t.avg_duration_s,
        })
        .collect();

    let token_series: Vec<TimeSeriesResponse> = token_series_res
        .unwrap_or_default()
        .into_iter()
        .map(|p| TimeSeriesResponse {
            bucket: p.bucket,
            value: p.value,
        })
        .collect();

    let request_series: Vec<TimeSeriesResponse> = request_series_res
        .unwrap_or_default()
        .into_iter()
        .map(|p| TimeSeriesResponse {
            bucket: p.bucket,
            value: p.value,
        })
        .collect();

    Json(AnalyticsSummaryResponse {
        window_hours,
        total_tokens_in: summary.total_tokens_in,
        total_tokens_out: summary.total_tokens_out,
        total_requests: summary.total_requests,
        total_tool_invocations: summary.total_tool_invocations,
        avg_duration_s: summary.avg_duration_s,
        error_count: summary.error_count,
        unique_models: summary.unique_models,
        models,
        tools,
        token_series,
        request_series,
    })
    .into_response()
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    use assistant_storage::StorageLayer;

    use super::{analytics_api_router, AnalyticsApiState};

    async fn test_state() -> (AnalyticsApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let state = AnalyticsApiState {
            pool: storage.pool.clone(),
            agent_id: Arc::new(RwLock::new("default".to_string())),
        };
        (state, storage)
    }

    fn app(state: AnalyticsApiState) -> axum::Router {
        analytics_api_router().with_state(state)
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    #[tokio::test]
    async fn get_analytics_default_window_returns_ok() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/analytics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["window_hours"], 24, "default window should be 24h");
        assert!(json["total_tokens_in"].is_number());
        assert!(json["models"].is_array());
        assert!(json["tools"].is_array());
        assert!(json["token_series"].is_array());
        assert!(json["request_series"].is_array());
    }

    #[tokio::test]
    async fn get_analytics_invalid_window_returns_default() {
        let (state, _) = test_state().await;
        // window=99 is not a valid value — should default to 24.
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/analytics?window=99")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(
            json["window_hours"], 24,
            "invalid window should fall back to 24h"
        );
    }

    #[tokio::test]
    async fn get_analytics_valid_window_1h() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/analytics?window=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json["window_hours"], 1);
    }
}
