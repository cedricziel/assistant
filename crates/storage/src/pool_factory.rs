//! Database pool factory for org/space resolution.
//!
//! Resolves org and space slugs to their respective database paths under
//! `~/.assistant/`. `org.db` lives at the install root (next to the
//! post-cutover `assistant.db.legacy` snapshot) while space databases live
//! under `orgs/{slug}/spaces/{space_slug}/space.db`.
//!
//! ## Production callers
//!
//! After the multi-org runtime cutover (`openspec/changes/multi-org-runtime-cutover/`)
//! the runtime path resolution lives in exactly these places:
//!
//! - `crates/interface-cli/src/main.rs` — orchestrator startup. Resolves
//!   the runtime database via `OrgPoolFactory::space_db_path("default",
//!   "default")` unless `config.storage.db_path` is set (deprecated dev
//!   override). Also calls `assistant_web_ui::install::ensure_migrated`
//!   before opening the database.
//! - `crates/interface-cli/src/cmd_migrate.rs` — `assistant migrate
//!   finalize` resolves the same `space.db` path to overwrite it from a
//!   live legacy database during recovery.
//! - `crates/web-ui/src/main.rs` — `assistant webui serve`. Uses the
//!   factory for both `org_db_path` (`OrgStorageLayer::new`) and
//!   `space_db_path("default", "default")` for `StorageLayer::new`.
//!   Like the CLI it honors `--db-path` only as a deprecated dev override.
//! - `crates/storage/src/migration.rs` — the `migrate_database` step of
//!   the legacy-to-multi-org migration writes the initial `space.db` at
//!   the path resolved by these helpers.
//!
//! Tests outside the crate may also call `space_db_path` to seed fixtures
//! against tempdirs; those are not part of the production runtime path.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use tokio::sync::OnceCell;

use crate::StorageLayer;
use crate::org_storage::OrgStorageLayer;

type SpaceKey = (String, String);
type SpaceSlot = Arc<OnceCell<Arc<StorageLayer>>>;

/// Resolves org slugs to [`OrgStorageLayer`] instances.
///
/// The directory layout under `base_dir` is:
/// ```text
/// base_dir/
///   org.db              ← org-level data (organizations, users, spaces, auth)
///   orgs/
///     {slug}/
///       spaces/
///         {space_slug}/
///           space.db    ← space-level data (conversations, personas)
/// ```
pub struct OrgPoolFactory {
    base_dir: PathBuf,
    spaces: Mutex<HashMap<SpaceKey, SpaceSlot>>,
}

impl OrgPoolFactory {
    /// Create a factory rooted at `base_dir` (typically `~/.assistant`).
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            spaces: Mutex::new(HashMap::new()),
        }
    }

    /// Create a factory using the default base directory (`~/.assistant`).
    pub fn default_base() -> Result<Self> {
        let home = dirs::home_dir().context("could not determine home directory")?;
        Ok(Self::new(home.join(".assistant")))
    }

    /// Return the org.db path (at the install root, next to `assistant.db`).
    pub fn org_db_path(&self) -> PathBuf {
        self.base_dir.join("org.db")
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

    /// Open (or create) the [`OrgStorageLayer`].
    ///
    /// Creates the database and runs migrations on first access.
    pub async fn org_storage(&self) -> Result<OrgStorageLayer> {
        let db_path = self.org_db_path();
        OrgStorageLayer::new(&db_path).await
    }

    /// Open (or reuse) the [`StorageLayer`] for one space.
    ///
    /// Pools are cached by `(org_slug, space_slug)`, and concurrent first
    /// calls for the same key open the database once. Both slugs are
    /// validated before they are joined into a path.
    pub async fn space_storage(
        &self,
        org_slug: &str,
        space_slug: &str,
    ) -> Result<Arc<StorageLayer>> {
        validate_slug(org_slug).context("invalid org slug")?;
        validate_slug(space_slug).context("invalid space slug")?;

        let slot = {
            let mut spaces = self
                .spaces
                .lock()
                .map_err(|_| anyhow::anyhow!("space pool cache poisoned"))?;
            spaces
                .entry((org_slug.to_owned(), space_slug.to_owned()))
                .or_default()
                .clone()
        };

        let storage = slot
            .get_or_try_init(|| async {
                let db_path = self.space_db_path(org_slug, space_slug);
                StorageLayer::new(&db_path)
                    .await
                    .map(Arc::new)
                    .with_context(|| format!("opening space.db at {}", db_path.display()))
            })
            .await?;
        Ok(storage.clone())
    }

    /// Forget the cached pool for a space, e.g. after the space is deleted.
    ///
    /// Callers still holding the old `Arc` keep a working pool; the next
    /// [`space_storage`](Self::space_storage) call opens a fresh one.
    pub fn evict_space(&self, org_slug: &str, space_slug: &str) {
        if let Ok(mut spaces) = self.spaces.lock() {
            spaces.remove(&(org_slug.to_owned(), space_slug.to_owned()));
        }
    }

    /// Return the base directory.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }
}

