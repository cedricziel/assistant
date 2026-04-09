//! Push subscription management and VAPID public key endpoints.
//!
//! Routes (all under `/api/push/`):
//! - `GET  /api/push/vapid-public-key`  — returns the base64url public key
//! - `POST /api/push/subscribe`         — upsert a push subscription (201)
//! - `DELETE /api/push/subscribe`       — remove a push subscription (204)

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use assistant_storage::PushSubscriptionStore;

use crate::api::internal_error;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct PushApiState {
    pub store: Arc<PushSubscriptionStore>,
    /// Base64url-encoded VAPID public key (uncompressed P-256 point).
    pub vapid_public_key: Arc<String>,
}

// -- Router ------------------------------------------------------------------

pub fn push_api_router() -> Router<PushApiState> {
    Router::new()
        .route("/push/vapid-public-key", get(vapid_public_key))
        .route("/push/subscribe", post(subscribe))
        .route("/push/subscribe", delete(unsubscribe))
}

// -- Request / Response types ------------------------------------------------

/// Browser push subscription key material.
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct SubscribeRequest {
    /// The push endpoint URL assigned by the browser's push service.
    pub endpoint: String,
    /// Base64url-encoded P-256 ECDH public key from the browser subscription.
    pub p256dh: String,
    /// Base64url-encoded 16-byte authentication secret from the browser subscription.
    pub auth: String,
}

/// Endpoint URL of the subscription to remove.
#[derive(Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct UnsubscribeRequest {
    /// The push endpoint URL to remove.
    pub endpoint: String,
}

/// VAPID public key returned to the client.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VapidKeyResponse {
    /// Base64url-encoded uncompressed P-256 public key (65 bytes).
    /// Pass this as `applicationServerKey` to `PushManager.subscribe()`.
    pub public_key: String,
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/push/vapid-public-key`
///
/// Returns the server's VAPID public key as a base64url-encoded string.
/// The Flutter PWA uses this as the `applicationServerKey` argument to
/// `PushManager.subscribe()`.
#[utoipa::path(
    get,
    path = "/api/push/vapid-public-key",
    tag = "web-push",
    responses(
        (status = 200, description = "VAPID public key", body = VapidKeyResponse),
    ),
    security(("bearer_token" = [])),
)]
pub async fn vapid_public_key(State(state): State<PushApiState>) -> Json<VapidKeyResponse> {
    Json(VapidKeyResponse {
        public_key: (*state.vapid_public_key).clone(),
    })
}

/// `POST /api/push/subscribe`
///
/// Upsert a browser push subscription (endpoint + key material).  Returns
/// `201 Created` on success.
#[utoipa::path(
    post,
    path = "/api/push/subscribe",
    tag = "web-push",
    request_body = SubscribeRequest,
    responses(
        (status = 201, description = "Subscription registered"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = [])),
)]
pub async fn subscribe(
    State(state): State<PushApiState>,
    Json(body): Json<SubscribeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .store
        .upsert(&body.endpoint, &body.p256dh, &body.auth)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::CREATED)
}

/// `DELETE /api/push/subscribe`
///
/// Remove a push subscription by its endpoint URL.  Returns `204 No Content`
/// whether or not the subscription existed.
#[utoipa::path(
    delete,
    path = "/api/push/subscribe",
    tag = "web-push",
    request_body = UnsubscribeRequest,
    responses(
        (status = 204, description = "Subscription removed"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = [])),
)]
pub async fn unsubscribe(
    State(state): State<PushApiState>,
    Json(body): Json<UnsubscribeRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    state
        .store
        .delete(&body.endpoint)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}
