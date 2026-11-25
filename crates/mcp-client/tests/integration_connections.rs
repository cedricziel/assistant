//! `IntegrationConnections` against a scripted MCP server speaking Streamable
//! HTTP: lazy connect, header injection from the integration's template, tool
//! caching, idle eviction, invalidation and the connect timeout.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use assistant_core::clock::FakeClock;
use assistant_core::identity::{OrgId, UserId};
use assistant_core::integrations::{
    CredentialKind, CredentialMode, CredentialStatus, EffectiveIntegration,
    InMemoryIntegrationStore, Integration, IntegrationAuthKind, IntegrationCredentialStore,
    IntegrationHealthStore, IntegrationLayer, IntegrationStore, IntegrationTrust, SecretBox,
    StoredCredential, credential_aad,
};
use assistant_core::secret::Secret;
use assistant_mcp_client::{ConnectError, ConnectionsConfig, IntegrationConnections};

fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap()
}

/// Answers the JSON-RPC methods of one MCP session.
#[derive(Clone, Default)]
struct Script {
    initialize_delay: Option<Duration>,
    healthy: Arc<AtomicBool>,
    tools: Arc<std::sync::Mutex<Vec<&'static str>>>,
    /// How long `tools/list` takes to answer; changeable mid-test.
    list_delay: Arc<std::sync::Mutex<Option<Duration>>>,
}

impl Script {
    fn new(tools: &[&'static str]) -> Self {
        Self {
            initialize_delay: None,
            healthy: Arc::new(AtomicBool::new(true)),
            tools: Arc::new(std::sync::Mutex::new(tools.to_vec())),
            list_delay: Arc::default(),
        }
    }
}

fn rpc_result(id: &Value, result: Value) -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("mcp-session-id", "session-1")
        .set_body_json(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

impl Respond for Script {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        if !self.healthy.load(Ordering::SeqCst) {
            return ResponseTemplate::new(500);
        }
        let body: Value = serde_json::from_slice(&request.body).unwrap_or(Value::Null);
        let id = body["id"].clone();
        match body["method"].as_str().unwrap_or_default() {
            "initialize" => {
                let response = rpc_result(
                    &id,
                    json!({
                        "protocolVersion": body["params"]["protocolVersion"],
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "scripted", "version": "1.0.0"},
                    }),
                );
                match self.initialize_delay {
                    Some(delay) => response.set_delay(delay),
                    None => response,
                }
            }
            "tools/list" => {
                let tools: Vec<Value> = self
                    .tools
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|name| {
                        json!({
                            "name": name,
                            "description": format!("the {name} tool"),
                            "inputSchema": {"type": "object"},
                        })
                    })
                    .collect();
                let response = rpc_result(&id, json!({"tools": tools}));
                match *self.list_delay.lock().unwrap() {
                    Some(delay) => response.set_delay(delay),
                    None => response,
                }
            }
            _ => ResponseTemplate::new(202),
        }
    }
}

async fn serve(script: &Script) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(script.clone())
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

/// How many JSON-RPC requests of `rpc` the server has seen.
async fn count(server: &MockServer, rpc: &str) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| {
            serde_json::from_slice::<Value>(&r.body)
                .is_ok_and(|b| b["method"].as_str() == Some(rpc))
        })
        .count()
}

struct World {
    store: Arc<InMemoryIntegrationStore>,
    credentials: Arc<dyn IntegrationCredentialStore>,
    health: Arc<dyn IntegrationHealthStore>,
    secrets: Arc<SecretBox>,
    clock: FakeClock,
    pool: IntegrationConnections,
}

impl World {
    /// A pool that may talk plain HTTP, since the scripted server does.
    fn new(config: ConnectionsConfig) -> Self {
        Self::with_exact_config(ConnectionsConfig {
            allow_plain_http: true,
            ..config
        })
    }

