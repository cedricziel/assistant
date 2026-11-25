//! Request and response types of the integration management routes.
//!
//! The enums mirror the domain enums in `assistant_core::integrations`, which
//! carry no OpenAPI schema.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use assistant_core::integrations as domain;
use assistant_core::integrations::Availability;

/// Declares an API enum with the same variants as a domain enum, plus both
/// `From` conversions. Wire names are the snake_case variant names.
macro_rules! mirror_enum {
    ($(#[$meta:meta])* $api:ident <=> $domain:ty { $($variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $api {
            $($variant),+
        }

        impl From<$domain> for $api {
            fn from(value: $domain) -> Self {
                match value {
                    $(<$domain>::$variant => Self::$variant),+
                }
            }
        }

        impl From<$api> for $domain {
            fn from(value: $api) -> Self {
                match value {
                    $($api::$variant => Self::$variant),+
                }
            }
        }
    };
}

mirror_enum! {
    /// Who owns an integration.
    IntegrationLayer <=> domain::IntegrationLayer { Org, Space, User }
}

mirror_enum! {
    /// How an integration authenticates to its MCP server.
    IntegrationAuthKind <=> domain::IntegrationAuthKind { None, Header, Oauth, Mixed }
}

mirror_enum! {
    /// Whose credential an integration uses.
    CredentialMode <=> domain::CredentialMode { Shared, PerUser }
}

mirror_enum! {
    /// Whether tool calls need user confirmation.
    IntegrationTrust <=> domain::IntegrationTrust { Confirm, Trust }
}

mirror_enum! {
    /// An org-layer integration's relation to a space: `allow` restricts the
    /// integration to listed spaces, `opt_out` is a space's own opt-out.
    SpaceAccessMode <=> domain::SpaceAccessMode { Allow, OptOut }
}

mirror_enum! {
    /// Which catalog entries lower layers may install.
    CatalogMode <=> domain::CatalogMode { All, Allowlist, None }
}

/// Whether the caller can use an integration right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationStatus {
    Ready,
    /// The caller has no credential yet.
    NeedsConnection,
    /// The caller's credential is unusable and must be set again.
    AuthExpired,
    /// The MCP server could not be reached. Reported once connections are
    /// tested; never returned by the management routes alone.
    Unreachable,
    /// The org policy no longer permits this integration.
    BlockedByPolicy,
    Disabled,
}

impl From<Availability> for IntegrationStatus {
    fn from(availability: Availability) -> Self {
        match availability {
            Availability::Available => Self::Ready,
            Availability::Disabled => Self::Disabled,
            Availability::BlockedByPolicy => Self::BlockedByPolicy,
            Availability::NeedsConnection => Self::NeedsConnection,
            Availability::AuthExpired => Self::AuthExpired,
        }
    }
}

// -- Integrations ------------------------------------------------------------

/// An installed integration, as listed.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationSummary {
    /// `int_<uuid>`.
    pub id: String,
    pub layer: IntegrationLayer,
    /// The space of a space-layer integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    /// The owner of a user-layer integration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_user_id: Option<String>,
    /// The catalog entry it was installed from; absent for a custom URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_entry_id: Option<String>,
    /// Unique within its layer and owner. Part of its tool names.
    pub slug: String,
    pub name: String,
    pub connect_url: String,
    pub auth_kind: IntegrationAuthKind,
    pub credential_mode: CredentialMode,
    pub trust: IntegrationTrust,
    pub enabled: bool,
    /// Whether the caller can use it. For an org admin looking at another
    /// user's integration, the status is the owner's.
    pub status: IntegrationStatus,
    /// Whether a credential is stored for the caller's subject: the shared
    /// credential, or the caller's own for a per-user integration. The value
    /// itself is never returned.
    pub has_credential: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// One space row of an org-layer integration.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SpaceAccessEntry {
    pub space_id: String,
    pub mode: SpaceAccessMode,
}

/// An installed integration with its auth template and space rows.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationDetail {
    /// `int_<uuid>`.
    pub id: String,
    pub layer: IntegrationLayer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_entry_id: Option<String>,
    pub slug: String,
    pub name: String,
    pub connect_url: String,
    pub auth_kind: IntegrationAuthKind,
    /// Header that carries the credential, for `header` and `mixed` auth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    /// Text before the secret in the header value, e.g. `Bearer `.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_value_prefix: Option<String>,
    pub credential_mode: CredentialMode,
    pub trust: IntegrationTrust,
    pub enabled: bool,
    pub status: IntegrationStatus,
    pub has_credential: bool,
    /// Space rows of an org-layer integration, limited to the spaces the
    /// caller belongs to unless the caller is an org admin.
    pub space_access: Vec<SpaceAccessEntry>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The integrations the caller may see.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationListResponse {
    pub items: Vec<IntegrationSummary>,
}

