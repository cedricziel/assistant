//! MCP Apps API: the app an external MCP tool attached to its result, and
//! the calls that app makes back to its server.
//!
//! - `POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call`
//! - `POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read`
//!
//! The server is the one stored with the tool call's app, reached with the
//! caller's own access. Only its tools whose visibility includes `app` can be
//! called, and every call goes through the space's permission policy.
//!
//! Spec: `openspec/changes/mcp-full-spec/specs/mcp-apps-host/spec.md`

use std::sync::Arc;

use axum::Extension;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tracing::warn;
use utoipa::ToSchema;
use uuid::Uuid;

use assistant_core::auth::AuthContext;
use assistant_core::mcp_app::{McpApp, McpAppCsp, McpAppPermissions, McpAppServer};
use assistant_core::permission::{Outcome, PermissionRequest};
use assistant_core::tool::TurnScope;
use assistant_core::types::conversation::{ExecutionContext, Interface, MessageRole};
use assistant_storage::ConversationStore as _;

use super::ApiState;
use super::conversations::{caller_store, owned_conversation};
use super::interactions::rejection_status;
use super::messages::caller_can_post;
use crate::errors::json_error;
use crate::openapi::ErrorBody;
use crate::space::{Access, SpaceCtx};

/// Largest accepted request body.
pub const MAX_APP_REQUEST_BODY_BYTES: usize = 256 * 1024;

// -- Requests --

/// Body of `POST .../app/tools-call`: a call the app makes to one of its
/// server's tools.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CallMcpAppToolRequest {
    /// The tool's name on the app's server.
    pub name: String,
    /// The tool's arguments.
    #[serde(default)]
    #[schema(value_type = Object)]
    pub arguments: Map<String, Value>,
    /// The user confirmed this call. Required for a tool without
    /// `readOnlyHint: true`, and for any call the permission policy leaves
    /// to a human; without it such a call is refused with `409`.
    #[serde(default)]
    pub confirmed: bool,
}

/// Body of `POST .../app/resources-read`.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadMcpAppResourceRequest {
    /// The resource to read from the app's server.
    pub uri: String,
}

// -- Views --

/// The interactive UI an external MCP tool declared for its result.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpAppView {
    /// Namespace of the server the tool came from, as in
    /// `mcp__{server}__{tool}`.
    pub server: String,
    /// The `ui://` resource the HTML was read from.
    pub resource_uri: String,
    /// The app document, served as `text/html;profile=mcp-app`.
    pub html: String,
    pub csp: McpAppCspView,
    pub permissions: McpAppPermissionsView,
    /// Whether the app asks to be drawn with a border.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefers_border: Option<bool>,
}

/// The origins the app may reach. An empty list means the restrictive default.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpAppCspView {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub connect_domains: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resource_domains: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frame_domains: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub base_uri_domains: Vec<String>,
}

/// The device permissions the app asks for.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct McpAppPermissionsView {
    pub camera: bool,
    pub microphone: bool,
    pub geolocation: bool,
    pub clipboard_write: bool,
}

impl From<McpApp> for McpAppView {
    fn from(app: McpApp) -> Self {
        Self {
            server: app.server,
            resource_uri: app.resource_uri,
            html: app.html,
            csp: app.csp.into(),
            permissions: app.permissions.into(),
            prefers_border: app.prefers_border,
        }
    }
}

impl From<McpAppCsp> for McpAppCspView {
    fn from(csp: McpAppCsp) -> Self {
        Self {
            connect_domains: csp.connect_domains,
            resource_domains: csp.resource_domains,
            frame_domains: csp.frame_domains,
            base_uri_domains: csp.base_uri_domains,
        }
    }
}

impl From<McpAppPermissions> for McpAppPermissionsView {
    fn from(p: McpAppPermissions) -> Self {
        Self {
            camera: p.camera,
            microphone: p.microphone,
            geolocation: p.geolocation,
            clipboard_write: p.clipboard_write,
        }
    }
}

/// The app stored on a tool row, if it parses.
pub(crate) fn stored_app(raw: Option<&str>) -> Option<McpApp> {
    serde_json::from_str(raw?).ok()
}

