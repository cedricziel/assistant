//! Resource catalog abstraction.
//!
//! Organizations maintain a catalog of shared resources (skills, templates,
//! interfaces). Spaces explicitly subscribe to catalog entries to make them
//! available. The [`CatalogResolver`] trait handles resolution:
//! space-local first, then catalog subscriptions, then not found.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::identity::SpaceId;

/// The type of resource in the catalog.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogResourceType {
    Skill,
    Template,
    Interface,
}

impl std::fmt::Display for CatalogResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skill => f.write_str("skill"),
            Self::Template => f.write_str("template"),
            Self::Interface => f.write_str("interface"),
        }
    }
}

/// A reference to a catalog resource.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// The resource identifier (e.g., skill name).
    pub resource_id: String,
    /// The type of resource.
    pub resource_type: CatalogResourceType,
    /// Whether this entry comes from the org catalog (vs. space-local).
    pub from_catalog: bool,
}

/// Resolves resources by checking space-local first, then catalog subscriptions.
///
/// Implemented in the storage layer where both the filesystem (space-local
/// resources) and database (catalog subscriptions) are accessible.
#[async_trait]
pub trait CatalogResolver: Send + Sync {
    /// Resolve a skill by name within a space.
    ///
    /// Resolution order:
    /// 1. Space-local skill directory
    /// 2. Org catalog (if the space has subscribed)
    /// 3. `None` (not available)
    async fn resolve_skill(&self, space: &SpaceId, name: &str) -> Result<Option<CatalogEntry>>;

    /// Resolve a persona template by name within a space.
    async fn resolve_template(&self, space: &SpaceId, name: &str) -> Result<Option<CatalogEntry>>;

    /// List all available resources of the given type for a space,
    /// combining space-local and subscribed catalog entries.
    async fn list_available(
        &self,
        space: &SpaceId,
        resource_type: &CatalogResourceType,
    ) -> Result<Vec<CatalogEntry>>;
}
