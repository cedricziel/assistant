//! Test harness shared by the integration route tests: an in-memory org
//! database with a few users and spaces, the catalog fixture, and a request
//! helper that acts as one of them.

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::{Extension, Router};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use tower::ServiceExt;

use assistant_core::auth::AuthContext;
use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
use assistant_core::integrations::{
    CredentialKind, CredentialMode, CredentialStatus, Integration, IntegrationAuthKind,
    IntegrationLayer, IntegrationPolicy, IntegrationTrust, StoredCredential,
};
use assistant_core::types::features::CatalogConfig;
use assistant_storage::OrgStorageLayer;

use super::{IntegrationsApiState, integrations_api_router};
use crate::catalog_sync::CatalogSync;

const FIXTURE: &str = include_str!("../../../../core/tests/fixtures/integrations-catalog.json");

/// The key id the harness treats as the server's.
pub const KEY_ID: &str = "k";

/// The callers the tests act as. `Admin` is an org admin; `EngAdmin` and
/// `SalesAdmin` administer one space; `Member` and `Viewer` belong to `sp_eng`;
/// `Other` is a plain member of `sp_sales` only.
#[derive(Clone, Copy, Debug)]
pub enum Who {
    Admin,
    EngAdmin,
    Member,
    Viewer,
    Other,
    SalesAdmin,
}

impl Who {
    pub const ALL: [Who; 6] = [
        Who::Admin,
        Who::EngAdmin,
        Who::Member,
        Who::Viewer,
        Who::Other,
        Who::SalesAdmin,
    ];

    pub fn user_id(self) -> &'static str {
        match self {
            Who::Admin => "admin",
            Who::EngAdmin => "eng_admin",
            Who::Member => "member",
            Who::Viewer => "viewer",
            Who::Other => "other",
            Who::SalesAdmin => "sales_admin",
        }
    }

    pub fn user(self) -> UserId {
        UserId::from(self.user_id())
    }

    fn roles(self) -> HashMap<SpaceId, Role> {
        let (space, role) = match self {
            Who::Admin => ("sp_eng", Role::OrgAdmin),
            Who::EngAdmin => ("sp_eng", Role::SpaceAdmin),
            Who::Member => ("sp_eng", Role::Member),
            Who::Viewer => ("sp_eng", Role::Viewer),
            Who::Other => ("sp_sales", Role::Member),
            Who::SalesAdmin => ("sp_sales", Role::SpaceAdmin),
        };
        HashMap::from([(SpaceId::from(space), role)])
    }

    pub fn context(self) -> AuthContext {
        AuthContext {
            user_id: self.user(),
            org_id: OrgId::from("org_1"),
            email: format!("{}@test.local", self.user_id()),
            space_roles: self.roles(),
            scopes: vec![],
            client_id: "test".into(),
        }
    }
}

pub fn eng() -> SpaceId {
    SpaceId::from("sp_eng")
}

pub fn at(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(1_700_000_000 + secs, 0).unwrap()
}

pub struct Harness {
    pub state: IntegrationsApiState,
    pub org_storage: Arc<OrgStorageLayer>,
    _dir: tempfile::TempDir,
}