    fn with_exact_config(config: ConnectionsConfig) -> Self {
        let store = Arc::new(InMemoryIntegrationStore::new());
        let credentials: Arc<dyn IntegrationCredentialStore> = store.credential_store();
        let health: Arc<dyn IntegrationHealthStore> = store.health_store();
        let secrets = Arc::new(SecretBox::from_key([9; 32]));
        let clock = FakeClock::new(at(0));
        let pool = IntegrationConnections::new(
            store.clone(),
            credentials.clone(),
            health.clone(),
            secrets.clone(),
            Arc::new(clock.clone()),
        )
        .with_config(config);
        Self {
            store,
            credentials,
            health,
            secrets,
            clock,
            pool,
        }
    }

    async fn integration(&self, url: &str, mutate: impl FnOnce(&mut Integration)) -> Integration {
        let mut integration = Integration {
            id: "int_1".into(),
            org_id: OrgId::from("org_1"),
            layer: IntegrationLayer::Org,
            space_id: None,
            owner_user_id: None,
            catalog_entry_id: None,
            connect_url: url.into(),
            slug: "linear".into(),
            name: "Linear".into(),
            auth_kind: IntegrationAuthKind::Header,
            header_name: Some("Authorization".into()),
            header_value_prefix: Some("Bearer ".into()),
            credential_mode: CredentialMode::Shared,
            trust: IntegrationTrust::Confirm,
            enabled: true,
            oauth_client_id_override: None,
            created_at: at(0),
            updated_at: at(0),
        };
        mutate(&mut integration);
        self.store.create(&integration).await.unwrap();
        integration
    }

    /// Store `value` as the credential of `(integration, subject)`.
    async fn credential(
        &self,
        integration: &Integration,
        subject: Option<&UserId>,
        value: &str,
        updated: i64,
    ) {
        let aad = credential_aad(
            &integration.id,
            subject.map(|u| u.0.as_str()),
            CredentialKind::Header.as_str(),
        );
        let sealed = self
            .secrets
            .seal(&Secret::new(value), aad.as_bytes())
            .unwrap();
        self.credentials
            .put(&StoredCredential {
                integration_id: integration.id.clone(),
                subject_user_id: subject.cloned(),
                kind: CredentialKind::Header,
                ciphertext: sealed.ciphertext,
                nonce: sealed.nonce,
                key_id: sealed.key_id,
                expires_at: None,
                status: CredentialStatus::Ready,
                updated_at: at(updated),
            })
            .await
            .unwrap();
    }
}

fn effective(integration: &Integration, subject: Option<&UserId>) -> EffectiveIntegration {
    EffectiveIntegration {
        integration: integration.clone(),
        credential_subject: subject.cloned(),
    }
}

fn names(tools: &[rmcp::model::Tool]) -> Vec<String> {
    tools.iter().map(|t| t.name.to_string()).collect()
}

const SIX_MINUTES: Duration = Duration::from_secs(6 * 60);

#[tokio::test]
async fn nothing_connects_until_a_turn_asks() {
    let server = serve(&Script::new(&["create_issue"])).await;
    let world = World::new(ConnectionsConfig::default());
    world.integration(&server.uri(), |_| {}).await;

    assert_eq!(world.pool.connection_count().await, 0);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn the_first_connect_lists_the_servers_tools() {
    let server = serve(&Script::new(&["create_issue", "list_issues"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;

    let connected = world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();

    assert_eq!(names(&connected.tools), vec!["create_issue", "list_issues"]);
    assert_eq!(world.pool.connection_count().await, 1);
}

#[tokio::test]
async fn every_request_carries_the_header_the_template_forms() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;

    world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let posts: Vec<_> = requests
        .iter()
        .filter(|r| r.method.as_str() == "POST")
        .collect();
    assert!(posts.len() >= 2, "initialize and tools/list at least");
    for request in posts {
        assert_eq!(
            request
                .headers
                .get("authorization")
                .and_then(|v| v.to_str().ok()),
            Some("Bearer ghp_abc")
        );
    }
}

#[tokio::test]
async fn a_custom_header_name_without_a_prefix_is_sent_as_is() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world
        .integration(&server.uri(), |i| {
            i.header_name = Some("x-api-key".into());
            i.header_value_prefix = None;
        })
        .await;
    world.credential(&integration, None, "key-1", 0).await;

    world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .filter(|r| r.method.as_str() == "POST")
            .all(|r| r.headers.get("x-api-key").and_then(|v| v.to_str().ok()) == Some("key-1"))
    );
}

#[tokio::test]
async fn an_integration_without_auth_connects_without_a_credential() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world
        .integration(&server.uri(), |i| {
            i.auth_kind = IntegrationAuthKind::None;
            i.header_name = None;
            i.header_value_prefix = None;
        })
        .await;

    let connected = world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();
    assert_eq!(names(&connected.tools), vec!["t"]);
}

