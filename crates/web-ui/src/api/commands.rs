//! Slash-command REST API endpoints.
//!
//! - `GET /api/commands` — list all registered commands (for autocomplete).
//! - `POST /api/conversations/{id}/command` — execute a slash command.
//! - `GET /api/conversations/{id}/events` — list command events for timeline.

use assistant_core::CommandDef;
use assistant_storage::CommandEventRow;
use assistant_storage::ConversationStore;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use super::ApiState;

// -- Request / Response types ------------------------------------------------

/// A command argument definition for the autocomplete UI.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CommandArgResponse {
    /// Argument name shown in autocomplete.
    pub name: String,
    /// Whether this argument is required.
    pub required: bool,
    /// Optional API endpoint that returns completions for this argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions_endpoint: Option<String>,
}

/// A command definition for the autocomplete UI.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CommandDefResponse {
    /// Command name without the leading `/`.
    pub name: String,
    /// One-line description.
    pub description: String,
    /// Grouping category (e.g. `"session"`, `"config"`, `"debug"`).
    pub category: String,
    /// Argument definitions.
    pub args: Vec<CommandArgResponse>,
}

impl From<&CommandDef> for CommandDefResponse {
    fn from(def: &CommandDef) -> Self {
        Self {
            name: def.name.clone(),
            description: def.description.clone(),
            category: def.category.clone(),
            args: def
                .args
                .iter()
                .map(|a| CommandArgResponse {
                    name: a.name.clone(),
                    required: a.required,
                    completions_endpoint: a.completions_endpoint.clone(),
                })
                .collect(),
        }
    }
}

/// Request body for `POST /api/conversations/{id}/command`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ExecuteCommandRequest {
    /// Command name without the leading `/` (e.g. `"model"`).
    pub command: String,
    /// Optional command arguments.
    #[serde(default)]
    pub args: Vec<String>,
}

/// Response body for a command event.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CommandEventResponse {
    /// Unique event ID.
    pub id: Uuid,
    /// Event type (always `"command"` for now).
    pub event_type: String,
    /// Command that was executed.
    pub command: String,
    /// Optional payload associated with the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    /// Acknowledgement text returned by the command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_text: Option<String>,
    /// When the event was created.
    pub created_at: DateTime<Utc>,
}