// -- Handlers --

/// `POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call`
///
/// Call a tool on the server of the app this tool call rendered, as the
/// caller. The tool must be on that server and visible to apps; the call is
/// checked against the permission policy, and one that needs a human (any
/// tool not marked read-only, an untrusted server, or a policy `ask`) runs
/// only with `confirmed: true`. Returns the MCP `CallToolResult`.
#[utoipa::path(
    post,
    path = "/api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/tools-call",
    params(
        ("conversation_id" = Uuid, Path, description = "Conversation UUID"),
        ("tool_call_id" = String, Path, description = "Id of the tool call that carries the app"),
    ),
    request_body = CallMcpAppToolRequest,
    responses(
        (status = 200, description = "The MCP CallToolResult", body = Object),
        (status = 400, description = "Malformed body", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Viewer access, missing conversations:write scope, a tool not on the app's server or not visible to apps, or denied by the permission policy", body = ErrorBody),
        (status = 404, description = "Conversation or tool call not found, or the tool call has no app", body = ErrorBody),
        (status = 409, description = "The call needs the user's confirmation", body = ErrorBody),
        (status = 413, description = "Body larger than 256 KiB", body = ErrorBody),
        (status = 502, description = "The app's server failed the call", body = ErrorBody),
        (status = 503, description = "The app's server is not available to the caller", body = ErrorBody),
    ),
    operation_id = "call_mcp_app_tool",
    tag = "conversations",
    security(("bearer_token" = [])),
)]
pub async fn call_mcp_app_tool(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path((conversation_id, tool_call_id)): Path<(String, String)>,
    body: Result<Json<CallMcpAppToolRequest>, JsonRejection>,
) -> Response {
    let request = match body {
        Ok(Json(request)) => request,
        Err(rejection) => return json_error(rejection_status(&rejection), rejection.body_text()),
    };
    let (conversation_id, app, server) =
        match app_server(&ctx, &state, &auth, &conversation_id, &tool_call_id).await {
            Ok(found) => found,
            Err(response) => return response,
        };
    let tool = match server.tool(&request.name) {
        Some(tool) if tool.visible_to_app => tool,
        _ => {
            return json_error(
                StatusCode::FORBIDDEN,
                format!(
                    "'{}' is not a tool the app's server '{}' offers to apps",
                    request.name, app.server
                ),
            );
        }
    };

    let runtime = match state.runtime(&ctx).await {
        Ok(runtime) => runtime,
        Err(response) => return response,
    };
    let tool_name = assistant_mcp_client::bridge::namespaced_name(&app.server, &request.name);
    let params = Value::Object(request.arguments);
    let exec_ctx = ExecutionContext {
        conversation_id,
        agent_id: state.agent_id.read().await.clone(),
        turn: 0,
        interface: Interface::Web,
        interactive: true,
        run_id: None,
        allowed_tools: None,
        depth: 0,
        user_id: None,
        org_id: None,
        space_id: None,
    }
    .with_auth(&ctx.auth);
    let resolution = runtime
        .orchestrator
        .permission_policy()
        .evaluate(&PermissionRequest {
            tool_name: &tool_name,
            params: &params,
            ctx: &exec_ctx,
            requires_confirmation: tool.needs_confirmation,
            task_context: None,
        })
        .await;
    if resolution.outcome == Outcome::Deny {
        let reason = resolution.refusal(&tool_name).unwrap_or_default();
        return json_error(StatusCode::FORBIDDEN, reason);
    }
    let needs_confirmation = tool.needs_confirmation || resolution.outcome == Outcome::Ask;
    if needs_confirmation && !request.confirmed {
        return json_error(
            StatusCode::CONFLICT,
            format!(
                "'{tool_name}' needs the user to confirm the call: {}",
                resolution.decision.reason
            ),
        );
    }

    let Value::Object(arguments) = params else {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Arguments are not an object",
        );
    };
    match server.call_tool(&request.name, arguments).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => upstream_error(&app, &e),
    }
}