#[tokio::test]
async fn tools_are_cached_across_turns() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);

    for _ in 0..3 {
        world.pool.connect(&eff).await.unwrap();
    }

    assert_eq!(count(&server, "initialize").await, 1);
    assert_eq!(count(&server, "tools/list").await, 1);
}

#[tokio::test]
async fn the_tool_list_is_refreshed_after_its_ttl_without_reconnecting() {
    let script = Script::new(&["t"]);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig {
        tools_ttl: Duration::from_secs(5 * 60),
        ..ConnectionsConfig::default()
    });
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);
    world.pool.connect(&eff).await.unwrap();

    script.tools.lock().unwrap().push("added");
    world.clock.advance(SIX_MINUTES);
    let again = world.pool.connect(&eff).await.unwrap();

    assert_eq!(names(&again.tools), vec!["t", "added"]);
    assert_eq!(count(&server, "initialize").await, 1);
}

#[tokio::test]
async fn a_connection_idle_for_ten_minutes_is_evicted() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);
    world.pool.connect(&eff).await.unwrap();

    world.clock.advance(Duration::from_secs(9 * 60));
    world.pool.connect(&eff).await.unwrap();
    assert_eq!(
        count(&server, "initialize").await,
        1,
        "still warm after 9 min"
    );

    world.clock.advance(Duration::from_secs(10 * 60 + 1));
    world.pool.evict_idle().await;
    assert_eq!(world.pool.connection_count().await, 0);

    world.pool.connect(&eff).await.unwrap();
    assert_eq!(
        count(&server, "initialize").await,
        2,
        "reconnects after eviction"
    );
}

#[tokio::test]
async fn using_a_connection_keeps_it_from_going_idle() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);

    for _ in 0..4 {
        world.pool.connect(&eff).await.unwrap();
        world.clock.advance(Duration::from_secs(8 * 60));
    }

    assert_eq!(count(&server, "initialize").await, 1);
}

#[tokio::test]
async fn a_changed_integration_reconnects() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();

    let mut edited = integration.clone();
    edited.updated_at = at(60);
    world.store.update(&edited).await.unwrap();
    world.pool.connect(&effective(&edited, None)).await.unwrap();

    assert_eq!(count(&server, "initialize").await, 2);
}

#[tokio::test]
async fn a_replaced_credential_reconnects_with_the_new_value() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    let eff = effective(&integration, None);
    world.credential(&integration, None, "old", 0).await;
    world.pool.connect(&eff).await.unwrap();

    world.credential(&integration, None, "new", 60).await;
    world.pool.connect(&eff).await.unwrap();

    assert_eq!(count(&server, "initialize").await, 2);
    let requests = server.received_requests().await.unwrap();
    let last_auth = requests
        .iter()
        .rev()
        .find_map(|r| r.headers.get("authorization").and_then(|v| v.to_str().ok()));
    assert_eq!(last_auth, Some("Bearer new"));
}

#[tokio::test]
async fn each_subject_gets_its_own_connection_and_credential() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world
        .integration(&server.uri(), |i| {
            i.credential_mode = CredentialMode::PerUser
        })
        .await;
    let (u1, u2) = (UserId::from("usr_1"), UserId::from("usr_2"));
    world
        .credential(&integration, Some(&u1), "token-1", 0)
        .await;
    world
        .credential(&integration, Some(&u2), "token-2", 0)
        .await;

    world
        .pool
        .connect(&effective(&integration, Some(&u1)))
        .await
        .unwrap();
    world
        .pool
        .connect(&effective(&integration, Some(&u2)))
        .await
        .unwrap();
    world
        .pool
        .connect(&effective(&integration, Some(&u1)))
        .await
        .unwrap();

    assert_eq!(world.pool.connection_count().await, 2);
    assert_eq!(count(&server, "initialize").await, 2);
    let requests = server.received_requests().await.unwrap();
    for token in ["Bearer token-1", "Bearer token-2"] {
        assert!(
            requests.iter().any(
                |r| r.headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(token)
            ),
            "{token} was never sent"
        );
    }
}

