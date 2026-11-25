//! Cross-tenant isolation harness for the data-plane routes.
//!
//! Contract under test (`openspec/changes/multi-tenant-data-plane`): a caller
//! reaches only the spaces they hold a role in, each route enforces a minimum
//! role, and conversations are private to their owner.
//!
//! [`Tenancy::build`] is the single place that knows how the data-plane
//! routers are wired. The tests only speak HTTP, so rewiring the routers to
//! per-request space resolution changes `build` and nothing else.
//!
//! Topology:
//!
//! ```text
//! org_a ── A1 (alice, amy: member · vera: viewer · adam: space_admin)
//!       └─ A2 (carol: member)
//! org_b ── B1 (bob: member)
//! ```
//!
//! Every fixture row lives in A1.
//!
//! The isolation tests are `#[ignore]`d so the stack stays mergeable: they fail
//! today, because the routers share one pool and never read `AuthContext`.
//! Reproduce the red state with
//! `cargo test -p assistant-web-ui --lib tenancy_tests -- --ignored`. The PRs
//! that implement resolution and authorization remove the attribute as each
//! module goes green.

#![cfg(test)]

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

use assistant_core::AttachmentMeta;
use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
use assistant_core::types::conversation::Message;
use assistant_storage::{
    AttachmentStore as _, CommandEventStore as _, ConversationEventStore as _, ConversationStore,
    PushSubscriptionStore, SqliteCommandEventStore, SqliteConversationStore, StorageLayer,
    WebhookStore,
};
use assistant_transcription::{TtsProvider, TtsRequest, TtsResult};

use super::push::{PushApiState, push_api_router};
use super::test_helpers::{
    TestSpace, body_bytes, event_log_state, multipart_body, resolver_for, tiny_png,
};
use super::webhooks::{WebhooksApiState, webhooks_api_router};
use super::workflows::{WorkflowsApiState, workflows_api_router};
use super::{ApiState, api_router};
use crate::space::SpaceResolver;

const ORG_A: &str = "org_a";
const ORG_B: &str = "org_b";
const A1: &str = "spc_a1";
const A2: &str = "spc_a2";
const B1: &str = "spc_b1";

const WEBHOOK_SECRET: &str = "whsec_fixture";

// -- Principals ----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Who {
    /// Member of A1; owns the fixture conversation.
    Alice,
    /// Member of A1; owns nothing, same space as Alice.
    Amy,
    /// Viewer of A1.
    Vera,
    /// Space admin of A1.
    Adam,
    /// Member of A2, sibling space in the same org.
    Carol,
    /// Member of B1 in a different org.
    Bob,
}

impl Who {
    const ALL: [Who; 6] = [
        Who::Alice,
        Who::Amy,
        Who::Vera,
        Who::Adam,
        Who::Carol,
        Who::Bob,
    ];

    fn name(self) -> &'static str {
        match self {
            Who::Alice => "alice",
            Who::Amy => "amy",
            Who::Vera => "vera",
            Who::Adam => "adam",
            Who::Carol => "carol",
            Who::Bob => "bob",
        }
    }

    fn context(self) -> AuthContext {
        let (org, space, role) = match self {
            Who::Alice | Who::Amy => (ORG_A, A1, Role::Member),
            Who::Vera => (ORG_A, A1, Role::Viewer),
            Who::Adam => (ORG_A, A1, Role::SpaceAdmin),
            Who::Carol => (ORG_A, A2, Role::Member),
            Who::Bob => (ORG_B, B1, Role::Member),
        };
        AuthContext {
            user_id: UserId::from(self.name()),
            org_id: OrgId::from(org),
            email: format!("{}@example.test", self.name()),
            space_roles: HashMap::from([(SpaceId::from(space), role)]),
            scopes: vec![],
            client_id: "test".into(),
        }
    }
}

