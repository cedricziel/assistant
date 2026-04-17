//! REST JSON API for webhook management.
//!
//! ## Routes
//!
//! | Method | Path                              | Description                   |
//! |--------|-----------------------------------|-------------------------------|
//! | GET    | `/api/webhooks`                   | List all webhooks             |
//! | POST   | `/api/webhooks`                   | Create a webhook              |
//! | GET    | `/api/webhooks/{id}`              | Get webhook detail            |
//! | PATCH  | `/api/webhooks/{id}`              | Update a webhook              |
//! | DELETE | `/api/webhooks/{id}`              | Delete a webhook              |
//! | POST   | `/api/webhooks/{id}/toggle`       | Toggle active state           |
//! | POST   | `/api/webhooks/{id}/rotate-secret`| Rotate signing secret         |
//! | POST   | `/api/webhooks/{id}/verify`       | Send a signed verification    |

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tracing::{info, warn};

use assistant_storage::WebhookStore;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct WebhooksApiState {
    pub pool: SqlitePool,
    pub agent_id: Arc<RwLock<String>>,
}

// -- Response types ----------------------------------------------------------

/// A webhook entry returned by the API.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WebhookResponse {
    pub id: String,
    pub name: String,
    pub url: String,
    pub secret: String,
    pub event_types: Vec<String>,
    pub active: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Response after rotating a webhook secret.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RotateSecretResponse {
    pub secret: String,
}

/// Response after attempting webhook verification.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VerifyWebhookResponse {
    pub success: bool,
    pub detail: String,
}

// -- Request types -----------------------------------------------------------

/// Body for `POST /api/webhooks`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub event_types: Vec<String>,
    /// Defaults to `true` when omitted.
    pub active: Option<bool>,
}

/// Body for `PATCH /api/webhooks/{id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateWebhookRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub event_types: Option<Vec<String>>,
    pub active: Option<bool>,
}

// -- Helpers -----------------------------------------------------------------