#[tokio::test]
async fn a_connection_is_recorded_as_healthy_with_its_tool_count() {
    let server = serve(&Script::new(&["a", "b", "c"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    world.clock.advance(Duration::from_secs(42));

    world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();

    let health = world.health.get("int_1", None).await.unwrap().unwrap();
    assert!(health.reachable);
    assert_eq!(health.tool_count, Some(3));
    assert_eq!(health.last_connected_at, Some(at(42)));
}

#[tokio::test]
async fn a_slow_server_times_out_and_is_marked_unreachable() {
    let mut script = Script::new(&["t"]);
    script.initialize_delay = Some(Duration::from_secs(30));
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig {
        connect_timeout: Duration::from_millis(200),
        ..ConnectionsConfig::default()
    });
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;

    let started = std::time::Instant::now();
    let result = world.pool.connect(&effective(&integration, None)).await;

    assert!(
        matches!(result, Err(ConnectError::Unreachable(_))),
        "got {:?}",
        result.err()
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    let health = world.health.get("int_1", None).await.unwrap().unwrap();
    assert!(!health.reachable);
    assert_eq!(health.last_error.as_deref(), Some("timeout"));
    assert_eq!(world.pool.connection_count().await, 0);
}

#[tokio::test]
async fn an_unreachable_integration_is_not_retried_on_every_turn() {
    let script = Script::new(&["t"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig {
        retry_after: Duration::from_secs(30),
        ..ConnectionsConfig::default()
    });
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);

    assert!(world.pool.connect(&eff).await.is_err());
    let after_first = server.received_requests().await.unwrap().len();
    assert!(world.pool.connect(&eff).await.is_err());
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        after_first,
        "a recent failure must be answered without another attempt"
    );

    script.healthy.store(true, Ordering::SeqCst);
    world.clock.advance(Duration::from_secs(31));
    assert!(
        world.pool.connect(&eff).await.is_ok(),
        "retries once the pause is over"
    );
    assert!(
        world
            .health
            .get("int_1", None)
            .await
            .unwrap()
            .unwrap()
            .reachable
    );
}

#[tokio::test]
async fn editing_the_integration_ends_the_pause_after_a_failure() {
    let script = Script::new(&["t"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    assert!(
        world
            .pool
            .connect(&effective(&integration, None))
            .await
            .is_err()
    );

    script.healthy.store(true, Ordering::SeqCst);
    let mut fixed = integration.clone();
    fixed.updated_at = at(5);
    world.store.update(&fixed).await.unwrap();
    assert!(world.pool.connect(&effective(&fixed, None)).await.is_ok());
}

#[tokio::test]
async fn a_missing_credential_fails_without_contacting_the_server() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;

    let result = world.pool.connect(&effective(&integration, None)).await;

    assert!(matches!(result, Err(ConnectError::Credential(_))));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_credential_sealed_under_another_key_fails_without_contacting_the_server() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    let foreign = SecretBox::from_key([1; 32]);
    let aad = credential_aad("int_1", None, "header");
    let sealed = foreign.seal(&Secret::new("x"), aad.as_bytes()).unwrap();
    world
        .credentials
        .put(&StoredCredential {
            integration_id: "int_1".into(),
            subject_user_id: None,
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

    let result = world.pool.connect(&effective(&integration, None)).await;

    assert!(matches!(result, Err(ConnectError::Credential(_))));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn the_error_never_contains_the_credential() {
    let script = Script::new(&["t"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world
        .credential(&integration, None, "ghp_supersecret", 0)
        .await;

    let error = world
        .pool
        .connect(&effective(&integration, None))
        .await
        .err()
        .unwrap();
    let health = world.health.get("int_1", None).await.unwrap().unwrap();

    assert!(!error.to_string().contains("ghp_supersecret"));
    assert!(
        !health
            .last_error
            .unwrap_or_default()
            .contains("ghp_supersecret")
    );
}

// -- A server that stops answering ------------------------------------------

#[tokio::test]
async fn a_hung_tool_refresh_fails_within_the_timeout_and_drops_the_connection() {
    let script = Script::new(&["t"]);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig {
        connect_timeout: Duration::from_millis(300),
        tools_ttl: Duration::from_secs(5 * 60),
        ..ConnectionsConfig::default()
    });
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);
    world.pool.connect(&eff).await.unwrap();

    *script.list_delay.lock().unwrap() = Some(Duration::from_secs(30));
    world.clock.advance(SIX_MINUTES);
    let started = std::time::Instant::now();
    let result = world.pool.connect(&eff).await;

    assert!(
        matches!(result, Err(ConnectError::Unreachable(_))),
        "got {:?}",
        result.err()
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "a hung refresh must not block the turn, took {:?}",
        started.elapsed()
    );
    assert_eq!(world.pool.connection_count().await, 0);
    assert!(
        !world
            .health
            .get("int_1", None)
            .await
            .unwrap()
            .unwrap()
            .reachable
    );

    let again = std::time::Instant::now();
    assert!(world.pool.connect(&eff).await.is_err());
    assert!(
        again.elapsed() < Duration::from_millis(100),
        "the next turn is answered from the failure, not another wait"
    );
}

// -- A credential read before the lock goes stale ----------------------------

/// Reads from the inner store, then holds the first `get` until released, so
/// its answer is stale by the time the caller sees it.
struct StaleFirstRead {
    inner: Arc<dyn IntegrationCredentialStore>,
    first: std::sync::atomic::AtomicBool,
    paused: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}

#[async_trait::async_trait]
impl IntegrationCredentialStore for StaleFirstRead {
    async fn put(&self, c: &StoredCredential) -> anyhow::Result<()> {
        self.inner.put(c).await
    }
    async fn get(
        &self,
        integration_id: &str,
        subject: Option<&UserId>,
    ) -> anyhow::Result<Option<StoredCredential>> {
        let read = self.inner.get(integration_id, subject).await?;
        if !self.first.swap(true, Ordering::SeqCst) {
            self.paused.notify_one();
            self.release.notified().await;
        }
        Ok(read)
    }
    async fn delete(&self, integration_id: &str, subject: Option<&UserId>) -> anyhow::Result<bool> {
        self.inner.delete(integration_id, subject).await
    }
    async fn compare_and_swap(
        &self,
        expected: DateTime<Utc>,
        c: &StoredCredential,
    ) -> anyhow::Result<bool> {
        self.inner.compare_and_swap(expected, c).await
    }
}

#[tokio::test]
async fn a_credential_replaced_during_a_slow_read_ends_up_as_the_connection_in_use() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let paused = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let pool = Arc::new(
        IntegrationConnections::new(
            world.store.clone(),
            Arc::new(StaleFirstRead {
                inner: world.credentials.clone(),
                first: Default::default(),
                paused: paused.clone(),
                release: release.clone(),
            }),
            world.health.clone(),
            world.secrets.clone(),
            Arc::new(world.clock.clone()),
        )
        .with_config(ConnectionsConfig {
            allow_plain_http: true,
            ..ConnectionsConfig::default()
        }),
    );
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "old", 0).await;
    let eff = effective(&integration, None);

    let a = tokio::spawn({
        let (pool, eff) = (pool.clone(), eff.clone());
        async move { pool.connect(&eff).await.map(|_| ()) }
    });
    paused.notified().await;
    world.credential(&integration, None, "new", 60).await;
    let b = tokio::spawn({
        let (pool, eff) = (pool.clone(), eff.clone());
        async move { pool.connect(&eff).await.map(|_| ()) }
    });
    tokio::time::sleep(Duration::from_millis(150)).await;
    release.notify_one();
    a.await.unwrap().unwrap();
    b.await.unwrap().unwrap();
    pool.connect(&eff).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let last_auth = requests
        .iter()
        .rev()
        .find_map(|r| r.headers.get("authorization").and_then(|v| v.to_str().ok()));
    assert_eq!(
        last_auth,
        Some("Bearer new"),
        "the live connection must carry the newest credential"
    );
    assert_eq!(
        count(&server, "initialize").await,
        2,
        "a stale read must not cost an extra reconnect"
    );
}

// -- The pool does not grow without bound -----------------------------------

#[tokio::test]
async fn an_evicted_connection_leaves_no_entry_behind() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();
    assert_eq!(world.pool.tracked(), 1);

    world.clock.advance(Duration::from_secs(10 * 60 + 1));
    world.pool.evict_idle().await;

    assert_eq!(world.pool.tracked(), 0);
}

#[tokio::test]
async fn an_expired_failure_leaves_no_entry_behind() {
    let script = Script::new(&["t"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig {
        retry_after: Duration::from_secs(30),
        ..ConnectionsConfig::default()
    });
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    assert!(
        world
            .pool
            .connect(&effective(&integration, None))
            .await
            .is_err()
    );
    assert_eq!(
        world.pool.tracked(),
        1,
        "the failure is remembered for a while"
    );

    world.clock.advance(Duration::from_secs(31));
    world.pool.evict_idle().await;

    assert_eq!(world.pool.tracked(), 0);
}

#[tokio::test]
async fn the_pool_sweeps_itself_without_being_asked() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig {
        sweep_interval: Duration::from_millis(20),
        ..ConnectionsConfig::default()
    });
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    world
        .pool
        .connect(&effective(&integration, None))
        .await
        .unwrap();

    world.clock.advance(Duration::from_secs(11 * 60));
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(
        world.pool.tracked(),
        0,
        "an idle connection is dropped by the background sweep"
    );
}

// -- probing ------------------------------------------------------------------

#[tokio::test]
async fn probing_the_same_integration_again_too_soon_is_throttled() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig {
        probe_interval: Duration::from_secs(5),
        ..ConnectionsConfig::default()
    });
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);

    world.pool.probe(&eff).await.unwrap();
    let requests = server.received_requests().await.unwrap().len();
    world.clock.advance(Duration::from_secs(2));
    let again = world.pool.probe(&eff).await;

    assert_eq!(
        again.err(),
        Some(ConnectError::Throttled {
            retry_after: Duration::from_secs(3)
        })
    );
    assert_eq!(server.received_requests().await.unwrap().len(), requests);

    world.clock.advance(Duration::from_secs(3));
    world.pool.probe(&eff).await.unwrap();
}