/// Test-only stand-in for `require_auth`: resolves the `x-test-principal`
/// header to a canned [`AuthContext`].
async fn inject_principal(
    State(principals): State<Arc<HashMap<&'static str, AuthContext>>>,
    mut request: Request,
    next: Next,
) -> Response {
    let ctx = request
        .headers()
        .get("x-test-principal")
        .and_then(|v| v.to_str().ok())
        .and_then(|name| principals.get(name))
        .cloned();
    match ctx {
        Some(ctx) => {
            request.extensions_mut().insert(ctx);
            next.run(request).await
        }
        None => Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::empty())
            .unwrap_or_default(),
    }
}

struct StubTts;

#[async_trait]
impl TtsProvider for StubTts {
    fn name(&self) -> &str {
        "stub"
    }

    async fn synthesize(&self, _request: TtsRequest) -> anyhow::Result<TtsResult> {
        Ok(TtsResult {
            audio_data: b"audio".to_vec(),
            mime_type: "audio/mpeg".into(),
        })
    }
}

// -- Harness -------------------------------------------------------------------

struct Resp {
    status: StatusCode,
    body: Vec<u8>,
}

impl Resp {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
}

/// Resolver for the topology above. A1 is served from `a1`; the other spaces
/// get empty storage of their own.
async fn topology_resolver(a1: &Arc<StorageLayer>) -> SpaceResolver {
    let empty = || async { Arc::new(StorageLayer::new_in_memory().await.unwrap()) };
    resolver_for(&[
        TestSpace {
            id: A1,
            org_id: ORG_A,
            org_slug: "acme",
            slug: "eng",
            storage: a1.clone(),
        },
        TestSpace {
            id: A2,
            org_id: ORG_A,
            org_slug: "acme",
            slug: "ops",
            storage: empty().await,
        },
        TestSpace {
            id: B1,
            org_id: ORG_B,
            org_slug: "globex",
            slug: "eng",
            storage: empty().await,
        },
    ])
    .await
}

/// The wired data-plane router plus ids of the rows seeded into space A1.
struct Tenancy {
    router: Router,
    /// Space A1's storage, for seeding and for asserting nothing leaked in.
    a1: Arc<StorageLayer>,
    audio_id: Uuid,
    alice_conv: Uuid,
    amy_conv: Uuid,
    alice_message: Uuid,
    turn_id: Uuid,
    attachment_id: Uuid,
    webhook_id: String,
    workflow_id: String,
}

impl Tenancy {
    async fn build() -> Self {
        let (mut api_state, a1) = event_log_state().await;
        api_state.tts_provider = Some(Arc::new(StubTts));
        api_state.spaces = topology_resolver(&a1).await;
        let agent_id = Arc::new(RwLock::new("default".to_string()));

        let router = api_router()
            .with_state(api_state.clone())
            .merge(webhooks_api_router().with_state(WebhooksApiState {
                pool: a1.pool.clone(),
                agent_id: agent_id.clone(),
            }))
            .merge(workflows_api_router().with_state(WorkflowsApiState {
                pool: a1.pool.clone(),
                agent_id,
            }))
            .merge(push_api_router().with_state(PushApiState {
                store: Arc::new(a1.push_subscription_store()),
                vapid_public_key: Arc::new("vapid-key".to_string()),
            }));
        let principals: HashMap<&'static str, AuthContext> =
            Who::ALL.iter().map(|w| (w.name(), w.context())).collect();
        let router = router.layer(axum::middleware::from_fn_with_state(
            Arc::new(principals),
            inject_principal,
        ));

        let mut tenancy = Self {
            router,
            a1,
            audio_id: Uuid::nil(),
            alice_conv: Uuid::nil(),
            amy_conv: Uuid::nil(),
            alice_message: Uuid::nil(),
            turn_id: Uuid::new_v4(),
            attachment_id: Uuid::new_v4(),
            webhook_id: "wh_fixture".into(),
            workflow_id: String::new(),
        };
        tenancy.seed(&api_state).await;
        tenancy
    }

