//! Domain types and store traits for scoped integrations.
//!
//! Implementations: `Sqlite*` in `assistant-storage`, `InMemory*` in
//! [`super::memory`].

use std::fmt;
use std::str::FromStr;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::identity::{OrgId, SpaceId, UserId};

/// Declares a string-backed enum with `as_str`, `Display` and `FromStr`.
macro_rules! string_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $(
                #[serde(rename = $text)]
                $variant,
            )+
        }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $text,)+ }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> Result<Self> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => anyhow::bail!("unknown {} '{other}'", stringify!($name)),
                }
            }
        }
    };
}

string_enum! {
    /// Who owns an integration.
    IntegrationLayer { Org => "org", Space => "space", User => "user" }
}

string_enum! {
    /// How an integration authenticates to its MCP server.
    IntegrationAuthKind { None => "none", Header => "header", Oauth => "oauth", Mixed => "mixed" }
}

string_enum! {
    /// Whose credential an integration uses.
    CredentialMode { Shared => "shared", PerUser => "per_user" }
}

string_enum! {
    /// Whether tool calls need user confirmation.
    IntegrationTrust { Confirm => "confirm", Trust => "trust" }
}

string_enum! {
    /// An org-layer integration's relation to a space.
    SpaceAccessMode { Allow => "allow", OptOut => "opt_out" }
}

string_enum! {
    /// Which catalog entries lower layers may install.
    CatalogMode { All => "all", Allowlist => "allowlist", None => "none" }
}

string_enum! {
    /// What a stored credential holds.
    CredentialKind { Header => "header", Oauth => "oauth" }
}

string_enum! {
    /// Whether a stored credential can be used.
    CredentialStatus { Ready => "ready", AuthExpired => "auth_expired" }
}

/// An installed remote MCP integration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integration {
    /// `int_<uuid>`.
    pub id: String,
    pub org_id: OrgId,
    pub layer: IntegrationLayer,
    /// Set for the space layer only.
    pub space_id: Option<SpaceId>,
    /// Set for the user layer only.
    pub owner_user_id: Option<UserId>,
    /// `None` for a custom URL.
    pub catalog_entry_id: Option<String>,
    pub connect_url: String,
    pub slug: String,
    pub name: String,
    pub auth_kind: IntegrationAuthKind,
    pub header_name: Option<String>,
    pub header_value_prefix: Option<String>,
    pub credential_mode: CredentialMode,
    pub trust: IntegrationTrust,
    pub enabled: bool,
    pub oauth_client_id_override: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Narrows [`IntegrationStore::list`]. Unset fields match everything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntegrationFilter {
    pub layer: Option<IntegrationLayer>,
    pub space_id: Option<SpaceId>,
    pub owner_user_id: Option<UserId>,
}

/// One `integration_space_access` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceAccess {
    pub integration_id: String,
    pub space_id: SpaceId,
    pub mode: SpaceAccessMode,
}

/// An org's limits on what spaces and users may install.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationPolicy {
    pub org_id: OrgId,
    pub catalog_mode: CatalogMode,
    pub allowed_catalog_ids: Vec<String>,
    pub allow_custom_urls: bool,
    pub allow_user_integrations: bool,
    pub updated_at: DateTime<Utc>,
}

impl IntegrationPolicy {
    /// The policy of an org that never set one (`updated_at` is the Unix epoch).
    pub fn default_for(org_id: OrgId) -> Self {
        Self {
            org_id,
            catalog_mode: CatalogMode::All,
            allowed_catalog_ids: Vec::new(),
            allow_custom_urls: false,
            allow_user_integrations: true,
            updated_at: DateTime::UNIX_EPOCH,
        }
    }