#[tokio::test]
async fn a_failed_probe_is_throttled_too() {
    let script = Script::new(&["t"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);

    assert!(matches!(
        world.pool.probe(&eff).await,
        Err(ConnectError::Unreachable(_))
    ));
    assert!(matches!(
        world.pool.probe(&eff).await,
        Err(ConnectError::Throttled { .. })
    ));
}

#[tokio::test]
async fn the_throttle_is_per_integration_and_subject() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world
        .integration(&server.uri(), |i| {
            i.credential_mode = CredentialMode::PerUser
        })
        .await;
    let (u1, u2) = (UserId::from("usr_1"), UserId::from("usr_2"));
    world.credential(&integration, Some(&u1), "a", 0).await;
    world.credential(&integration, Some(&u2), "b", 0).await;

    world
        .pool
        .probe(&effective(&integration, Some(&u1)))
        .await
        .unwrap();
    world
        .pool
        .probe(&effective(&integration, Some(&u2)))
        .await
        .unwrap();
}

// -- what a failure says --------------------------------------------------------

async fn unreachable_message(world: &World, integration: &Integration) -> String {
    let error = world
        .pool
        .connect(&effective(integration, None))
        .await
        .err()
        .expect("the connection must fail");
    let health = world.health.get("int_1", None).await.unwrap().unwrap();
    assert_eq!(health.last_error, Some(error.message()));
    error.message()
}