    async fn seed(&mut self, state: &ApiState) {
        let owned_by = |user: &str| {
            SqliteConversationStore::for_agent(self.a1.pool.clone(), "default").with_user_id(user)
        };
        let alice_conv = owned_by("alice")
            .create_conversation(Some("alice chat"))
            .await
            .unwrap();
        let amy_conv = owned_by("amy")
            .create_conversation(Some("amy chat"))
            .await
            .unwrap();
        self.alice_conv = alice_conv.id;
        self.amy_conv = amy_conv.id;

        let reply = Message::assistant(self.alice_conv, "hello from alice");
        self.alice_message = reply.id;
        owned_by("alice").save_message(&reply).await.unwrap();

        self.a1
            .conversation_event_store()
            .append_event(
                &self.turn_id.to_string(),
                &self.alice_conv.to_string(),
                1,
                "token",
                &json!({"text": "hi"}),
            )
            .await
            .unwrap();
        SqliteCommandEventStore::new(self.a1.pool.clone())
            .save_event(self.alice_conv, "model", None, "alice-only ack")
            .await
            .unwrap();
        self.a1
            .attachment_store()
            .store(
                &AttachmentMeta {
                    id: self.attachment_id,
                    message_id: None,
                    conversation_id: self.alice_conv,
                    agent_id: "default".into(),
                    filename: "secret.png".into(),
                    mime_type: "image/png".into(),
                    size_bytes: tiny_png().len() as u64,
                    created_at: chrono::DateTime::UNIX_EPOCH,
                },
                &tiny_png(),
            )
            .await
            .unwrap();
        self.audio_id = state
            .audio_store
            .insert(b"audio".to_vec(), "audio/mpeg".into())
            .await;

        self.a1
            .webhook_store_for_agent("default")
            .create(
                &self.webhook_id,
                "fixture",
                "https://example.test/hook",
                WEBHOOK_SECRET,
                &["conversation.created".to_string()],
            )
            .await
            .unwrap();

        let created = self
            .send(
                Who::Adam,
                Method::POST,
                "/workflows",
                None,
                Some(json!({
                    "name": "fixture flow",
                    "graph": flow_graph(),
                })),
            )
            .await;
        assert_eq!(created.status, StatusCode::CREATED, "seeding a workflow");
        self.workflow_id = created.json()["id"]
            .as_str()
            .unwrap_or_default()
            .to_string();
    }

    /// Send a JSON request as `who`, optionally selecting a space.
    async fn send(
        &self,
        who: Who,
        method: Method,
        uri: &str,
        space: Option<&str>,
        body: Option<Value>,
    ) -> Resp {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("x-test-principal", who.name());
        if let Some(space) = space {
            builder = builder.header("X-Space-Id", space);
        }
        let request = match body {
            Some(json) => builder
                .header("content-type", "application/json")
                .body(Body::from(json.to_string())),
            None => builder.body(Body::empty()),
        }
        .unwrap();
        self.dispatch(request).await
    }

    async fn dispatch(&self, request: Request) -> Resp {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        // SSE responses never end; only the status line matters for them.
        let body = if response
            .headers()
            .get("content-type")
            .is_some_and(|v| v.as_bytes().starts_with(b"text/event-stream"))
        {
            Vec::new()
        } else {
            body_bytes(response.into_body()).await
        };
        Resp { status, body }
    }

    async fn get(&self, who: Who, uri: &str, space: Option<&str>) -> Resp {
        self.send(who, Method::GET, uri, space, None).await
    }

    async fn conversation_exists(&self, id: Uuid) -> bool {
        SqliteConversationStore::for_agent(self.a1.pool.clone(), "default")
            .get_conversation(id)
            .await
            .unwrap()
            .is_some()
    }

    async fn webhook_secret(&self) -> String {
        self.a1
            .webhook_store_for_agent("default")
            .get(&self.webhook_id)
            .await
            .unwrap()
            .map(|w| w.secret)
            .unwrap_or_default()
    }
}

