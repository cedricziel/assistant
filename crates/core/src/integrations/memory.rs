//! In-memory [`IntegrationStore`] and [`IntegrationCredentialStore`] for tests.
//!
//! They enforce the same invariants as the SQLite schema: the layer/owner
//! columns, `user` layer implies per-user credentials, slug length and
//! uniqueness per scope, one credential per subject, credentials and space
//! access only for existing integrations, and cascading delete. They do not
//! check foreign keys to orgs, spaces or users.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use anyhow::{Result, bail};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::model::*;
use crate::identity::{OrgId, SpaceId, UserId};

/// State shared by a store and its paired credential store, so credentials
/// can be checked against, and cascade with, their integration.
#[derive(Default)]
struct State {
    integrations: Vec<Integration>,
    access: Vec<SpaceAccess>,
    policies: HashMap<OrgId, IntegrationPolicy>,
    credentials: Vec<StoredCredential>,
}

type Shared = Arc<Mutex<State>>;

fn lock(state: &Shared) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// In-memory [`IntegrationStore`]. Its credentials are reached through
/// [`credential_store`](Self::credential_store).
#[derive(Default)]
pub struct InMemoryIntegrationStore {
    state: Shared,
}

impl InMemoryIntegrationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The credential store over the same integrations.
    pub fn credential_store(&self) -> Arc<InMemoryIntegrationCredentialStore> {
        Arc::new(InMemoryIntegrationCredentialStore {
            state: self.state.clone(),
        })
    }
}

/// Mirrors the table `CHECK` constraints.
fn check_invariants(i: &Integration) -> Result<()> {
    let owners_match = match i.layer {
        IntegrationLayer::Org => i.space_id.is_none() && i.owner_user_id.is_none(),
        IntegrationLayer::Space => i.space_id.is_some() && i.owner_user_id.is_none(),
        IntegrationLayer::User => i.space_id.is_none() && i.owner_user_id.is_some(),
    };
    if !owners_match {
        bail!(
            "integration {}: owner columns do not match layer {}",
            i.id,
            i.layer
        );
    }
    if i.layer == IntegrationLayer::User && i.credential_mode != CredentialMode::PerUser {
        bail!(
            "integration {}: user-layer integrations must be per_user",
            i.id
        );
    }
    if !(1..=MAX_SLUG_LEN).contains(&i.slug.chars().count()) {
        bail!(
            "integration {}: slug must be 1-{MAX_SLUG_LEN} characters",
            i.id
        );
    }
    Ok(())
}

fn same_scope(a: &Integration, b: &Integration) -> bool {
    a.org_id == b.org_id
        && a.layer == b.layer
        && a.space_id == b.space_id
        && a.owner_user_id == b.owner_user_id
}

fn duplicate_slug(slug: &str) -> anyhow::Error {
    IntegrationStoreError::DuplicateSlug { slug: slug.into() }.into()
}