/// Query parameters of `GET /api/integrations`.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListIntegrationsQuery {
    /// Only integrations of this layer.
    pub layer: Option<IntegrationLayer>,
    /// Only space-layer integrations of this space.
    pub space_id: Option<String>,
}

/// Body of `POST /api/integrations`.
///
/// Name exactly one of `catalog_entry_id` and `connect_url`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateIntegrationRequest {
    pub layer: IntegrationLayer,
    /// Required for the space layer, rejected otherwise.
    #[serde(default)]
    pub space_id: Option<String>,
    /// Install this catalog entry. Its URL, auth kind and header template are
    /// copied at install time.
    #[serde(default)]
    pub catalog_entry_id: Option<String>,
    /// Install a custom remote MCP server. Needs `name` and `auth_kind`.
    #[serde(default)]
    pub connect_url: Option<String>,
    /// Kebab-case, 1–32 characters. Defaults to the catalog entry's slug.
    #[serde(default)]
    pub slug: Option<String>,
    /// Defaults to the catalog entry's name. Required for a custom URL.
    #[serde(default)]
    pub name: Option<String>,
    /// Required for a custom URL and for a catalog entry whose auth kind is
    /// unknown (choose `none`, `header` or `oauth`). A `mixed` catalog entry
    /// may be narrowed to `header` or `oauth`.
    #[serde(default)]
    pub auth_kind: Option<IntegrationAuthKind>,
    /// Header that carries the credential. Required for `header` auth on a
    /// custom URL or an unknown catalog entry; rejected when the catalog
    /// entry supplies its own template.
    #[serde(default)]
    pub header_name: Option<String>,
    /// Text before the secret in the header value. Same rules as `header_name`.
    #[serde(default)]
    pub header_value_prefix: Option<String>,
    /// Defaults to `per_user` for OAuth and `shared` otherwise. User-layer
    /// integrations are always `per_user`.
    #[serde(default)]
    pub credential_mode: Option<CredentialMode>,
    /// Defaults to `confirm`.
    #[serde(default)]
    pub trust: Option<IntegrationTrust>,
    /// Defaults to `true`.
    #[serde(default)]
    pub enabled: Option<bool>,
}

/// Body of `PATCH /api/integrations/{id}`. Absent fields stay as they are.
///
/// The layer, owner, catalog entry, auth kind and credential mode cannot
/// change: they decide whose credentials exist.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
pub struct UpdateIntegrationRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub trust: Option<IntegrationTrust>,
    /// Custom URL integrations only. Changing it deletes the stored
    /// credentials, so they cannot be sent to the new host.
    #[serde(default)]
    pub connect_url: Option<String>,
    /// Custom URL integrations only.
    #[serde(default)]
    pub header_name: Option<String>,
    /// Custom URL integrations only.
    #[serde(default)]
    pub header_value_prefix: Option<String>,
}

// -- Policy ------------------------------------------------------------------

/// What lower layers may install in this org.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct IntegrationPolicyResponse {
    pub catalog_mode: CatalogMode,
    /// Catalog entry ids allowed when `catalog_mode` is `allowlist`.
    pub allowed_catalog_ids: Vec<String>,
    pub allow_custom_urls: bool,
    pub allow_user_integrations: bool,
    /// Absent until an org admin first saves the policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Body of `PUT /api/integrations/policy`. Replaces the whole policy.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateIntegrationPolicyRequest {
    pub catalog_mode: CatalogMode,
    #[serde(default)]
    pub allowed_catalog_ids: Vec<String>,
    pub allow_custom_urls: bool,
    pub allow_user_integrations: bool,
}

// -- Space availability ------------------------------------------------------

/// Body of `PUT /api/integrations/{id}/spaces/{space_id}`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SetSpaceAccessRequest {
    pub mode: SpaceAccessMode,
}

/// Query parameters of `DELETE /api/integrations/{id}/spaces/{space_id}`.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct DeleteSpaceAccessQuery {
    /// Which row to remove.
    pub mode: SpaceAccessMode,
}
