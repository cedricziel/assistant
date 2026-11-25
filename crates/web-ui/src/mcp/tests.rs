//! `/mcp` over HTTP: credentials, scopes, origins, spaces and the protected
//! resource metadata.
//!
//! Topology:
//!
//! ```text
//! org_a ── A1 (alice: member · adam: org_admin)
//!       └─ A2 (carol: member)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderValue, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use rmcp::ServiceExt;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde_json::{Value, json};
use tower::ServiceExt as _;

use assistant_auth::api_keys::{InMemoryApiKeyStore, generate_key};
use assistant_auth::jwt::{JwtKeyPair, JwtManager};
use assistant_auth::middleware::AuthState;
use assistant_auth::oauth2::server::InMemoryRefreshTokenStore;
use assistant_core::auth::{ApiKeyRecord, ApiKeyStore as _, AuthContext};
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
        let now = chrono::Utc::now();
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
                created_at: chrono::Utc::now(),
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

    /// List the tools over Streamable HTTP with rmcp's client.
    async fn list_tools(
        &self,
        token: &str,
        space: Option<&str>,
    ) -> Result<Vec<String>, rmcp::service::ClientInitializeError> {
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
        let client = ().serve(StreamableHttpClientTransport::from_config(config)).await?;
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
