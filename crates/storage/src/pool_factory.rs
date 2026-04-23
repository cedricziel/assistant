//! Database pool factory for org/space resolution.
//!
//! Resolves org and space slugs to their respective database paths under
//! `~/.assistant/orgs/{slug}/`. Creates directories on first access.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::org_storage::OrgStorageLayer;

/// Resolves org slugs to [`OrgStorageLayer`] instances.
///
/// The directory layout under `base_dir` is:
/// ```text
/// base_dir/
///   orgs/
///     {slug}/
///       org.db          ← org-level data (organizations, users, spaces, auth)
///       spaces/
///         {space_slug}/
///           space.db    ← space-level data (future: conversations, personas)
/// ```
pub struct OrgPoolFactory {
    base_dir: PathBuf,
}

impl OrgPoolFactory {
    /// Create a factory rooted at `base_dir` (typically `~/.assistant`).
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Create a factory using the default base directory (`~/.assistant`).
    pub fn default_base() -> Result<Self> {
        let home = dirs::home_dir().context("could not determine home directory")?;
        Ok(Self::new(home.join(".assistant")))
    }

    /// Return the org.db path for a given org slug.
    pub fn org_db_path(&self, org_slug: &str) -> PathBuf {
        self.base_dir.join("orgs").join(org_slug).join("org.db")
    }

    /// Return the space.db path for a given org + space slug.
    pub fn space_db_path(&self, org_slug: &str, space_slug: &str) -> PathBuf {
        self.base_dir
            .join("orgs")
            .join(org_slug)
            .join("spaces")
            .join(space_slug)
            .join("space.db")
    }

    /// Open (or create) the [`OrgStorageLayer`] for the given org slug.
    ///
    /// Creates directories and runs migrations on first access.
    pub async fn org_storage(&self, org_slug: &str) -> Result<OrgStorageLayer> {
        let db_path = self.org_db_path(org_slug);
        OrgStorageLayer::new(&db_path).await
    }

    /// Return the base directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assistant_core::store::OrgStore;

    #[test]
    fn org_db_path_resolves() {
        let factory = OrgPoolFactory::new("/data");
        assert_eq!(
            factory.org_db_path("acme"),
            PathBuf::from("/data/orgs/acme/org.db")
        );
    }

    #[test]
    fn space_db_path_resolves() {
        let factory = OrgPoolFactory::new("/data");
        assert_eq!(
            factory.space_db_path("acme", "engineering"),
            PathBuf::from("/data/orgs/acme/spaces/engineering/space.db")
        );
    }

    #[tokio::test]
    async fn org_storage_creates_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let factory = OrgPoolFactory::new(dir.path());

        let layer = factory.org_storage("acme").await.unwrap();

        // Verify it works by querying.
        let orgs = layer.org_store().list_orgs().await.unwrap();
        assert!(orgs.is_empty());

        // Verify the file was created.
        assert!(factory.org_db_path("acme").exists());
    }
}
