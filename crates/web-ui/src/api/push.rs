//! Push subscription management and VAPID public key endpoints.
//!
//! Routes (all under `/api/push/`):
//! - `GET  /api/push/vapid-public-key`  — returns the base64url public key
//! - `POST /api/push/subscribe`         — upsert a push subscription (201)
//! - `DELETE /api/push/subscribe`       — remove a push subscription (204)

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRef, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};

use assistant_storage::PushSubscriptionStore as _;

use crate::api::internal_error;
use crate::space::{Access, SpaceCtx, SpaceError, SpaceResolver};

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct PushApiState {
    /// Resolves each request to the caller's space, whose database holds
    /// that space's subscriptions.
    pub spaces: SpaceResolver,
    /// Base64url-encoded VAPID public key (uncompressed P-256 point).
    pub vapid_public_key: Arc<String>,
}

impl FromRef<PushApiState> for SpaceResolver {
    fn from_ref(state: &PushApiState) -> Self {
        state.spaces.clone()
    }
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
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = [])),
)]
pub async fn vapid_public_key(
    ctx: SpaceCtx,
    State(state): State<PushApiState>,
) -> Result<Json<VapidKeyResponse>, SpaceError> {
    ctx.require(Access::Read)?;
    Ok(Json(VapidKeyResponse {
        public_key: (*state.vapid_public_key).clone(),
    }))
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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = [])),
)]
pub async fn subscribe(
    ctx: SpaceCtx,
    Json(body): Json<SubscribeRequest>,
) -> Result<StatusCode, Response> {
    ctx.require(Access::Read)
        .map_err(IntoResponse::into_response)?;
    ctx.storage
        .push_subscription_store()
        .upsert(
            &body.endpoint,
            &body.p256dh,
            &body.auth,
            Some(&ctx.auth.user_id.0),
        )
        .await
        .map_err(|e| internal_error(e).into_response())?;

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
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
    security(("bearer_token" = [])),
)]
pub async fn unsubscribe(
    ctx: SpaceCtx,
    Json(body): Json<UnsubscribeRequest>,
) -> Result<StatusCode, Response> {
    ctx.require(Access::Read)
        .map_err(IntoResponse::into_response)?;
    ctx.storage
        .push_subscription_store()
        .delete(&body.endpoint)
        .await
        .map_err(|e| internal_error(e).into_response())?;

    Ok(StatusCode::NO_CONTENT)
}
