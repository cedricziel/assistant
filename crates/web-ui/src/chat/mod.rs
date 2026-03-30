//! Chat interface: serves the single-page chat shell.
//!
//! All conversation management and messaging is handled client-side by
//! `chat-controller.js`, which calls the JSON REST API at `/api/conversations/*`.
//!
//! The server only renders the static HTML shell:
//! - `GET /chat`       — empty shell (no conversation selected)
//! - `GET /chat/{id}`  — shell with the target conversation ID embedded so
//!   the Stimulus controller can open it on first load.

use askama::Template;
use axum::{extract::State, response::Response, routing::get, Router};

use crate::common;
use crate::common::StaticUrls;

// -- State -------------------------------------------------------------------

/// Shared state for chat route handlers.
#[derive(Clone)]
pub struct ChatState {
    pub agent_id: String,
}

impl ChatState {
    pub fn new(agent_id: String) -> Self {
        Self { agent_id }
    }
}

// -- Template ----------------------------------------------------------------

/// Chat page shell.  Stimulus controllers hydrate it via the JSON API.
#[derive(Template)]
#[template(path = "chat/page.html")]
struct ChatPageTemplate {
    active_page: &'static str,
}

impl StaticUrls for ChatPageTemplate {}

// -- Router ------------------------------------------------------------------

/// Build the chat sub-router.  Mounted under the auth-protected scope.
pub fn chat_router() -> Router<ChatState> {
    Router::new()
        .route("/chat", get(chat_page))
        // Deep-link: /chat/{id} serves the same shell; the controller reads
        // the conversation ID from window.location on connect().
        .route("/chat/{id}", get(chat_page))
}

// -- Handlers ----------------------------------------------------------------

/// Render the page shell.  The Stimulus controllers take over from here.
async fn chat_page(_state: State<ChatState>) -> Response {
    common::render_template(ChatPageTemplate {
        active_page: "chat",
    })
}