fn flow_graph() -> Value {
    json!({
        "version": 1,
        "nodes": [
            { "id": "t1", "kind": "trigger", "config": { "type": "manual" } },
            { "id": "a1", "kind": "action",
              "config": { "type": "assistant_turn", "prompt": "hello" } },
        ],
        "edges": [{ "from": "t1", "to": "a1" }],
        "execution": { "max_steps": 100, "max_visits_per_node": 10 }
    })
}

/// Assert `resp` is a denial (not a success and not a server error).
#[track_caller]
fn assert_forbidden(resp: &Resp, what: &str) {
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{what}");
}

#[track_caller]
fn assert_not_found(resp: &Resp, what: &str) {
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{what}");
}

fn ids_in(list: &Value) -> Vec<String> {
    list.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// -- Conversations -------------------------------------------------------------

#[tokio::test]
async fn conversations_foreign_org_selecting_a1_is_forbidden() {
    let t = Tenancy::build().await;
    let conv = format!("/conversations/{}", t.alice_conv);

    assert_forbidden(
        &t.get(Who::Bob, "/conversations", Some(A1)).await,
        "list conversations",
    );
    assert_forbidden(
        &t.get(Who::Bob, "/conversations/stream", Some(A1)).await,
        "stream conversations",
    );
    assert_forbidden(&t.get(Who::Bob, &conv, Some(A1)).await, "get conversation");
    assert_forbidden(
        &t.send(Who::Bob, Method::DELETE, &conv, Some(A1), None)
            .await,
        "delete conversation",
    );
    assert_forbidden(
        &t.send(
            Who::Bob,
            Method::PATCH,
            &conv,
            Some(A1),
            Some(json!({"title": "pwned"})),
        )
        .await,
        "rename conversation",
    );
    assert_forbidden(
        &t.send(
            Who::Bob,
            Method::POST,
            "/conversations",
            Some(A1),
            Some(json!({"title": "planted"})),
        )
        .await,
        "create conversation",
    );
    assert!(
        t.conversation_exists(t.alice_conv).await,
        "row must survive"
    );
}

#[tokio::test]
async fn conversations_sibling_space_member_selecting_a1_is_forbidden() {
    let t = Tenancy::build().await;
    let conv = format!("/conversations/{}", t.alice_conv);

    assert_forbidden(
        &t.get(Who::Carol, "/conversations", Some(A1)).await,
        "list conversations",
    );
    assert_forbidden(
        &t.get(Who::Carol, &conv, Some(A1)).await,
        "get conversation",
    );
    assert_forbidden(
        &t.send(Who::Carol, Method::DELETE, &conv, Some(A1), None)
            .await,
        "delete conversation",
    );
    assert!(
        t.conversation_exists(t.alice_conv).await,
        "row must survive"
    );
}

#[tokio::test]
async fn conversations_unknown_space_selector_is_forbidden() {
    let t = Tenancy::build().await;
    assert_forbidden(
        &t.get(Who::Alice, "/conversations", Some("spc_does_not_exist"))
            .await,
        "unknown space",
    );
}

#[tokio::test]
async fn conversations_without_selector_stay_in_the_callers_own_space() {
    let t = Tenancy::build().await;
    let conv = format!("/conversations/{}", t.alice_conv);

    let list = t.get(Who::Bob, "/conversations", None).await;
    assert_eq!(list.status, StatusCode::OK);
    assert!(
        !ids_in(&list.json()).contains(&t.alice_conv.to_string()),
        "bob's list must not contain A1 conversations"
    );
    assert_not_found(&t.get(Who::Bob, &conv, None).await, "get A1 conversation");
    assert_not_found(
        &t.send(Who::Bob, Method::DELETE, &conv, None, None).await,
        "delete A1 conversation",
    );
    assert!(
        t.conversation_exists(t.alice_conv).await,
        "row must survive"
    );
}

#[tokio::test]
async fn conversations_are_private_to_their_owner() {
    let t = Tenancy::build().await;
    let conv = format!("/conversations/{}", t.alice_conv);

    let list = t.get(Who::Amy, "/conversations", None).await;
    let ids = ids_in(&list.json());
    assert!(ids.contains(&t.amy_conv.to_string()), "amy sees her own");
    assert!(
        !ids.contains(&t.alice_conv.to_string()),
        "amy must not see alice's conversation"
    );
    assert_not_found(&t.get(Who::Amy, &conv, None).await, "amy reads alice's");
    assert_not_found(
        &t.send(
            Who::Amy,
            Method::PATCH,
            &conv,
            None,
            Some(json!({"title": "mine now"})),
        )
        .await,
        "amy renames alice's",
    );
    assert_not_found(
        &t.send(Who::Amy, Method::DELETE, &conv, None, None).await,
        "amy deletes alice's",
    );
    assert_not_found(
        &t.get(Who::Adam, &conv, None).await,
        "even a space admin is not the owner",
    );
    assert!(
        t.conversation_exists(t.alice_conv).await,
        "row must survive"
    );
}

#[tokio::test]
async fn conversations_viewer_cannot_write() {
    let t = Tenancy::build().await;
    let conv = format!("/conversations/{}", t.alice_conv);

    assert_forbidden(
        &t.send(
            Who::Vera,
            Method::POST,
            "/conversations",
            None,
            Some(json!({"title": "viewer chat"})),
        )
        .await,
        "viewer creates conversation",
    );
    assert_forbidden(
        &t.send(Who::Vera, Method::DELETE, &conv, None, None).await,
        "viewer deletes conversation",
    );
    assert!(
        t.conversation_exists(t.alice_conv).await,
        "row must survive"
    );
}

#[tokio::test]
async fn conversations_viewer_can_read_their_own() {
    let t = Tenancy::build().await;
    assert_eq!(
        t.get(Who::Vera, "/conversations", None).await.status,
        StatusCode::OK
    );
}

// -- Turns ---------------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn turns_are_isolated_across_tenants_and_owners() {
    let t = Tenancy::build().await;
    let status = format!("/conversations/{}/turns/{}/status", t.alice_conv, t.turn_id);
    let cancel = format!("/conversations/{}/turns/{}/cancel", t.alice_conv, t.turn_id);

    assert_forbidden(&t.get(Who::Bob, &status, Some(A1)).await, "bob status");
    assert_forbidden(&t.get(Who::Carol, &status, Some(A1)).await, "carol status");
    assert_forbidden(
        &t.send(Who::Bob, Method::POST, &cancel, Some(A1), None)
            .await,
        "bob cancel",
    );
    assert_not_found(
        &t.get(Who::Bob, &status, None).await,
        "bob status without selector",
    );
    assert_not_found(&t.get(Who::Amy, &status, None).await, "amy status");
    assert_not_found(
        &t.send(Who::Amy, Method::POST, &cancel, None, None).await,
        "amy cancel",
    );
    assert_forbidden(
        &t.send(Who::Vera, Method::POST, &cancel, None, None).await,
        "viewer cancel",
    );
}

