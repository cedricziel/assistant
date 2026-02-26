//! Axum HTTP handlers for the A2A protocol.
//!
//! Each handler maps to one RPC in the `A2AService`. The handlers operate on
//! the in-memory [`TaskStore`] and produce/consume the canonical JSON types
//! from `assistant_a2a_json_schema`.

use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures::stream::Stream;
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

use assistant_a2a_json_schema::agent_card::*;
use assistant_a2a_json_schema::requests::*;
use assistant_a2a_json_schema::responses::*;
use assistant_a2a_json_schema::types::*;

use super::task_store::TaskStore;

// -- Shared state --

/// Shared state for all A2A handlers.
#[derive(Clone)]
pub struct A2AState {
    /// In-memory task store.
    pub task_store: TaskStore,
    /// The agent card describing this agent.
    pub agent_card: AgentCard,
}

// -- Error helper --

/// A2A error response body following JSON-RPC style.
#[derive(serde::Serialize)]
struct A2AError {
    code: i32,
    message: String,
}

fn not_found(msg: impl Into<String>) -> Response {
    let body = A2AError {
        code: 404,
        message: msg.into(),
    };
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

fn bad_request(msg: impl Into<String>) -> Response {
    let body = A2AError {
        code: 400,
        message: msg.into(),
    };
    (StatusCode::BAD_REQUEST, Json(body)).into_response()
}

// -- Agent Card --

/// `GET /.well-known/agent.json` -- Returns the public agent card.
pub async fn get_agent_card_well_known(State(state): State<A2AState>) -> Json<AgentCard> {
    Json(state.agent_card.clone())
}

/// `GET /agent/authenticatedExtendedCard` -- Returns the extended agent card
/// (same as public for now).
pub async fn get_extended_agent_card(State(state): State<A2AState>) -> Json<AgentCard> {
    Json(state.agent_card.clone())
}

// -- SendMessage --

/// `POST /message/send` -- Sends a message to the agent (unary).
///
/// Creates a task, records the user message, transitions to Working, produces
/// an agent reply, and returns the final task state.
pub async fn send_message(
    State(state): State<A2AState>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    let context_id = req.message.context_id.clone();
    let task = state.task_store.create_task(context_id).await;

    // Record the incoming user message in history.
    let mut user_msg = req.message.clone();
    user_msg.task_id = Some(task.id.clone());
    state.task_store.append_history(&task.id, user_msg).await;

    // Transition to Working.
    state
        .task_store
        .update_status(&task.id, TaskState::TaskStateWorking, None)
        .await;

    // Extract the user's text content for processing.
    let user_text: String = req
        .message
        .parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    // Build an agent reply.
    // TODO: Wire to Orchestrator for real LLM processing.
    let reply_text = format!(
        "Received your message ({} chars). Processing is not yet wired to the LLM backend.",
        user_text.len()
    );
    let agent_msg = Message {
        message_id: Uuid::new_v4().to_string(),
        context_id: Some(task.context_id.clone()),
        task_id: Some(task.id.clone()),
        role: Role::RoleAgent,
        parts: vec![Part::text(reply_text)],
        metadata: None,
        extensions: vec![],
        reference_task_ids: vec![],
    };

    state
        .task_store
        .append_history(&task.id, agent_msg.clone())
        .await;

    // Transition to Completed.
    state
        .task_store
        .update_status(&task.id, TaskState::TaskStateCompleted, Some(agent_msg))
        .await;

    let final_task = state.task_store.get_task(&task.id).await.unwrap();

    let resp = SendMessageResponse {
        task: Some(final_task),
        message: None,
    };

    Json(resp).into_response()
}

/// `POST /message/stream` -- Sends a message with streaming response (SSE).
pub async fn send_message_streaming(
    State(state): State<A2AState>,
    Json(req): Json<SendMessageRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let context_id = req.message.context_id.clone();
    let task = state.task_store.create_task(context_id).await;
    let task_id = task.id.clone();

    // Record incoming message.
    let mut user_msg = req.message.clone();
    user_msg.task_id = Some(task_id.clone());
    state.task_store.append_history(&task_id, user_msg).await;

    // Subscribe before starting work so we don't miss events.
    let rx = state.task_store.subscribe(&task_id).await;

    // Spawn background work.
    let store = state.task_store.clone();
    let parts = req.message.parts.clone();
    let ctx_id = task.context_id.clone();
    tokio::spawn(async move {
        // Transition to Working.
        store
            .update_status(&task_id, TaskState::TaskStateWorking, None)
            .await;

        // Simulate processing.
        // TODO: Wire to Orchestrator for real LLM streaming.
        let user_text: String = parts
            .iter()
            .filter_map(|p| p.text.as_deref())
            .collect::<Vec<_>>()
            .join("\n");

        let reply_text = format!(
            "Received your message ({} chars). Processing is not yet wired to the LLM backend.",
            user_text.len()
        );
        let agent_msg = Message {
            message_id: Uuid::new_v4().to_string(),
            context_id: Some(ctx_id),
            task_id: Some(task_id.clone()),
            role: Role::RoleAgent,
            parts: vec![Part::text(reply_text)],
            metadata: None,
            extensions: vec![],
            reference_task_ids: vec![],
        };

        store.append_history(&task_id, agent_msg.clone()).await;

        store
            .update_status(&task_id, TaskState::TaskStateCompleted, Some(agent_msg))
            .await;

        store.cleanup_subscribers(&task_id).await;
    });

    // Stream task snapshots as SSE events.
    let stream = match rx {
        Some(rx) => {
            let stream = UnboundedReceiverStream::new(rx);
            stream
                .map(|task_snapshot| {
                    let resp = StreamResponse::from_task(task_snapshot);
                    let data = serde_json::to_string(&resp).unwrap_or_default();
                    Ok(Event::default().data(data))
                })
                .chain(futures::stream::once(async {
                    Ok(Event::default().data("[DONE]"))
                }))
                .boxed()
        }
        None => {
            // Task already terminal or doesn't exist -- send current state.
            let task_snapshot = state.task_store.get_task(&task.id).await;
            futures::stream::once(async move {
                let data = match task_snapshot {
                    Some(t) => {
                        serde_json::to_string(&StreamResponse::from_task(t)).unwrap_or_default()
                    }
                    None => "[DONE]".to_string(),
                };
                Ok::<_, Infallible>(Event::default().data(data))
            })
            .boxed()
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// -- Task operations --

/// `GET /tasks/:id` -- Gets the latest state of a task.
pub async fn get_task(State(state): State<A2AState>, Path(id): Path<String>) -> Response {
    match state.task_store.get_task(&id).await {
        Some(task) => Json(task).into_response(),
        None => not_found(format!("Task '{id}' not found")),
    }
}

/// Query parameters for `GET /tasks`.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksQuery {
    pub context_id: Option<String>,
    pub status: Option<TaskState>,
    pub page_size: Option<i32>,
}

/// `GET /tasks` -- Lists tasks matching optional filters.
pub async fn list_tasks(
    State(state): State<A2AState>,
    Query(query): Query<ListTasksQuery>,
) -> Json<ListTasksResponse> {
    let limit = query.page_size.unwrap_or(50).clamp(1, 100) as usize;

    let tasks = state
        .task_store
        .list_tasks(query.context_id.as_deref(), query.status, limit)
        .await;

    let total_size = tasks.len() as i32;
    Json(ListTasksResponse {
        tasks,
        next_page_token: String::new(),
        page_size: limit as i32,
        total_size,
    })
}

/// `POST /tasks/:id/cancel` -- Cancels a task.
pub async fn cancel_task(State(state): State<A2AState>, Path(id): Path<String>) -> Response {
    match state.task_store.cancel_task(&id).await {
        Some(task) => Json(task).into_response(),
        None => not_found(format!("Task '{id}' not found")),
    }
}

/// `GET /tasks/:id/subscribe` -- Subscribes to task updates (SSE).
pub async fn subscribe_to_task(State(state): State<A2AState>, Path(id): Path<String>) -> Response {
    let rx = state.task_store.subscribe(&id).await;

    match rx {
        Some(rx) => {
            let task_id = id.clone();
            let store = state.task_store.clone();

            let stream = UnboundedReceiverStream::new(rx)
                .map(|task_snapshot| {
                    let resp = StreamResponse::from_task(task_snapshot);
                    let data = serde_json::to_string(&resp).unwrap_or_default();
                    Ok::<_, Infallible>(Event::default().data(data))
                })
                .chain(futures::stream::once(async {
                    Ok(Event::default().data("[DONE]"))
                }));

            // Cleanup when the stream ends.
            let cleanup_store = store.clone();
            let cleanup_id = task_id.clone();
            tokio::spawn(async move {
                // Wait a bit for stream to settle then clean up.
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                cleanup_store.cleanup_subscribers(&cleanup_id).await;
            });

            Sse::new(stream)
                .keep_alive(KeepAlive::default())
                .into_response()
        }
        None => {
            // Task doesn't exist or is already terminal.
            let task = state.task_store.get_task(&id).await;
            match task {
                Some(t) if t.status.state.is_terminal() => bad_request(format!(
                    "Task '{id}' is already in terminal state: {:?}",
                    t.status.state
                )),
                _ => not_found(format!("Task '{id}' not found")),
            }
        }
    }
}

// -- Push notification config operations --

/// `POST /tasks/:task_id/pushNotificationConfigs`
pub async fn create_push_notification_config(
    State(state): State<A2AState>,
    Path(task_id): Path<String>,
    Json(req): Json<CreateTaskPushNotificationConfigRequest>,
) -> Response {
    match state
        .task_store
        .create_push_config(&task_id, req.config)
        .await
    {
        Some(config) => (StatusCode::CREATED, Json(config)).into_response(),
        None => not_found(format!("Task '{task_id}' not found")),
    }
}

/// `GET /tasks/:task_id/pushNotificationConfigs/:config_id`
pub async fn get_push_notification_config(
    State(state): State<A2AState>,
    Path((task_id, config_id)): Path<(String, String)>,
) -> Response {
    match state.task_store.get_push_config(&task_id, &config_id).await {
        Some(config) => Json(config).into_response(),
        None => not_found(format!(
            "Push notification config '{config_id}' not found for task '{task_id}'"
        )),
    }
}

/// Query params for listing push notification configs.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ListPushConfigsQuery {
    pub page_size: Option<i32>,
    pub page_token: Option<String>,
}

/// `GET /tasks/:task_id/pushNotificationConfigs`
pub async fn list_push_notification_configs(
    State(state): State<A2AState>,
    Path(task_id): Path<String>,
    Query(_query): Query<ListPushConfigsQuery>,
) -> Json<ListTaskPushNotificationConfigsResponse> {
    let configs = state.task_store.list_push_configs(&task_id).await;
    Json(ListTaskPushNotificationConfigsResponse {
        configs,
        next_page_token: None,
    })
}

/// `DELETE /tasks/:task_id/pushNotificationConfigs/:config_id`
pub async fn delete_push_notification_config(
    State(state): State<A2AState>,
    Path((task_id, config_id)): Path<(String, String)>,
) -> Response {
    if state
        .task_store
        .delete_push_config(&task_id, &config_id)
        .await
    {
        StatusCode::NO_CONTENT.into_response()
    } else {
        not_found(format!(
            "Push notification config '{config_id}' not found for task '{task_id}'"
        ))
    }
}

/// Builds a default [`AgentCard`] for this assistant instance.
pub fn build_default_agent_card(base_url: &str) -> AgentCard {
    AgentCard {
        name: "Assistant".to_string(),
        description: "A general-purpose AI assistant with tool-use capabilities.".to_string(),
        supported_interfaces: vec![AgentInterface {
            url: base_url.to_string(),
            protocol_binding: "HTTP+JSON".to_string(),
            tenant: None,
            protocol_version: "1.0".to_string(),
        }],
        provider: None,
        version: env!("CARGO_PKG_VERSION").to_string(),
        documentation_url: None,
        capabilities: AgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
            extensions: vec![],
            extended_agent_card: None,
        },
        security_schemes: HashMap::new(),
        security_requirements: vec![],
        default_input_modes: vec!["text/plain".to_string()],
        default_output_modes: vec!["text/plain".to_string()],
        skills: vec![AgentSkill {
            id: "general-assistant".to_string(),
            name: "General Assistant".to_string(),
            description: "General-purpose conversational assistance with tool use.".to_string(),
            tags: vec![
                "general".to_string(),
                "conversation".to_string(),
                "tools".to_string(),
            ],
            examples: vec![
                "Help me write a function".to_string(),
                "Explain this error".to_string(),
                "Search the web for...".to_string(),
            ],
            input_modes: vec![],
            output_modes: vec![],
            security_requirements: vec![],
        }],
        signatures: vec![],
        icon_url: None,
    }
}
