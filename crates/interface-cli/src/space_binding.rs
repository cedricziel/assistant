//! Which space a messenger interface runs in.
//!
//! A messenger is bound to a space through its interface instance in `org.db`
//! (`interface_instances.space_id`). A messenger with no instance runs in the
//! default space.

use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::{InterfaceInstanceStore as _, OrgStore as _, SpaceStore as _};
use assistant_runtime::{SpaceRef, SpaceRuntimes};
use assistant_storage::OrgStorageLayer;
use assistant_storage::migration::{DEFAULT_ORG_SLUG, DEFAULT_SPACE_SLUG};
use assistant_web_ui::install::DefaultSpace;

/// A space named by both its ids and its slugs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceBinding {
    pub org_id: OrgId,
    pub space_id: SpaceId,
    pub org_slug: String,
    pub space_slug: String,
}

impl SpaceBinding {
    pub fn default_space(default: &DefaultSpace) -> Self {
        Self {
            org_id: default.org_id.clone(),
            space_id: default.space_id.clone(),
            org_slug: DEFAULT_ORG_SLUG.to_string(),
            space_slug: DEFAULT_SPACE_SLUG.to_string(),
        }
    }

    /// The runtime of this space.
    pub async fn space_ref(&self, runtimes: &Arc<SpaceRuntimes>) -> Result<SpaceRef> {
        let runtime = runtimes
            .get(&self.org_slug, &self.space_slug)
            .await
            .with_context(|| format!("opening space {}/{}", self.org_slug, self.space_slug))?;
        Ok(SpaceRef {
            org_id: self.org_id.clone(),
            space_id: self.space_id.clone(),
            runtime,
        })
    }
}

/// The space the `interface_type` messenger runs in.
///
/// When several instances of one type exist the oldest wins, so a later
/// instance cannot silently move a running messenger.
pub async fn messenger_space(
    org_storage: &OrgStorageLayer,
    interface_type: &str,
    default: &DefaultSpace,
) -> Result<SpaceBinding> {
    let mut oldest: Option<(DateTime<Utc>, SpaceBinding)> = None;
    for org in org_storage.org_store().list_orgs().await? {
        for space in org_storage.space_store().list_spaces(&org.id).await? {
            let instances = org_storage
                .interface_instance_store()
                .list_instances(&space.id)
                .await?;
            for instance in instances
                .into_iter()
                .filter(|i| i.interface_type == interface_type)
            {
                if oldest
                    .as_ref()
                    .is_none_or(|(at, _)| instance.created_at < *at)
                {
                    oldest = Some((
                        instance.created_at,
                        SpaceBinding {
                            org_id: org.id.clone(),
                            space_id: space.id.clone(),
                            org_slug: org.slug.clone(),
                            space_slug: space.slug.clone(),
                        },
                    ));
                }
            }
        }
    }
    Ok(oldest.map_or_else(
        || SpaceBinding::default_space(default),
        |(_, binding)| binding,
    ))
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use assistant_core::identity::{OrgId, SpaceId};
    use assistant_core::store::{
        InterfaceInstance, InterfaceInstanceStore as _, OrgStore as _, Organization, Space,
        SpaceStore as _,
    };
    use assistant_storage::OrgStorageLayer;

    use super::*;

    async fn org_with_spaces(spaces: &[&str]) -> OrgStorageLayer {
        let org_storage = OrgStorageLayer::new_in_memory().await.unwrap();
        let epoch = DateTime::UNIX_EPOCH;
        org_storage
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("org_acme"),
                name: "acme".into(),
                slug: "acme".into(),
                auth_mode: "password".into(),
                created_at: epoch,
                updated_at: epoch,
            })
            .await
            .unwrap();
        for slug in spaces {
            org_storage
                .space_store()
                .create_space(&Space {
                    id: SpaceId::from(format!("spc_{slug}")),
                    org_id: OrgId::from("org_acme"),
                    name: (*slug).into(),
                    slug: (*slug).into(),
                    created_at: epoch,
                    updated_at: epoch,
                })
                .await
                .unwrap();
        }
        org_storage
    }

    async fn add_instance(
        org_storage: &OrgStorageLayer,
        id: &str,
        interface_type: &str,
        space: &str,
        created_at: DateTime<Utc>,
    ) {
        org_storage
            .interface_instance_store()
            .create_instance(&InterfaceInstance {
                id: id.into(),
                org_id: OrgId::from("org_acme"),
                space_id: SpaceId::from(format!("spc_{space}")),
                interface_type: interface_type.into(),
                config: "{}".into(),
                created_at,
            })
            .await
            .unwrap();
    }

    fn default() -> DefaultSpace {
        DefaultSpace {
            org_id: OrgId::from("org_default"),
            space_id: SpaceId::from("spc_default"),
        }
    }

    fn slugs(binding: SpaceBinding) -> (String, String) {
        (binding.org_slug, binding.space_slug)
    }

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    #[tokio::test]
    async fn an_unbound_messenger_uses_the_default_space() {
        let org_storage = org_with_spaces(&["alpha"]).await;

        let space = slugs(
            messenger_space(&org_storage, "slack", &default())
                .await
                .unwrap(),
        );

        assert_eq!(space, ("default".to_string(), "default".to_string()));
    }

    #[tokio::test]
    async fn a_messenger_runs_in_the_space_its_instance_is_bound_to() {
        let org_storage = org_with_spaces(&["alpha", "beta"]).await;
        add_instance(&org_storage, "i1", "slack", "beta", at(10)).await;
        add_instance(&org_storage, "i2", "matrix", "alpha", at(10)).await;

        let slack = slugs(
            messenger_space(&org_storage, "slack", &default())
                .await
                .unwrap(),
        );
        let matrix = slugs(
            messenger_space(&org_storage, "matrix", &default())
                .await
                .unwrap(),
        );

        assert_eq!(slack, ("acme".to_string(), "beta".to_string()));
        assert_eq!(matrix, ("acme".to_string(), "alpha".to_string()));
    }

    #[tokio::test]
    async fn the_oldest_instance_decides_when_a_type_is_bound_twice() {
        let org_storage = org_with_spaces(&["alpha", "beta"]).await;
        add_instance(&org_storage, "late", "slack", "alpha", at(20)).await;
        add_instance(&org_storage, "early", "slack", "beta", at(10)).await;

        let space = slugs(
            messenger_space(&org_storage, "slack", &default())
                .await
                .unwrap(),
        );

        assert_eq!(space, ("acme".to_string(), "beta".to_string()));
    }
}