fn to_response(r: assistant_storage::WebhookRecord) -> WebhookResponse {
    WebhookResponse {
        id: r.id,
        name: r.name,
        url: r.url,
        secret: r.secret,
        event_types: r.event_types,
        active: r.active,
        verified_at: r.verified_at,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

// -- Router ------------------------------------------------------------------

pub fn webhooks_api_router() -> Router<WebhooksApiState> {
    Router::new()
        .route("/webhooks", get(list_webhooks).post(create_webhook))
        .route(
            "/webhooks/{id}",
            get(get_webhook)
                .patch(update_webhook)
                .delete(delete_webhook),
        )
        .route("/webhooks/{id}/toggle", post(toggle_webhook))
        .route("/webhooks/{id}/rotate-secret", post(rotate_secret))
        .route("/webhooks/{id}/verify", post(verify_webhook))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/webhooks` — list all webhooks.
#[utoipa::path(
    get,
    path = "/api/webhooks",
    tag = "webhooks",
    responses(
        (status = 200, description = "List of webhooks", body = Vec<WebhookResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_webhooks(State(state): State<WebhooksApiState>) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);
    match store.list().await {
        Ok(webhooks) => {
            Json(webhooks.into_iter().map(to_response).collect::<Vec<_>>()).into_response()
        }
        Err(e) => {
            warn!("Failed to list webhooks: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list webhooks").into_response()
        }
    }
}

/// `POST /api/webhooks` — create a new webhook.
#[utoipa::path(
    post,
    path = "/api/webhooks",
    tag = "webhooks",
    request_body = CreateWebhookRequest,
    responses(
        (status = 201, description = "Webhook created", body = WebhookResponse),
        (status = 400, description = "Invalid URL or empty name"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = []))
)]
pub async fn create_webhook(
    State(state): State<WebhooksApiState>,
    Json(body): Json<CreateWebhookRequest>,
) -> Response {
    if body.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Name cannot be empty"})),
        )
            .into_response();
    }

    if let Err(e) = validate_webhook_url(&body.url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
            .into_response();
    }

    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);
    let id = uuid::Uuid::new_v4().to_string();
    let secret = generate_secret();

    match store
        .create(&id, &body.name, &body.url, &secret, &body.event_types)
        .await
    {
        Ok(()) => {
            info!(webhook_id = %id, name = %body.name, "Webhook created via API");
            match store.get(&id).await {
                Ok(Some(wh)) => (StatusCode::CREATED, Json(to_response(wh))).into_response(),
                _ => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch created webhook",
                )
                    .into_response(),
            }
        }
        Err(e) => {
            warn!("Failed to create webhook: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create webhook",
            )
                .into_response()
        }
    }
}

/// `GET /api/webhooks/{id}` — get a webhook by ID.
#[utoipa::path(
    get,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    params(("id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Webhook detail", body = WebhookResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn get_webhook(
    State(state): State<WebhooksApiState>,
    Path(id): Path<String>,
) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);
    match store.get(&id).await {
        Ok(Some(wh)) => Json(to_response(wh)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Webhook not found"})),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to get webhook {id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response()
        }
    }
}

/// `PATCH /api/webhooks/{id}` — update a webhook.
#[utoipa::path(
    patch,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    params(("id" = String, Path, description = "Webhook ID")),
    request_body = UpdateWebhookRequest,
    responses(
        (status = 200, description = "Updated webhook", body = WebhookResponse),
        (status = 400, description = "Invalid URL"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn update_webhook(
    State(state): State<WebhooksApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateWebhookRequest>,
) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);

    // Fetch the existing record to merge with.
    let existing = match store.get(&id).await {
        Ok(Some(wh)) => wh,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Webhook not found"})),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to get webhook {id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    let new_name = body.name.as_deref().unwrap_or(&existing.name);
    let new_url = body.url.as_deref().unwrap_or(&existing.url);
    let new_event_types = body
        .event_types
        .as_ref()
        .unwrap_or(&existing.event_types)
        .clone();
    let new_active = body.active.unwrap_or(existing.active);

    // Validate the URL if it changed.
    if body.url.is_some()
        && let Err(e) = validate_webhook_url(new_url) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            )
                .into_response();
        }

    match store
        .update(&id, new_name, new_url, &new_event_types, new_active)
        .await
    {
        Ok(true) => match store.get(&id).await {
            Ok(Some(wh)) => Json(to_response(wh)).into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch updated webhook",
            )
                .into_response(),
        },
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Webhook not found"})),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to update webhook {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update webhook",
            )
                .into_response()
        }
    }
}

/// `DELETE /api/webhooks/{id}` — delete a webhook.
#[utoipa::path(
    delete,
    path = "/api/webhooks/{id}",
    tag = "webhooks",
    params(("id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn delete_webhook(
    State(state): State<WebhooksApiState>,
    Path(id): Path<String>,
) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);
    match store.delete(&id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Webhook not found"})),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to delete webhook {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to delete webhook",
            )
                .into_response()
        }
    }
}

/// `POST /api/webhooks/{id}/toggle` — toggle a webhook's active state.
#[utoipa::path(
    post,
    path = "/api/webhooks/{id}/toggle",
    tag = "webhooks",
    params(("id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Updated webhook", body = WebhookResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn toggle_webhook(
    State(state): State<WebhooksApiState>,
    Path(id): Path<String>,
) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);
    match store.toggle_active(&id).await {
        Ok(true) => match store.get(&id).await {
            Ok(Some(wh)) => Json(to_response(wh)).into_response(),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch toggled webhook",
            )
                .into_response(),
        },
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Webhook not found"})),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to toggle webhook {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to toggle webhook",
            )
                .into_response()
        }
    }
}

/// `POST /api/webhooks/{id}/rotate-secret` — regenerate a webhook's signing secret.
#[utoipa::path(
    post,
    path = "/api/webhooks/{id}/rotate-secret",
    tag = "webhooks",
    params(("id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "New secret", body = RotateSecretResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn rotate_secret(
    State(state): State<WebhooksApiState>,
    Path(id): Path<String>,
) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);
    let new_secret = generate_secret();
    match store.rotate_secret(&id, &new_secret).await {
        Ok(true) => Json(RotateSecretResponse { secret: new_secret }).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Webhook not found"})),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to rotate secret for webhook {id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to rotate secret").into_response()
        }
    }
}

/// `POST /api/webhooks/{id}/verify` — send a signed test payload to the webhook URL.
///
/// Always returns HTTP 200; the `success` field in the body indicates whether
/// the remote endpoint responded with a 2xx status.
#[utoipa::path(
    post,
    path = "/api/webhooks/{id}/verify",
    tag = "webhooks",
    params(("id" = String, Path, description = "Webhook ID")),
    responses(
        (status = 200, description = "Verification result", body = VerifyWebhookResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Webhook not found"),
    ),
    security(("bearer_token" = []))
)]
pub async fn verify_webhook(
    State(state): State<WebhooksApiState>,
    Path(id): Path<String>,
) -> Response {
    let agent_id = state.agent_id.read().await.clone();
    let store = WebhookStore::for_agent(state.pool, &agent_id);

    let wh = match store.get(&id).await {
        Ok(Some(wh)) => wh,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Webhook not found"})),
            )
                .into_response()
        }
        Err(e) => {
            warn!("Failed to get webhook {id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
    };

    // SSRF protection: validate before sending.
    if let Err(e) = validate_webhook_url(&wh.url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
            .into_response();
    }

    let payload = serde_json::json!({
        "type": "webhook.verify",
        "webhook_id": wh.id,
        "timestamp": Utc::now().to_rfc3339(),
    });
    let body_str = serde_json::to_string(&payload).unwrap_or_default();
    let signature = compute_signature(&wh.secret, &body_str);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("Failed to build HTTP client: {e}")})),
            )
                .into_response()
        }
    };

    let result = client
        .post(&wh.url)
        .header("Content-Type", "application/json")
        .header("X-Webhook-Signature", format!("sha256={signature}"))
        .header("X-Webhook-Event", "webhook.verify")
        .header("X-Webhook-Id", &wh.id)
        .body(body_str)
        .send()
        .await;

    let (success, detail) = match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                info!(webhook_id = %wh.id, "Webhook verified successfully via API");
                if let Err(e) = store.mark_verified(&wh.id).await {
                    warn!(webhook_id = %wh.id, error = %e, "Failed to mark webhook as verified");
                }
                (true, format!("Remote responded with {status}"))
            } else {
                warn!(webhook_id = %wh.id, %status, "Webhook verification failed: non-2xx");
                (false, format!("Remote responded with {status}"))
            }
        }
        Err(e) => {
            warn!(webhook_id = %wh.id, error = %e, "Webhook verification failed: connection error");
            (false, format!("Connection error: {e}"))
        }
    };

    Json(VerifyWebhookResponse { success, detail }).into_response()
}

// -- Webhook helpers ---------------------------------------------------------

/// Validate that a webhook URL is a safe, publicly-reachable HTTP(S) endpoint.
fn validate_webhook_url(url: &str) -> Result<(), String> {
    use std::net::IpAddr;
    let parsed = url::Url::parse(url).map_err(|e| format!("Invalid URL: {e}"))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(format!(
                "Unsupported scheme '{other}': only http and https are allowed"
            ))
        }
    }
    let host = parsed.host_str().ok_or("URL has no host")?;
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" || lower == "127.0.0.1" || lower == "::1" || lower == "[::1]" {
        return Err("Loopback addresses are not allowed".to_string());
    }
    if let Ok(ip) = host.parse::<IpAddr>()
        && is_private_ip(ip) {
            return Err(format!("Private/reserved IP address {ip} is not allowed"));
        }
    let trimmed = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = trimmed.parse::<IpAddr>()
        && is_private_ip(ip) {
            return Err(format!("Private/reserved IP address {ip} is not allowed"));
        }
    Ok(())
}

fn is_private_ip(ip: std::net::IpAddr) -> bool {
    use std::net::IpAddr;
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Generate a 32-byte random hex secret.
fn generate_secret() -> String {
    use std::fmt::Write as _;
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).expect("OS RNG should be available");
    let mut s = String::with_capacity(64);
    for b in buf {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Compute HMAC-SHA256 of `body` using `secret`, returning hex.
fn compute_signature(secret: &str, body: &str) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    type HmacSha256 = Hmac<sha2::Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
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

    use super::{webhooks_api_router, WebhooksApiState};

    async fn test_state() -> (WebhooksApiState, Arc<StorageLayer>) {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let state = WebhooksApiState {
            pool: storage.pool.clone(),
            agent_id: Arc::new(RwLock::new("default".to_string())),
        };
        (state, storage)
    }

    fn app(state: WebhooksApiState) -> axum::Router {
        webhooks_api_router().with_state(state)
    }

    async fn body_json(body: Body) -> serde_json::Value {
        let b = body.collect().await.unwrap().to_bytes();
        serde_json::from_slice(&b).unwrap()
    }

    #[tokio::test]
    async fn list_webhooks_returns_empty_array() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/webhooks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp.into_body()).await;
        assert!(json.is_array(), "should return an array");
        assert_eq!(json.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn create_webhook_invalid_url_returns_400() {
        let (state, _) = test_state().await;
        // localhost is blocked by the SSRF check.
        let body = r#"{"name":"My Hook","url":"http://localhost/test","event_types":[]}"#;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn get_webhook_unknown_returns_404() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .uri("/webhooks/no-such-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_webhook_unknown_returns_404() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/webhooks/no-such-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn toggle_unknown_webhook_returns_404() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks/no-such-id/toggle")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn rotate_secret_unknown_returns_404() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks/no-such-id/rotate-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn verify_unknown_webhook_returns_404() {
        let (state, _) = test_state().await;
        let resp = app(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks/no-such-id/verify")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
