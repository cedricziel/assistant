//! Per-request space resolution for data-plane routes.
//!
//! A data-plane handler takes a [`SpaceCtx`] extractor. It resolves the
//! caller's space from the [`AuthContext`] that `require_auth` injected and a
//! selector (`X-Space-Id` header or `space_id` query parameter), checks that
//! the caller holds a role there, and hands back that space's
//! [`StorageLayer`] from the shared pool cache.
//!
//! Unknown spaces, spaces of another org, and spaces the caller holds no role
//! in all produce the same `403`, so the response cannot be used to enumerate
//! spaces. See `openspec/specs/web-data-plane-tenancy/spec.md` and
//! `docs/adr/adr-0013-per-space-runtimes.md`.

use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use tracing::error;

use assistant_core::auth::AuthContext;
use assistant_core::identity::{Role, SpaceId};
use assistant_core::store::{OrgStore as _, SpaceStore as _};
use assistant_storage::{OrgPoolFactory, OrgStorageLayer, StorageLayer};

use crate::errors::json_error;

/// Header carrying the selected space id.
pub const SPACE_HEADER: &str = "x-space-id";
/// Query parameter equivalent of [`SPACE_HEADER`], for clients that cannot set
/// headers (the browser `EventSource`).
pub const SPACE_QUERY_PARAM: &str = "space_id";

/// Minimum access a route needs, ordered `Read < Write < Admin`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Access {
    /// Viewer and above: read-only routes.
    Read,
    /// Member and above: routes that create or change data.
    Write,
    /// Space admin and org admin: destructive and secret operations.
    Admin,
}

impl Access {
    fn granted_to(role: &Role) -> Self {
        match role {
            Role::Viewer => Self::Read,
            Role::Member => Self::Write,
            Role::SpaceAdmin | Role::OrgAdmin => Self::Admin,
        }
    }
}

/// Why a request could not be bound to a space.
#[derive(Debug, PartialEq, Eq)]
pub enum SpaceError {
    /// No selector was sent and the caller holds zero or several space roles.
    SelectorRequired,
    /// Unknown space, foreign org, or no role in the space.
    AccessDenied,
    /// The caller's role is below the route's [`Access`] level.
    InsufficientRole,
    /// The request carried no [`AuthContext`]; `require_auth` did not run.
    Unauthenticated,
    /// Storage lookup or pool open failed.
    Internal,
}