// -- Attachments ---------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn attachments_are_isolated_across_tenants_and_owners() {
    let t = Tenancy::build().await;
    let serve = format!("/attachments/{}", t.attachment_id);

    assert_forbidden(&t.get(Who::Bob, &serve, Some(A1)).await, "bob serve");
    assert_forbidden(&t.get(Who::Carol, &serve, Some(A1)).await, "carol serve");
    assert_not_found(
        &t.get(Who::Bob, &serve, None).await,
        "bob serve without selector",
    );
    assert_not_found(&t.get(Who::Amy, &serve, None).await, "amy serve");
}

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn attachments_upload_requires_write_in_the_callers_space() {
    let t = Tenancy::build().await;
    let uri = format!("/conversations/{}/attachments", t.alice_conv);
    let (content_type, body) = multipart_body("file", "x.png", "image/png", &tiny_png());

    for (who, space, what) in [
        (Who::Bob, Some(A1), "foreign org"),
        (Who::Carol, Some(A1), "sibling space"),
        (Who::Vera, None, "viewer"),
    ] {
        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(&uri)
            .header("x-test-principal", who.name())
            .header("content-type", &content_type);
        if let Some(space) = space {
            builder = builder.header("X-Space-Id", space);
        }
        let resp = t
            .dispatch(builder.body(Body::from(body.clone())).unwrap())
            .await;
        assert_forbidden(&resp, what);
    }
}

