//! First-run install helper shared by the web-ui and orchestrator binaries:
//! seeds the default org and space in a fresh `org.db`.

use anyhow::{Context, Result};

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

#[cfg(test)]
mod tests {
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
}
