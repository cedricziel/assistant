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
use assistant_mcp_server::{AssistantMcpServer, ListChangeListeners, streamable_http_service};
use assistant_runtime::SpaceRuntimes;

use crate::auth::{bearer_token, is_same_origin};
use crate::oauth::mcp_resource;
use crate::space::{SPACE_HEADER, SpaceResolver};

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
    /// Sessions to notify when installed skills change. One registry for the
    /// whole endpoint, because every request gets a new server.
    pub listeners: ListChangeListeners,
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
        state.listeners.clone(),
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

#[cfg(test)]
mod tests {
    // `/mcp` over HTTP: credentials, scopes, origins, spaces and the protected
    // resource metadata.
    //
    // Topology:
    //
    // ```text
    // org_a ── A1 (alice: member · adam: org_admin)
    //       └─ A2 (carol: member)
    // ```

    use std::collections::HashMap;
    use std::sync::Arc;

    use axum::Router;
    use axum::body::Body;
    use axum::http::{HeaderValue, Method, Request, StatusCode, header};
    use http_body_util::BodyExt;
    use rmcp::ServiceExt;
    use rmcp::model::{ErrorCode, GetPromptRequestParams, ProtocolVersion, SubscriptionFilter};
    use rmcp::service::{ClientLifecycleMode, ServiceError, serve_client_with_lifecycle};
    use rmcp::transport::StreamableHttpClientTransport;
    use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
    use serde_json::{Value, json};
    use tower::ServiceExt as _;

    use assistant_auth::api_keys::{InMemoryApiKeyStore, generate_key};
    use assistant_auth::jwt::{JwtKeyPair, JwtManager};
    use assistant_auth::middleware::AuthState;
    use assistant_auth::oauth2::server::InMemoryRefreshTokenStore;
    use assistant_core::auth::{ApiKeyRecord, ApiKeyStore as _, AuthContext};
    use assistant_core::clock::{Clock as _, SystemClock};
    use assistant_core::identity::{Action, OrgId, ResourceKind, Role, Scope, SpaceId, UserId};
    use assistant_core::store::{
        InMemoryMembershipStore, InMemoryUserStore, MembershipStore as _, SpaceMembership, User,
        UserStore as _,
    };
    use assistant_storage::StorageLayer;

    use super::{McpState, mcp_router};
    use crate::api::test_helpers::{TestSpace, state_over};

    const ISSUER: &str = "https://assistant.test";
    const MCP_URL: &str = "https://assistant.test/mcp";
    const METADATA_URL: &str = "https://assistant.test/.well-known/oauth-protected-resource/mcp";
    const ORG_A: &str = "org_a";
    const A1: &str = "spc_a1";
    const A2: &str = "spc_a2";

    struct Fixture {
        router: Router,
        jwt: Arc<JwtManager>,
        api_keys: Arc<InMemoryApiKeyStore>,
    }

