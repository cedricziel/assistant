//! Axum HTTP handlers for the A2A protocol.
//!
//! Each handler maps to one RPC in the `A2AService`. The handlers operate on
//! the in-memory [`TaskStore`] and produce/consume the canonical JSON types
//! from `assistant_a2a_json_schema`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::Extension;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use sqlx::SqlitePool;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

use assistant_a2a_json_schema::agent_card::*;
use assistant_a2a_json_schema::requests::*;
use assistant_a2a_json_schema::responses::*;
use assistant_a2a_json_schema::types::*;
use assistant_core::auth::AuthContext;
use assistant_core::types::conversation::Interface;
use assistant_runtime::AssistantInterface;
use assistant_runtime::projection::StreamProjector;
use assistant_storage::{ConversationStore, SqliteConversationStore};

use super::task_store::A2aTaskStore;
use crate::api::messages::caller_can_post;
use crate::openapi::ApiErrorResponse;

// -- Shared state --

/// Shared state for all A2A handlers.
#[derive(Clone)]
pub struct A2AState {
    /// Durable A2A task store, scoped to the active agent's space db.
    pub task_store: Arc<dyn A2aTaskStore>,
    /// The agent card describing this agent.
    pub agent_card: AgentCard,
    /// Orchestrator used to run real turns for A2A messages.
    pub orchestrator: Arc<dyn AssistantInterface>,
    /// Space db pool, for resolving/creating conversations.
    pub pool: SqlitePool,
    /// The active agent id (space scope) for conversation resolution.
    pub agent_id: String,
}

/// Resolve the conversation for an A2A message. A returned `context_id` is the
/// conversation UUID we previously surfaced, so reuse it when it parses and
/// exists; otherwise start a fresh conversation.
async fn resolve_conversation(store: &SqliteConversationStore, context_id: Option<&str>) -> Uuid {
    if let Some(ctx) = context_id
        && let Ok(id) = Uuid::parse_str(ctx)
        && matches!(store.get_conversation(id).await, Ok(Some(_)))
    {
        return id;
    }
    match store.create_conversation(None).await {
        Ok(conv) => conv.id,
        Err(_) => Uuid::new_v4(),
    }
}

/// Build an agent-authored A2A message scoped to a task.
fn agent_message(task: &Task, text: impl Into<String>) -> Message {
    Message {
        message_id: Uuid::new_v4().to_string(),
        context_id: Some(task.context_id.clone()),
        task_id: Some(task.id.clone()),
        role: Role::RoleAgent,
        parts: vec![Part::text(text)],
        metadata: None,
        extensions: vec![],
        reference_task_ids: vec![],
    }
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
#[utoipa::path(
    get,
    path = "/.well-known/agent.json",
    responses(
        (status = 200, description = "Public agent card", body = AgentCard,
         content_type = "application/json"),
        (status = 404, description = "Agent card not found")
    ),
    tag = "agent-card",
    security()
)]
pub async fn get_agent_card_well_known(State(state): State<A2AState>) -> Json<AgentCard> {
    Json(state.agent_card.clone())
}

/// `GET /agent/authenticatedExtendedCard` -- Returns the extended agent card
/// (same as public for now).
#[utoipa::path(
    get,
    path = "/agent/authenticatedExtendedCard",
    responses(
        (status = 200, description = "Authenticated extended agent card", body = AgentCard,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "agent-card",
    security(("bearer_token" = []))
)]
pub async fn get_extended_agent_card(State(state): State<A2AState>) -> Json<AgentCard> {
    Json(state.agent_card.clone())
}

// -- SendMessage --