#[async_trait]
impl IntegrationStore for InMemoryIntegrationStore {
    async fn create(&self, integration: &Integration) -> Result<()> {
        check_invariants(integration)?;
        let mut state = lock(&self.state);
        if state.integrations.iter().any(|i| i.id == integration.id) {
            bail!("integration already exists: {}", integration.id);
        }
        if state
            .integrations
            .iter()
            .any(|i| same_scope(i, integration) && i.slug == integration.slug)
        {
            return Err(duplicate_slug(&integration.slug));
        }
        state.integrations.push(integration.clone());
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Integration>> {
        Ok(lock(&self.state)
            .integrations
            .iter()
            .find(|i| i.id == id)
            .cloned())
    }

    async fn list(&self, org_id: &OrgId, filter: &IntegrationFilter) -> Result<Vec<Integration>> {
        let mut rows: Vec<Integration> = lock(&self.state)
            .integrations
            .iter()
            .filter(|i| {
                &i.org_id == org_id
                    && filter.layer.is_none_or(|l| l == i.layer)
                    && filter
                        .space_id
                        .as_ref()
                        .is_none_or(|s| i.space_id.as_ref() == Some(s))
                    && filter
                        .owner_user_id
                        .as_ref()
                        .is_none_or(|u| i.owner_user_id.as_ref() == Some(u))
            })
            .cloned()
            .collect();
        rows.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
        Ok(rows)
    }

    async fn update(&self, integration: &Integration) -> Result<bool> {
        let mut state = lock(&self.state);
        let Some(pos) = state
            .integrations
            .iter()
            .position(|i| i.id == integration.id)
        else {
            return Ok(false);
        };
        let current = &state.integrations[pos];
        let url_changed = current.connect_url != integration.connect_url;
        if url_changed && current.catalog_entry_id.is_some() {
            return Err(IntegrationStoreError::ConnectUrlImmutable {
                id: integration.id.clone(),
            }
            .into());
        }
        let taken = state
            .integrations
            .iter()
            .any(|i| i.id != current.id && same_scope(i, current) && i.slug == integration.slug);
        if taken {
            return Err(duplicate_slug(&integration.slug));
        }
        // Keep the immutable fields of the stored row.
        let mut next = integration.clone();
        next.org_id = current.org_id.clone();
        next.layer = current.layer;
        next.space_id = current.space_id.clone();
        next.owner_user_id = current.owner_user_id.clone();
        next.catalog_entry_id = current.catalog_entry_id.clone();
        next.created_at = current.created_at;
        check_invariants(&next)?;
        state.integrations[pos] = next;
        if url_changed {
            state
                .credentials
                .retain(|c| c.integration_id != integration.id);
        }
        Ok(true)
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        let mut state = lock(&self.state);
        let before = state.integrations.len();
        state.integrations.retain(|i| i.id != id);
        if state.integrations.len() == before {
            return Ok(false);
        }
        state.access.retain(|a| a.integration_id != id);
        state.credentials.retain(|c| c.integration_id != id);
        Ok(true)
    }

    async fn add_space_access(
        &self,
        integration_id: &str,
        space_id: &SpaceId,
        mode: SpaceAccessMode,
    ) -> Result<()> {
        let mut state = lock(&self.state);
        if !state.integrations.iter().any(|i| i.id == integration_id) {
            bail!("unknown integration: {integration_id}");
        }
        let row = SpaceAccess {
            integration_id: integration_id.into(),
            space_id: space_id.clone(),
            mode,
        };
        if !state.access.contains(&row) {
            state.access.push(row);
        }
        Ok(())
    }

    async fn remove_space_access(
        &self,
        integration_id: &str,
        space_id: &SpaceId,
        mode: SpaceAccessMode,
    ) -> Result<bool> {
        let mut state = lock(&self.state);
        let before = state.access.len();
        state.access.retain(|a| {
            !(a.integration_id == integration_id && &a.space_id == space_id && a.mode == mode)
        });
        Ok(state.access.len() < before)
    }

    async fn list_space_access(&self, integration_id: &str) -> Result<Vec<SpaceAccess>> {
        Ok(lock(&self.state)
            .access
            .iter()
            .filter(|a| a.integration_id == integration_id)
            .cloned()
            .collect())
    }

    async fn list_org_space_access(&self, org_id: &OrgId) -> Result<Vec<SpaceAccess>> {
        let state = lock(&self.state);
        Ok(state
            .access
            .iter()
            .filter(|a| {
                state
                    .integrations
                    .iter()
                    .any(|i| i.id == a.integration_id && &i.org_id == org_id)
            })
            .cloned()
            .collect())
    }

    async fn get_policy(&self, org_id: &OrgId) -> Result<IntegrationPolicy> {
        Ok(lock(&self.state)
            .policies
            .get(org_id)
            .cloned()
            .unwrap_or_else(|| IntegrationPolicy::default_for(org_id.clone())))
    }

    async fn put_policy(&self, policy: &IntegrationPolicy) -> Result<()> {
        lock(&self.state)
            .policies
            .insert(policy.org_id.clone(), policy.clone());
        Ok(())
    }
}

/// In-memory [`IntegrationCredentialStore`], obtained from
/// [`InMemoryIntegrationStore::credential_store`].
pub struct InMemoryIntegrationCredentialStore {
    state: Shared,
}

fn is_row(c: &StoredCredential, integration_id: &str, subject: Option<&UserId>) -> bool {
    c.integration_id == integration_id && c.subject_user_id.as_ref() == subject
}

fn find_mut<'a>(
    rows: &'a mut [StoredCredential],
    wanted: &StoredCredential,
) -> Option<&'a mut StoredCredential> {
    let subject = wanted.subject_user_id.as_ref();
    rows.iter_mut()
        .find(|c| is_row(c, &wanted.integration_id, subject))
}

#[async_trait]
impl IntegrationCredentialStore for InMemoryIntegrationCredentialStore {
    async fn put(&self, credential: &StoredCredential) -> Result<()> {
        let mut state = lock(&self.state);
        if !state
            .integrations
            .iter()
            .any(|i| i.id == credential.integration_id)
        {
            bail!("unknown integration: {}", credential.integration_id);
        }
        match find_mut(&mut state.credentials, credential) {
            Some(existing) => *existing = credential.clone(),
            None => state.credentials.push(credential.clone()),
        }
        Ok(())
    }

    async fn get(
        &self,
        integration_id: &str,
        subject: Option<&UserId>,
    ) -> Result<Option<StoredCredential>> {
        Ok(lock(&self.state)
            .credentials
            .iter()
            .find(|c| is_row(c, integration_id, subject))
            .cloned())
    }

    async fn delete(&self, integration_id: &str, subject: Option<&UserId>) -> Result<bool> {
        let mut state = lock(&self.state);
        let before = state.credentials.len();
        state
            .credentials
            .retain(|c| !is_row(c, integration_id, subject));
        Ok(state.credentials.len() < before)
    }

    async fn compare_and_swap(
        &self,
        expected_updated_at: DateTime<Utc>,
        credential: &StoredCredential,
    ) -> Result<bool> {
        let mut state = lock(&self.state);
        match find_mut(&mut state.credentials, credential) {
            Some(existing) if existing.updated_at == expected_updated_at => {
                *existing = credential.clone();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