    impl Fixture {
        async fn new() -> Self {
            let spaces = [
                TestSpace {
                    id: A1,
                    org_id: ORG_A,
                    org_slug: "acme",
                    slug: "eng",
                    storage: Arc::new(StorageLayer::new_in_memory().await.unwrap()),
                },
                TestSpace {
                    id: A2,
                    org_id: ORG_A,
                    org_slug: "acme",
                    slug: "ops",
                    storage: Arc::new(StorageLayer::new_in_memory().await.unwrap()),
                },
            ];
            // No test reaches the LLM.
            let (api, resolver) = state_over("http://127.0.0.1:9", &spaces, false).await;

            let users = Arc::new(InMemoryUserStore::default());
            let memberships = Arc::new(InMemoryMembershipStore::default());
            let now = SystemClock.now();
            for (user, space, role) in [
                ("alice", A1, Role::Member),
                ("adam", A1, Role::OrgAdmin),
                ("carol", A2, Role::Member),
            ] {
                users
                    .create_user(&User {
                        id: UserId::from(user),
                        org_id: OrgId::from(ORG_A),
                        email: format!("{user}@acme.test"),
                        name: user.into(),
                        password_hash: String::new(),
                        idp_issuer: None,
                        idp_subject: None,
                        created_at: now,
                        updated_at: now,
                    })
                    .await
                    .unwrap();
                memberships
                    .add_membership(&SpaceMembership {
                        user_id: UserId::from(user),
                        space_id: SpaceId::from(space),
                        role,
                        created_at: now,
                    })
                    .await
                    .unwrap();
            }

            let jwt = Arc::new(JwtManager::new(
                JwtKeyPair::from_secret(b"test-secret-that-is-long-enough!!").unwrap(),
                ISSUER.into(),
                ISSUER.into(),
            ));
            let api_keys = Arc::new(InMemoryApiKeyStore::new());
            let auth = AuthState {
                jwt_manager: jwt.clone(),
                api_key_store: api_keys.clone(),
                users,
                memberships,
                refresh_tokens: Arc::new(InMemoryRefreshTokenStore::default()),
                legacy_token: None,
                legacy_context: None,
            };
            let router = mcp_router(McpState {
                auth,
                spaces: resolver,
                runtimes: api.runtimes,
                issuer: ISSUER.into(),
                listeners: Default::default(),
            });
            Self {
                router,
                jwt,
                api_keys,
            }
        }

        fn identity(user: &str, scopes: Vec<Scope>) -> AuthContext {
            AuthContext {
                user_id: UserId::from(user),
                org_id: OrgId::from(ORG_A),
                email: format!("{user}@acme.test"),
                space_roles: HashMap::new(),
                scopes,
                client_id: "test-client".into(),
            }
        }

        /// A JWT for `user` issued for the MCP resource.
        fn mcp_token(&self, user: &str) -> String {
            let ctx = Self::identity(user, vec![mcp_execute()]);
            self.jwt
                .sign_for_audience(&ctx, ORG_A, user, 0, MCP_URL)
                .unwrap()
        }

        /// A JWT for `user` issued for the web API (no `resource`).
        fn api_token(&self, user: &str) -> String {
            let ctx = Self::identity(user, vec![mcp_execute()]);
            self.jwt.sign(&ctx, ORG_A, user, 0).unwrap()
        }

        /// An `ask_live_` key for `user` in `space` with `scopes`.
        async fn api_key(&self, user: &str, space: &str, scopes: Vec<Scope>) -> String {
            let key = generate_key();
            self.api_keys
                .store_key(ApiKeyRecord {
                    id: format!("key-{}", key.key_prefix),
                    user_id: UserId::from(user),
                    name: "test".into(),
                    key_hash: key.key_hash.clone(),
                    key_prefix: key.key_prefix.clone(),
                    org_id: OrgId::from(ORG_A),
                    space_roles: HashMap::from([(SpaceId::from(space), Role::Member)]),
                    scopes,
                    expires_at: None,
                    created_at: SystemClock.now(),
                })
                .await
                .unwrap();
            key.plaintext
        }

        async fn send(&self, request: Request<Body>) -> axum::response::Response {
            self.router.clone().oneshot(request).await.unwrap()
        }

        /// Serve the router on a loopback port and return the `/mcp` URL.
        async fn serve(&self) -> String {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let router = self.router.clone();
            tokio::spawn(async move { axum::serve(listener, router).await });
            format!("http://{addr}/mcp")
        }

        /// A Streamable HTTP transport to `/mcp` carrying `token`, selecting
        /// `space` when given.
        async fn transport(
            &self,
            token: &str,
            space: Option<&str>,
        ) -> StreamableHttpClientTransport<reqwest::Client> {
            let mut headers = HashMap::new();
            if let Some(space) = space {
                headers.insert(
                    header::HeaderName::from_static("x-space-id"),
                    HeaderValue::from_str(space).unwrap(),
                );
            }
            let config = StreamableHttpClientTransportConfig::with_uri(self.serve().await)
                .auth_header(token)
                .custom_headers(headers);
            StreamableHttpClientTransport::from_config(config)
        }

