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

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

#[derive(Debug, Deserialize)]
pub struct UnsubscribeRequest {
    pub endpoint: String,
}

#[derive(Debug, Serialize)]
struct VapidKeyResponse {
    public_key: String,
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/push/vapid-public-key`
///
/// Returns the server's VAPID public key as a base64url-encoded string.
/// The Flutter PWA uses this as the `applicationServerKey` argument to
/// `PushManager.subscribe()`.
async fn vapid_public_key(State(state): State<PushApiState>) -> Json<VapidKeyResponse> {
    Json(VapidKeyResponse {
        public_key: (*state.vapid_public_key).clone(),
    })
}

/// `POST /api/push/subscribe`
///
/// Upsert a browser push subscription (endpoint + key material).  Returns
/// `201 Created` on success.
async fn subscribe(
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
async fn unsubscribe(
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