/// Accept only `[A-Za-z0-9_-]+` so a slug cannot add path separators or `..`.
fn validate_slug(slug: &str) -> Result<()> {
    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!("slug {slug:?} must be non-empty and contain only letters, digits, '-' and '_'");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConversationStore as _;
    use assistant_core::store::OrgStore;

    #[test]
    fn org_db_path_resolves_to_root() {
        let factory = OrgPoolFactory::new("/data");
        assert_eq!(factory.org_db_path(), PathBuf::from("/data/org.db"));
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

        let layer = factory.org_storage().await.unwrap();

        // Verify it works by querying.
        let orgs = layer.org_store().list_orgs().await.unwrap();
        assert!(orgs.is_empty());

        // Verify the file was created at the install root.
        assert!(factory.org_db_path().exists());
    }

    #[tokio::test]
    async fn space_storage_returns_one_pool_per_org_and_space() {
        let dir = tempfile::tempdir().unwrap();
        let factory = OrgPoolFactory::new(dir.path());

        let first = factory.space_storage("acme", "eng").await.unwrap();
        let again = factory.space_storage("acme", "eng").await.unwrap();
        assert!(Arc::ptr_eq(&first, &again), "same key must reuse the pool");

        assert!(factory.space_db_path("acme", "eng").exists());
    }

    #[tokio::test]
    async fn space_storage_separates_orgs_that_share_a_space_slug() {
        let dir = tempfile::tempdir().unwrap();
        let factory = OrgPoolFactory::new(dir.path());

        let acme = factory.space_storage("acme", "eng").await.unwrap();
        let globex = factory.space_storage("globex", "eng").await.unwrap();
        assert!(!Arc::ptr_eq(&acme, &globex));

        acme.conversation_store_for_agent("default")
            .create_conversation(Some("acme only"))
            .await
            .unwrap();
        let seen_by_globex = globex
            .conversation_store_for_agent("default")
            .list_conversations()
            .await
            .unwrap();
        assert!(seen_by_globex.is_empty(), "globex must not see acme rows");
    }

    #[tokio::test]
    async fn space_storage_opens_each_space_once_under_concurrency() {
        let dir = tempfile::tempdir().unwrap();
        let factory = Arc::new(OrgPoolFactory::new(dir.path()));

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let factory = factory.clone();
                tokio::spawn(async move { factory.space_storage("acme", "eng").await.unwrap() })
            })
            .collect();
        let mut pools = Vec::new();
        for handle in handles {
            pools.push(handle.await.unwrap());
        }
        assert!(pools.windows(2).all(|w| Arc::ptr_eq(&w[0], &w[1])));
    }

    #[tokio::test]
    async fn space_storage_rejects_slugs_that_escape_the_base_directory() {
        let dir = tempfile::tempdir().unwrap();
        let factory = OrgPoolFactory::new(dir.path());

        for (org, space) in [
            ("..", "eng"),
            ("acme", ".."),
            ("a/b", "eng"),
            ("acme", "e\\ng"),
            ("", "eng"),
            ("acme", ""),
        ] {
            assert!(
                factory.space_storage(org, space).await.is_err(),
                "({org:?}, {space:?}) must be rejected"
            );
        }
    }

    #[tokio::test]
    async fn evict_space_forgets_the_cached_pool() {
        let dir = tempfile::tempdir().unwrap();
        let factory = OrgPoolFactory::new(dir.path());

        let first = factory.space_storage("acme", "eng").await.unwrap();
        factory.evict_space("acme", "eng");
        let second = factory.space_storage("acme", "eng").await.unwrap();
        assert!(!Arc::ptr_eq(&first, &second));
    }
}