    /// `false` when this policy forbids `integration` to exist.
    ///
    /// Org-layer integrations are managed by org admins and never restricted.
    pub fn permits(&self, integration: &Integration) -> bool {
        if integration.layer == IntegrationLayer::Org {
            return true;
        }
        if integration.layer == IntegrationLayer::User && !self.allow_user_integrations {
            return false;
        }
        match &integration.catalog_entry_id {
            None => self.allow_custom_urls,
            Some(entry) => match self.catalog_mode {
                CatalogMode::All => true,
                CatalogMode::None => false,
                CatalogMode::Allowlist => self.allowed_catalog_ids.contains(entry),
            },
        }
    }
}

/// A credential as stored: ciphertext only. Encrypt and decrypt with
/// [`super::secret::SecretBox`] in the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredCredential {
    pub integration_id: String,
    /// `None` is the shared credential.
    pub subject_user_id: Option<UserId>,
    pub kind: CredentialKind,
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: CredentialStatus,
    pub updated_at: DateTime<Utc>,
}

/// Store-level failures callers map to specific API responses.
#[derive(Debug, PartialEq, Eq)]
pub enum IntegrationStoreError {
    /// The slug is already taken in this layer and owner scope.
    DuplicateSlug { slug: String },
}

impl fmt::Display for IntegrationStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSlug { slug } => {
                write!(f, "integration slug '{slug}' already exists in this scope")
            }
        }
    }
}

impl std::error::Error for IntegrationStoreError {}

/// Integrations, their space access rows and the org policy.
#[async_trait]
pub trait IntegrationStore: Send + Sync {
    /// Fails with [`IntegrationStoreError::DuplicateSlug`] when the slug is taken.
    async fn create(&self, integration: &Integration) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Integration>>;
    /// Integrations of `org_id` matching `filter`, oldest first.
    async fn list(&self, org_id: &OrgId, filter: &IntegrationFilter) -> Result<Vec<Integration>>;
    /// Replaces the mutable fields (everything except id, org, layer, owner
    /// columns, catalog entry and `created_at`). `false` if `id` is unknown.
    async fn update(&self, integration: &Integration) -> Result<bool>;
    /// Deletes the integration with its credentials, space access and OAuth
    /// state. `false` if `id` is unknown.
    async fn delete(&self, id: &str) -> Result<bool>;

    /// Idempotent.
    async fn add_space_access(
        &self,
        integration_id: &str,
        space_id: &SpaceId,
        mode: SpaceAccessMode,
    ) -> Result<()>;
    /// `false` if no such row existed.
    async fn remove_space_access(
        &self,
        integration_id: &str,
        space_id: &SpaceId,
        mode: SpaceAccessMode,
    ) -> Result<bool>;
    async fn list_space_access(&self, integration_id: &str) -> Result<Vec<SpaceAccess>>;
    /// Every space access row of every integration in `org_id`.
    async fn list_org_space_access(&self, org_id: &OrgId) -> Result<Vec<SpaceAccess>>;

    /// The org's policy, or [`IntegrationPolicy::default_for`] if never set.
    async fn get_policy(&self, org_id: &OrgId) -> Result<IntegrationPolicy>;
    async fn put_policy(&self, policy: &IntegrationPolicy) -> Result<()>;
}

/// Encrypted credentials, one per `(integration, subject)`.
#[async_trait]
pub trait IntegrationCredentialStore: Send + Sync {
    /// Insert or replace the credential of its `(integration, subject)`.
    async fn put(&self, credential: &StoredCredential) -> Result<()>;
    async fn get(
        &self,
        integration_id: &str,
        subject: Option<&UserId>,
    ) -> Result<Option<StoredCredential>>;
    /// `false` if there was none.
    async fn delete(&self, integration_id: &str, subject: Option<&UserId>) -> Result<bool>;
    /// Replace the credential only if its `updated_at` still equals
    /// `expected_updated_at`. `false` means someone else changed it first.
    async fn compare_and_swap(
        &self,
        expected_updated_at: DateTime<Utc>,
        credential: &StoredCredential,
    ) -> Result<bool>;
}
