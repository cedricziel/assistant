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

/// The longest slug the schema accepts.
pub const MAX_SLUG_LEN: usize = 32;

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
        self.install_denial(integration.layer, integration.catalog_entry_id.as_deref())
            .is_none()
    }

    /// Why this policy forbids installing at `layer`, or `None` when it allows
    /// it. `catalog_entry_id` is `None` for a custom URL.
    pub fn install_denial(
        &self,
        layer: IntegrationLayer,
        catalog_entry_id: Option<&str>,
    ) -> Option<&'static str> {
        if layer == IntegrationLayer::Org {
            return None;
        }
        if layer == IntegrationLayer::User && !self.allow_user_integrations {
            return Some("org policy does not allow personal integrations");
        }
        match catalog_entry_id {
            None if !self.allow_custom_urls => {
                Some("org policy does not allow custom integration URLs")
            }
            None => None,
            Some(_) if self.catalog_mode == CatalogMode::None => {
                Some("org policy does not allow catalog integrations")
            }
            Some(entry)
                if self.catalog_mode == CatalogMode::Allowlist
                    && !self.allowed_catalog_ids.iter().any(|id| id == entry) =>
            {
                Some("org policy does not allow this catalog entry")
            }
            Some(_) => None,
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
    /// A catalog install's `connect_url` cannot change: it would re-point the
    /// stored credentials at a different host.
    ConnectUrlImmutable { id: String },
    /// The integration changed since the caller read it.
    Stale { id: String },
}

impl fmt::Display for IntegrationStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSlug { slug } => {
                write!(f, "integration slug '{slug}' already exists in this scope")
            }
            Self::ConnectUrlImmutable { id } => write!(
                f,
                "integration {id} comes from the catalog; its connect_url cannot change"
            ),
            Self::Stale { id } => {
                write!(
                    f,
                    "integration {id} changed since it was read; reload and retry"
                )
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
    ///
    /// `connect_url` is immutable for catalog installs (fails with
    /// [`IntegrationStoreError::ConnectUrlImmutable`]). Changing it on a custom
    /// URL integration deletes all of its credentials in the same transaction.
    async fn update(&self, integration: &Integration) -> Result<bool>;
    /// [`IntegrationStore::update`], but only if the stored `updated_at` still
    /// equals `expected_updated_at`; otherwise fails with
    /// [`IntegrationStoreError::Stale`] and changes nothing. Use it for
    /// read-modify-write cycles, so a concurrent writer's change is not lost.
    async fn update_if_unchanged(
        &self,
        integration: &Integration,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<bool>;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(f: impl FnOnce(&mut IntegrationPolicy)) -> IntegrationPolicy {
        let mut policy = IntegrationPolicy::default_for(OrgId::from("org_1"));
        f(&mut policy);
        policy
    }

    #[test]
    fn org_layer_installs_are_never_denied() {
        let strict = policy(|p| {
            p.catalog_mode = CatalogMode::None;
            p.allow_user_integrations = false;
        });
        assert_eq!(strict.install_denial(IntegrationLayer::Org, None), None);
        assert_eq!(
            strict.install_denial(IntegrationLayer::Org, Some("curated/x")),
            None
        );
    }

    #[test]
    fn default_policy_allows_catalog_installs_but_not_custom_urls() {
        let default = policy(|_| {});
        assert_eq!(
            default.install_denial(IntegrationLayer::Space, Some("curated/x")),
            None
        );
        assert!(
            default
                .install_denial(IntegrationLayer::Space, None)
                .is_some()
        );
    }

    #[test]
    fn allowlist_denies_entries_outside_it() {
        let allowlist = policy(|p| {
            p.catalog_mode = CatalogMode::Allowlist;
            p.allowed_catalog_ids = vec!["curated/a".into()];
        });
        assert_eq!(
            allowlist.install_denial(IntegrationLayer::User, Some("curated/a")),
            None
        );
        assert!(
            allowlist
                .install_denial(IntegrationLayer::User, Some("curated/b"))
                .is_some()
        );
    }

    #[test]
    fn mode_none_denies_every_catalog_install_below_the_org() {
        let none = policy(|p| p.catalog_mode = CatalogMode::None);
        assert!(
            none.install_denial(IntegrationLayer::Space, Some("curated/a"))
                .is_some()
        );
    }

    #[test]
    fn user_layer_needs_allow_user_integrations() {
        let no_users = policy(|p| p.allow_user_integrations = false);
        assert!(
            no_users
                .install_denial(IntegrationLayer::User, Some("curated/a"))
                .is_some()
        );
        assert_eq!(
            no_users.install_denial(IntegrationLayer::Space, Some("curated/a")),
            None
        );
    }

    #[test]
    fn custom_urls_are_allowed_once_the_policy_says_so() {
        let custom = policy(|p| p.allow_custom_urls = true);
        assert_eq!(custom.install_denial(IntegrationLayer::User, None), None);
    }
}