/// `POST /message/send` -- Sends a message to the agent (unary).
///
/// Creates a task, records the user message, transitions to Working, produces
/// an agent reply, and returns the final task state.
#[utoipa::path(
    post,
    path = "/message/send",
    operation_id = "a2a_send_message",
    request_body(content = SendMessageRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Task created and completed", body = SendMessageResponse,
         content_type = "application/json"),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "messages",
    security(("bearer_token" = []))
)]
pub async fn send_message(
    State(state): State<A2AState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    if !caller_can_post(&auth) {
        return (StatusCode::FORBIDDEN, "Missing conversations:write scope").into_response();
    }

    let user_text: String = req
        .message
        .parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    // Resolve (or create) the conversation, then key the task to it so the
    // client can reuse the returned context_id for follow-up turns.
    let conv_store = SqliteConversationStore::for_agent(state.pool.clone(), &state.agent_id);
    let conv_id = resolve_conversation(&conv_store, req.message.context_id.as_deref()).await;

    let task = state
        .task_store
        .create_task(Some(conv_id.to_string()))
        .await;
    let mut user_msg = req.message.clone();
    user_msg.task_id = Some(task.id.clone());
    state.task_store.append_history(&task.id, user_msg).await;
    state
        .task_store
        .update_status(&task.id, TaskState::TaskStateWorking, None)
        .await;

    // Run the real turn through the Orchestrator.
    let result = state
        .orchestrator
        .submit_turn(&auth, &user_text, conv_id, Interface::A2a, None)
        .await;

    let answer = match result {
        Ok(turn) => turn.answer,
        Err(e) => {
            let err_msg = agent_message(&task, format!("Turn failed: {e}"));
            state
                .task_store
                .update_status(&task.id, TaskState::TaskStateFailed, Some(err_msg))
                .await;
            let failed = state.task_store.get_task(&task.id).await;
            return Json(SendMessageResponse {
                task: failed,
                message: None,
            })
            .into_response();
        }
    };

    let agent_msg = agent_message(&task, answer);
    state
        .task_store
        .append_history(&task.id, agent_msg.clone())
        .await;
    state
        .task_store
        .update_status(&task.id, TaskState::TaskStateCompleted, Some(agent_msg))
        .await;

    let Some(final_task) = state.task_store.get_task(&task.id).await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "task disappeared after creation"})),
        )
            .into_response();
    };

    Json(SendMessageResponse {
        task: Some(final_task),
        message: None,
    })
    .into_response()
}

/// `POST /message/stream` -- Sends a message with streaming response (SSE).
#[utoipa::path(
    post,
    path = "/message/stream",
    operation_id = "a2a_send_message_streaming",
    request_body(content = SendMessageRequest, content_type = "application/json"),
    responses(
        (status = 200, description = "Streaming SSE response; each event is a JSON-encoded StreamResponse",
         content_type = "text/event-stream", body = StreamResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Missing conversations:write scope")
    ),
    tag = "messages",
    security(("bearer_token" = []))
)]
pub async fn send_message_streaming(
    State(state): State<A2AState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<SendMessageRequest>,
) -> Response {
    if !caller_can_post(&auth) {
        return (StatusCode::FORBIDDEN, "Missing conversations:write scope").into_response();
    }

    let user_text: String = req
        .message
        .parts
        .iter()
        .filter_map(|p| p.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    let conv_store = SqliteConversationStore::for_agent(state.pool.clone(), &state.agent_id);
    let conv_id = resolve_conversation(&conv_store, req.message.context_id.as_deref()).await;

    let task = state
        .task_store
        .create_task(Some(conv_id.to_string()))
        .await;
    let task_id = task.id.clone();
    let ctx = task.context_id.clone();
    let mut user_msg = req.message.clone();
    user_msg.task_id = Some(task_id.clone());
    state.task_store.append_history(&task_id, user_msg).await;
    state
        .task_store
        .update_status(&task_id, TaskState::TaskStateWorking, None)
        .await;

    // SSE output channel + orchestrator event sink.
    let (sse_tx, sse_rx) = tokio::sync::mpsc::unbounded_channel::<Result<Event, Infallible>>();
    let (event_tx, mut event_rx) =
        tokio::sync::mpsc::channel::<assistant_runtime::OrchestratorEvent>(64);
    state
        .orchestrator
        .register_token_sink(conv_id, event_tx)
        .await;

    // Run the turn; capture the final result via a oneshot.
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    let orchestrator = state.orchestrator.clone();
    let turn_auth = auth.clone();
    let turn_text = user_text.clone();
    tokio::spawn(async move {
        let r = orchestrator
            .submit_turn(&turn_auth, &turn_text, conv_id, Interface::A2a, None)
            .await;
        let _ = result_tx.send(r);
    });

    // Drain orchestrator events -> A2A StreamResponse frames -> SSE, then persist
    // the final answer and emit the terminal task snapshot.
    let projector = crate::a2a::projection::A2aProjector::new(task_id.clone(), ctx.clone());
    let task_store = state.task_store.clone();
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            for frame in projector.project(&event) {
                let data = serde_json::to_string(&frame).unwrap_or_default();
                if sse_tx.send(Ok(Event::default().data(data))).is_err() {
                    return;
                }
            }
        }

        // The turn finished (its event sink was dropped). Finalize the task.
        match result_rx.await {
            Ok(Ok(turn)) => {
                let agent_msg = Message {
                    message_id: Uuid::new_v4().to_string(),
                    context_id: Some(ctx.clone()),
                    task_id: Some(task_id.clone()),
                    role: Role::RoleAgent,
                    parts: vec![Part::text(turn.answer)],
                    metadata: None,
                    extensions: vec![],
                    reference_task_ids: vec![],
                };
                task_store.append_history(&task_id, agent_msg.clone()).await;
                task_store
                    .update_status(&task_id, TaskState::TaskStateCompleted, Some(agent_msg))
                    .await;
            }
            _ => {
                task_store
                    .update_status(&task_id, TaskState::TaskStateFailed, None)
                    .await;
            }
        }

        if let Some(final_task) = task_store.get_task(&task_id).await {
            let data =
                serde_json::to_string(&StreamResponse::from_task(final_task)).unwrap_or_default();
            let _ = sse_tx.send(Ok(Event::default().data(data)));
        }
        let _ = sse_tx.send(Ok(Event::default().data("[DONE]")));
        task_store.cleanup_subscribers(&task_id).await;
    });

    Sse::new(UnboundedReceiverStream::new(sse_rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}

// -- Task operations --

/// `GET /tasks/:id` -- Gets the latest state of a task.
#[utoipa::path(
    get,
    path = "/tasks/{id}",
    params(
        ("id" = String, Path, description = "Task ID"),
        GetTaskQuery
    ),
    responses(
        (status = 200, description = "Task details", body = Task,
         content_type = "application/json"),
        (status = 404, description = "Task not found", body = ApiErrorResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "tasks",
    security(("bearer_token" = []))
)]
pub async fn get_task(
    State(state): State<A2AState>,
    Path(id): Path<String>,
    Query(_query): Query<GetTaskQuery>,
) -> Response {
    match state.task_store.get_task(&id).await {
        Some(task) => Json(task).into_response(),
        None => not_found(format!("Task '{id}' not found")),
    }
}

/// Query parameters for `GET /tasks/{id}`.
///
/// `history_length` and `tenant` are accepted per the A2A protocol spec
/// for forward-compatibility but not yet plumbed through the in-memory
/// `TaskStore`. Kept on the struct so the OpenAPI surface matches the
/// protocol; remove the `#[allow(dead_code)]` once they're wired up.
#[derive(Debug, Default, serde::Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct GetTaskQuery {
    /// Max number of recent messages to include in history.
    #[allow(dead_code)]
    pub history_length: Option<i32>,
    /// Optional tenant ID.
    #[allow(dead_code)]
    pub tenant: Option<String>,
}

