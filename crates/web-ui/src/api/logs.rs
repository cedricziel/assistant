//! REST JSON API for log entry retrieval.
//!
//! ## Routes
//!
//! | Method | Path       | Description               |
//! |--------|------------|---------------------------|
//! | GET    | `/api/logs`| List recent log entries   |

use axum::{
    extract::{Query, State},
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
pub struct LogsApiState {
    pub pool: SqlitePool,
}

// -- Response types ----------------------------------------------------------

/// A log entry row returned by the API.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LogEntryResponse {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub severity: String,
    pub target: String,
    pub message: String,
    pub fields: serde_json::Value,
    pub trace_id: Option<String>,
    pub conversation_id: Option<String>,
}

// -- Query params ------------------------------------------------------------

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct LogsQueryParams {
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub limit: Option<i64>,
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub offset: Option<i64>,
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub search: Option<String>,
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub severity: Option<String>,
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub since: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub until: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub trace_id: Option<String>,
    #[serde(default, deserialize_with = "super::empty_string_as_none")]
    pub conversation: Option<String>,
}

// -- Router ------------------------------------------------------------------

pub fn logs_router() -> Router<LogsApiState> {
    Router::new().route("/logs", get(list_logs))
}

// -- Severity text → numeric -------------------------------------------------

fn severity_text_to_number(severity: &str) -> Option<i32> {
    match severity.to_uppercase().as_str() {
        "TRACE" => Some(1),
        "DEBUG" => Some(5),
        "INFO" => Some(9),
        "WARN" => Some(13),
        "ERROR" => Some(17),
        _ => None,
    }
}

fn severity_number_to_text(num: i32) -> &'static str {
    match num {
        1..=4 => "TRACE",
        5..=8 => "DEBUG",
        9..=12 => "INFO",
        13..=16 => "WARN",
        17..=20 => "ERROR",
        _ => "UNKNOWN",
    }
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/logs` — list recent log entries, newest first.
#[utoipa::path(
    get,
    path = "/api/logs",
    tag = "logs",
    params(LogsQueryParams),
    responses(
        (status = 200, description = "List of log entries", body = Vec<LogEntryResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_logs(
    State(state): State<LogsApiState>,
    Query(params): Query<LogsQueryParams>,
) -> Response {
    let limit = params.limit.unwrap_or(100).min(500);
    let store = assistant_storage::logs::LogStore::new(state.pool);

    let min_severity = params.severity.as_deref().and_then(severity_text_to_number);

    let search = params.search.as_deref().filter(|s| !s.is_empty());
    let trace_id = params.trace_id.as_deref();

    // Use list_recent (not scoped to agent) so all logs are visible.
    match store
        .list_recent(
            limit,
            min_severity,
            None, // target filter not exposed in API v1
            search,
            trace_id,
        )
        .await
    {
        Ok(logs) => {
            let mut entries: Vec<LogEntryResponse> = logs
                .into_iter()
                .filter(|l| {
                    // Apply post-fetch time and conversation filters.
                    if let Some(since) = params.since {
                        if l.timestamp < since {
                            return false;
                        }
                    }
                    if let Some(until) = params.until {
                        if l.timestamp > until {
                            return false;
                        }
                    }
                    true
                })
                .map(|l| {
                    let severity = l
                        .severity_number
                        .map(severity_number_to_text)
                        .unwrap_or("INFO")
                        .to_string();
                    let severity_override = l.severity_text.clone().unwrap_or(severity);

                    // Extract conversation_id from attributes if available.
                    let conversation_id = l
                        .attributes
                        .get("conversation_id")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    LogEntryResponse {
                        id: l.id,
                        timestamp: l.timestamp,
                        severity: severity_override,
                        target: l.target.unwrap_or_default(),
                        message: l.body.unwrap_or_default(),
                        fields: l.attributes,
                        trace_id: l.trace_id,
                        conversation_id,
                    }
                })
                .collect();

            // Apply offset.
            let offset = params.offset.unwrap_or(0) as usize;
            if offset < entries.len() {
                entries = entries.into_iter().skip(offset).collect();
            } else {
                entries.clear();
            }

            Json(entries).into_response()
        }
        Err(e) => {
            warn!("Failed to list logs: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list logs").into_response()
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

    use super::{logs_router, LogsApiState};

    async fn test_state() -> (LogsApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let state = LogsApiState {
            pool: storage.pool.clone(),
        };
        (state, storage)
    }

    fn app(state: LogsApiState) -> axum::Router {
        logs_router().with_state(state)
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    #[tokio::test]
    async fn list_logs_empty_returns_empty_array() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(Request::builder().uri("/logs").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn list_logs_with_search_param_returns_ok() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/logs?search=test&limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    // -- Empty-string query param tolerance (generated client sends "" for null) --

    #[tokio::test]
    async fn list_logs_with_empty_string_params_returns_200() {
        // Simulates the real Dart/Dio request:
        // GET /api/logs?limit=100&offset=&search=&severity=&since=&until=&trace_id=&conversation=
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/logs?limit=100&offset=&search=&severity=&since=&until=&trace_id=&conversation=")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert!(json.is_array());
    }

    #[tokio::test]
    async fn list_logs_with_empty_search_treated_as_no_filter() {
        // An empty search= should be treated as no filter (not "search for empty string").
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/logs?search=")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