impl IntoResponse for SpaceError {
    fn into_response(self) -> Response {
        match self {
            Self::SelectorRequired => {
                json_error(StatusCode::BAD_REQUEST, "space selector required")
            }
            Self::AccessDenied => json_error(StatusCode::FORBIDDEN, "access denied"),
            Self::InsufficientRole => json_error(StatusCode::FORBIDDEN, "insufficient role"),
            Self::Unauthenticated => json_error(StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Internal => {
                json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        }
    }
}

/// Resolves `(AuthContext, selector)` to a [`SpaceCtx`].
#[derive(Clone)]
pub struct SpaceResolver {
    org_storage: Arc<OrgStorageLayer>,
    pools: Arc<OrgPoolFactory>,
}

impl SpaceResolver {
    pub fn new(org_storage: Arc<OrgStorageLayer>, pools: Arc<OrgPoolFactory>) -> Self {
        Self { org_storage, pools }
    }

    /// The pool cache this resolver opens spaces from.
    pub fn pools(&self) -> &Arc<OrgPoolFactory> {
        &self.pools
    }

    /// Bind `auth` to one space. `selector` is the raw space id, if the client
    /// sent one; an empty string counts as absent.
    pub async fn resolve(
        &self,
        auth: &AuthContext,
        selector: Option<&str>,
    ) -> Result<SpaceCtx, SpaceError> {
        let space_id = match selector.filter(|s| !s.is_empty()) {
            Some(id) => SpaceId::from(id),
            None => single_space(auth)?,
        };

        let space = self
            .org_storage
            .space_store()
            .get_space(&space_id)
            .await
            .map_err(internal)?
            .ok_or(SpaceError::AccessDenied)?;
        if space.org_id != auth.org_id {
            return Err(SpaceError::AccessDenied);
        }
        let role = match auth.role_in(&space_id) {
            Some(role) => role.clone(),
            None if auth.is_org_admin() => Role::OrgAdmin,
            None => return Err(SpaceError::AccessDenied),
        };

        let org = self
            .org_storage
            .org_store()
            .get_org(&space.org_id)
            .await
            .map_err(internal)?
            .ok_or(SpaceError::Internal)?;
        let storage = self
            .pools
            .space_storage(&org.slug, &space.slug)
            .await
            .map_err(internal)?;

        Ok(SpaceCtx {
            auth: auth.clone(),
            org_slug: org.slug,
            space_slug: space.slug,
            role,
            storage,
        })
    }
}

impl SpaceResolver {
    /// Storage for a space named by slugs, for callers that carry no
    /// [`AuthContext`] (the public workflow webhook ingress). The space must
    /// exist in `org.db`, so a request cannot make the server create a
    /// database for an arbitrary slug pair.
    pub async fn storage_for_slugs(
        &self,
        org_slug: &str,
        space_slug: &str,
    ) -> Result<Arc<StorageLayer>, SpaceError> {
        let org = self
            .org_storage
            .org_store()
            .get_org_by_slug(org_slug)
            .await
            .map_err(internal)?
            .ok_or(SpaceError::AccessDenied)?;
        let space = self
            .org_storage
            .space_store()
            .list_spaces(&org.id)
            .await
            .map_err(internal)?
            .into_iter()
            .find(|s| s.slug == space_slug)
            .ok_or(SpaceError::AccessDenied)?;
        self.pools
            .space_storage(&org.slug, &space.slug)
            .await
            .map_err(internal)
    }
}

fn single_space(auth: &AuthContext) -> Result<SpaceId, SpaceError> {
    let mut spaces = auth.space_roles.keys();
    match (spaces.next(), spaces.next()) {
        (Some(only), None) => Ok(only.clone()),
        _ => Err(SpaceError::SelectorRequired),
    }
}

fn internal(err: anyhow::Error) -> SpaceError {
    error!("space resolution failed: {err:#}");
    SpaceError::Internal
}

/// The space a data-plane request is bound to.
pub struct SpaceCtx {
    pub auth: AuthContext,
    pub org_slug: String,
    pub space_slug: String,
    /// The caller's effective role in this space.
    pub role: Role,
    pub storage: Arc<StorageLayer>,
}

impl SpaceCtx {
    /// Fail with [`SpaceError::InsufficientRole`] unless the caller's role
    /// grants at least `needed`.
    pub fn require(&self, needed: Access) -> Result<(), SpaceError> {
        if Access::granted_to(&self.role) >= needed {
            Ok(())
        } else {
            Err(SpaceError::InsufficientRole)
        }
    }
}

impl<S> FromRequestParts<S> for SpaceCtx
where
    S: Send + Sync,
    SpaceResolver: FromRef<S>,
{
    type Rejection = SpaceError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<AuthContext>()
            .ok_or(SpaceError::Unauthenticated)?;
        let selector = parts
            .headers
            .get(SPACE_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
            .or_else(|| query_param(parts.uri.query(), SPACE_QUERY_PARAM));
        SpaceResolver::from_ref(state)
            .resolve(auth, selector.as_deref())
            .await
    }
}

fn query_param(query: Option<&str>, name: &str) -> Option<String> {
    query?.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        (key == name).then(|| value.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use chrono::Utc;
    use tower::ServiceExt;

    use assistant_core::identity::{OrgId, UserId};
    use assistant_core::store::{Organization, Space};

    use super::*;

    const ORG_A: &str = "org_a";
    const ORG_B: &str = "org_b";

    async fn resolver() -> (SpaceResolver, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let org_storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        let now = Utc::now();
        for (id, slug) in [(ORG_A, "acme"), (ORG_B, "globex")] {
            org_storage
                .org_store()
                .create_org(&Organization {
                    id: OrgId::from(id),
                    name: slug.into(),
                    slug: slug.into(),
                    auth_mode: "password".into(),
                    created_at: now,
                    updated_at: now,
                })
                .await
                .unwrap();
        }
        for (id, org, slug) in [
            ("spc_a1", ORG_A, "eng"),
            ("spc_a2", ORG_A, "ops"),
            ("spc_b1", ORG_B, "eng"),
        ] {
            org_storage
                .space_store()
                .create_space(&Space {
                    id: SpaceId::from(id),
                    org_id: OrgId::from(org),
                    name: slug.into(),
                    slug: slug.into(),
                    created_at: now,
                    updated_at: now,
                })
                .await
                .unwrap();
        }
        let pools = Arc::new(OrgPoolFactory::new(dir.path()));
        (SpaceResolver::new(org_storage, pools), dir)
    }

    fn auth(org: &str, roles: &[(&str, Role)]) -> AuthContext {
        AuthContext {
            user_id: UserId::from("u"),
            org_id: OrgId::from(org),
            email: String::new(),
            space_roles: roles
                .iter()
                .map(|(s, r)| (SpaceId::from(*s), r.clone()))
                .collect::<HashMap<_, _>>(),
            scopes: vec![],
            client_id: "test".into(),
        }
    }

    #[tokio::test]
    async fn single_role_resolves_without_selector() {
        let (resolver, _dir) = resolver().await;
        let ctx = resolver
            .resolve(&auth(ORG_A, &[("spc_a1", Role::Member)]), None)
            .await
            .unwrap();
        assert_eq!(
            (ctx.org_slug.as_str(), ctx.space_slug.as_str()),
            ("acme", "eng")
        );
        assert_eq!(ctx.role, Role::Member);
    }

    #[tokio::test]
    async fn slugs_resolve_only_spaces_that_exist_in_the_org() {
        let (resolver, dir) = resolver().await;

        assert!(resolver.storage_for_slugs("acme", "eng").await.is_ok());
        for (org, space) in [
            ("acme", "missing"),
            ("missing", "eng"),
            ("globex", "ops"),
            ("..", "eng"),
        ] {
            assert_eq!(
                resolver.storage_for_slugs(org, space).await.err(),
                Some(SpaceError::AccessDenied),
                "{org}/{space}"
            );
        }
        assert!(
            !dir.path().join("orgs/acme/spaces/missing").exists(),
            "an unknown slug must not create a database directory"
        );
    }

    #[tokio::test]
    async fn empty_selector_counts_as_absent() {
        let (resolver, _dir) = resolver().await;
        let ctx = resolver
            .resolve(&auth(ORG_A, &[("spc_a1", Role::Viewer)]), Some(""))
            .await
            .unwrap();
        assert_eq!(ctx.space_slug, "eng");
    }

    #[tokio::test]
    async fn several_roles_without_selector_is_a_bad_request() {
        let (resolver, _dir) = resolver().await;
        let caller = auth(ORG_A, &[("spc_a1", Role::Member), ("spc_a2", Role::Member)]);
        assert_eq!(
            resolver.resolve(&caller, None).await.err(),
            Some(SpaceError::SelectorRequired)
        );
        let ctx = resolver.resolve(&caller, Some("spc_a2")).await.unwrap();
        assert_eq!(ctx.space_slug, "ops");
    }

    #[tokio::test]
    async fn no_roles_and_no_selector_is_a_bad_request() {
        let (resolver, _dir) = resolver().await;
        assert_eq!(
            resolver.resolve(&auth(ORG_A, &[]), None).await.err(),
            Some(SpaceError::SelectorRequired)
        );
    }

    #[tokio::test]
    async fn foreign_org_sibling_space_and_unknown_space_are_all_denied() {
        let (resolver, _dir) = resolver().await;
        let bob = auth(ORG_B, &[("spc_b1", Role::Member)]);
        let carol = auth(ORG_A, &[("spc_a2", Role::Member)]);

        for (caller, selector) in [
            (&bob, "spc_a1"),
            (&carol, "spc_a1"),
            (&carol, "spc_does_not_exist"),
        ] {
            assert_eq!(
                resolver.resolve(caller, Some(selector)).await.err(),
                Some(SpaceError::AccessDenied),
                "selector {selector}"
            );
        }
    }

    #[tokio::test]
    async fn stale_role_for_a_space_of_another_org_is_denied() {
        let (resolver, _dir) = resolver().await;
        let caller = auth(ORG_B, &[("spc_a1", Role::Member)]);
        assert_eq!(
            resolver.resolve(&caller, Some("spc_a1")).await.err(),
            Some(SpaceError::AccessDenied)
        );
    }

    #[tokio::test]
    async fn org_admin_reaches_every_space_of_their_own_org_only() {
        let (resolver, _dir) = resolver().await;
        let admin = auth(ORG_A, &[("spc_a1", Role::OrgAdmin)]);

        let ctx = resolver.resolve(&admin, Some("spc_a2")).await.unwrap();
        assert_eq!(ctx.role, Role::OrgAdmin);
        assert_eq!(
            resolver.resolve(&admin, Some("spc_b1")).await.err(),
            Some(SpaceError::AccessDenied)
        );
    }

    #[tokio::test]
    async fn same_slug_in_different_orgs_gets_different_storage() {
        let (resolver, _dir) = resolver().await;
        let a = resolver
            .resolve(&auth(ORG_A, &[("spc_a1", Role::Member)]), None)
            .await
            .unwrap();
        let b = resolver
            .resolve(&auth(ORG_B, &[("spc_b1", Role::Member)]), None)
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&a.storage, &b.storage));
    }

    #[tokio::test]
    async fn require_orders_viewer_member_admin() {
        let (resolver, _dir) = resolver().await;
        for (role, read, write, admin) in [
            (Role::Viewer, true, false, false),
            (Role::Member, true, true, false),
            (Role::SpaceAdmin, true, true, true),
            (Role::OrgAdmin, true, true, true),
        ] {
            let ctx = resolver
                .resolve(&auth(ORG_A, &[("spc_a1", role.clone())]), None)
                .await
                .unwrap();
            assert_eq!(ctx.require(Access::Read).is_ok(), read, "{role} read");
            assert_eq!(ctx.require(Access::Write).is_ok(), write, "{role} write");
            assert_eq!(ctx.require(Access::Admin).is_ok(), admin, "{role} admin");
        }
    }

    // -- Extractor over HTTP --

    async fn whoami(ctx: SpaceCtx) -> String {
        format!("{}/{}", ctx.org_slug, ctx.space_slug)
    }

    fn app(resolver: SpaceResolver, caller: Option<AuthContext>) -> Router {
        let router = Router::new()
            .route("/probe", get(whoami))
            .with_state(resolver);
        match caller {
            Some(caller) => router.layer(axum::Extension(caller)),
            None => router,
        }
    }

    async fn probe(app: Router, uri: &str, header: Option<&str>) -> (StatusCode, String) {
        let mut request = Request::builder().uri(uri);
        if let Some(value) = header {
            request = request.header(SPACE_HEADER, value);
        }
        let response = app
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    #[tokio::test]
    async fn extractor_reads_header_then_query_parameter() {
        let (resolver, _dir) = resolver().await;
        let caller = auth(ORG_A, &[("spc_a1", Role::Member), ("spc_a2", Role::Member)]);
        let app = app(resolver, Some(caller));

        let (status, body) = probe(app.clone(), "/probe", Some("spc_a2")).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "acme/ops"));

        let (status, body) = probe(app.clone(), "/probe?space_id=spc_a1", None).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, "acme/eng"));

        let (status, body) = probe(app, "/probe?space_id=spc_a1", Some("spc_a2")).await;
        assert_eq!(
            (status, body.as_str()),
            (StatusCode::OK, "acme/ops"),
            "the header wins over the query parameter"
        );
    }

    #[tokio::test]
    async fn extractor_maps_errors_to_the_json_envelope() {
        let (resolver, _dir) = resolver().await;
        let caller = auth(ORG_A, &[("spc_a1", Role::Member), ("spc_a2", Role::Member)]);
        let app = app(resolver.clone(), Some(caller));

        let (status, body) = probe(app.clone(), "/probe", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body, r#"{"error":"space selector required"}"#);

        let (status, body) = probe(app, "/probe", Some("spc_b1")).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body, r#"{"error":"access denied"}"#);

        let (status, _) = probe(app_without_auth(resolver), "/probe", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    fn app_without_auth(resolver: SpaceResolver) -> Router {
        app(resolver, None)
    }
}