/// `GET /tasks` -- Lists tasks matching optional filters.
#[utoipa::path(
    get,
    path = "/tasks",
    params(ListTasksRequest),
    responses(
        (status = 200, description = "Paginated task list", body = ListTasksResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "tasks",
    security(("bearer_token" = []))
)]
pub async fn list_tasks(
    State(state): State<A2AState>,
    Query(query): Query<ListTasksRequest>,
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
#[utoipa::path(
    post,
    path = "/tasks/{id}/cancel",
    params(
        ("id" = String, Path, description = "Task ID to cancel")
    ),
    request_body(content = CancelTaskRequest, content_type = "application/json",
                 description = "Optional cancellation metadata"),
    responses(
        (status = 200, description = "Cancelled task", body = Task,
         content_type = "application/json"),
        (status = 404, description = "Task not found", body = ApiErrorResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "tasks",
    security(("bearer_token" = []))
)]
pub async fn cancel_task(
    State(state): State<A2AState>,
    Path(id): Path<String>,
    body: Option<axum::Json<CancelTaskRequest>>,
) -> Response {
    let _ = body; // body fields are available for future use (e.g. metadata)
    match state.task_store.cancel_task(&id).await {
        Some(task) => Json(task).into_response(),
        None => not_found(format!("Task '{id}' not found")),
    }
}

/// `GET /tasks/:id/subscribe` -- Subscribes to task updates (SSE).
#[utoipa::path(
    get,
    path = "/tasks/{id}/subscribe",
    params(
        ("id" = String, Path, description = "Task ID to subscribe to")
    ),
    responses(
        (status = 200, description = "SSE stream of StreamResponse events",
         content_type = "text/event-stream", body = StreamResponse),
        (status = 400, description = "Task already in terminal state", body = ApiErrorResponse,
         content_type = "application/json"),
        (status = 404, description = "Task not found", body = ApiErrorResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "tasks",
    security(("bearer_token" = []))
)]
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
#[utoipa::path(
    post,
    path = "/tasks/{task_id}/pushNotificationConfigs",
    params(
        ("task_id" = String, Path, description = "Parent task ID")
    ),
    request_body(content = CreateTaskPushNotificationConfigRequest, content_type = "application/json"),
    responses(
        (status = 201, description = "Push notification config created", body = TaskPushNotificationConfig,
         content_type = "application/json"),
        (status = 404, description = "Task not found", body = ApiErrorResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "push-notifications",
    security(("bearer_token" = []))
)]
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
#[utoipa::path(
    get,
    path = "/tasks/{task_id}/pushNotificationConfigs/{config_id}",
    params(
        ("task_id" = String, Path, description = "Parent task ID"),
        ("config_id" = String, Path, description = "Push notification config ID")
    ),
    responses(
        (status = 200, description = "Push notification config", body = TaskPushNotificationConfig,
         content_type = "application/json"),
        (status = 404, description = "Config not found", body = ApiErrorResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "push-notifications",
    security(("bearer_token" = []))
)]
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
#[derive(Debug, Default, serde::Deserialize, utoipa::IntoParams)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ListPushConfigsQuery {
    pub page_size: Option<i32>,
    pub page_token: Option<String>,
}