// -- Audio ---------------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn audio_is_isolated_across_tenants_and_owners() {
    let t = Tenancy::build().await;
    let message_audio = format!("/messages/{}/audio", t.alice_message);
    let blob = format!("/audio/{}", t.audio_id);

    assert_forbidden(
        &t.get(Who::Bob, &message_audio, Some(A1)).await,
        "bob message audio",
    );
    assert_forbidden(&t.get(Who::Bob, &blob, Some(A1)).await, "bob audio blob");
    assert_forbidden(
        &t.get(Who::Carol, &blob, Some(A1)).await,
        "carol audio blob",
    );
    assert_not_found(
        &t.get(Who::Bob, &message_audio, None).await,
        "bob message audio without selector",
    );
    assert_not_found(
        &t.get(Who::Amy, &message_audio, None).await,
        "amy reads alice's message audio",
    );
}

// -- Commands ------------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn commands_are_isolated_across_tenants_and_owners() {
    let t = Tenancy::build().await;
    let events = format!("/conversations/{}/events", t.alice_conv);
    let run = format!("/conversations/{}/command", t.alice_conv);

    assert_forbidden(&t.get(Who::Bob, "/commands", Some(A1)).await, "bob list");
    assert_forbidden(&t.get(Who::Bob, &events, Some(A1)).await, "bob events");
    assert_forbidden(&t.get(Who::Carol, &events, Some(A1)).await, "carol events");
    assert_forbidden(
        &t.send(
            Who::Bob,
            Method::POST,
            &run,
            Some(A1),
            Some(json!({"command": "model"})),
        )
        .await,
        "bob executes command",
    );
    assert_not_found(
        &t.get(Who::Bob, &events, None).await,
        "bob events without selector",
    );
    assert_not_found(&t.get(Who::Amy, &events, None).await, "amy events");
    assert_forbidden(
        &t.send(
            Who::Vera,
            Method::POST,
            &run,
            None,
            Some(json!({"command": "model"})),
        )
        .await,
        "viewer executes command",
    );
}

// -- Capabilities --------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn capabilities_require_a_role_in_the_selected_space() {
    let t = Tenancy::build().await;

    assert_forbidden(
        &t.get(Who::Bob, "/capabilities", Some(A1)).await,
        "foreign org",
    );
    assert_forbidden(
        &t.get(Who::Carol, "/capabilities", Some(A1)).await,
        "sibling space",
    );
    assert_forbidden(
        &t.get(Who::Alice, "/capabilities", Some("spc_does_not_exist"))
            .await,
        "unknown space",
    );
    assert_eq!(
        t.get(Who::Vera, "/capabilities", None).await.status,
        StatusCode::OK,
        "a viewer may read capabilities"
    );
}

// -- Push ----------------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn push_subscriptions_never_cross_spaces() {
    let t = Tenancy::build().await;
    let subscribe = json!({"endpoint": "https://push.test/bob", "p256dh": "k", "auth": "a"});

    assert_forbidden(
        &t.get(Who::Bob, "/push/vapid-public-key", Some(A1)).await,
        "bob vapid key",
    );
    assert_forbidden(
        &t.send(
            Who::Bob,
            Method::POST,
            "/push/subscribe",
            Some(A1),
            Some(subscribe.clone()),
        )
        .await,
        "bob subscribes into A1",
    );
    // Bob's own space is B1: whatever happens, A1's store stays empty.
    let _ = t
        .send(
            Who::Bob,
            Method::POST,
            "/push/subscribe",
            None,
            Some(subscribe),
        )
        .await;
    assert!(
        t.a1.push_subscription_store()
            .list_all()
            .await
            .unwrap()
            .is_empty(),
        "bob's subscription must not land in space A1"
    );
}