/// `POST /api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read`
///
/// Read a resource from the server of the app this tool call rendered, as the
/// caller. Returns the MCP `ReadResourceResult`.
#[utoipa::path(
    post,
    path = "/api/conversations/{conversation_id}/tool-calls/{tool_call_id}/app/resources-read",
    params(
        ("conversation_id" = Uuid, Path, description = "Conversation UUID"),
        ("tool_call_id" = String, Path, description = "Id of the tool call that carries the app"),
    ),
    request_body = ReadMcpAppResourceRequest,
    responses(
        (status = 200, description = "The MCP ReadResourceResult", body = Object),
        (status = 400, description = "Malformed body", body = ErrorBody),
        (status = 401, description = "Unauthorized", body = ErrorBody),
        (status = 403, description = "Viewer access, or missing conversations:write scope", body = ErrorBody),
        (status = 404, description = "Conversation or tool call not found, or the tool call has no app", body = ErrorBody),
        (status = 413, description = "Body larger than 256 KiB", body = ErrorBody),
        (status = 502, description = "The app's server failed the read", body = ErrorBody),
        (status = 503, description = "The app's server is not available to the caller", body = ErrorBody),
    ),
    operation_id = "read_mcp_app_resource",
    tag = "conversations",
    security(("bearer_token" = [])),
)]
pub async fn read_mcp_app_resource(
    ctx: SpaceCtx,
    State(state): State<ApiState>,
    Extension(auth): Extension<AuthContext>,
    Path((conversation_id, tool_call_id)): Path<(String, String)>,
    body: Result<Json<ReadMcpAppResourceRequest>, JsonRejection>,
) -> Response {
    let request = match body {
        Ok(Json(request)) => request,
        Err(rejection) => return json_error(rejection_status(&rejection), rejection.body_text()),
    };
    let (_, app, server) =
        match app_server(&ctx, &state, &auth, &conversation_id, &tool_call_id).await {
            Ok(found) => found,
            Err(response) => return response,
        };
    match server.read_resource(&request.uri).await {
        Ok(result) => Json(result).into_response(),
        Err(e) => upstream_error(&app, &e),
    }
}

/// The caller's conversation, the app stored with its tool call, and that
/// app's server as the caller reaches it.
async fn app_server(
    ctx: &SpaceCtx,
    state: &ApiState,
    auth: &AuthContext,
    conversation_id: &str,
    tool_call_id: &str,
) -> Result<(Uuid, McpApp, Arc<dyn McpAppServer>), Response> {
    ctx.require(Access::Write)
        .map_err(IntoResponse::into_response)?;
    if !caller_can_post(auth) {
        return Err(json_error(
            StatusCode::FORBIDDEN,
            "Missing conversations:write scope",
        ));
    }
    let Ok(conversation_id) = Uuid::parse_str(conversation_id) else {
        return Err(json_error(
            StatusCode::BAD_REQUEST,
            "Invalid conversation ID",
        ));
    };
    let store = caller_store(ctx, state).await;
    owned_conversation(&store, conversation_id).await?;
    let history = store.load_history(conversation_id).await.map_err(|e| {
        warn!("Failed to load conversation {conversation_id}: {e:#}");
        json_error(StatusCode::INTERNAL_SERVER_ERROR, "Database error")
    })?;
    let app = history
        .into_iter()
        .filter(|m| matches!(m.role, MessageRole::Tool))
        .filter(|m| m.tool_call_id.as_deref() == Some(tool_call_id))
        .find_map(|m| stored_app(m.tool_app_json.as_deref()))
        .ok_or_else(|| json_error(StatusCode::NOT_FOUND, "This tool call has no app"))?;

    let scope = TurnScope {
        org_slug: &ctx.org_slug,
        space_slug: &ctx.space_slug,
        user_id: Some(&ctx.auth.user_id),
    };
    let unavailable = || {
        json_error(
            StatusCode::SERVICE_UNAVAILABLE,
            format!("The app's server '{}' is not available", app.server),
        )
    };
    match state.mcp_apps.server(&scope, &app.server).await {
        Ok(Some(server)) => Ok((conversation_id, app, server)),
        Ok(None) => Err(unavailable()),
        Err(e) => {
            warn!(server = %app.server, "MCP app server lookup failed: {e:#}");
            Err(unavailable())
        }
    }
}

