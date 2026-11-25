//! `/api/integrations[/{id}]`: install, list, read, update and remove.

use std::collections::HashMap;

use axum::{
    Extension, Json,
    extract::{
        Path, Query, State,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderName, HeaderValue, StatusCode},
};

use assistant_core::auth::AuthContext;
use assistant_core::identity::SpaceId;
use assistant_core::integrations::catalog::{CatalogAuthKind, CatalogEntry, is_plain_url};
use assistant_core::integrations::{
    self as domain, Integration, IntegrationFilter, IntegrationPolicy, MAX_SLUG_LEN, SpaceAccess,
};

use super::IntegrationsApiState;
use super::authz::{self, Access};
use super::error::{ApiError, ApiResult};
use super::types::{
    CreateIntegrationRequest, IntegrationDetail, IntegrationListResponse, IntegrationStatus,
    IntegrationSummary, ListIntegrationsQuery, SpaceAccessEntry, UpdateIntegrationRequest,
};
use crate::openapi::ErrorBody;

const MAX_NAME_LEN: usize = 100;

// -- Handlers ----------------------------------------------------------------

/// `GET /api/integrations` — list the integrations the caller may see.
///
/// Members see the org-layer integrations offered to their spaces, the
/// integrations of spaces they belong to, and their own user-layer ones. Org
/// admins see everything, including other users' integrations (without
/// credential material).
#[utoipa::path(
    get,
    path = "/api/integrations",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(ListIntegrationsQuery),
    responses(
        (status = 200, description = "The visible integrations", body = IntegrationListResponse),
        (status = 400, description = "Invalid query", body = ErrorBody),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
    )
)]
pub async fn list_integrations(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    query: Result<Query<ListIntegrationsQuery>, QueryRejection>,
) -> ApiResult<Json<IntegrationListResponse>> {
    let Query(query) = query?;
    let layer: Option<domain::IntegrationLayer> = query.layer.map(Into::into);
    let space = query.space_id.map(SpaceId::from);
    let wants = |wanted| layer.is_none_or(|layer| layer == wanted);
    let by_layer = |layer, space_id| IntegrationFilter {
        layer: Some(layer),
        space_id,
        owner_user_id: None,
    };
    let (integrations, rows, policy) = futures::try_join!(
        async {
            let org = &ctx.org_id;
            let Some(space) = &space else {
                let all = IntegrationFilter {
                    layer,
                    ..IntegrationFilter::default()
                };
                return state.store.list(org, &all).await;
            };
            // A space has its own integrations and the org-layer ones offered
            // to it; the latter are narrowed in the loop below.
            let mut found = Vec::new();
            if wants(domain::IntegrationLayer::Space) {
                let filter = by_layer(domain::IntegrationLayer::Space, Some(space.clone()));
                found.extend(state.store.list(org, &filter).await?);
            }
            if wants(domain::IntegrationLayer::Org) {
                let filter = by_layer(domain::IntegrationLayer::Org, None);
                found.extend(state.store.list(org, &filter).await?);
            }
            Ok(found)
        },
        async {
            // Space rows only matter for org-layer integrations.
            if wants(domain::IntegrationLayer::Org) {
                state.store.list_org_space_access(&ctx.org_id).await
            } else {
                Ok(Vec::new())
            }
        },
        state.store.get_policy(&ctx.org_id),
    )?;
    let mut rows_by_integration: HashMap<String, Vec<SpaceAccess>> = HashMap::new();
    for row in rows {
        rows_by_integration
            .entry(row.integration_id.clone())
            .or_default()
            .push(row);
    }
    let mut items = Vec::new();
    for integration in integrations {
        let own_rows = rows_by_integration
            .get(&integration.id)
            .map_or(&[][..], Vec::as_slice);
        if !authz::access(&ctx, &integration, own_rows).can_view() {
            continue;
        }
        // Which spaces allow or opt out of an integration is configuration of
        // those spaces: only their members (and org admins) may read it.
        let offered = space.as_ref().is_none_or(|space| {
            integration.layer != domain::IntegrationLayer::Org
                || ((ctx.is_org_admin() || ctx.role_in(space).is_some())
                    && authz::active_in(own_rows, space))
        });
        if !offered {
            continue;
        }
        let view = caller_view(&state, &ctx, &policy, &integration).await?;
        items.push(summary(&integration, view));
    }
    Ok(Json(IntegrationListResponse { items }))
}

/// `POST /api/integrations` — install an integration.
///
/// Org admins install at the org layer; space admins (or org admins) at the
/// space layer; anyone at the user layer. The org policy limits the space and
/// user layers (`403`). A catalog install copies its URL and auth template
/// from the catalog index.
#[utoipa::path(
    post,
    path = "/api/integrations",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    request_body = CreateIntegrationRequest,
    responses(
        (status = 201, description = "Installed", body = IntegrationDetail),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may not install at this layer, or the org policy forbids it", body = ErrorBody),
        (status = 404, description = "No such space", body = ErrorBody),
        (status = 409, description = "The slug is already taken in this layer and owner scope", body = ErrorBody),
        (status = 422, description = "The request is invalid, e.g. a bad slug or missing auth details", body = ErrorBody),
    )
)]
pub async fn create_integration(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    body: Result<Json<CreateIntegrationRequest>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<IntegrationDetail>)> {
    let Json(request) = body?;
    let detail = create(&state, &ctx, request).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

async fn create(
    state: &IntegrationsApiState,
    ctx: &AuthContext,
    request: CreateIntegrationRequest,
) -> ApiResult<IntegrationDetail> {
    let layer: domain::IntegrationLayer = request.layer.into();
    let space_id = layer_target(ctx, layer, request.space_id.as_deref())?;
    if let Some(space) = &space_id {
        ensure_space_in_org(state, ctx, space).await?;
    }

    let origin = match (&request.catalog_entry_id, &request.connect_url) {
        (Some(id), None) => Origin::Catalog(id),
        (None, Some(url)) => Origin::Custom(url),
        (Some(_), Some(_)) => {
            return Err(ApiError::unprocessable(
                "give either catalog_entry_id or connect_url, not both",
            ));
        }
        (None, None) => {
            return Err(ApiError::unprocessable(
                "catalog_entry_id or connect_url is required",
            ));
        }
    };
    let catalog_entry = match origin {
        Origin::Catalog(id) => Some(id),
        Origin::Custom(_) => None,
    };

    let policy = state.store.get_policy(&ctx.org_id).await?;
    if let Some(reason) = policy.install_denial(layer, catalog_entry) {
        return Err(ApiError::forbidden(reason));
    }

    let source = match origin {
        Origin::Catalog(id) => {
            let index = state.catalog.index().await;
            let entry = index
                .get(id)
                .ok_or_else(|| ApiError::unprocessable("unknown catalog entry"))?;
            Source::from_catalog(entry, &request)?
        }
        Origin::Custom(url) => Source::custom(url, &request)?,
    };

    let slug = request
        .slug
        .or(source.default_slug)
        .ok_or_else(|| ApiError::unprocessable("slug is required for a custom URL"))?;
    validate_slug(&slug)?;
    let name = request
        .name
        .or(source.default_name)
        .ok_or_else(|| ApiError::unprocessable("name is required for a custom URL"))?;
    let name = clean_name(name)?;

    let credential_mode: domain::CredentialMode = match (layer, request.credential_mode) {
        (domain::IntegrationLayer::User, Some(mode))
            if mode != super::types::CredentialMode::PerUser =>
        {
            return Err(ApiError::unprocessable(
                "personal integrations always use the owner's own credential",
            ));
        }
        (domain::IntegrationLayer::User, _) => domain::CredentialMode::PerUser,
        (_, Some(mode)) => mode.into(),
        (_, None)
            if matches!(
                source.auth_kind,
                domain::IntegrationAuthKind::Oauth | domain::IntegrationAuthKind::Mixed
            ) =>
        {
            domain::CredentialMode::PerUser
        }
        (_, None) => domain::CredentialMode::Shared,
    };

    let now = state.clock.now();
    let integration = Integration {
        id: format!("int_{}", uuid::Uuid::new_v4()),
        org_id: ctx.org_id.clone(),
        layer,
        space_id,
        owner_user_id: (layer == domain::IntegrationLayer::User).then(|| ctx.user_id.clone()),
        catalog_entry_id: catalog_entry.map(str::to_owned),
        connect_url: source.connect_url,
        slug,
        name,
        auth_kind: source.auth_kind,
        header_name: source.header_name,
        header_value_prefix: source.header_value_prefix,
        credential_mode,
        trust: request
            .trust
            .map(Into::into)
            .unwrap_or(domain::IntegrationTrust::Confirm),
        enabled: request.enabled.unwrap_or(true),
        oauth_client_id_override: None,
        created_at: now,
        updated_at: now,
    };
    state.store.create(&integration).await?;
    // Nothing fallible may follow the write: the caller must learn the new id.
    Ok(detail(&integration, fresh_view(&integration), Vec::new()))
}

/// `GET /api/integrations/{id}` — get one integration.
///
/// `404` when the caller may not see it, including another user's personal
/// integration. Org admins are the exception: they can read a personal
/// integration's metadata (never its credential) so they can remove it.
#[utoipa::path(
    get,
    path = "/api/integrations/{id}",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("id" = String, Path, description = "Integration id, `int_<uuid>`")),
    responses(
        (status = 200, description = "The integration", body = IntegrationDetail),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 404, description = "No such integration, or the caller may not see it (another user's personal integration, except to org admins)", body = ErrorBody),
    )
)]
pub async fn get_integration(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path(id): Path<String>,
) -> ApiResult<Json<IntegrationDetail>> {
    let visible = load_visible(&state, &ctx, &id).await?;
    let policy = state.store.get_policy(&ctx.org_id).await?;
    let detail = detail_for(
        &state,
        &ctx,
        &policy,
        &visible.integration,
        visible.space_access(&ctx),
    )
    .await?;
    Ok(Json(detail))
}

