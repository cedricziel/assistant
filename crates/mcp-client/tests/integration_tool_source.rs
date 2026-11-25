//! `IntegrationToolSource`: which integration tools a turn gets, and under what
//! names, for a user in a space.

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use assistant_core::clock::FakeClock;
use assistant_core::identity::{OrgId, SpaceId, UserId};
use assistant_core::integrations::{
    CredentialKind, CredentialMode, CredentialStatus, EffectiveIntegrations,
    InMemoryIntegrationStore, Integration, IntegrationAuthKind, IntegrationCredentialStore,
    IntegrationLayer, IntegrationPolicy, IntegrationStore, IntegrationTrust, SecretBox,
    StoredCredential, credential_aad,
};
use assistant_core::secret::Secret;
use assistant_core::store::{InMemoryOrgStore, InMemorySpaceStore, OrgStore, Organization, Space};
use assistant_core::tool::{ToolHandler, TurnScope, TurnToolSource};
use assistant_mcp_client::{ConnectionsConfig, IntegrationConnections, IntegrationToolSource};

fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap()
}

/// One MCP server whose single tool is `ping`.
struct Server {
    delay: Option<Duration>,
    healthy: bool,
    /// How long `tools/list` takes; changeable mid-test.
    list_delay: Arc<std::sync::Mutex<Option<Duration>>>,
}

impl Respond for Server {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        if !self.healthy {
            return ResponseTemplate::new(500);
        }
        let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        let id = body["id"].clone();
        let reply = |result: Value| {
            let response = ResponseTemplate::new(200)
                .insert_header("mcp-session-id", "s")
                .set_body_json(json!({"jsonrpc": "2.0", "id": id, "result": result}));
            match self.delay {
                Some(delay) => response.set_delay(delay),
                None => response,
            }
        };
        match body["method"].as_str().unwrap_or_default() {
            "initialize" => reply(json!({
                "protocolVersion": body["params"]["protocolVersion"],
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "s", "version": "1"},
            })),
            "tools/list" => {
                let response = reply(json!({"tools": [{
                    "name": "ping", "description": "ping", "inputSchema": {"type": "object"},
                }]}));
                match *self.list_delay.lock().unwrap() {
                    Some(delay) => response.set_delay(delay),
                    None => response,
                }
            }
            _ => ResponseTemplate::new(202),
        }
    }
}

async fn serve(delay: Option<Duration>, healthy: bool) -> MockServer {
    serve_with(Server {
        delay,
        healthy,
        list_delay: Arc::default(),
    })
    .await
}

async fn serve_with(script: Server) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(script)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(405))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;
    server
}

struct World {
    store: Arc<InMemoryIntegrationStore>,
    secrets: Arc<SecretBox>,
    clock: FakeClock,
    source: IntegrationToolSource,
}

const U: &str = "usr_u";
const V: &str = "usr_v";

impl World {
    async fn new(config: ConnectionsConfig) -> Self {
        let store = Arc::new(InMemoryIntegrationStore::new());
        let secrets = Arc::new(SecretBox::from_key([5; 32]));
        let credentials = store.credential_store();
        let clock = FakeClock::new(at(0));
        let connections = Arc::new(
            IntegrationConnections::new(
                credentials.clone(),
                store.health_store(),
                secrets.clone(),
                Arc::new(clock.clone()),
            )
            .with_config(config),
        );
        let orgs = Arc::new(InMemoryOrgStore::default());
        orgs.create_org(&Organization {
            id: OrgId::from("org_1"),
            name: "Acme".into(),
            slug: "acme".into(),
            auth_mode: "password".into(),
            created_at: at(0),
            updated_at: at(0),
        })
        .await
        .unwrap();
        let spaces = Arc::new(InMemorySpaceStore::default());
        for (id, slug) in [("sp_eng", "eng"), ("sp_sales", "sales")] {
            assistant_core::store::SpaceStore::create_space(
                spaces.as_ref(),
                &Space {
                    id: SpaceId::from(id),
                    org_id: OrgId::from("org_1"),
                    name: slug.into(),
                    slug: slug.into(),
                    created_at: at(0),
                    updated_at: at(0),
                },
            )
            .await
            .unwrap();
        }
        // The fixtures are custom URLs, which the default policy forbids below the org.
        let mut policy = IntegrationPolicy::default_for(OrgId::from("org_1"));
        policy.allow_custom_urls = true;
        store.put_policy(&policy).await.unwrap();
        let effective = Arc::new(EffectiveIntegrations::new(
            store.clone(),
            credentials,
            secrets.key_id(),
        ));
        let source = IntegrationToolSource::new(effective, connections, orgs, spaces);
        Self {
            store,
            secrets,
            clock,
            source,
        }
    }