fn upstream_error(app: &McpApp, e: &anyhow::Error) -> Response {
    warn!(server = %app.server, "MCP app request failed: {e:#}");
    json_error(
        StatusCode::BAD_GATEWAY,
        format!(
            "The app's server '{}' failed the request: {e:#}",
            app.server
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{Map, Value, json};
    use tower::ServiceExt;
    use uuid::Uuid;

    use assistant_core::mcp_app::{McpAppServer, McpAppServers, McpAppTool};
    use assistant_core::tool::TurnScope;
    use assistant_core::types::agent::OrgConfig;
    use assistant_core::types::conversation::{Message, MessageRole};
    use assistant_storage::{ConversationStore as _, SqliteConversationStore, StorageLayer};

    use super::super::test_helpers::{app, body_json, default_space, state_over_config};
    use crate::api::ApiState;

    /// The `weather` server: `forecast` and `radar` are read-only, `book` and
    /// `nuke` mutate, and `secret` is for the model only.
    #[derive(Default)]
    struct Weather {
        calls: Mutex<Vec<String>>,
        callers: Mutex<Vec<Option<String>>>,
    }

    #[async_trait]
    impl McpAppServer for Weather {
        fn tool(&self, name: &str) -> Option<McpAppTool> {
            let (visible_to_app, needs_confirmation) = match name {
                "forecast" | "radar" => (true, false),
                "book" | "nuke" => (true, true),
                "secret" => (false, false),
                _ => return None,
            };
            Some(McpAppTool {
                visible_to_app,
                needs_confirmation,
            })
        }

        async fn call_tool(
            &self,
            name: &str,
            arguments: Map<String, Value>,
        ) -> anyhow::Result<Value> {
            self.calls.lock().unwrap().push(name.to_string());
            Ok(
                json!({"content": [{"type": "text", "text": format!("called {name}")}], "arguments": arguments}),
            )
        }

        async fn read_resource(&self, uri: &str) -> anyhow::Result<Value> {
            Ok(json!({"contents": [{"uri": uri, "text": "<p>more</p>"}]}))
        }
    }

    struct Servers(Arc<Weather>);

    #[async_trait]
    impl McpAppServers for Servers {
        async fn server(
            &self,
            scope: &TurnScope<'_>,
            server: &str,
        ) -> anyhow::Result<Option<Arc<dyn McpAppServer>>> {
            self.0
                .callers
                .lock()
                .unwrap()
                .push(scope.user_id.map(|u| u.0.clone()));
            Ok((server == "weather").then(|| self.0.clone() as Arc<dyn McpAppServer>))
        }
    }

    struct Fx {
        state: ApiState,
        weather: Arc<Weather>,
        conv: Uuid,
    }

    const CALL: &str = "call_app";

    /// A conversation with one `mcp__{server}__forecast` call that carries an
    /// app, under a policy that allows `forecast` and `book` and denies `nuke`.
    async fn fx_for(server: &str) -> Fx {
        let storage = Arc::new(StorageLayer::new_in_memory().await.unwrap());
        let mut config = OrgConfig::default();
        config.memory.enabled = false;
        config.permissions.allow = vec![
            "mcp__weather__forecast".parse().unwrap(),
            "mcp__weather__book".parse().unwrap(),
        ];
        config.permissions.deny = vec!["mcp__weather__nuke".parse().unwrap()];
        let (state, _) = state_over_config(
            std::path::Path::new("/nonexistent/assistant-test-root"),
            "http://127.0.0.1:1",
            &[default_space(&storage)],
            false,
            config,
        )
        .await;
        let weather = Arc::new(Weather::default());
        let state = state.with_mcp_apps(Arc::new(Servers(weather.clone())));

        let store = SqliteConversationStore::for_agent(storage.pool.clone(), "default")
            .with_user_id("system");
        let conv = store.create_conversation(None).await.unwrap().id;
        let mut row = Message::new(conv, MessageRole::Tool, "21 degrees");
        row.skill_name = Some(format!("mcp__{server}__forecast"));
        row.tool_call_id = Some(CALL.into());
        row.tool_status = Some("ok".into());
        row.tool_app_json = Some(
            json!({"server": server, "resource_uri": "ui://weather/forecast", "html": "<p/>"})
                .to_string(),
        );
        store.save_message(&row).await.unwrap();
        Fx {
            state,
            weather,
            conv,
        }
    }

    async fn fx() -> Fx {
        fx_for("weather").await
    }

    impl Fx {
        async fn post(&self, call: &str, action: &str, body: Value) -> (StatusCode, Value) {
            let response = app(self.state.clone())
                .oneshot(
                    Request::post(format!(
                        "/conversations/{}/tool-calls/{call}/app/{action}",
                        self.conv
                    ))
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
                )
                .await
                .unwrap();
            let status = response.status();
            (status, body_json(response.into_body()).await)
        }

        async fn call(&self, body: Value) -> (StatusCode, Value) {
            self.post(CALL, "tools-call", body).await
        }

        fn calls(&self) -> Vec<String> {
            self.weather.calls.lock().unwrap().clone()
        }
    }

    #[tokio::test]
    async fn read_only_tool_allowed_by_policy_runs_as_the_caller() {
        let fx = fx().await;
        let (status, body) = fx
            .call(json!({"name": "forecast", "arguments": {"city": "Kiel"}}))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["content"][0]["text"], "called forecast");
        assert_eq!(body["arguments"]["city"], "Kiel");
        assert_eq!(
            fx.weather.callers.lock().unwrap().clone(),
            vec![Some("system".to_string())],
            "the server is reached as the calling user"
        );
    }

    #[tokio::test]
    async fn mutating_tool_needs_confirmation() {
        let fx = fx().await;
        let (status, body) = fx.call(json!({"name": "book"})).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert!(
            body["error"].as_str().unwrap().contains("confirm"),
            "{body}"
        );
        assert!(fx.calls().is_empty(), "nothing is sent unconfirmed");
    }

    #[tokio::test]
    async fn confirmed_mutating_tool_runs() {
        let fx = fx().await;
        let (status, body) = fx.call(json!({"name": "book", "confirmed": true})).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(fx.calls(), vec!["book"]);
    }

    #[tokio::test]
    async fn tool_hidden_from_apps_is_forbidden() {
        let fx = fx().await;
        let (status, body) = fx.call(json!({"name": "secret", "confirmed": true})).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert!(fx.calls().is_empty());
    }

    #[tokio::test]
    async fn another_servers_tool_is_forbidden() {
        let fx = fx().await;
        let (status, body) = fx
            .call(json!({"name": "mcp__github__create_issue", "confirmed": true}))
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert!(fx.calls().is_empty());
    }

    #[tokio::test]
    async fn policy_denial_wins_over_confirmation() {
        let fx = fx().await;
        let (status, body) = fx.call(json!({"name": "nuke", "confirmed": true})).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert!(fx.calls().is_empty());
    }

    #[tokio::test]
    async fn read_only_tool_the_policy_leaves_open_needs_confirmation() {
        let fx = fx().await;
        let (status, body) = fx.call(json!({"name": "radar"})).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        let (status, _) = fx.call(json!({"name": "radar", "confirmed": true})).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "a confirmation answers the policy's ask"
        );
        assert_eq!(fx.calls(), vec!["radar"]);
    }

    #[tokio::test]
    async fn tool_call_without_an_app_is_not_found() {
        let fx = fx().await;
        let (status, _) = fx
            .post("no-such-call", "tools-call", json!({"name": "forecast"}))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unavailable_server_is_service_unavailable() {
        let fx = fx_for("gone").await;
        let (status, body) = fx.call(json!({"name": "forecast"})).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    }

    #[tokio::test]
    async fn resources_read_reaches_the_apps_server() {
        let fx = fx().await;
        let (status, body) = fx
            .post(CALL, "resources-read", json!({"uri": "ui://weather/more"}))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["contents"][0]["uri"], "ui://weather/more");
    }

    #[tokio::test]
    async fn resources_read_without_an_app_is_not_found() {
        let fx = fx().await;
        let (status, _) = fx
            .post("no-such-call", "resources-read", json!({"uri": "ui://x"}))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