/// `GET /tasks/:task_id/pushNotificationConfigs`
#[utoipa::path(
    get,
    path = "/tasks/{task_id}/pushNotificationConfigs",
    params(
        ("task_id" = String, Path, description = "Parent task ID"),
        ListPushConfigsQuery
    ),
    responses(
        (status = 200, description = "List of push notification configs",
         body = ListTaskPushNotificationConfigsResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "push-notifications",
    security(("bearer_token" = []))
)]
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
#[utoipa::path(
    delete,
    path = "/tasks/{task_id}/pushNotificationConfigs/{config_id}",
    params(
        ("task_id" = String, Path, description = "Parent task ID"),
        ("config_id" = String, Path, description = "Push notification config ID")
    ),
    responses(
        (status = 204, description = "Config deleted"),
        (status = 404, description = "Config not found", body = ApiErrorResponse,
         content_type = "application/json"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "push-notifications",
    security(("bearer_token" = []))
)]
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

#[cfg(test)]
pub(crate) mod test_support {
    //! Shared helpers for the A2A handler/router tests: a no-op orchestrator and
    //! an `A2AState` backed by in-memory storage.
    use std::sync::Arc;

    use assistant_core::auth::AuthContext;
    use assistant_core::types::conversation::Interface;
    use assistant_runtime::{AssistantInterface, OrchestratorEvent, TurnResult};
    use async_trait::async_trait;
    use tokio::sync::mpsc;
    use uuid::Uuid;

    use super::{A2AState, build_default_agent_card};
    use crate::a2a::task_store::InMemoryA2aTaskStore;

    /// Minimal `AssistantInterface` that echoes the prompt as the answer.
    pub struct EchoInterface;

    #[async_trait]
    impl AssistantInterface for EchoInterface {
        async fn register_token_sink(
            &self,
            _conversation_id: Uuid,
            _sink: mpsc::Sender<OrchestratorEvent>,
        ) {
        }

        async fn submit_turn_with_attachments(
            &self,
            _auth: &AuthContext,
            prompt: &str,
            _conversation_id: Uuid,
            _interface: Interface,
            _timestamp: Option<chrono::DateTime<chrono::Utc>>,
            _attachment_ids: Vec<Uuid>,
        ) -> anyhow::Result<TurnResult> {
            Ok(TurnResult {
                answer: format!("echo: {prompt}"),
                attachments: vec![],
                attachment_ids: vec![],
                message_id: None,
                had_errors: false,
            })
        }

        async fn run_boot(
            &self,
            _conversation_id: Uuid,
            _interface: Interface,
        ) -> anyhow::Result<bool> {
            Ok(false)
        }
    }

    /// Build an `A2AState` backed by in-memory storage + an echo orchestrator.
    pub async fn test_state(base_url: &str) -> A2AState {
        let storage = assistant_storage::StorageLayer::new_in_memory()
            .await
            .expect("in-memory storage");
        A2AState {
            task_store: Arc::new(InMemoryA2aTaskStore::new()),
            agent_card: build_default_agent_card(base_url),
            orchestrator: Arc::new(EchoInterface),
            pool: storage.pool.clone(),
            agent_id: "default".to_string(),
        }
    }
}