    async fn add(
        &self,
        id: &str,
        layer: IntegrationLayer,
        url: &str,
        trust: IntegrationTrust,
        mode: CredentialMode,
        mutate: impl FnOnce(&mut Integration),
    ) -> Integration {
        let mut integration = Integration {
            id: id.into(),
            org_id: OrgId::from("org_1"),
            layer,
            space_id: (layer == IntegrationLayer::Space).then(|| SpaceId::from("sp_eng")),
            owner_user_id: (layer == IntegrationLayer::User).then(|| UserId::from(U)),
            catalog_entry_id: None,
            connect_url: url.into(),
            slug: "linear".into(),
            name: id.into(),
            auth_kind: IntegrationAuthKind::None,
            header_name: None,
            header_value_prefix: None,
            credential_mode: mode,
            trust,
            enabled: true,
            oauth_client_id_override: None,
            created_at: at(0),
            updated_at: at(0),
        };
        mutate(&mut integration);
        self.store.create(&integration).await.unwrap();
        integration
    }

    /// An integration that needs no credential, so every user qualifies.
    async fn open(&self, id: &str, layer: IntegrationLayer, url: &str, trust: IntegrationTrust) {
        self.add(id, layer, url, trust, CredentialMode::Shared, |i| {
            if layer == IntegrationLayer::User {
                i.credential_mode = CredentialMode::PerUser;
            }
        })
        .await;
    }

    async fn names(&self, user: Option<&str>, space: &str) -> Vec<String> {
        let user = user.map(UserId::from);
        let mut names: Vec<String> = self
            .tools(user.as_ref(), "acme", space)
            .await
            .iter()
            .map(|h| h.name().to_string())
            .collect();
        names.sort();
        names
    }

    async fn tools(
        &self,
        user: Option<&UserId>,
        org: &str,
        space: &str,
    ) -> Vec<Arc<dyn ToolHandler>> {
        self.source
            .tools_for_turn(&TurnScope {
                org_slug: org,
                space_slug: space,
                user_id: user,
            })
            .await
    }
}

#[tokio::test]
async fn a_user_gets_org_space_and_user_tools_under_their_layer_names() {
    let server = serve(None, true).await;
    let world = World::new(ConnectionsConfig::default()).await;
    for (id, layer) in [
        ("int_o", IntegrationLayer::Org),
        ("int_s", IntegrationLayer::Space),
        ("int_u", IntegrationLayer::User),
    ] {
        world
            .open(id, layer, &server.uri(), IntegrationTrust::Trust)
            .await;
    }

    assert_eq!(
        world.names(Some(U), "eng").await,
        vec![
            "mcp__org_linear__ping",
            "mcp__space_linear__ping",
            "mcp__user_linear__ping"
        ]
    );
}

#[tokio::test]
async fn another_user_in_the_same_space_does_not_get_the_first_users_tools() {
    let server = serve(None, true).await;
    let world = World::new(ConnectionsConfig::default()).await;
    for (id, layer) in [
        ("int_o", IntegrationLayer::Org),
        ("int_s", IntegrationLayer::Space),
        ("int_u", IntegrationLayer::User),
    ] {
        world
            .open(id, layer, &server.uri(), IntegrationTrust::Trust)
            .await;
    }

    assert_eq!(
        world.names(Some(V), "eng").await,
        vec!["mcp__org_linear__ping", "mcp__space_linear__ping"]
    );
}

#[tokio::test]
async fn a_space_integration_belongs_to_its_space_only() {
    let server = serve(None, true).await;
    let world = World::new(ConnectionsConfig::default()).await;
    world
        .open(
            "int_s",
            IntegrationLayer::Space,
            &server.uri(),
            IntegrationTrust::Trust,
        )
        .await;

    assert_eq!(world.names(Some(U), "sales").await, Vec::<String>::new());
}

#[tokio::test]
async fn a_per_user_integration_gives_each_user_their_own_credential() {
    let server = serve(None, true).await;
    let world = World::new(ConnectionsConfig::default()).await;
    let integration = world
        .add(
            "int_o",
            IntegrationLayer::Org,
            &server.uri(),
            IntegrationTrust::Trust,
            CredentialMode::PerUser,
            |i| {
                i.auth_kind = IntegrationAuthKind::Header;
                i.header_name = Some("authorization".into());
                i.header_value_prefix = Some("Bearer ".into());
            },
        )
        .await;
    let aad = credential_aad(&integration.id, Some(U), "header");
    let sealed = world
        .secrets
        .seal(&Secret::new("token-u"), aad.as_bytes())
        .unwrap();
    let credentials = world.store.credential_store();
    credentials
        .put(&StoredCredential {
            integration_id: integration.id.clone(),
            subject_user_id: Some(UserId::from(U)),
            kind: CredentialKind::Header,
            ciphertext: sealed.ciphertext,
            nonce: sealed.nonce,
            key_id: sealed.key_id,
            expires_at: None,
            status: CredentialStatus::Ready,
            updated_at: at(0),
        })
        .await
        .unwrap();

    assert_eq!(
        world.names(Some(U), "eng").await,
        vec!["mcp__org_linear__ping"]
    );
    assert_eq!(world.names(Some(V), "eng").await, Vec::<String>::new());
    let seen = server.received_requests().await.unwrap();
    assert!(seen.iter().any(
        |r| r.headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer token-u")
    ));
}