// -- Webhooks ------------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn webhooks_foreign_org_selecting_a1_is_forbidden() {
    let t = Tenancy::build().await;
    let hook = format!("/webhooks/{}", t.webhook_id);

    for (who, what) in [(Who::Bob, "foreign org"), (Who::Carol, "sibling space")] {
        assert_forbidden(&t.get(who, "/webhooks", Some(A1)).await, what);
        assert_forbidden(&t.get(who, &hook, Some(A1)).await, what);
        assert_forbidden(
            &t.send(
                who,
                Method::POST,
                "/webhooks",
                Some(A1),
                Some(json!({"name": "x", "url": "https://x.test", "event_types": []})),
            )
            .await,
            what,
        );
        assert_forbidden(
            &t.send(
                who,
                Method::PATCH,
                &hook,
                Some(A1),
                Some(json!({"name": "x"})),
            )
            .await,
            what,
        );
        assert_forbidden(
            &t.send(
                who,
                Method::POST,
                &format!("{hook}/rotate-secret"),
                Some(A1),
                None,
            )
            .await,
            what,
        );
        assert_forbidden(
            &t.send(who, Method::DELETE, &hook, Some(A1), None).await,
            what,
        );
    }
    assert_eq!(t.webhook_secret().await, WEBHOOK_SECRET, "secret unchanged");
}

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn webhooks_without_selector_stay_in_the_callers_own_space() {
    let t = Tenancy::build().await;
    let hook = format!("/webhooks/{}", t.webhook_id);

    let list = t.get(Who::Bob, "/webhooks", None).await;
    assert_eq!(list.status, StatusCode::OK);
    assert!(
        !ids_in(&list.json()).contains(&t.webhook_id),
        "bob's list must not contain A1 webhooks"
    );
    assert_not_found(&t.get(Who::Bob, &hook, None).await, "get A1 webhook");
    assert_not_found(
        &t.send(
            Who::Bob,
            Method::POST,
            &format!("{hook}/rotate-secret"),
            None,
            None,
        )
        .await,
        "rotate A1 webhook secret",
    );
    assert_not_found(
        &t.send(Who::Bob, Method::DELETE, &hook, None, None).await,
        "delete A1 webhook",
    );
    assert_eq!(t.webhook_secret().await, WEBHOOK_SECRET, "secret unchanged");
}

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn webhooks_secret_operations_require_space_admin() {
    let t = Tenancy::build().await;
    let hook = format!("/webhooks/{}", t.webhook_id);
    let rotate = format!("{hook}/rotate-secret");

    for who in [Who::Vera, Who::Alice] {
        assert_forbidden(
            &t.send(who, Method::POST, &rotate, None, None).await,
            "rotate secret below admin",
        );
        assert_forbidden(
            &t.send(who, Method::DELETE, &hook, None, None).await,
            "delete webhook below admin",
        );
    }
    assert_forbidden(
        &t.send(
            Who::Vera,
            Method::POST,
            "/webhooks",
            None,
            Some(json!({"name": "x", "url": "https://x.test", "event_types": []})),
        )
        .await,
        "viewer creates webhook",
    );
    assert_eq!(t.webhook_secret().await, WEBHOOK_SECRET, "secret unchanged");

    let rotated = t.send(Who::Adam, Method::POST, &rotate, None, None).await;
    assert_eq!(rotated.status, StatusCode::OK, "space admin may rotate");
    assert_ne!(t.webhook_secret().await, WEBHOOK_SECRET);
}

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn webhooks_secret_is_visible_only_to_space_admins() {
    let t = Tenancy::build().await;
    let hook = format!("/webhooks/{}", t.webhook_id);

    for who in [Who::Vera, Who::Alice] {
        let one = t.get(who, &hook, None).await;
        assert_eq!(one.status, StatusCode::OK);
        assert!(
            one.json().get("secret").is_none(),
            "{who:?} must not receive the signing secret"
        );
        let list = t.get(who, "/webhooks", None).await;
        assert!(
            list.json()
                .as_array()
                .is_some_and(|items| items.iter().all(|i| i.get("secret").is_none())),
            "{who:?} must not receive secrets in the list"
        );
    }
    let admin = t.get(Who::Adam, &hook, None).await;
    assert_eq!(admin.json()["secret"], WEBHOOK_SECRET);
}