impl From<CommandEventRow> for CommandEventResponse {
    fn from(row: CommandEventRow) -> Self {
        Self {
            id: row.id,
            event_type: row.event_type,
            command: row.command,
            payload: row.payload,
            ack_text: row.ack_text,
            created_at: row.created_at,
        }
    }
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/commands` — list all registered commands.
#[utoipa::path(
    get,
    path = "/api/commands",
    tag = "commands",
    operation_id = "list_commands",
    responses(
        (status = 200, description = "List of available commands", body = Vec<CommandDefResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_commands(State(state): State<ApiState>) -> Response {
    let defs: Vec<CommandDefResponse> = state
        .command_registry
        .list()
        .iter()
        .map(CommandDefResponse::from)
        .collect();
    Json(defs).into_response()
}

/// `POST /api/conversations/{id}/command` — execute a slash command.
#[utoipa::path(
    post,
    path = "/api/conversations/{id}/command",
    tag = "commands",
    operation_id = "execute_command",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    request_body = ExecuteCommandRequest,
    responses(
        (status = 200, description = "Command executed successfully", body = CommandEventResponse),
        (status = 400, description = "Unknown command"),
    ),
    security(("bearer_token" = []))
)]
pub async fn execute_command(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<ExecuteCommandRequest>,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    // Verify the conversation exists.
    let agent_id = state.agent_id.read().await.clone();
    let store =
        assistant_storage::SqliteConversationStore::for_agent(state.pool.clone(), &agent_id);
    match store.get_conversation(conv_id).await {
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Conversation not found").into_response();
        }
        Err(e) => {
            warn!("Failed to check conversation {conv_id}: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response();
        }
        _ => {}
    }

    // Verify the command is known.
    if state.command_registry.get(&body.command).is_none() {
        return (
            StatusCode::BAD_REQUEST,
            format!("Unknown command: {}", body.command),
        )
            .into_response();
    }

    // Build a minimal command context for Web UI execution.
    let ctx = assistant_runtime::CommandContext {
        conversation_id: conv_id,
        conversation_configs: state.conversation_configs.clone(),
        orchestrator: state.orchestrator_ref.clone(),
        active_turns: state.active_turns.clone(),
        evict_conversation: None, // Web UI doesn't have an LRU cache to evict
        default_model: state.default_model.clone(),
    };

    let result = state
        .command_registry
        .execute(&body.command, &body.args, ctx)
        .await;

    // Build a semantic payload for the event.
    let payload = match body.command.as_str() {
        "model" if !body.args.is_empty() => {
            Some(serde_json::json!({ "model_name": body.args.join(" ") }))
        }
        _ if !body.args.is_empty() => Some(serde_json::json!({ "args": body.args })),
        _ => None,
    };

    match state
        .command_event_store
        .save_event(conv_id, &body.command, payload.as_ref(), &result.ack_text)
        .await
    {
        Ok(event) => Json(CommandEventResponse::from(event)).into_response(),
        Err(e) => {
            warn!("Failed to persist command event: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to persist command event",
            )
                .into_response()
        }
    }
}

/// `GET /api/conversations/{id}/events` — list command events for the timeline.
#[utoipa::path(
    get,
    path = "/api/conversations/{id}/events",
    tag = "commands",
    operation_id = "list_conversation_events",
    params(("id" = Uuid, Path, description = "Conversation ID")),
    responses(
        (status = 200, description = "List of command events", body = Vec<CommandEventResponse>),
        (status = 400, description = "Invalid ID"),
    ),
    security(("bearer_token" = []))
)]
pub async fn list_command_events(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Response {
    let conv_id = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid conversation ID").into_response(),
    };

    match state.command_event_store.list_events(conv_id).await {
        Ok(events) => {
            let responses: Vec<CommandEventResponse> =
                events.into_iter().map(CommandEventResponse::from).collect();
            Json(responses).into_response()
        }
        Err(e) => {
            warn!("Failed to list command events: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to list command events",
            )
                .into_response()
        }
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_runtime::CommandRegistry;
    use assistant_storage::{CommandEventStore, SqliteCommandEventStore};

    #[test]
    fn list_commands_returns_all_builtins() {
        let registry = CommandRegistry::new();
        let defs: Vec<CommandDefResponse> = registry
            .list()
            .iter()
            .map(CommandDefResponse::from)
            .collect();
        assert_eq!(defs.len(), 6);

        let names: Vec<&str> = defs.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"help"));
        assert!(names.contains(&"new"));
        assert!(names.contains(&"model"));
        assert!(names.contains(&"stop"));
        assert!(names.contains(&"compact"));
        assert!(names.contains(&"status"));
    }

    #[test]
    fn unknown_command_not_in_registry() {
        let registry = CommandRegistry::new();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn command_def_response_preserves_args() {
        let registry = CommandRegistry::new();
        let model_cmd = registry.get("model").unwrap();
        let resp = CommandDefResponse::from(model_cmd);
        assert_eq!(resp.args.len(), 1);
        assert_eq!(resp.args[0].name, "model_name");
        assert!(resp.args[0].completions_endpoint.is_some());
    }

    #[tokio::test]
    async fn event_store_roundtrip() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let store = SqliteCommandEventStore::new(storage.pool.clone());
        let conv_id = Uuid::new_v4();

        // Save an event.
        let event = store
            .save_event(conv_id, "help", None, "Available commands: ...")
            .await
            .unwrap();
        assert_eq!(event.command, "help");

        // Convert to response type.
        let resp = CommandEventResponse::from(event);
        assert_eq!(resp.command, "help");
        assert_eq!(resp.ack_text.as_deref(), Some("Available commands: ..."));

        // List events.
        let events = store.list_events(conv_id).await.unwrap();
        assert_eq!(events.len(), 1);
        let responses: Vec<CommandEventResponse> =
            events.into_iter().map(CommandEventResponse::from).collect();
        assert_eq!(responses[0].command, "help");
    }

    #[tokio::test]
    async fn list_events_empty_for_unknown_conversation() {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .unwrap();
        let store = SqliteCommandEventStore::new(storage.pool.clone());

        let events = store.list_events(Uuid::new_v4()).await.unwrap();
        let responses: Vec<CommandEventResponse> =
            events.into_iter().map(CommandEventResponse::from).collect();
        assert!(responses.is_empty());
    }
}