impl Harness {
    /// One org (`org_1`), spaces `sp_eng` and `sp_sales`, one user per [`Who`],
    /// and the catalog fixture served from the disk cache.
    pub async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        seed_org(&org_storage).await;
        let state =
            IntegrationsApiState::new(catalog_with_fixture(dir.path()).await, &org_storage, KEY_ID);
        Self {
            state,
            org_storage,
            _dir: dir,
        }
    }

    pub fn app(&self, who: Who) -> Router {
        Router::new()
            .nest(
                "/api",
                integrations_api_router().with_state(self.state.clone()),
            )
            .layer(Extension(who.context()))
    }

    /// Send a request as `who` and return the status and JSON body
    /// (`Null` when the body is empty).
    pub async fn call(
        &self,
        who: Who,
        method: &str,
        uri: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        let body = match body {
            Some(json) => {
                request = request.header("content-type", "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = self
            .app(who)
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    pub async fn get(&self, who: Who, uri: &str) -> (StatusCode, Value) {
        self.call(who, "GET", uri, None).await
    }

    pub async fn post(&self, who: Who, uri: &str, body: Value) -> (StatusCode, Value) {
        self.call(who, "POST", uri, Some(body)).await
    }

    pub async fn patch(&self, who: Who, uri: &str, body: Value) -> (StatusCode, Value) {
        self.call(who, "PATCH", uri, Some(body)).await
    }

    pub async fn put(&self, who: Who, uri: &str, body: Value) -> (StatusCode, Value) {
        self.call(who, "PUT", uri, Some(body)).await
    }

    pub async fn delete(&self, who: Who, uri: &str) -> (StatusCode, Value) {
        self.call(who, "DELETE", uri, None).await
    }

    /// Store an integration directly. Defaults: a shared, header-authenticated
    /// catalog install owned by the layer's natural owner.
    pub async fn seed(
        &self,
        id: &str,
        layer: IntegrationLayer,
        owner: Option<Who>,
        mutate: impl FnOnce(&mut Integration),
    ) -> Integration {
        let mut integration = Integration {
            id: id.into(),
            org_id: OrgId::from("org_1"),
            layer,
            space_id: (layer == IntegrationLayer::Space).then(eng),
            owner_user_id: (layer == IntegrationLayer::User)
                .then(|| owner.unwrap_or(Who::Member).user()),
            catalog_entry_id: Some("curated/linear-app-mcp".into()),
            connect_url: "https://mcp.linear.app/mcp".into(),
            slug: id.into(),
            name: id.into(),
            auth_kind: IntegrationAuthKind::Header,
            header_name: Some("Authorization".into()),
            header_value_prefix: Some("Bearer ".into()),
            credential_mode: if layer == IntegrationLayer::User {
                CredentialMode::PerUser
            } else {
                CredentialMode::Shared
            },
            trust: IntegrationTrust::Confirm,
            enabled: true,
            oauth_client_id_override: None,
            created_at: at(0),
            updated_at: at(0),
        };
        mutate(&mut integration);
        self.state.store.create(&integration).await.unwrap();
        integration
    }

    /// Store a credential row (ciphertext is a stand-in).
    pub async fn seed_credential(&self, id: &str, subject: Option<Who>, status: CredentialStatus) {
        self.seed_credential_with_key(id, subject, status, KEY_ID)
            .await;
    }

    pub async fn seed_credential_with_key(
        &self,
        id: &str,
        subject: Option<Who>,
        status: CredentialStatus,
        key_id: &str,
    ) {
        self.state
            .credentials
            .put(&StoredCredential {
                integration_id: id.into(),
                subject_user_id: subject.map(Who::user),
                kind: CredentialKind::Header,
                ciphertext: vec![1, 2, 3],
                nonce: vec![0; 12],
                key_id: key_id.into(),
                expires_at: None,
                status,
                updated_at: at(0),
            })
            .await
            .unwrap();
    }

    pub async fn has_credential(&self, id: &str, subject: Option<Who>) -> bool {
        self.state
            .credentials
            .get(id, subject.map(Who::user).as_ref())
            .await
            .unwrap()
            .is_some()
    }

    pub async fn set_policy(&self, change: impl FnOnce(&mut IntegrationPolicy)) {
        let mut policy = IntegrationPolicy::default_for(OrgId::from("org_1"));
        change(&mut policy);
        policy.updated_at = at(1);
        self.state.store.put_policy(&policy).await.unwrap();
    }
}

async fn seed_org(org_storage: &OrgStorageLayer) {
    for sql in [
        "INSERT INTO organizations (id, name, slug) VALUES ('org_1', 'Acme', 'acme')",
        "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_eng', 'org_1', 'Eng', 'eng')",
        "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_sales', 'org_1', 'Sales', 'sales')",
    ] {
        sqlx::query(sql).execute(org_storage.pool()).await.unwrap();
    }
    for who in Who::ALL {
        sqlx::query("INSERT INTO users (id, org_id, email) VALUES (?, 'org_1', ?)")
            .bind(who.user_id())
            .bind(format!("{}@test.local", who.user_id()))
            .execute(org_storage.pool())
            .await
            .unwrap();
    }
}

/// A catalog service that serves the fixture from its disk cache and never
/// reaches the network.
pub async fn catalog_with_fixture(dir: &std::path::Path) -> Arc<CatalogSync> {
    let cache = dir.join("cache").join("integrations");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(cache.join("catalog.json"), FIXTURE).unwrap();
    let sync = CatalogSync::new(
        CatalogConfig {
            enabled: true,
            source_url: "http://127.0.0.1:9/api.json".into(),
        },
        dir,
    )
    .unwrap();
    sync.load_cached().await;
    Arc::new(sync)
}
