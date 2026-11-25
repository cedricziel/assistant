//! REST JSON API for integrations.
//!
//! These routes belong to the org-DB router family: they carry no space
//! context (`SpaceCtx`), because integrations exist at org and user level too.
//!
//! ## Routes
//!
//! | Method | Path                                | Description                       |
//! |--------|-------------------------------------|-----------------------------------|
//! | GET    | `/api/integrations/catalog`         | Search the integrations catalog   |
//! | GET    | `/api/integrations/catalog/{id}`    | Get one catalog entry             |

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use assistant_core::auth::AuthContext;
use assistant_core::integrations::catalog::{
    CatalogAuth, CatalogAuthKind, CatalogEntry, CatalogQuery,
};

use crate::catalog_sync::CatalogSync;
use crate::errors::json_error;
use crate::openapi::ErrorBody;

// -- State -------------------------------------------------------------------

#[derive(Clone)]
pub struct IntegrationsApiState {
    pub catalog: Arc<CatalogSync>,
}

// -- Request/response types --------------------------------------------------

/// Query parameters of `GET /api/integrations/catalog`.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCatalogEntriesQuery {
    /// Case-insensitive text matched against name, domain and description.
    pub q: Option<String>,
    /// Only entries in this category (case-insensitive).
    pub category: Option<String>,
    /// Page size. Default 50, maximum 200.
    pub limit: Option<usize>,
    /// `next_cursor` of the previous page.
    pub cursor: Option<String>,
}

/// How a catalog entry authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CatalogAuthKindResponse {
    None,
    Header,
    Oauth,
    Mixed,
    Unknown,
}

/// Authentication hints of a catalog entry.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogAuthResponse {
    pub kind: CatalogAuthKindResponse,
    /// Header to send, for `header` and `mixed` entries whose template parsed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    /// Name of the placeholder the user fills in, e.g. `api_key`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// Literal text before the secret in the header value, e.g. `Bearer `.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_prefix: Option<String>,
}

/// One installable remote MCP server from the integrations.sh catalog.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogEntrySummary {
    /// Upstream catalog id, e.g. `curated/linear-app-mcp`. Contains a slash.
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub categories: Vec<String>,
    /// HTTPS endpoint of the remote MCP server.
    pub connect_url: String,
    pub auth: CatalogAuthResponse,
}

/// One page of catalog search results.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CatalogEntryListResponse {
    pub items: Vec<CatalogEntrySummary>,
    /// Pass as `cursor` to fetch the next page; absent on the last page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl From<CatalogAuthKind> for CatalogAuthKindResponse {
    fn from(kind: CatalogAuthKind) -> Self {
        match kind {
            CatalogAuthKind::None => Self::None,
            CatalogAuthKind::Header => Self::Header,
            CatalogAuthKind::Oauth => Self::Oauth,
            CatalogAuthKind::Mixed => Self::Mixed,
            CatalogAuthKind::Unknown => Self::Unknown,
        }
    }
}

impl From<&CatalogAuth> for CatalogAuthResponse {
    fn from(auth: &CatalogAuth) -> Self {
        Self {
            kind: auth.kind.into(),
            header_name: auth.header_name.clone(),
            placeholder: auth.placeholder.clone(),
            value_prefix: auth.value_prefix.clone(),
        }
    }
}

impl From<&CatalogEntry> for CatalogEntrySummary {
    fn from(entry: &CatalogEntry) -> Self {
        Self {
            id: entry.id.clone(),
            name: entry.name.clone(),
            description: entry.description.clone(),
            domain: entry.domain.clone(),
            icon_url: entry.icon_url.clone(),
            categories: entry.categories.clone(),
            connect_url: entry.connect_url.clone(),
            auth: (&entry.auth).into(),
        }
    }
}

// -- Router ------------------------------------------------------------------

pub fn integrations_api_router() -> Router<IntegrationsApiState> {
    Router::new()
        .route("/integrations/catalog", get(list_catalog_entries))
        // Catalog ids contain a slash (`curated/linear-app-mcp`); a wildcard
        // accepts both that and the percent-encoded `curated%2Flinear-app-mcp`.
        .route("/integrations/catalog/{*id}", get(get_catalog_entry))
}

// -- Handlers ----------------------------------------------------------------

/// `GET /api/integrations/catalog` — search the integrations catalog.
#[utoipa::path(
    get,
    path = "/api/integrations/catalog",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(ListCatalogEntriesQuery),
    responses(
        (status = 200, description = "A page of catalog entries", body = CatalogEntryListResponse),
        (status = 400, description = "Invalid query or cursor", body = ErrorBody),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
    )
)]
pub async fn list_catalog_entries(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    query: Result<Query<ListCatalogEntriesQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(rejection) => return json_error(StatusCode::BAD_REQUEST, rejection.body_text()),
    };
    let index = state.catalog.index().await;
    let search = CatalogQuery {
        q: query.q.as_deref(),
        category: query.category.as_deref().filter(|c| !c.is_empty()),
        limit: query.limit,
        cursor: query.cursor.as_deref().filter(|c| !c.is_empty()),
    };
    match index.search(&search) {
        Ok(page) => Json(CatalogEntryListResponse {
            items: page.items.into_iter().map(Into::into).collect(),
            next_cursor: page.next_cursor,
        })
        .into_response(),
        Err(e) => json_error(StatusCode::BAD_REQUEST, e.to_string()),
    }
}

