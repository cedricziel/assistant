//! First-run install helper shared by the web-ui and orchestrator binaries:
//! seeds the default org and space in a fresh `org.db`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

use assistant_auth::bootstrap::ensure_admin_user;
use assistant_core::clock::{Clock, SystemClock};
use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::{OrgStore as _, Organization, Space, SpaceStore as _};
use assistant_storage::{DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG, OrgStorageLayer};
use uuid::Uuid;

/// The org and space that an install starts with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefaultSpace {
    pub org_id: OrgId,
    pub space_id: SpaceId,
}

/// Return the `default` org and its `default` space, creating whichever is
/// missing. Existing rows are reused; new ones get `org_<uuid>` / `spc_<uuid>` ids.
pub async fn ensure_default_space(org_storage: &OrgStorageLayer) -> Result<DefaultSpace> {
    let now = SystemClock.now();
    let org_store = org_storage.org_store();
    let space_store = org_storage.space_store();

    let org = match org_store.get_org_by_slug(DEFAULT_ORG_SLUG).await? {
        Some(org) => org,
        None => {
            let org = Organization {
                id: OrgId::from(format!("org_{}", Uuid::new_v4())),
                name: "Default Organization".into(),
                slug: DEFAULT_ORG_SLUG.into(),
                auth_mode: "password".into(),
                created_at: now,
                updated_at: now,
            };
            org_store
                .create_org(&org)
                .await
                .context("creating default organization")?;
            org
        }
    };

    let existing = space_store
        .list_spaces(&org.id)
        .await?
        .into_iter()
        .find(|s| s.slug == DEFAULT_SPACE_SLUG);
    let space = match existing {
        Some(space) => space,
        None => {
            let space = Space {
                id: SpaceId::from(format!("spc_{}", Uuid::new_v4())),
                org_id: org.id.clone(),
                name: "Default Space".into(),
                slug: DEFAULT_SPACE_SLUG.into(),
                created_at: now,
                updated_at: now,
            };
            space_store
                .create_space(&space)
                .await
                .context("creating default space")?;
            space
        }
    };

    Ok(DefaultSpace {
        org_id: org.id,
        space_id: space.id,
    })
}

/// File name of the initial admin credentials, kept in the org's directory.
const ADMIN_CREDENTIALS_FILE: &str = "admin_credentials.txt";

/// Create the initial admin of the default org when it has no users yet.
///
/// Only password-mode orgs get one; an OIDC org provisions users on first
/// sign-in. The generated password goes to
/// `{base_dir}/orgs/{slug}/admin_credentials.txt` (mode 0600) and only that
/// path is logged. Returns the file path when an admin was created.
pub async fn ensure_initial_admin(
    org_storage: &OrgStorageLayer,
    default_space: &DefaultSpace,
    base_dir: &Path,
) -> Result<Option<PathBuf>> {
    let org = org_storage
        .org_store()
        .get_org(&default_space.org_id)
        .await
        .context("loading the default organization")?
        .context("the default organization disappeared")?;
    if org.auth_mode != "password" {
        return Ok(None);
    }

    let credentials_file = base_dir
        .join("orgs")
        .join(&org.slug)
        .join(ADMIN_CREDENTIALS_FILE);
    let created = ensure_admin_user(
        &org_storage.user_store(),
        &org_storage.membership_store(),
        &default_space.org_id,
        &default_space.space_id,
        &credentials_file,
        &SystemClock,
    )
    .await
    .context("creating the initial admin user")?;

    Ok(created.map(|admin| {
        info!(
            email = %admin.email,
            credentials_file = %credentials_file.display(),
            "created the initial admin user; its password is in the credentials file"
        );
        credentials_file
    }))
}

#[cfg(test)]
mod tests {
    use assistant_core::store::{MembershipStore as _, UserStore as _};

    use super::*;

    #[tokio::test]
    async fn ensure_default_space_seeds_an_empty_org_db_once() {
        let org_storage = assistant_storage::OrgStorageLayer::new_in_memory()
            .await
            .unwrap();

        let first = ensure_default_space(&org_storage).await.unwrap();
        let again = ensure_default_space(&org_storage).await.unwrap();

        assert_eq!(first, again, "a second call must reuse the seeded rows");
        assert!(first.org_id.0.starts_with("org_"), "{:?}", first.org_id);
        assert!(first.space_id.0.starts_with("spc_"), "{:?}", first.space_id);
        let org = org_storage
            .org_store()
            .get_org_by_slug("default")
            .await
            .unwrap()
            .expect("default org");
        assert_eq!(org.id, first.org_id);
        let spaces = org_storage
            .space_store()
            .list_spaces(&org.id)
            .await
            .unwrap();
        assert_eq!(spaces.len(), 1);
        assert_eq!(
            (spaces[0].slug.as_str(), &spaces[0].id),
            ("default", &first.space_id)
        );
    }

    async fn seeded() -> (OrgStorageLayer, DefaultSpace) {
        let org_storage = OrgStorageLayer::new_in_memory().await.unwrap();
        let default_space = ensure_default_space(&org_storage).await.unwrap();
        (org_storage, default_space)
    }

    #[tokio::test]
    async fn first_start_creates_exactly_one_admin_and_the_credentials_file() {
        let dir = tempfile::tempdir().unwrap();
        let (org_storage, default_space) = seeded().await;

        let file = ensure_initial_admin(&org_storage, &default_space, dir.path())
            .await
            .unwrap()
            .expect("a fresh install gets an admin");

        assert_eq!(file, dir.path().join("orgs/default/admin_credentials.txt"));
        assert!(file.is_file());
        let users = org_storage
            .user_store()
            .list_users(&default_space.org_id)
            .await
            .unwrap();
        assert_eq!(users.len(), 1);
        let roles = org_storage
            .membership_store()
            .get_space_roles(&users[0].id)
            .await
            .unwrap();
        assert_eq!(
            roles.get(&default_space.space_id),
            Some(&assistant_core::identity::Role::OrgAdmin)
        );
    }

    #[tokio::test]
    async fn second_start_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (org_storage, default_space) = seeded().await;
        let file = ensure_initial_admin(&org_storage, &default_space, dir.path())
            .await
            .unwrap()
            .unwrap();
        let before = std::fs::read_to_string(&file).unwrap();

        let again = ensure_initial_admin(&org_storage, &default_space, dir.path())
            .await
            .unwrap();

        assert_eq!(again, None);
        let users = org_storage
            .user_store()
            .list_users(&default_space.org_id)
            .await
            .unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    }

    #[tokio::test]
    async fn oidc_orgs_get_no_admin() {
        let dir = tempfile::tempdir().unwrap();
        let (org_storage, default_space) = seeded().await;
        let org_store = org_storage.org_store();
        let mut org = org_store
            .get_org(&default_space.org_id)
            .await
            .unwrap()
            .unwrap();
        org.auth_mode = "oidc".into();
        org_store.update_org(&org).await.unwrap();

        let created = ensure_initial_admin(&org_storage, &default_space, dir.path())
            .await
            .unwrap();

        assert_eq!(created, None);
        assert!(
            org_storage
                .user_store()
                .list_users(&default_space.org_id)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