        /// List the tools over Streamable HTTP with rmcp's client.
        async fn list_tools(
            &self,
            token: &str,
            space: Option<&str>,
        ) -> Result<Vec<String>, rmcp::service::ClientInitializeError> {
            let client = ().serve(self.transport(token, space).await).await?;
            let tools = client.list_all_tools().await.unwrap();
            Ok(tools.into_iter().map(|t| t.name.into_owned()).collect())
        }
    }

    fn mcp_execute() -> Scope {
        Scope::new(ResourceKind::Mcp, Action::Execute)
    }

    fn initialize() -> Request<Body> {
        post_mcp(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }
        }))
    }

    fn post_mcp(body: Value) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header(header::HOST, "assistant.test")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "application/json, text/event-stream")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn with_bearer(mut request: Request<Body>, token: &str) -> Request<Body> {
        request.headers_mut().insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        request
    }

    fn with_header(mut request: Request<Body>, name: &'static str, value: &str) -> Request<Body> {
        request
            .headers_mut()
            .insert(name, HeaderValue::from_str(value).unwrap());
        request
    }

    fn challenge(response: &axum::response::Response) -> String {
        response
            .headers()
            .get(header::WWW_AUTHENTICATE)
            .expect("WWW-Authenticate header")
            .to_str()
            .unwrap()
            .to_owned()
    }

    // -- Credentials ---------------------------------------------------------------

    #[tokio::test]
    async fn missing_token_gets_a_challenge_pointing_at_the_resource_metadata() {
        let fx = Fixture::new().await;
        let response = fx.send(initialize()).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let challenge = challenge(&response);
        assert!(challenge.starts_with("Bearer "), "{challenge}");
        assert!(
            challenge.contains(&format!(r#"resource_metadata="{METADATA_URL}""#)),
            "{challenge}"
        );
        assert!(challenge.contains(r#"scope="mcp:execute""#), "{challenge}");
    }

    #[tokio::test]
    async fn invalid_token_gets_the_same_challenge() {
        let fx = Fixture::new().await;
        let response = fx.send(with_bearer(initialize(), "not-a-token")).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(challenge(&response).contains(METADATA_URL));
    }

    #[tokio::test]
    async fn api_audience_jwt_is_rejected() {
        let fx = Fixture::new().await;
        let response = fx
            .send(with_bearer(initialize(), &fx.api_token("alice")))
            .await;

        assert_eq!(
            response.status(),
            StatusCode::UNAUTHORIZED,
            "a token issued for the web API must not open /mcp"
        );
        assert!(challenge(&response).contains(METADATA_URL));
    }

    #[tokio::test]
    async fn api_key_without_mcp_scope_is_forbidden_with_insufficient_scope() {
        let fx = Fixture::new().await;
        let key = fx
            .api_key(
                "alice",
                A1,
                vec![Scope::new(ResourceKind::Conversations, Action::Read)],
            )
            .await;
        let response = fx.send(with_bearer(initialize(), &key)).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let challenge = challenge(&response);
        assert!(
            challenge.contains(r#"error="insufficient_scope""#),
            "{challenge}"
        );
        assert!(challenge.contains(r#"scope="mcp:execute""#), "{challenge}");
        assert!(challenge.contains(METADATA_URL), "{challenge}");
    }

    #[tokio::test]
    async fn mcp_audience_token_lists_tools() {
        let fx = Fixture::new().await;
        let tools = fx
            .list_tools(&fx.mcp_token("alice"), None)
            .await
            .expect("an MCP-audience token is accepted");

        assert!(tools.iter().any(|t| t == "run-prompt"), "{tools:?}");
    }

    #[tokio::test]
    async fn api_key_with_mcp_scope_lists_tools() {
        let fx = Fixture::new().await;
        let key = fx.api_key("alice", A1, vec![mcp_execute()]).await;
        let tools = fx
            .list_tools(&key, None)
            .await
            .expect("an API key with mcp:execute is accepted");

        assert!(tools.iter().any(|t| t == "run-prompt"), "{tools:?}");
    }

    // -- Prompts, completions and subscriptions ---------------------------------------

    #[tokio::test]
    async fn prompts_and_completions_reach_the_space_server() {
        let fx = Fixture::new().await;
        let client = ().serve(fx.transport(&fx.mcp_token("alice"), None).await).await.unwrap();

        client.list_all_prompts().await.unwrap();
        let error = client
            .get_prompt(GetPromptRequestParams::new("no-such-skill"))
            .await
            .unwrap_err();
        let ServiceError::McpError(error) = error else {
            panic!("expected a protocol error, got {error:?}");
        };
        assert_eq!(
            error.code,
            ErrorCode::INVALID_PARAMS,
            "the space server answers prompts/get: {error:?}"
        );
        client
            .complete_resource_simple("skills://{name}/{category}/{+path}", "name", "x")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn listen_subscriptions_reach_the_space_server() {
        let fx = Fixture::new().await;
        let modern = ClientLifecycleMode::Discover {
            preferred_versions: vec![ProtocolVersion::V_2026_07_28],
        };
        let client = serve_client_with_lifecycle(
            (),
            fx.transport(&fx.mcp_token("alice"), None).await,
            modern,
        )
        .await
        .unwrap();

        let subscription = client
            .listen(
                SubscriptionFilter::builder()
                    .tools_list_changed()
                    .prompts_list_changed()
                    .build(),
            )
            .await
            .expect("subscriptions/listen is served");

        assert_eq!(subscription.acknowledged().tools_list_changed, Some(true));
        assert_eq!(subscription.acknowledged().prompts_list_changed, Some(true));
    }

    // -- Transport -----------------------------------------------------------------

    #[tokio::test]
    async fn a_present_origin_must_match_the_host() {
        let fx = Fixture::new().await;
        for (origin, expected) in [
            ("https://evil.test", StatusCode::FORBIDDEN),
            ("https://assistant.test", StatusCode::OK),
        ] {
            let request = with_bearer(initialize(), &fx.mcp_token("alice"));
            let response = fx.send(with_header(request, "origin", origin)).await;
            assert_eq!(response.status(), expected, "Origin {origin}");
        }
    }

    #[tokio::test]
    async fn undefined_methods_get_405() {
        let fx = Fixture::new().await;
        let mut request = with_bearer(initialize(), &fx.mcp_token("alice"));
        *request.method_mut() = Method::PUT;
        let response = fx.send(request).await;

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    // -- Spaces --------------------------------------------------------------------

    #[tokio::test]
    async fn a_member_cannot_select_a_space_they_do_not_belong_to() {
        let fx = Fixture::new().await;
        let request = with_bearer(initialize(), &fx.mcp_token("carol"));
        let response = fx.send(with_header(request, "x-space-id", A1)).await;

        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "carol holds no role in A1"
        );
    }

    #[tokio::test]
    async fn a_member_reaches_their_own_space() {
        let fx = Fixture::new().await;
        let tools = fx
            .list_tools(&fx.mcp_token("carol"), Some(A2))
            .await
            .expect("carol is a member of A2");

        assert!(!tools.is_empty());
    }

    #[tokio::test]
    async fn an_org_admin_cannot_name_a_space_that_does_not_exist() {
        let fx = Fixture::new().await;
        let request = with_bearer(initialize(), &fx.mcp_token("adam"));
        let response = fx
            .send(with_header(request, "x-space-id", "spc_missing"))
            .await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    // -- Protected resource metadata -----------------------------------------------

    #[tokio::test]
    async fn protected_resource_metadata_names_the_mcp_resource_and_issuer() {
        let fx = Fixture::new().await;
        let response = fx
            .send(
                Request::get("/.well-known/oauth-protected-resource/mcp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let metadata: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            metadata,
            json!({
                "resource": MCP_URL,
                "authorization_servers": [ISSUER],
                "scopes_supported": ["mcp:execute"],
                "bearer_methods_supported": ["header"],
            })
        );
    }
}