#[tokio::test]
async fn a_turn_without_a_user_gets_shared_integrations_only() {
    let server = serve(None, true).await;
    let world = World::new(ConnectionsConfig::default()).await;
    world
        .open(
            "int_o",
            IntegrationLayer::Org,
            &server.uri(),
            IntegrationTrust::Trust,
        )
        .await;
    world
        .open(
            "int_u",
            IntegrationLayer::User,
            &server.uri(),
            IntegrationTrust::Trust,
        )
        .await;

    assert_eq!(
        world.names(None, "eng").await,
        vec!["mcp__org_linear__ping"]
    );
}

#[tokio::test]
async fn trust_decides_whether_a_tool_needs_confirmation() {
    let server = serve(None, true).await;
    let world = World::new(ConnectionsConfig::default()).await;
    world
        .open(
            "int_o",
            IntegrationLayer::Org,
            &server.uri(),
            IntegrationTrust::Confirm,
        )
        .await;
    world
        .add(
            "int_s",
            IntegrationLayer::Space,
            &server.uri(),
            IntegrationTrust::Trust,
            CredentialMode::Shared,
            |i| i.slug = "sentry".into(),
        )
        .await;

    let tools = world.tools(Some(&UserId::from(U)), "acme", "eng").await;
    let flag = |name: &str| {
        tools
            .iter()
            .find(|h| h.name() == name)
            .map(|h| h.requires_confirmation())
    };
    assert_eq!(flag("mcp__org_linear__ping"), Some(true));
    assert_eq!(flag("mcp__space_sentry__ping"), Some(false));
}

#[tokio::test]
async fn a_broken_integration_is_skipped_and_the_rest_still_load() {
    let good = serve(None, true).await;
    let broken = serve(None, false).await;
    let world = World::new(ConnectionsConfig::default()).await;
    world
        .open(
            "int_o",
            IntegrationLayer::Org,
            &good.uri(),
            IntegrationTrust::Trust,
        )
        .await;
    world
        .add(
            "int_s",
            IntegrationLayer::Space,
            &broken.uri(),
            IntegrationTrust::Trust,
            CredentialMode::Shared,
            |i| i.slug = "sentry".into(),
        )
        .await;

    assert_eq!(
        world.names(Some(U), "eng").await,
        vec!["mcp__org_linear__ping"]
    );
}

#[tokio::test]
async fn slow_integrations_connect_in_parallel_within_the_timeout() {
    let slow_a = serve(Some(Duration::from_secs(30)), true).await;
    let slow_b = serve(Some(Duration::from_secs(30)), true).await;
    let world = World::new(ConnectionsConfig {
        connect_timeout: Duration::from_millis(400),
        ..ConnectionsConfig::default()
    })
    .await;
    world
        .open(
            "int_o",
            IntegrationLayer::Org,
            &slow_a.uri(),
            IntegrationTrust::Trust,
        )
        .await;
    world
        .add(
            "int_s",
            IntegrationLayer::Space,
            &slow_b.uri(),
            IntegrationTrust::Trust,
            CredentialMode::Shared,
            |i| i.slug = "sentry".into(),
        )
        .await;

    let started = Instant::now();
    let names = world.names(Some(U), "eng").await;

    assert!(names.is_empty());
    assert!(
        started.elapsed() < Duration::from_millis(750),
        "two 400 ms timeouts must overlap, took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn an_unknown_org_or_space_yields_no_tools() {
    let server = serve(None, true).await;
    let world = World::new(ConnectionsConfig::default()).await;
    world
        .open(
            "int_o",
            IntegrationLayer::Org,
            &server.uri(),
            IntegrationTrust::Trust,
        )
        .await;
    let user = UserId::from(U);

    assert!(world.tools(Some(&user), "nope", "eng").await.is_empty());
    assert!(world.tools(Some(&user), "acme", "nope").await.is_empty());
}

#[tokio::test]
async fn an_integration_that_stops_answering_costs_the_turn_one_timeout_and_nothing_more() {
    let healthy = serve(None, true).await;
    let list_delay = Arc::new(std::sync::Mutex::new(None));
    let hanging = serve_with(Server {
        delay: None,
        healthy: true,
        list_delay: list_delay.clone(),
    })
    .await;
    let world = World::new(ConnectionsConfig {
        connect_timeout: Duration::from_millis(300),
        tools_ttl: Duration::from_secs(5 * 60),
        ..ConnectionsConfig::default()
    })
    .await;
    world
        .open(
            "int_o",
            IntegrationLayer::Org,
            &healthy.uri(),
            IntegrationTrust::Trust,
        )
        .await;
    world
        .add(
            "int_s",
            IntegrationLayer::Space,
            &hanging.uri(),
            IntegrationTrust::Trust,
            CredentialMode::Shared,
            |i| i.slug = "sentry".into(),
        )
        .await;
    assert_eq!(
        world.names(Some(U), "eng").await.len(),
        2,
        "both load while healthy"
    );

    *list_delay.lock().unwrap() = Some(Duration::from_secs(30));
    world.clock.advance(Duration::from_secs(6 * 60));
    let started = Instant::now();
    let names = world.names(Some(U), "eng").await;

    assert_eq!(names, vec!["mcp__org_linear__ping"]);
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the turn must proceed, took {:?}",
        started.elapsed()
    );
}