/// `GET /api/integrations/catalog/{id}` — get one catalog entry.
#[utoipa::path(
    get,
    path = "/api/integrations/catalog/{id}",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("id" = String, Path, description = "Catalog entry id, e.g. `curated/linear-app-mcp`. It contains one slash; send it as is or percent-encoded.")),
    responses(
        (status = 200, description = "The catalog entry", body = CatalogEntrySummary),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 404, description = "No such catalog entry", body = ErrorBody),
    )
)]
pub async fn get_catalog_entry(
    Extension(_ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path(id): Path<String>,
) -> Response {
    match state.catalog.index().await.get(&id) {
        Some(entry) => Json(CatalogEntrySummary::from(entry)).into_response(),
        None => json_error(StatusCode::NOT_FOUND, "catalog entry not found"),
    }
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    use axum::body::Body;
    use axum::http::Request;
    use serde_json::Value;
    use tower::ServiceExt;

    use assistant_auth::api_keys::InMemoryApiKeyStore;
    use assistant_auth::jwt::{JwtKeyPair, JwtManager};
    use assistant_auth::middleware::AuthState;
    use assistant_auth::oauth2::server::InMemoryRefreshTokenStore;
    use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
    use assistant_core::store::{InMemoryMembershipStore, InMemoryUserStore};
    use assistant_core::types::features::CatalogConfig;

    use crate::auth::{WebAuthConfig, require_auth};

    const FIXTURE: &str = include_str!("../../../core/tests/fixtures/integrations-catalog.json");
    const TOKEN: &str = "test-token";
    /// Entries of the fixture that are installable remote MCP servers.
    const FIXTURE_ENTRIES: usize = 13;

    fn member_context() -> AuthContext {
        AuthContext {
            user_id: UserId::from("alice"),
            org_id: OrgId::from("org_1"),
            email: "alice@test.local".into(),
            space_roles: HashMap::from([(SpaceId::from("sp_eng"), Role::Member)]),
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    /// The router behind the real auth middleware, mounted under `/api`.
    fn app(catalog: Arc<CatalogSync>) -> Router {
        let secret = b"test-secret-that-is-long-enough!!";
        let jwt = Arc::new(JwtManager::new(
            JwtKeyPair::from_secret(secret).unwrap(),
            "https://test.local".into(),
            "https://test.local".into(),
        ));
        let auth = WebAuthConfig {
            auth_state: AuthState {
                jwt_manager: jwt,
                api_key_store: Arc::new(InMemoryApiKeyStore::new()),
                users: Arc::new(InMemoryUserStore::default()),
                memberships: Arc::new(InMemoryMembershipStore::default()),
                refresh_tokens: Arc::new(InMemoryRefreshTokenStore::default()),
                legacy_token: Some(TOKEN.into()),
                legacy_context: Some(member_context()),
            },
            secure_cookie: false,
        };
        Router::new()
            .nest(
                "/api",
                integrations_api_router().with_state(IntegrationsApiState { catalog }),
            )
            .route_layer(axum::middleware::from_fn_with_state(auth, require_auth))
    }

    /// A catalog service that serves the fixture from its disk cache and
    /// never reaches the network.
    async fn catalog_with_fixture(dir: &std::path::Path) -> Arc<CatalogSync> {
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

    async fn get(app: &Router, uri: &str, authenticated: bool) -> (StatusCode, Value) {
        let mut request = Request::builder().uri(uri);
        if authenticated {
            request = request.header("authorization", format!("Bearer {TOKEN}"));
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
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

    fn item_ids(body: &Value) -> Vec<String> {
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap().to_owned())
            .collect()
    }

    #[tokio::test]
    async fn list_requires_authentication() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, _) = get(&app, "/api/integrations/catalog", false).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn get_requires_authentication() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, _) = get(
            &app,
            "/api/integrations/catalog/curated/linear-app-mcp",
            false,
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn list_returns_the_normalized_entries() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, body) = get(&app, "/api/integrations/catalog", true).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(item_ids(&body).len(), FIXTURE_ENTRIES);
        assert!(body.get("next_cursor").is_none(), "single page: {body}");
        let linear = &body["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == "curated/linear-app-mcp")
            .unwrap()
            .clone();
        assert_eq!(linear["name"], "Linear");
        assert_eq!(linear["domain"], "linear.app");
        assert_eq!(linear["connect_url"], "https://mcp.linear.app/mcp");
        assert_eq!(linear["auth"], serde_json::json!({"kind": "oauth"}));
        assert!(linear.get("installable_layers").is_none());
    }

    #[tokio::test]
    async fn list_exposes_header_templates() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (_, body) = get(&app, "/api/integrations/catalog?q=github", true).await;

        assert_eq!(
            body["items"][0]["auth"],
            serde_json::json!({
                "kind": "mixed",
                "header_name": "Authorization",
                "placeholder": "pat",
                "value_prefix": "Bearer "
            })
        );
    }

    #[tokio::test]
    async fn list_filters_by_text() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, body) = get(&app, "/api/integrations/catalog?q=LiNeAr", true).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(item_ids(&body), ["curated/linear-app-mcp"]);
    }

    #[tokio::test]
    async fn list_filters_by_category() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (_, body) = get(
            &app,
            "/api/integrations/catalog?category=observability",
            true,
        )
        .await;

        let mut ids = item_ids(&body);
        ids.sort();
        assert_eq!(ids, ["curated/axiom-co-mcp", "curated/sentry-io-mcp"]);
    }

    #[tokio::test]
    async fn list_pages_with_limit_and_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (_, first) = get(&app, "/api/integrations/catalog?limit=5", true).await;
        assert_eq!(item_ids(&first).len(), 5);
        let cursor = first["next_cursor"].as_str().unwrap().to_owned();
        let (_, second) = get(
            &app,
            &format!("/api/integrations/catalog?limit=5&cursor={cursor}"),
            true,
        )
        .await;
        assert_eq!(item_ids(&second).len(), 5);
        let cursor = second["next_cursor"].as_str().unwrap().to_owned();
        let (_, third) = get(
            &app,
            &format!("/api/integrations/catalog?limit=5&cursor={cursor}"),
            true,
        )
        .await;

        assert_eq!(item_ids(&third).len(), FIXTURE_ENTRIES - 10);
        assert!(third.get("next_cursor").is_none());
        let mut all = item_ids(&first);
        all.extend(item_ids(&second));
        all.extend(item_ids(&third));
        all.sort();
        all.dedup();
        assert_eq!(all.len(), FIXTURE_ENTRIES, "no entry repeats or is lost");
    }

    #[tokio::test]
    async fn list_rejects_a_bad_cursor_with_the_error_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, body) = get(&app, "/api/integrations/catalog?cursor=zzz", true).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body["error"], "invalid cursor");
    }

    #[tokio::test]
    async fn list_rejects_a_non_numeric_limit_with_the_error_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, body) = get(&app, "/api/integrations/catalog?limit=many", true).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            body["error"].as_str().is_some_and(|e| !e.is_empty()),
            "{body}"
        );
    }

    #[tokio::test]
    async fn list_is_empty_when_the_catalog_is_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let sync = CatalogSync::new(
            CatalogConfig {
                enabled: false,
                ..CatalogConfig::default()
            },
            dir.path(),
        )
        .unwrap();
        let app = app(Arc::new(sync));

        let (status, body) = get(&app, "/api/integrations/catalog", true).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, serde_json::json!({"items": []}));
    }

    #[tokio::test]
    async fn get_returns_one_entry_by_its_slashed_id() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, body) = get(
            &app,
            "/api/integrations/catalog/curated/linear-app-mcp",
            true,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], "curated/linear-app-mcp");
        assert_eq!(body["name"], "Linear");
    }

    #[tokio::test]
    async fn get_accepts_a_percent_encoded_id() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, body) = get(
            &app,
            "/api/integrations/catalog/curated%2Flinear-app-mcp",
            true,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], "curated/linear-app-mcp");
    }

    #[tokio::test]
    async fn get_unknown_entry_is_404_with_the_error_envelope() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, body) = get(&app, "/api/integrations/catalog/curated/nope", true).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["error"], "catalog entry not found");
    }

    #[tokio::test]
    async fn get_hides_entries_that_are_not_installable() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, _) = get(&app, "/api/integrations/catalog/curated/exa", true).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_treats_empty_cursor_and_category_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let (status, empty) =
            get(&app, "/api/integrations/catalog?cursor=&category=&q=", true).await;
        let (_, plain) = get(&app, "/api/integrations/catalog", true).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(empty, plain);
    }

    #[tokio::test]
    async fn unauthenticated_response_is_the_plain_text_the_middleware_sends() {
        let dir = tempfile::tempdir().unwrap();
        let app = app(catalog_with_fixture(dir.path()).await);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/integrations/catalog")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Unauthorized");
    }

    #[test]
    fn documented_401_matches_the_plain_text_the_middleware_sends() {
        use utoipa::OpenApi;

        let spec = serde_json::to_value(crate::openapi::ApiDoc::openapi()).unwrap();

        for path in [
            "/api/integrations/catalog",
            "/api/integrations/catalog/{id}",
        ] {
            let content = &spec["paths"][path]["get"]["responses"]["401"]["content"];
            assert!(content.get("text/plain").is_some(), "{path}: {content}");
            assert!(
                content.get("application/json").is_none(),
                "{path}: {content}"
            );
        }
    }
}