/// `PATCH /api/integrations/{id}` — update an integration.
///
/// Layer, owner, catalog entry, auth kind and credential mode are fixed at
/// install time. A catalog install cannot change its URL or header template;
/// moving a custom URL deletes its stored credentials. A change that races
/// with another one on the same integration fails with `409` instead of
/// overwriting it.
#[utoipa::path(
    patch,
    path = "/api/integrations/{id}",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("id" = String, Path, description = "Integration id, `int_<uuid>`")),
    request_body = UpdateIntegrationRequest,
    responses(
        (status = 200, description = "The updated integration", body = IntegrationDetail),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may see but not change it", body = ErrorBody),
        (status = 404, description = "No such integration, or the caller may not see it", body = ErrorBody),
        (status = 409, description = "The slug is already taken, or another request changed the integration since it was read (reload and retry)", body = ErrorBody),
        (status = 422, description = "Invalid slug or a field that cannot change", body = ErrorBody),
    )
)]
pub async fn update_integration(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path(id): Path<String>,
    body: Result<Json<UpdateIntegrationRequest>, JsonRejection>,
) -> ApiResult<Json<IntegrationDetail>> {
    let Json(request) = body?;
    let visible = load_visible(&state, &ctx, &id).await?;
    if !visible.access.can_manage() {
        return Err(ApiError::forbidden("you may not change this integration"));
    }
    let mut integration = visible.integration.clone();
    apply_update(&mut integration, request)?;
    integration.updated_at = state.clock.now();

    // Everything fallible comes before the write, so a failure never follows a
    // saved change. Moving the URL deletes the credentials, so the view of the
    // moved integration is that of a fresh one.
    let view = if integration.connect_url != visible.integration.connect_url {
        fresh_view(&integration)
    } else {
        let policy = state.store.get_policy(&ctx.org_id).await?;
        caller_view(&state, &ctx, &policy, &integration).await?
    };

    // The row may only change if nobody else changed it since it was read.
    if !state
        .store
        .update_if_unchanged(&integration, visible.integration.updated_at)
        .await?
    {
        return Err(ApiError::not_found("integration not found"));
    }
    Ok(Json(detail(&integration, view, visible.space_access(&ctx))))
}

