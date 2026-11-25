//! The MCP endpoint: Streamable HTTP at `/mcp`, protected as an OAuth 2.0
//! resource server.
//!
//! `/mcp` accepts JWTs issued for its canonical URL (RFC 8707) and `ask_live_`
//! API keys that carry `mcp:execute`. Each request is bound to one space, the
//! way the web API binds it, and served by that space's runtime. See
//! `openspec/changes/mcp-full-spec/specs/mcp-http-endpoint/spec.md`.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{ORIGIN, WWW_AUTHENTICATE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tracing::error;

use assistant_auth::api_keys;
use assistant_auth::middleware::{AuthState, SessionError};
use assistant_core::auth::AuthContext;
use assistant_core::identity::{Action, ResourceKind};
use assistant_mcp_server::{AssistantMcpServer, streamable_http_service};
use assistant_runtime::SpaceRuntimes;

use crate::auth::{bearer_token, is_same_origin};
use crate::oauth::mcp_resource;
use crate::space::{SPACE_HEADER, SpaceResolver};

#[cfg(test)]
mod tests;

const MCP_SCOPE: &str = "mcp:execute";
const METADATA_PATH: &str = "/.well-known/oauth-protected-resource/mcp";

/// What the `/mcp` routes need.
#[derive(Clone)]
pub struct McpState {
    pub auth: AuthState,
    pub spaces: SpaceResolver,
    pub runtimes: Arc<SpaceRuntimes>,
    /// The server's issuer URL; the canonical MCP URL is derived from it.
    pub issuer: String,
}

impl McpState {
    fn resource_metadata_url(&self) -> String {
        format!("{}{METADATA_PATH}", self.issuer.trim_end_matches('/'))
    }
}

/// `/mcp` and its protected resource metadata.
pub fn mcp_router(state: McpState) -> Router {
    Router::new()
        .route_service("/mcp", streamable_http_service())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_mcp_access,
        ))
        .route(METADATA_PATH, get(protected_resource_metadata))
        .with_state(state)
}

// -- Protected resource metadata (RFC 9728) ------------------------------------

/// OAuth 2.0 protected resource metadata for `/mcp` (RFC 9728).
#[derive(Serialize, utoipa::ToSchema)]
pub struct ProtectedResourceMetadata {
    /// Canonical URL of the MCP endpoint; tokens must be issued for it.
    pub resource: String,
    /// Authorization servers that issue tokens for this resource.
    pub authorization_servers: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub bearer_methods_supported: Vec<String>,
}

/// GET /.well-known/oauth-protected-resource/mcp
#[utoipa::path(
    get,
    path = "/.well-known/oauth-protected-resource/mcp",
    tag = "oauth",
    security(()),
    responses(
        (status = 200, description = "Protected resource metadata for the MCP endpoint", body = ProtectedResourceMetadata),
    )
)]
pub async fn protected_resource_metadata(
    State(state): State<McpState>,
) -> Json<ProtectedResourceMetadata> {
    Json(ProtectedResourceMetadata {
        resource: mcp_resource(&state.issuer),
        authorization_servers: vec![state.issuer.trim_end_matches('/').to_owned()],
        scopes_supported: vec![MCP_SCOPE.into()],
        bearer_methods_supported: vec!["header".into()],
    })
}

// -- Access ----------------------------------------------------------------------

/// Checks `Origin`, the credential and its scope, binds the request to a space
/// and attaches the caller's [`AuthContext`] and that space's
/// [`AssistantMcpServer`] to the request extensions.
async fn require_mcp_access(
    State(state): State<McpState>,
    mut request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    if headers.contains_key(ORIGIN) && !is_same_origin(headers, &[ORIGIN]) {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }

    let Some(token) = bearer_token(headers) else {
        return unauthorized(&state);
    };
    let auth = match authenticate(&state, token).await {
        Ok(auth) => auth,
        Err(SessionError::Rejected) => return unauthorized(&state),
        Err(SessionError::Unavailable(e)) => {
            error!("MCP session check failed: {e:#}");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Authentication unavailable",
            )
                .into_response();
        }
    };
    if !auth
        .scopes
        .iter()
        .any(|s| s.resource == ResourceKind::Mcp && s.action == Action::Execute)
    {
        return insufficient_scope(&state);
    }

    let selector = headers.get(SPACE_HEADER).and_then(|v| v.to_str().ok());
    let space = match state.spaces.resolve(&auth, selector).await {
        Ok(space) => space,
        Err(e) => return e.into_response(),
    };
    let runtime = match state.runtimes.get(&space.org_slug, &space.space_slug).await {
        Ok(runtime) => runtime,
        Err(e) => {
            error!("MCP space runtime failed to load: {e:#}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let server = Arc::new(AssistantMcpServer::new(
        runtime.orchestrator.clone(),
        runtime.executor.clone(),
        runtime.registry.clone(),
        runtime.orchestrator.permission_policy().clone(),
        state.runtimes.pools().base_dir().join("skills"),
    ));

    request.extensions_mut().insert(auth);
    request.extensions_mut().insert(server);
    next.run(request).await
}

/// Resolve `token`: an API key, or a JWT whose audience is the MCP endpoint.
async fn authenticate(state: &McpState, token: &str) -> Result<AuthContext, SessionError> {
    if token.starts_with("ask_live_") {
        let ctx = api_keys::resolve_key(token, state.auth.api_key_store.as_ref())
            .await
            .map_err(|_| SessionError::Rejected)?;
        return state.auth.bind_api_key(ctx).await;
    }
    state
        .auth
        .authenticate_jwt_for_audience(token, &mcp_resource(&state.issuer))
        .await
}

fn unauthorized(state: &McpState) -> Response {
    challenge(state, StatusCode::UNAUTHORIZED, "")
}

fn insufficient_scope(state: &McpState) -> Response {
    challenge(
        state,
        StatusCode::FORBIDDEN,
        r#"error="insufficient_scope", "#,
    )
}

fn challenge(state: &McpState, status: StatusCode, error: &str) -> Response {
    let value = format!(
        r#"Bearer {error}scope="{MCP_SCOPE}", resource_metadata="{}""#,
        state.resource_metadata_url()
    );
    let mut response = status.into_response();
    if let Ok(value) = HeaderValue::from_str(&value) {
        response.headers_mut().insert(WWW_AUTHENTICATE, value);
    }
    response
}
