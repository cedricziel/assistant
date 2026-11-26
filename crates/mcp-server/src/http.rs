//! Serving MCP over Streamable HTTP.
//!
//! One [`StreamableHttpService`] serves every space. The HTTP layer in front
//! of it authenticates each request, picks the caller's space and attaches
//! that space's [`AssistantMcpServer`] to the request extensions;
//! [`RequestBoundServer`] then hands the request to it.

use std::borrow::Cow;
use std::sync::Arc;

use http::request::Parts;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CompleteRequestParams, CompleteResult, Extensions,
    GetPromptRequestParams, GetPromptResponse, ListPromptsResult, ListResourceTemplatesResult,
    ListResourcesResult, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
    ReadResourceRequestParams, ReadResourceResponse, ServerConfig, SubscriptionFilter,
};
use rmcp::service::{NotificationContext, RequestContext, SubscriptionContext};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{ErrorData, RoleServer, ServerHandler};
use tracing::debug;

use crate::server::{AssistantMcpServer, SUPPORTED_VERSIONS, listenable, server_info};

/// The Streamable HTTP service for `/mcp`.
///
/// Sessions stay on for `2025-11-25` clients; `2026-07-28` requests are
/// stateless regardless. Responses stream so progress can reach the client.
/// rmcp's own `Host` allowlist is off: the server runs under whatever public
/// host it is deployed at, and the HTTP layer checks `Origin` itself.
pub fn streamable_http_service() -> StreamableHttpService<RequestBoundServer, LocalSessionManager> {
    let mut config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(true)
        .disable_allowed_hosts();
    config.json_response = false;
    StreamableHttpService::new(
        || Ok(RequestBoundServer),
        Arc::new(LocalSessionManager::default()),
        config,
    )
}

/// Serves each request with the `Arc<AssistantMcpServer>` found in its HTTP
/// request extensions.
#[derive(Clone, Copy, Debug)]
pub struct RequestBoundServer;

fn bound(extensions: &Extensions) -> Result<Arc<AssistantMcpServer>, ErrorData> {
    extensions
        .get::<Parts>()
        .and_then(|parts| parts.extensions.get::<Arc<AssistantMcpServer>>())
        .cloned()
        .ok_or_else(|| ErrorData::internal_error("no MCP server bound to this request", None))
}

impl ServerHandler for RequestBoundServer {
    fn get_info(&self) -> ServerConfig {
        server_info()
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(SUPPORTED_VERSIONS)
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        bound(&context.extensions)?
            .list_tools(request, context)
            .await
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        bound(&context.extensions)?
            .call_tool(request, context)
            .await
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        bound(&context.extensions)?
            .list_resources(request, context)
            .await
    }

    async fn list_resource_templates(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, ErrorData> {
        bound(&context.extensions)?
            .list_resource_templates(request, context)
            .await
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        bound(&context.extensions)?
            .read_resource(request, context)
            .await
    }

    async fn list_prompts(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
        bound(&context.extensions)?
            .list_prompts(request, context)
            .await
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<GetPromptResponse, ErrorData> {
        bound(&context.extensions)?
            .get_prompt(request, context)
            .await
    }

    async fn complete(
        &self,
        request: CompleteRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CompleteResult, ErrorData> {
        bound(&context.extensions)?.complete(request, context).await
    }

    fn accepted_subscription_filter(
        &self,
        _requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        Some(listenable())
    }

    async fn listen(&self, subscription: SubscriptionContext) -> Result<(), ErrorData> {
        bound(&subscription.request_context().extensions)?
            .listen(subscription)
            .await
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        match bound(&context.extensions) {
            Ok(server) => server.on_initialized(context).await,
            Err(error) => debug!(%error, "initialized session not registered for notifications"),
        }
    }
}