/// `DELETE /api/integrations/{id}` — remove an integration and its credentials.
///
/// Org admins may remove any user's personal integration.
#[utoipa::path(
    delete,
    path = "/api/integrations/{id}",
    tag = "integrations",
    security(("bearer_token" = []), ("oauth2" = [])),
    params(("id" = String, Path, description = "Integration id, `int_<uuid>`")),
    responses(
        (status = 204, description = "Removed"),
        (status = 401, description = "Unauthorized; the body is the plain text `Unauthorized`", content_type = "text/plain", body = String),
        (status = 403, description = "The caller may see but not remove it", body = ErrorBody),
        (status = 404, description = "No such integration, or the caller may not see it", body = ErrorBody),
    )
)]
pub async fn delete_integration(
    Extension(ctx): Extension<AuthContext>,
    State(state): State<IntegrationsApiState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let visible = load_visible(&state, &ctx, &id).await?;
    if !visible.access.can_delete() {
        return Err(ApiError::forbidden("you may not remove this integration"));
    }
    state.store.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// -- Shared with the other integration routes ----------------------------------

/// An integration the caller may see, with their access to it.
pub(super) struct Visible {
    pub integration: Integration,
    pub rows: Vec<SpaceAccess>,
    pub access: Access,
}

impl Visible {
    /// The space rows the caller may see: all for an org admin, otherwise
    /// those of spaces they belong to.
    fn space_access(&self, ctx: &AuthContext) -> Vec<SpaceAccess> {
        self.rows
            .iter()
            .filter(|row| ctx.is_org_admin() || ctx.role_in(&row.space_id).is_some())
            .cloned()
            .collect()
    }
}

/// Load `id`, or fail with `404` when it does not exist or the caller may not
/// see it.
pub(super) async fn load_visible(
    state: &IntegrationsApiState,
    ctx: &AuthContext,
    id: &str,
) -> ApiResult<Visible> {
    let not_found = || ApiError::not_found("integration not found");
    let integration = state.store.get(id).await?.ok_or_else(not_found)?;
    let rows = state.store.list_space_access(id).await?;
    let access = authz::access(ctx, &integration, &rows);
    if !access.can_view() {
        return Err(not_found());
    }
    Ok(Visible {
        integration,
        rows,
        access,
    })
}

/// Fail with `404` unless `space` exists in the caller's org.
pub(super) async fn ensure_space_in_org(
    state: &IntegrationsApiState,
    ctx: &AuthContext,
    space: &SpaceId,
) -> ApiResult<()> {
    let found = state.spaces.get_space(space).await?;
    if found.is_none_or(|found| found.org_id != ctx.org_id) {
        return Err(ApiError::not_found("space not found"));
    }
    Ok(())
}

// -- Create: validation ---------------------------------------------------------

/// Where an install comes from.
enum Origin<'a> {
    Catalog(&'a str),
    Custom(&'a str),
}

/// The URL and auth template of a new integration, from the catalog or the request.
struct Source {
    connect_url: String,
    auth_kind: domain::IntegrationAuthKind,
    header_name: Option<String>,
    header_value_prefix: Option<String>,
    default_name: Option<String>,
    default_slug: Option<String>,
}

impl Source {
    fn from_catalog(entry: &CatalogEntry, request: &CreateIntegrationRequest) -> ApiResult<Self> {
        use domain::IntegrationAuthKind as Kind;
        let asked: Option<Kind> = request.auth_kind.map(Into::into);
        let auth_kind = match (entry.auth.kind, asked) {
            (CatalogAuthKind::Unknown, Some(kind @ (Kind::None | Kind::Header | Kind::Oauth))) => {
                kind
            }
            (CatalogAuthKind::Unknown, _) => {
                return Err(ApiError::unprocessable(
                    "this entry does not say how it authenticates: auth_kind must be none, header or oauth",
                ));
            }
            (CatalogAuthKind::Mixed, Some(kind @ (Kind::Header | Kind::Oauth))) => kind,
            (CatalogAuthKind::Mixed, None | Some(Kind::Mixed)) => Kind::Mixed,
            (CatalogAuthKind::None, None | Some(Kind::None)) => Kind::None,
            (CatalogAuthKind::Header, None | Some(Kind::Header)) => Kind::Header,
            (CatalogAuthKind::Oauth, None | Some(Kind::Oauth)) => Kind::Oauth,
            (_, Some(_)) => {
                return Err(ApiError::unprocessable(
                    "auth_kind does not match the catalog entry",
                ));
            }
        };
        let (header_name, header_value_prefix) = header_template(
            auth_kind,
            (
                entry.auth.header_name.clone(),
                entry.auth.value_prefix.clone(),
            ),
            request,
        )?;
        Ok(Self {
            connect_url: entry.connect_url.clone(),
            auth_kind,
            header_name,
            header_value_prefix,
            default_name: Some(entry.name.clone()),
            default_slug: Some(slug_from_catalog_id(&entry.id)),
        })
    }

    fn custom(url: &str, request: &CreateIntegrationRequest) -> ApiResult<Self> {
        use domain::IntegrationAuthKind as Kind;
        validate_connect_url(url)?;
        let auth_kind = match request.auth_kind.map(Into::into) {
            Some(kind @ (Kind::None | Kind::Header | Kind::Oauth)) => kind,
            _ => {
                return Err(ApiError::unprocessable(
                    "auth_kind must be none, header or oauth for a custom URL",
                ));
            }
        };
        let (header_name, header_value_prefix) = header_template(auth_kind, (None, None), request)?;
        Ok(Self {
            connect_url: url.to_owned(),
            auth_kind,
            header_name,
            header_value_prefix,
            default_name: None,
            default_slug: None,
        })
    }
}

/// The header that carries the credential: the catalog's template when it has
/// one, else the request's. Only header-style auth has one.
fn header_template(
    auth_kind: domain::IntegrationAuthKind,
    from_catalog: (Option<String>, Option<String>),
    request: &CreateIntegrationRequest,
) -> ApiResult<(Option<String>, Option<String>)> {
    use domain::IntegrationAuthKind as Kind;
    let given = request.header_name.is_some() || request.header_value_prefix.is_some();
    if !matches!(auth_kind, Kind::Header | Kind::Mixed) {
        return if given {
            Err(ApiError::unprocessable(
                "header_name and header_value_prefix only apply to header auth",
            ))
        } else {
            Ok((None, None))
        };
    }
    if from_catalog.0.is_some() {
        return if given {
            Err(ApiError::unprocessable(
                "the header template comes from the catalog entry and cannot be changed",
            ))
        } else {
            Ok(from_catalog)
        };
    }
    let Some(name) = request.header_name.clone() else {
        return Err(ApiError::unprocessable(
            "header_name is required for header auth",
        ));
    };
    validate_header(&name, request.header_value_prefix.as_deref())?;
    Ok((Some(name), request.header_value_prefix.clone()))
}

pub(super) fn validate_header(name: &str, prefix: Option<&str>) -> ApiResult<()> {
    if HeaderName::from_bytes(name.as_bytes()).is_err() {
        return Err(ApiError::unprocessable(
            "header_name is not a valid HTTP header name",
        ));
    }
    if prefix.is_some_and(|prefix| HeaderValue::from_str(prefix).is_err()) {
        return Err(ApiError::unprocessable(
            "header_value_prefix is not a valid HTTP header value",
        ));
    }
    Ok(())
}

fn validate_connect_url(url: &str) -> ApiResult<()> {
    match url::Url::parse(url) {
        Ok(parsed) if matches!(parsed.scheme(), "http" | "https") && is_plain_url(&parsed) => {
            Ok(())
        }
        _ => Err(ApiError::unprocessable(
            "connect_url must be an http(s) URL without credentials or a fragment",
        )),
    }
}

/// The layer's target space, after checking the request names one exactly when
/// the layer needs it and the caller may install there.
fn layer_target(
    ctx: &AuthContext,
    layer: domain::IntegrationLayer,
    space_id: Option<&str>,
) -> ApiResult<Option<SpaceId>> {
    let space = match (layer, space_id) {
        (domain::IntegrationLayer::Space, Some(space)) => Some(SpaceId::from(space)),
        (domain::IntegrationLayer::Space, None) => {
            return Err(ApiError::unprocessable(
                "space_id is required for the space layer",
            ));
        }
        (_, Some(_)) => {
            return Err(ApiError::unprocessable(
                "space_id only applies to the space layer",
            ));
        }
        (_, None) => None,
    };
    if authz::can_install(ctx, layer, space.as_ref()) {
        Ok(space)
    } else {
        Err(ApiError::forbidden(match layer {
            domain::IntegrationLayer::Org => "org admin required",
            _ => "space admin or org admin required",
        }))
    }
}

/// `curated/linear-app-mcp` → `linear-app-mcp`, shortened to fit.
fn slug_from_catalog_id(id: &str) -> String {
    let tail = id.rsplit('/').next().unwrap_or(id).to_lowercase();
    let mut slug = String::new();
    for c in tail.chars() {
        let c = if c.is_ascii_alphanumeric() { c } else { '-' };
        if c != '-' || !(slug.is_empty() || slug.ends_with('-')) {
            slug.push(c);
        }
    }
    slug.truncate(MAX_SLUG_LEN);
    slug.trim_end_matches('-').to_owned()
}

fn validate_slug(slug: &str) -> ApiResult<()> {
    let kebab = slug.split('-').all(|part| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    });
    if (1..=MAX_SLUG_LEN).contains(&slug.len()) && kebab {
        Ok(())
    } else {
        Err(ApiError::unprocessable(format!(
            "slug must be kebab-case (a-z, 0-9, single hyphens), 1-{MAX_SLUG_LEN} characters"
        )))
    }
}

/// The trimmed name, if it is 1-`MAX_NAME_LEN` characters.
fn clean_name(name: String) -> ApiResult<String> {
    let name = name.trim();
    if (1..=MAX_NAME_LEN).contains(&name.chars().count()) {
        Ok(name.to_owned())
    } else {
        Err(ApiError::unprocessable(format!(
            "name must be 1-{MAX_NAME_LEN} characters"
        )))
    }
}

// -- Update ---------------------------------------------------------------------

fn apply_update(integration: &mut Integration, request: UpdateIntegrationRequest) -> ApiResult<()> {
    if let Some(slug) = request.slug {
        validate_slug(&slug)?;
        integration.slug = slug;
    }
    if let Some(name) = request.name {
        integration.name = clean_name(name)?;
    }
    if let Some(enabled) = request.enabled {
        integration.enabled = enabled;
    }
    if let Some(trust) = request.trust {
        integration.trust = trust.into();
    }
    if let Some(url) = request.connect_url {
        // A catalog install's URL is checked by the store, which refuses a change.
        validate_connect_url(&url)?;
        integration.connect_url = url;
    }
    if request.header_name.is_some() || request.header_value_prefix.is_some() {
        let editable = integration.catalog_entry_id.is_none()
            && matches!(
                integration.auth_kind,
                domain::IntegrationAuthKind::Header | domain::IntegrationAuthKind::Mixed
            );
        if !editable {
            return Err(ApiError::unprocessable(
                "the header template can only change on a custom URL integration with header auth",
            ));
        }
        let name = request
            .header_name
            .or_else(|| integration.header_name.clone());
        let prefix = request
            .header_value_prefix
            .or_else(|| integration.header_value_prefix.clone());
        if let Some(name) = &name {
            validate_header(name, prefix.as_deref())?;
        }
        integration.header_name = name;
        integration.header_value_prefix = prefix;
    }
    Ok(())
}

// -- Representation ---------------------------------------------------------------

/// What an integration looks like to the caller.
pub(super) struct CallerView {
    pub status: IntegrationStatus,
    /// Whether the caller has a credential stored: the shared one, or their
    /// own for a per-user integration.
    pub has_credential: bool,
    pub last_connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tool_count: Option<u32>,
}

/// The view of `integration` for the caller.
///
/// The status of a user-layer integration is its owner's, so an org admin
/// looking at someone else's sees that state. `has_credential` is always the
/// caller's own (the shared credential, or their own per-user one), so it
/// reveals nothing about another user. Never the credential itself. A usable
/// integration whose latest connection attempt failed is `unreachable`.
pub(super) async fn caller_view(
    state: &IntegrationsApiState,
    ctx: &AuthContext,
    policy: &IntegrationPolicy,
    integration: &Integration,
) -> anyhow::Result<CallerView> {
    let user = match integration.layer {
        domain::IntegrationLayer::User => integration.owner_user_id.clone(),
        _ => Some(ctx.user_id.clone()),
    };
    let (availability, credential_subject) = state
        .effective
        .assess(policy, integration, user.as_ref())
        .await?;
    let subject = match integration.credential_mode {
        domain::CredentialMode::Shared => None,
        domain::CredentialMode::PerUser => Some(ctx.user_id.clone()),
    };
    let has_credential = integration.auth_kind != domain::IntegrationAuthKind::None
        && state
            .credentials
            .get(&integration.id, subject.as_ref())
            .await?
            .is_some();
    // Health is recorded under the subject `assess` names (the owner's, for a
    // user-layer integration; none for one that needs no credential).
    let health = state
        .health
        .get(&integration.id, credential_subject.as_ref())
        .await?;
    let mut status: IntegrationStatus = availability.into();
    if status == IntegrationStatus::Ready && health.as_ref().is_some_and(|h| !h.reachable) {
        status = IntegrationStatus::Unreachable;
    }
    Ok(CallerView {
        status,
        has_credential,
        last_connected_at: health.as_ref().and_then(|h| h.last_connected_at),
        tool_count: health.and_then(|h| h.tool_count),
    })
}

/// The view of an integration that has no credential yet (just installed, or
/// its URL moved), without any lookup. Org policy is not consulted: installs
/// are checked against it first, and the org layer is exempt.
fn fresh_view(integration: &Integration) -> CallerView {
    let status = if !integration.enabled {
        IntegrationStatus::Disabled
    } else if integration.auth_kind == domain::IntegrationAuthKind::None {
        IntegrationStatus::Ready
    } else {
        IntegrationStatus::NeedsConnection
    };
    CallerView {
        status,
        has_credential: false,
        last_connected_at: None,
        tool_count: None,
    }
}

async fn detail_for(
    state: &IntegrationsApiState,
    ctx: &AuthContext,
    policy: &IntegrationPolicy,
    integration: &Integration,
    space_access: Vec<SpaceAccess>,
) -> ApiResult<IntegrationDetail> {
    let view = caller_view(state, ctx, policy, integration).await?;
    Ok(detail(integration, view, space_access))
}

fn summary(i: &Integration, view: CallerView) -> IntegrationSummary {
    IntegrationSummary {
        id: i.id.clone(),
        layer: i.layer.into(),
        space_id: i.space_id.as_ref().map(|s| s.0.clone()),
        owner_user_id: i.owner_user_id.as_ref().map(|u| u.0.clone()),
        catalog_entry_id: i.catalog_entry_id.clone(),
        slug: i.slug.clone(),
        name: i.name.clone(),
        connect_url: i.connect_url.clone(),
        auth_kind: i.auth_kind.into(),
        credential_mode: i.credential_mode.into(),
        trust: i.trust.into(),
        enabled: i.enabled,
        status: view.status,
        has_credential: view.has_credential,
        last_connected_at: view.last_connected_at,
        tool_count: view.tool_count,
        created_at: i.created_at,
        updated_at: i.updated_at,
    }
}

fn detail(i: &Integration, view: CallerView, space_access: Vec<SpaceAccess>) -> IntegrationDetail {
    IntegrationDetail {
        id: i.id.clone(),
        layer: i.layer.into(),
        space_id: i.space_id.as_ref().map(|s| s.0.clone()),
        owner_user_id: i.owner_user_id.as_ref().map(|u| u.0.clone()),
        catalog_entry_id: i.catalog_entry_id.clone(),
        slug: i.slug.clone(),
        name: i.name.clone(),
        connect_url: i.connect_url.clone(),
        auth_kind: i.auth_kind.into(),
        header_name: i.header_name.clone(),
        header_value_prefix: i.header_value_prefix.clone(),
        credential_mode: i.credential_mode.into(),
        trust: i.trust.into(),
        enabled: i.enabled,
        status: view.status,
        has_credential: view.has_credential,
        last_connected_at: view.last_connected_at,
        tool_count: view.tool_count,
        space_access: space_access
            .into_iter()
            .map(|row| SpaceAccessEntry {
                space_id: row.space_id.0,
                mode: row.mode.into(),
            })
            .collect(),
        created_at: i.created_at,
        updated_at: i.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use serde_json::{Value, json};

    use assistant_core::integrations::{CredentialStatus, IntegrationLayer, SpaceAccessMode};

    use super::super::testing::{Harness, Who, eng, now};

    const OK: StatusCode = StatusCode::OK;
    const CREATED: StatusCode = StatusCode::CREATED;
    const FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;
    const NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;
    const NO_CONTENT: StatusCode = StatusCode::NO_CONTENT;
    const CONFLICT: StatusCode = StatusCode::CONFLICT;
    const UNPROCESSABLE: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;

    fn linear(layer: &str, slug: &str) -> Value {
        let mut body = json!({
            "layer": layer,
            "catalog_entry_id": "curated/linear-app-mcp",
            "slug": slug,
        });
        if layer == "space" {
            body["space_id"] = json!("sp_eng");
        }
        body
    }

    fn ids(body: &Value) -> Vec<String> {
        let mut ids: Vec<String> = body["items"]
            .as_array()
            .unwrap_or_else(|| panic!("no items in {body}"))
            .iter()
            .map(|item| item["id"].as_str().unwrap().to_owned())
            .collect();
        ids.sort();
        ids
    }

    /// An org, a space and two user-layer integrations (`member`'s and `other`'s).
    async fn world() -> Harness {
        let h = Harness::new().await;
        h.seed("int_org", IntegrationLayer::Org, None, |_| {}).await;
        h.seed("int_space", IntegrationLayer::Space, None, |_| {})
            .await;
        h.seed(
            "int_member",
            IntegrationLayer::User,
            Some(Who::Member),
            |_| {},
        )
        .await;
        h.seed(
            "int_other",
            IntegrationLayer::User,
            Some(Who::Other),
            |_| {},
        )
        .await;
        h
    }

    // -- Create: permission matrix --------------------------------------------

    #[tokio::test]
    async fn who_may_create_at_each_layer() {
        let expected = [
            // (layer, [admin, eng_admin, member, viewer, other, sales_admin])
            (
                "org",
                [
                    CREATED, FORBIDDEN, FORBIDDEN, FORBIDDEN, FORBIDDEN, FORBIDDEN,
                ],
            ),
            (
                "space",
                [CREATED, CREATED, FORBIDDEN, FORBIDDEN, FORBIDDEN, FORBIDDEN],
            ),
            (
                "user",
                [CREATED, CREATED, CREATED, CREATED, CREATED, CREATED],
            ),
        ];
        for (layer, statuses) in expected {
            for (who, want) in Who::ALL.into_iter().zip(statuses) {
                let h = Harness::new().await;
                let (status, body) = h
                    .post(who, "/api/integrations", linear(layer, "linear"))
                    .await;
                assert_eq!(status, want, "{who:?} creating at {layer}: {body}");
            }
        }
    }

    #[tokio::test]
    async fn create_returns_the_detail_with_the_installers_status() {
        let h = Harness::new().await;

        let (status, body) = h
            .post(
                Who::EngAdmin,
                "/api/integrations",
                linear("space", "linear"),
            )
            .await;

        assert_eq!(status, CREATED, "{body}");
        assert!(body["id"].as_str().unwrap().starts_with("int_"));
        assert_eq!(body["layer"], "space");
        assert_eq!(body["space_id"], "sp_eng");
        assert_eq!(body["catalog_entry_id"], "curated/linear-app-mcp");
        assert_eq!(body["connect_url"], "https://mcp.linear.app/mcp");
        assert_eq!(body["auth_kind"], "oauth");
        assert_eq!(body["name"], "Linear");
        assert_eq!(body["trust"], "confirm");
        assert_eq!(body["enabled"], true);
        assert_eq!(body["credential_mode"], "per_user");
        assert_eq!(body["status"], "needs_connection");
        assert_eq!(body["has_credential"], false);
        assert!(body.get("owner_user_id").is_none());
    }

    #[tokio::test]
    async fn create_succeeds_even_when_credential_lookups_fail() {
        let mut h = Harness::new().await;
        h.break_credentials();

        let (status, body) = h
            .post(Who::Admin, "/api/integrations", linear("org", "linear"))
            .await;

        assert_eq!(
            status, CREATED,
            "the row is committed, so the caller must learn its id: {body}"
        );
        assert_eq!(body["status"], "needs_connection");
        assert_eq!(body["has_credential"], false);
        let rows = h
            .state
            .store
            .list(&"org_1".into(), &Default::default())
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn a_new_integration_that_needs_no_credential_is_ready() {
        let h = Harness::new().await;
        let mut body = linear("org", "open");
        body["catalog_entry_id"] = json!("curated/deepwiki-com-mcp");

        let (status, created) = h.post(Who::Admin, "/api/integrations", body).await;

        assert_eq!(status, CREATED, "{created}");
        assert_eq!(created["status"], "ready");
    }

    #[tokio::test]
    async fn create_without_a_slug_uses_the_catalog_slug() {
        let h = Harness::new().await;

        let (_, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({"layer": "org", "catalog_entry_id": "curated/linear-app-mcp"}),
            )
            .await;

        assert_eq!(body["slug"], "linear-app-mcp");
    }

    #[tokio::test]
    async fn create_copies_the_catalog_header_template() {
        let h = Harness::new().await;

        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({"layer": "org", "catalog_entry_id": "curated/posthog-com-mcp"}),
            )
            .await;

        assert_eq!(status, CREATED, "{body}");
        assert_eq!(body["auth_kind"], "header");
        assert_eq!(body["header_name"], "Authorization");
        assert_eq!(body["header_value_prefix"], "Bearer ");
        assert_eq!(body["credential_mode"], "shared");
    }

    #[tokio::test]
    async fn create_defaults_can_be_overridden() {
        let h = Harness::new().await;
        let mut body = linear("org", "linear");
        body["name"] = json!("Team Linear");
        body["credential_mode"] = json!("shared");
        body["trust"] = json!("trust");
        body["enabled"] = json!(false);

        let (status, created) = h.post(Who::Admin, "/api/integrations", body).await;

        assert_eq!(status, CREATED, "{created}");
        assert_eq!(created["name"], "Team Linear");
        assert_eq!(created["credential_mode"], "shared");
        assert_eq!(created["trust"], "trust");
        assert_eq!(created["enabled"], false);
        assert_eq!(created["status"], "disabled");
    }

    #[tokio::test]
    async fn unknown_catalog_auth_needs_a_choice() {
        let h = Harness::new().await;
        let miro = |extra: Value| {
            let mut body = json!({"layer": "org", "catalog_entry_id": "mcp/miro"});
            body.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            body
        };

        let (status, body) = h
            .post(Who::Admin, "/api/integrations", miro(json!({})))
            .await;
        assert_eq!(status, UNPROCESSABLE, "no choice: {body}");

        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                miro(json!({"auth_kind": "mixed"})),
            )
            .await;
        assert_eq!(status, UNPROCESSABLE, "mixed is not a choice: {body}");

        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                miro(json!({"auth_kind": "header"})),
            )
            .await;
        assert_eq!(status, UNPROCESSABLE, "header needs a header name: {body}");

        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                miro(json!({"auth_kind": "header", "header_name": "X-Api-Key"})),
            )
            .await;
        assert_eq!(status, CREATED, "{body}");
        assert_eq!(body["header_name"], "X-Api-Key");

        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                miro(json!({"auth_kind": "none", "slug": "miro-open"})),
            )
            .await;
        assert_eq!(status, CREATED, "{body}");
        assert_eq!(body["auth_kind"], "none");
        assert_eq!(body["status"], "ready");
    }

    #[tokio::test]
    async fn a_catalog_header_template_cannot_be_overridden() {
        let h = Harness::new().await;

        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({
                    "layer": "org",
                    "catalog_entry_id": "curated/posthog-com-mcp",
                    "header_name": "X-Other",
                }),
            )
            .await;

        assert_eq!(status, UNPROCESSABLE, "{body}");
    }

    #[tokio::test]
    async fn a_mixed_catalog_entry_may_be_narrowed() {
        let h = Harness::new().await;
        let github = |slug: &str, auth: &str| {
            json!({
                "layer": "org",
                "catalog_entry_id": "curated/github-com-mcp",
                "slug": slug,
                "auth_kind": auth,
            })
        };

        let (_, plain) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({"layer": "org", "catalog_entry_id": "curated/github-com-mcp"}),
            )
            .await;
        let (_, header) = h
            .post(Who::Admin, "/api/integrations", github("gh-pat", "header"))
            .await;
        let (status, wrong) = h
            .post(Who::Admin, "/api/integrations", github("gh-none", "none"))
            .await;

        assert_eq!(plain["auth_kind"], "mixed");
        assert_eq!(header["auth_kind"], "header");
        assert_eq!(header["header_name"], "Authorization");
        assert_eq!(status, UNPROCESSABLE, "{wrong}");
    }

    #[tokio::test]
    async fn custom_urls_need_a_name_and_an_auth_kind() {
        let h = Harness::new().await;
        h.set_policy(|p| p.allow_custom_urls = true).await;
        let custom = |extra: Value| {
            let mut body = json!({"layer": "org", "connect_url": "https://mcp.example.com/mcp"});
            body.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            body
        };

        let (status, body) = h
            .post(Who::Admin, "/api/integrations", custom(json!({})))
            .await;
        assert_eq!(status, UNPROCESSABLE, "{body}");
        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                custom(json!({"name": "Mine"})),
            )
            .await;
        assert_eq!(status, UNPROCESSABLE, "{body}");
        let (status, body) = h
            .post(
                Who::Admin,
                "/api/integrations",
                custom(json!({"name": "Mine", "slug": "mine", "auth_kind": "oauth"})),
            )
            .await;

        assert_eq!(status, CREATED, "{body}");
        assert!(body.get("catalog_entry_id").is_none());
        assert_eq!(body["connect_url"], "https://mcp.example.com/mcp");
    }

    #[tokio::test]
    async fn create_needs_exactly_one_source() {
        let h = Harness::new().await;
        h.set_policy(|p| p.allow_custom_urls = true).await;

        let (neither, _) = h
            .post(Who::Admin, "/api/integrations", json!({"layer": "org"}))
            .await;
        let (both, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({
                    "layer": "org",
                    "catalog_entry_id": "curated/linear-app-mcp",
                    "connect_url": "https://mcp.example.com",
                    "name": "x",
                    "auth_kind": "none",
                }),
            )
            .await;
        let (unknown_entry, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({"layer": "org", "catalog_entry_id": "curated/nope"}),
            )
            .await;
        let (not_a_url, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({"layer": "org", "connect_url": "not a url", "name": "x", "auth_kind": "none"}),
            )
            .await;

        assert_eq!(neither, UNPROCESSABLE);
        assert_eq!(both, UNPROCESSABLE);
        assert_eq!(unknown_entry, UNPROCESSABLE);
        assert_eq!(not_a_url, UNPROCESSABLE);
    }

    #[tokio::test]
    async fn invalid_slugs_are_unprocessable() {
        let h = Harness::new().await;
        for slug in [
            "",
            "Has Space",
            "UPPER",
            "-lead",
            "trail-",
            "double--dash",
            &"a".repeat(33),
        ] {
            let (status, body) = h
                .post(Who::Admin, "/api/integrations", linear("org", slug))
                .await;
            assert_eq!(status, UNPROCESSABLE, "slug {slug:?}: {body}");
        }
        let (status, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                linear("org", &"a".repeat(32)),
            )
            .await;
        assert_eq!(status, CREATED, "32 characters is the limit");
    }

    #[tokio::test]
    async fn duplicate_slug_in_a_layer_is_a_conflict_but_other_layers_are_fine() {
        let h = Harness::new().await;
        h.post(
            Who::EngAdmin,
            "/api/integrations",
            linear("space", "linear"),
        )
        .await;

        let (dup, body) = h
            .post(
                Who::EngAdmin,
                "/api/integrations",
                linear("space", "linear"),
            )
            .await;
        let (org, _) = h
            .post(Who::Admin, "/api/integrations", linear("org", "linear"))
            .await;
        let (user, _) = h
            .post(Who::Member, "/api/integrations", linear("user", "linear"))
            .await;
        let (other_user, _) = h
            .post(Who::Other, "/api/integrations", linear("user", "linear"))
            .await;
        let (user_dup, _) = h
            .post(Who::Member, "/api/integrations", linear("user", "linear"))
            .await;

        assert_eq!(dup, CONFLICT, "{body}");
        assert!(body["error"].as_str().unwrap().contains("linear"));
        assert_eq!((org, user, other_user), (CREATED, CREATED, CREATED));
        assert_eq!(user_dup, CONFLICT);
    }

    #[tokio::test]
    async fn space_layer_needs_a_real_space_and_other_layers_reject_one() {
        let h = Harness::new().await;

        let (missing, _) = h
            .post(
                Who::Admin,
                "/api/integrations",
                json!({"layer": "space", "catalog_entry_id": "curated/linear-app-mcp"}),
            )
            .await;
        let mut stray = linear("org", "linear");
        stray["space_id"] = json!("sp_eng");
        let (stray, _) = h.post(Who::Admin, "/api/integrations", stray).await;
        let mut elsewhere = linear("space", "linear");
        elsewhere["space_id"] = json!("sp_nope");
        let (unknown_space, _) = h.post(Who::Admin, "/api/integrations", elsewhere).await;

        assert_eq!(missing, UNPROCESSABLE);
        assert_eq!(stray, UNPROCESSABLE);
        assert_eq!(unknown_space, NOT_FOUND);
    }

    #[tokio::test]
    async fn user_layer_integrations_are_always_per_user_and_owned_by_the_caller() {
        let h = Harness::new().await;
        let mut shared = linear("user", "linear");
        shared["credential_mode"] = json!("shared");

        let (rejected, _) = h.post(Who::Member, "/api/integrations", shared).await;
        let (status, body) = h
            .post(Who::Member, "/api/integrations", linear("user", "linear"))
            .await;

        assert_eq!(rejected, UNPROCESSABLE);
        assert_eq!(status, CREATED, "{body}");
        assert_eq!(body["credential_mode"], "per_user");
        assert_eq!(body["owner_user_id"], "member");
    }

    #[tokio::test]
    async fn malformed_bodies_use_the_error_envelope() {
        let h = Harness::new().await;

        let (status, body) = h
            .post(Who::Admin, "/api/integrations", json!({"layer": "galaxy"}))
            .await;

        assert_eq!(status, UNPROCESSABLE);
        assert!(
            body["error"].as_str().is_some_and(|e| !e.is_empty()),
            "{body}"
        );
    }

    // -- List and get: visibility ---------------------------------------------

    #[tokio::test]
    async fn list_follows_visibility() {
        let h = world().await;
        let cases = [
            (
                Who::Admin,
                vec!["int_member", "int_org", "int_other", "int_space"],
            ),
            (Who::EngAdmin, vec!["int_org", "int_space"]),
            (Who::Member, vec!["int_member", "int_org", "int_space"]),
            (Who::Viewer, vec!["int_org", "int_space"]),
            (Who::Other, vec!["int_org", "int_other"]),
            (Who::SalesAdmin, vec!["int_org"]),
        ];
        for (who, want) in cases {
            let (status, body) = h.get(who, "/api/integrations").await;
            assert_eq!(status, OK, "{who:?}: {body}");
            assert_eq!(ids(&body), want, "{who:?}");
        }
    }

    #[tokio::test]
    async fn list_filters_by_layer_and_space() {
        let h = world().await;

        let (_, org) = h.get(Who::Admin, "/api/integrations?layer=org").await;
        let (_, user) = h.get(Who::Admin, "/api/integrations?layer=user").await;
        let (_, space) = h
            .get(Who::Admin, "/api/integrations?layer=space&space_id=sp_eng")
            .await;
        let (_, none) = h
            .get(
                Who::Admin,
                "/api/integrations?layer=space&space_id=sp_sales",
            )
            .await;
        let (bad, _) = h.get(Who::Admin, "/api/integrations?layer=galaxy").await;

        assert_eq!(ids(&org), ["int_org"]);
        assert_eq!(ids(&user), ["int_member", "int_other"]);
        assert_eq!(ids(&space), ["int_space"]);
        assert!(ids(&none).is_empty());
        assert_eq!(bad, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn the_space_filter_includes_org_integrations_offered_there() {
        let h = world().await;

        let (_, both) = h.get(Who::Admin, "/api/integrations?space_id=sp_eng").await;
        let (_, org_only) = h
            .get(Who::Admin, "/api/integrations?layer=org&space_id=sp_eng")
            .await;
        let (_, user) = h
            .get(Who::Admin, "/api/integrations?layer=user&space_id=sp_eng")
            .await;

        assert_eq!(ids(&both), ["int_org", "int_space"]);
        assert_eq!(ids(&org_only), ["int_org"]);
        assert!(
            ids(&user).is_empty(),
            "user-layer integrations have no space"
        );
    }

    #[tokio::test]
    async fn the_space_filter_drops_org_integrations_not_offered_there() {
        let h = world().await;
        h.state
            .store
            .add_space_access("int_org", &"sp_sales".into(), SpaceAccessMode::Allow)
            .await
            .unwrap();

        let (_, eng) = h.get(Who::Admin, "/api/integrations?space_id=sp_eng").await;
        let (_, sales) = h
            .get(Who::Admin, "/api/integrations?space_id=sp_sales")
            .await;

        assert_eq!(ids(&eng), ["int_space"], "restricted to sales");
        assert_eq!(ids(&sales), ["int_org"]);
    }

    #[tokio::test]
    async fn the_space_filter_drops_org_integrations_the_space_opted_out_of() {
        let h = world().await;
        h.state
            .store
            .add_space_access("int_org", &eng(), SpaceAccessMode::OptOut)
            .await
            .unwrap();

        let (_, filtered) = h
            .get(Who::EngAdmin, "/api/integrations?space_id=sp_eng")
            .await;
        let (_, unfiltered) = h.get(Who::EngAdmin, "/api/integrations").await;

        assert_eq!(ids(&filtered), ["int_space"]);
        assert!(
            ids(&unfiltered).contains(&"int_org".to_owned()),
            "still listed without the filter, so the space can opt back in"
        );
    }

    #[tokio::test]
    async fn the_space_filter_still_follows_visibility() {
        let h = world().await;

        let (_, body) = h.get(Who::Other, "/api/integrations?space_id=sp_eng").await;

        assert!(
            ids(&body).is_empty(),
            "eng's integrations are not sales' to see"
        );
    }

    #[tokio::test]
    async fn the_space_filter_does_not_reveal_other_spaces_availability() {
        let h = world().await;
        h.state
            .store
            .add_space_access("int_org", &"sp_sales".into(), SpaceAccessMode::Allow)
            .await
            .unwrap();

        let (_, member_of_eng) = h
            .get(Who::Member, "/api/integrations?space_id=sp_sales")
            .await;
        let (_, member_of_sales) = h
            .get(Who::Other, "/api/integrations?space_id=sp_sales")
            .await;
        let (_, admin) = h
            .get(Who::Admin, "/api/integrations?space_id=sp_sales")
            .await;

        assert!(ids(&member_of_eng).is_empty(), "eng may not probe sales");
        assert_eq!(ids(&member_of_sales), ["int_org"]);
        assert_eq!(ids(&admin), ["int_org"]);
    }

    // -- Input hygiene ---------------------------------------------------------

    #[tokio::test]
    async fn names_are_stored_trimmed() {
        let h = Harness::new().await;
        let mut body = linear("org", "linear");
        body["name"] = json!("  Team Linear \n");

        let (status, created) = h.post(Who::Admin, "/api/integrations", body).await;
        let (patched, updated) = h
            .patch(
                Who::Admin,
                &format!("/api/integrations/{}", created["id"].as_str().unwrap()),
                json!({"name": "\t Renamed  "}),
            )
            .await;

        assert_eq!(status, CREATED, "{created}");
        assert_eq!(created["name"], "Team Linear");
        assert_eq!(patched, OK, "{updated}");
        assert_eq!(updated["name"], "Renamed");
    }

    #[tokio::test]
    async fn blank_names_are_unprocessable() {
        let h = world().await;
        let mut body = linear("org", "linear");
        body["name"] = json!("   ");

        let (create, _) = h.post(Who::Admin, "/api/integrations", body).await;
        let (patch, _) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"name": " "}),
            )
            .await;

        assert_eq!((create, patch), (UNPROCESSABLE, UNPROCESSABLE));
    }

    #[tokio::test]
    async fn connect_urls_with_credentials_or_fragments_are_unprocessable() {
        let h = Harness::new().await;
        h.set_policy(|p| p.allow_custom_urls = true).await;
        h.seed("int_custom", IntegrationLayer::Org, None, |i| {
            i.catalog_entry_id = None;
        })
        .await;
        for url in [
            "https://bob:token@mcp.example.com/mcp",
            "https://bob@mcp.example.com/mcp",
            "https://:token@mcp.example.com/mcp",
            "https://mcp.example.com/mcp#frag",
        ] {
            let (create, body) = h
                .post(
                    Who::Admin,
                    "/api/integrations",
                    json!({"layer": "org", "connect_url": url, "name": "x", "slug": "x", "auth_kind": "none"}),
                )
                .await;
            let (patch, _) = h
                .patch(
                    Who::Admin,
                    "/api/integrations/int_custom",
                    json!({"connect_url": url}),
                )
                .await;
            assert_eq!(create, UNPROCESSABLE, "create {url}: {body}");
            assert_eq!(patch, UNPROCESSABLE, "patch {url}");
        }
    }

    #[tokio::test]
    async fn org_integrations_restricted_to_other_spaces_are_hidden() {
        let h = world().await;
        h.state
            .store
            .add_space_access("int_org", &"sp_sales".into(), SpaceAccessMode::Allow)
            .await
            .unwrap();

        let (_, member) = h.get(Who::Member, "/api/integrations").await;
        let (_, other) = h.get(Who::Other, "/api/integrations").await;
        let (get_status, _) = h.get(Who::Member, "/api/integrations/int_org").await;

        assert_eq!(ids(&member), ["int_member", "int_space"]);
        assert!(ids(&other).contains(&"int_org".to_owned()));
        assert_eq!(get_status, NOT_FOUND);
    }

    #[tokio::test]
    async fn a_space_that_opted_out_still_sees_the_integration_to_opt_back_in() {
        let h = world().await;
        h.state
            .store
            .add_space_access("int_org", &eng(), SpaceAccessMode::OptOut)
            .await
            .unwrap();

        let (_, body) = h.get(Who::EngAdmin, "/api/integrations").await;

        assert!(ids(&body).contains(&"int_org".to_owned()));
    }

    #[tokio::test]
    async fn get_follows_visibility() {
        let h = world().await;
        let cases = [
            (Who::Member, "int_org", OK),
            (Who::Other, "int_org", OK),
            (Who::Member, "int_space", OK),
            (Who::Viewer, "int_space", OK),
            (Who::Other, "int_space", NOT_FOUND),
            (Who::SalesAdmin, "int_space", NOT_FOUND),
            (Who::Member, "int_member", OK),
            (Who::Member, "int_other", NOT_FOUND),
            (Who::EngAdmin, "int_member", NOT_FOUND),
            (Who::Admin, "int_member", OK),
            (Who::Admin, "int_nope", NOT_FOUND),
        ];
        for (who, id, want) in cases {
            let (status, body) = h.get(who, &format!("/api/integrations/{id}")).await;
            assert_eq!(status, want, "{who:?} getting {id}: {body}");
            if want == NOT_FOUND {
                assert!(body["error"].is_string(), "{body}");
            }
        }
    }

    #[tokio::test]
    async fn another_orgs_integration_is_not_found() {
        let h = world().await;
        sqlx::query("INSERT INTO organizations (id, name, slug) VALUES ('org_2', 'Beta', 'beta')")
            .execute(h.org_storage.pool())
            .await
            .unwrap();
        h.seed("int_beta", IntegrationLayer::Org, None, |i| {
            i.org_id = "org_2".into();
        })
        .await;

        let (get, _) = h.get(Who::Admin, "/api/integrations/int_beta").await;
        let (delete, _) = h.delete(Who::Admin, "/api/integrations/int_beta").await;
        let (_, list) = h.get(Who::Admin, "/api/integrations").await;

        assert_eq!((get, delete), (NOT_FOUND, NOT_FOUND));
        assert!(!ids(&list).contains(&"int_beta".to_owned()));
    }

    #[tokio::test]
    async fn get_returns_the_detail_with_space_rows() {
        let h = world().await;
        for space in ["sp_eng", "sp_sales"] {
            h.state
                .store
                .add_space_access("int_org", &space.into(), SpaceAccessMode::Allow)
                .await
                .unwrap();
        }

        let (_, admin) = h.get(Who::Admin, "/api/integrations/int_org").await;
        let (_, member) = h.get(Who::Member, "/api/integrations/int_org").await;

        assert_eq!(admin["header_name"], "Authorization");
        assert_eq!(admin["space_access"].as_array().unwrap().len(), 2);
        assert_eq!(
            member["space_access"],
            json!([{"space_id": "sp_eng", "mode": "allow"}]),
            "members see only their own spaces' rows"
        );
    }

    // -- Update ----------------------------------------------------------------

    #[tokio::test]
    async fn who_may_patch() {
        let cases = [
            ("int_org", Who::Admin, OK),
            ("int_org", Who::EngAdmin, FORBIDDEN),
            ("int_org", Who::Member, FORBIDDEN),
            ("int_org", Who::Other, FORBIDDEN),
            ("int_space", Who::Admin, OK),
            ("int_space", Who::EngAdmin, OK),
            ("int_space", Who::Member, FORBIDDEN),
            ("int_space", Who::Viewer, FORBIDDEN),
            ("int_space", Who::Other, NOT_FOUND),
            ("int_space", Who::SalesAdmin, NOT_FOUND),
            ("int_member", Who::Member, OK),
            ("int_member", Who::Admin, FORBIDDEN),
            ("int_member", Who::EngAdmin, NOT_FOUND),
            ("int_member", Who::Other, NOT_FOUND),
        ];
        for (id, who, want) in cases {
            let h = world().await;
            let (status, body) = h
                .patch(
                    who,
                    &format!("/api/integrations/{id}"),
                    json!({"name": "Renamed"}),
                )
                .await;
            assert_eq!(status, want, "{who:?} patching {id}: {body}");
        }
    }

    #[tokio::test]
    async fn patch_updates_the_given_fields_and_returns_the_detail() {
        let h = world().await;

        let (status, body) = h
            .patch(
                Who::EngAdmin,
                "/api/integrations/int_space",
                json!({"name": "Renamed", "slug": "renamed", "enabled": false, "trust": "trust"}),
            )
            .await;

        assert_eq!(status, OK, "{body}");
        assert_eq!(body["name"], "Renamed");
        assert_eq!(body["slug"], "renamed");
        assert_eq!(body["enabled"], false);
        assert_eq!(body["trust"], "trust");
        assert_eq!(body["status"], "disabled");
        assert_eq!(
            body["connect_url"], "https://mcp.linear.app/mcp",
            "untouched"
        );
        assert!(body["updated_at"] != body["created_at"]);
        let (_, stored) = h.get(Who::EngAdmin, "/api/integrations/int_space").await;
        assert_eq!(stored["name"], "Renamed");
    }

    #[tokio::test]
    async fn timestamps_come_from_the_injected_clock() {
        let h = world().await;

        let (_, created) = h
            .post(Who::Admin, "/api/integrations", linear("org", "linear"))
            .await;
        let (_, patched) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"name": "New"}),
            )
            .await;

        assert_eq!(created["created_at"], json!(now()));
        assert_eq!(created["updated_at"], json!(now()));
        assert_eq!(patched["updated_at"], json!(now()));
    }

    #[tokio::test]
    async fn a_failed_status_lookup_leaves_the_row_unchanged() {
        let mut h = world().await;
        h.break_credentials();

        let (status, _) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"name": "Renamed"}),
            )
            .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        let stored = h.state.store.get("int_org").await.unwrap().unwrap();
        assert_eq!(stored.name, "int_org", "the failure came before the write");
    }

    #[tokio::test]
    async fn a_concurrent_change_makes_patch_conflict_without_losing_it() {
        let mut h = world().await;
        let mut rival = h.state.store.get("int_org").await.unwrap().unwrap();
        rival.enabled = false;
        rival.updated_at = super::super::testing::at(500);
        h.lose_the_next_update_to(rival);

        let (status, body) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"name": "Renamed"}),
            )
            .await;

        assert_eq!(status, CONFLICT, "{body}");
        let stored = h.state.store.get("int_org").await.unwrap().unwrap();
        assert!(!stored.enabled, "the rival's change survives");
        assert_eq!(stored.name, "int_org", "and the stale write did not land");
    }

    #[tokio::test]
    async fn patch_rejects_bad_slugs_and_duplicates() {
        let h = world().await;
        h.seed("int_org2", IntegrationLayer::Org, None, |i| {
            i.slug = "taken".into()
        })
        .await;

        let (bad, _) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"slug": "Not Valid"}),
            )
            .await;
        let (dup, _) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"slug": "taken"}),
            )
            .await;

        assert_eq!(bad, UNPROCESSABLE);
        assert_eq!(dup, CONFLICT);
    }

    #[tokio::test]
    async fn a_catalog_install_keeps_its_url() {
        let h = world().await;

        let (status, body) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"connect_url": "https://evil.example.com/mcp"}),
            )
            .await;

        assert_eq!(status, UNPROCESSABLE, "{body}");
        let (_, stored) = h.get(Who::Admin, "/api/integrations/int_org").await;
        assert_eq!(stored["connect_url"], "https://mcp.linear.app/mcp");
    }

    #[tokio::test]
    async fn a_custom_url_can_move_and_its_credentials_go_with_it() {
        let h = world().await;
        h.seed("int_custom", IntegrationLayer::Org, None, |i| {
            i.catalog_entry_id = None;
            i.connect_url = "https://old.example.com/mcp".into();
        })
        .await;
        h.seed_credential("int_custom", None, CredentialStatus::Ready)
            .await;

        let (status, body) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_custom",
                json!({"connect_url": "https://new.example.com/mcp", "header_name": "X-Key"}),
            )
            .await;

        assert_eq!(status, OK, "{body}");
        assert_eq!(body["connect_url"], "https://new.example.com/mcp");
        assert_eq!(body["header_name"], "X-Key");
        assert_eq!(body["has_credential"], false);
        assert_eq!(
            body["status"], "needs_connection",
            "the old credential is gone"
        );
        assert!(!h.has_credential("int_custom", None).await);
    }

    #[tokio::test]
    async fn header_fields_of_a_catalog_install_are_fixed() {
        let h = world().await;

        let (status, _) = h
            .patch(
                Who::Admin,
                "/api/integrations/int_org",
                json!({"header_name": "X-Other"}),
            )
            .await;

        assert_eq!(status, UNPROCESSABLE);
    }

    // -- Delete ----------------------------------------------------------------

    #[tokio::test]
    async fn who_may_delete() {
        let cases = [
            ("int_org", Who::Admin, NO_CONTENT),
            ("int_org", Who::EngAdmin, FORBIDDEN),
            ("int_org", Who::Member, FORBIDDEN),
            ("int_space", Who::Admin, NO_CONTENT),
            ("int_space", Who::EngAdmin, NO_CONTENT),
            ("int_space", Who::Member, FORBIDDEN),
            ("int_space", Who::Viewer, FORBIDDEN),
            ("int_space", Who::Other, NOT_FOUND),
            ("int_member", Who::Member, NO_CONTENT),
            ("int_member", Who::Admin, NO_CONTENT),
            ("int_member", Who::EngAdmin, NOT_FOUND),
            ("int_member", Who::Other, NOT_FOUND),
        ];
        for (id, who, want) in cases {
            let h = world().await;
            let (status, body) = h.delete(who, &format!("/api/integrations/{id}")).await;
            assert_eq!(status, want, "{who:?} deleting {id}: {body}");
            let (after, _) = h.get(Who::Admin, &format!("/api/integrations/{id}")).await;
            let still_there = if want == NO_CONTENT { NOT_FOUND } else { OK };
            assert_eq!(after, still_there, "{who:?} deleting {id}");
        }
    }

    #[tokio::test]
    async fn an_org_admin_removes_a_users_integration_and_its_credentials() {
        let h = world().await;
        h.seed_credential("int_member", Some(Who::Member), CredentialStatus::Ready)
            .await;

        let (status, body) = h.delete(Who::Admin, "/api/integrations/int_member").await;

        assert_eq!(status, NO_CONTENT);
        assert_eq!(body, Value::Null);
        assert!(!h.has_credential("int_member", Some(Who::Member)).await);
    }

    #[tokio::test]
    async fn an_org_admin_never_receives_credential_material() {
        let h = world().await;
        h.seed_credential("int_member", Some(Who::Member), CredentialStatus::Ready)
            .await;

        let (detail_status, detail) = h.get(Who::Admin, "/api/integrations/int_member").await;
        let (list_status, list) = h.get(Who::Admin, "/api/integrations").await;

        assert_eq!((detail_status, list_status), (OK, OK), "{detail} {list}");
        assert_eq!(detail["id"], "int_member");
        assert!(ids(&list).contains(&"int_member".to_owned()));
        for text in [detail.to_string(), list.to_string()] {
            for needle in [
                "ciphertext",
                "nonce",
                "secret",
                "token",
                "password",
                "key_id",
            ] {
                assert!(!text.contains(needle), "{needle} leaked: {text}");
            }
        }
    }

    // -- Status ----------------------------------------------------------------

    async fn status_of(h: &Harness, who: Who, id: &str) -> (Value, Value) {
        let (_, body) = h.get(who, &format!("/api/integrations/{id}")).await;
        (body["status"].clone(), body["has_credential"].clone())
    }

    #[tokio::test]
    async fn status_follows_the_callers_credential() {
        let h = world().await;
        assert_eq!(
            status_of(&h, Who::Member, "int_org").await,
            (json!("needs_connection"), json!(false))
        );

        h.seed_credential("int_org", None, CredentialStatus::Ready)
            .await;
        assert_eq!(
            status_of(&h, Who::Member, "int_org").await,
            (json!("ready"), json!(true))
        );
        assert_eq!(
            status_of(&h, Who::Other, "int_org").await,
            (json!("ready"), json!(true)),
            "shared"
        );

        h.seed_credential("int_member", Some(Who::Member), CredentialStatus::Ready)
            .await;
        assert_eq!(
            status_of(&h, Who::Member, "int_member").await,
            (json!("ready"), json!(true))
        );
    }

    #[tokio::test]
    async fn an_org_admin_learns_nothing_about_a_users_credential_from_has_credential() {
        let h = world().await;
        h.seed_credential("int_member", Some(Who::Member), CredentialStatus::Ready)
            .await;

        let (owner_status, owner_has) = status_of(&h, Who::Member, "int_member").await;
        let (admin_status, admin_has) = status_of(&h, Who::Admin, "int_member").await;

        assert_eq!((owner_status, owner_has), (json!("ready"), json!(true)));
        assert_eq!(admin_status, json!("ready"), "status is the owner's");
        assert_eq!(
            admin_has,
            json!(false),
            "has_credential is the caller's own"
        );
    }

    #[tokio::test]
    async fn per_user_status_is_per_caller() {
        let h = world().await;
        h.seed("int_pu", IntegrationLayer::Org, None, |i| {
            i.credential_mode = assistant_core::integrations::CredentialMode::PerUser;
        })
        .await;
        h.seed_credential("int_pu", Some(Who::Member), CredentialStatus::Ready)
            .await;

        assert_eq!(
            status_of(&h, Who::Member, "int_pu").await,
            (json!("ready"), json!(true))
        );
        assert_eq!(
            status_of(&h, Who::Other, "int_pu").await,
            (json!("needs_connection"), json!(false))
        );
    }

    #[tokio::test]
    async fn status_reports_expired_and_foreign_key_credentials() {
        let h = world().await;
        h.seed_credential("int_org", None, CredentialStatus::AuthExpired)
            .await;
        assert_eq!(
            status_of(&h, Who::Member, "int_org").await.0,
            json!("auth_expired")
        );

        h.seed_credential_with_key("int_org", None, CredentialStatus::Ready, "other-key")
            .await;
        assert_eq!(
            status_of(&h, Who::Member, "int_org").await.0,
            json!("auth_expired")
        );
    }

    #[tokio::test]
    async fn status_reports_disabled_integrations() {
        let h = world().await;
        h.seed("int_off", IntegrationLayer::Org, None, |i| {
            i.enabled = false
        })
        .await;

        assert_eq!(
            status_of(&h, Who::Member, "int_off").await.0,
            json!("disabled")
        );
    }

    #[tokio::test]
    async fn list_items_carry_status_and_never_a_credential() {
        let h = world().await;
        h.seed_credential("int_org", None, CredentialStatus::Ready)
            .await;

        let (_, body) = h.get(Who::Member, "/api/integrations").await;
        let org = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["id"] == "int_org")
            .unwrap();

        assert_eq!(org["status"], "ready");
        assert_eq!(org["has_credential"], true);
        assert!(org.get("space_access").is_none(), "summaries are flat");
    }
}