// -- Workflows -----------------------------------------------------------------

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn workflows_foreign_org_selecting_a1_is_forbidden() {
    let t = Tenancy::build().await;
    let flow = format!("/workflows/{}", t.workflow_id);

    for (who, what) in [(Who::Bob, "foreign org"), (Who::Carol, "sibling space")] {
        assert_forbidden(&t.get(who, "/workflows", Some(A1)).await, what);
        assert_forbidden(&t.get(who, &flow, Some(A1)).await, what);
        assert_forbidden(&t.get(who, &format!("{flow}/runs"), Some(A1)).await, what);
        assert_forbidden(
            &t.get(who, &format!("{flow}/webhook-secrets"), Some(A1))
                .await,
            what,
        );
        for action in ["activate", "deactivate", "test-run"] {
            assert_forbidden(
                &t.send(
                    who,
                    Method::POST,
                    &format!("{flow}/{action}"),
                    Some(A1),
                    None,
                )
                .await,
                what,
            );
        }
        assert_forbidden(
            &t.send(who, Method::DELETE, &flow, Some(A1), None).await,
            what,
        );
    }
    let still_there = t.get(Who::Adam, &flow, None).await;
    assert_eq!(still_there.status, StatusCode::OK, "workflow must survive");
}

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn workflows_without_selector_stay_in_the_callers_own_space() {
    let t = Tenancy::build().await;
    let flow = format!("/workflows/{}", t.workflow_id);

    let list = t.get(Who::Bob, "/workflows", None).await;
    assert_eq!(list.status, StatusCode::OK);
    assert!(
        !ids_in(&list.json()).contains(&t.workflow_id),
        "bob's list must not contain A1 workflows"
    );
    assert_not_found(&t.get(Who::Bob, &flow, None).await, "get A1 workflow");
    assert_not_found(
        &t.send(Who::Bob, Method::DELETE, &flow, None, None).await,
        "delete A1 workflow",
    );
}

#[tokio::test]
#[ignore = "red until per-request space resolution lands (multi-tenant-data-plane PR 2/3)"]
async fn workflows_destructive_and_secret_operations_require_space_admin() {
    let t = Tenancy::build().await;
    let flow = format!("/workflows/{}", t.workflow_id);
    let secrets = format!("{flow}/webhook-secrets");

    assert_forbidden(
        &t.send(
            Who::Vera,
            Method::POST,
            "/workflows",
            None,
            Some(json!({"name": "x", "graph": flow_graph()})),
        )
        .await,
        "viewer creates workflow",
    );
    for who in [Who::Vera, Who::Alice] {
        assert_forbidden(&t.get(who, &secrets, None).await, "webhook secrets");
        assert_forbidden(
            &t.send(who, Method::DELETE, &flow, None, None).await,
            "delete workflow below admin",
        );
    }
    assert_eq!(
        t.get(Who::Adam, &secrets, None).await.status,
        StatusCode::OK,
        "space admin reads webhook secrets"
    );
    assert_eq!(
        t.send(Who::Adam, Method::DELETE, &flow, None, None)
            .await
            .status,
        StatusCode::NO_CONTENT,
        "space admin deletes"
    );
}
