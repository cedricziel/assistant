//! Webhook management UI — list, create, edit, delete, and verify outgoing webhooks.

#[cfg(test)]
mod handler_tests;
pub mod pages;

use axum::routing::{get, post};
use axum::Router;

use pages::WebhookPagesState;

/// Builds the axum [`Router`] for webhook management HTML pages.
pub fn webhook_pages_router() -> Router<WebhookPagesState> {
    Router::new()
        .route(
            "/webhooks",
            get(pages::list_webhooks).post(pages::create_webhook),
        )
        .route("/webhooks/new", get(pages::new_webhook_form))
        .route("/webhooks/{id}", get(pages::show_webhook))
        .route(
            "/webhooks/{id}/edit",
            get(pages::edit_webhook_form).post(pages::update_webhook),
        )
        .route("/webhooks/{id}/delete", post(pages::delete_webhook))
        .route("/webhooks/{id}/verify", post(pages::verify_webhook))
        .route("/webhooks/{id}/toggle", post(pages::toggle_webhook))
        .route("/webhooks/{id}/rotate-secret", post(pages::rotate_secret))
}