#[tokio::test]
async fn failures_are_reported_by_kind_and_never_with_the_raw_transport_error() {
    // HTTP status.
    let script = Script::new(&["t"]);
    script.healthy.store(false, Ordering::SeqCst);
    let server = serve(&script).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    assert_eq!(unreachable_message(&world, &integration).await, "HTTP 500");

    // Nothing listening.
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration("http://127.0.0.1:9/mcp", |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    assert_eq!(
        unreachable_message(&world, &integration).await,
        "connection failed"
    );

    // Not an MCP server: a 200 that is not JSON-RPC.
    let garbage = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>hello</html>"))
        .mount(&garbage)
        .await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&garbage.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    assert_eq!(
        unreachable_message(&world, &integration).await,
        "protocol error"
    );

    // TLS spoken to a plain-HTTP server: rmcp does not say why the request failed.
    let plain = serve(&Script::new(&["t"])).await;
    let https_url = plain.uri().replacen("http://", "https://", 1);
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&https_url, |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    assert_eq!(
        unreachable_message(&world, &integration).await,
        "connection failed"
    );
}

// -- Credentials only travel over https ------------------------------------------

#[tokio::test]
async fn a_credential_is_never_sent_over_plain_http() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::with_exact_config(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);

    let connect = world.pool.connect(&eff).await.err();
    let probe = world.pool.probe(&eff).await.err();

    for result in [connect, probe] {
        assert!(
            matches!(&result, Some(ConnectError::Credential(m)) if m.contains("https")),
            "got {result:?}"
        );
    }
    assert!(
        server.received_requests().await.unwrap().is_empty(),
        "nothing may reach the server, let alone the credential"
    );
}

#[tokio::test]
async fn an_integration_without_a_credential_may_use_plain_http() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::with_exact_config(ConnectionsConfig::default());
    let integration = world
        .integration(&server.uri(), |i| {
            i.auth_kind = IntegrationAuthKind::None;
            i.header_name = None;
            i.header_value_prefix = None;
        })
        .await;
    assert!(
        world
            .pool
            .connect(&effective(&integration, None))
            .await
            .is_ok()
    );
}

// -- The credential belongs to the URL it is sent to ----------------------------

/// The integration moved to another host after the turn read it, and a
/// credential for the new host is already stored: the old URL must not get it.
async fn moved_since_read(world: &World, old_url: &str, new_url: &str) -> EffectiveIntegration {
    let integration = world
        .integration(old_url, |i| i.catalog_entry_id = None)
        .await;
    let snapshot = effective(&integration, None);
    let mut moved = integration.clone();
    moved.connect_url = new_url.into();
    moved.updated_at = at(60);
    world.store.update(&moved).await.unwrap(); // drops the old credentials
    world
        .credential(&moved, None, "token-for-the-new-host", 61)
        .await;
    snapshot
}

#[tokio::test]
async fn a_url_change_after_the_read_stops_connect_before_anything_is_sent() {
    let old = serve(&Script::new(&["t"])).await;
    let new = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let snapshot = moved_since_read(&world, &old.uri(), &new.uri()).await;

    let result = world.pool.connect(&snapshot).await;

    assert!(
        matches!(result, Err(ConnectError::Changed)),
        "got {:?}",
        result.err()
    );
    assert!(old.received_requests().await.unwrap().is_empty());
    assert!(new.received_requests().await.unwrap().is_empty());
    assert_eq!(world.pool.connection_count().await, 0);
}

#[tokio::test]
async fn a_url_change_after_the_read_stops_probe_before_anything_is_sent() {
    let old = serve(&Script::new(&["t"])).await;
    let new = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let snapshot = moved_since_read(&world, &old.uri(), &new.uri()).await;

    let result = world.pool.probe(&snapshot).await;

    assert!(
        matches!(result, Err(ConnectError::Changed)),
        "got {:?}",
        result.err()
    );
    assert!(old.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_integration_removed_after_the_read_is_not_connected() {
    let server = serve(&Script::new(&["t"])).await;
    let world = World::new(ConnectionsConfig::default());
    let integration = world.integration(&server.uri(), |_| {}).await;
    world.credential(&integration, None, "ghp_abc", 0).await;
    let eff = effective(&integration, None);
    world.store.delete("int_1").await.unwrap();

    let result = world.pool.connect(&eff).await;

    assert!(result.is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}
