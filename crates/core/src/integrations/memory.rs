//! In-memory [`IntegrationStore`] and [`IntegrationCredentialStore`] for tests.
//!
//! They honour the SQLite behaviour that matters to callers: slug uniqueness
//! per scope, one credential per subject, and cascading delete. They do not
//! check foreign keys to orgs, spaces or users.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::model::*;
use crate::identity::{OrgId, SpaceId, UserId};

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// In-memory [`IntegrationCredentialStore`].
#[derive(Default)]
pub struct InMemoryIntegrationCredentialStore {
    rows: Mutex<Vec<StoredCredential>>,
}

impl InMemoryIntegrationCredentialStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn remove_integration(&self, integration_id: &str) {
        lock(&self.rows).retain(|c| c.integration_id != integration_id);
    }
}

fn is_row(c: &StoredCredential, integration_id: &str, subject: Option<&UserId>) -> bool {
    c.integration_id == integration_id && c.subject_user_id.as_ref() == subject
}

#[async_trait]
impl IntegrationCredentialStore for InMemoryIntegrationCredentialStore {
    async fn put(&self, credential: &StoredCredential) -> Result<()> {
        let mut rows = lock(&self.rows);
        let subject = credential.subject_user_id.as_ref();
        match rows
            .iter_mut()
            .find(|c| is_row(c, &credential.integration_id, subject))
        {
            Some(existing) => *existing = credential.clone(),
            None => rows.push(credential.clone()),
        }
        Ok(())
    }

    async fn get(
        &self,
        integration_id: &str,
        subject: Option<&UserId>,
    ) -> Result<Option<StoredCredential>> {
        Ok(lock(&self.rows)
            .iter()
            .find(|c| is_row(c, integration_id, subject))
            .cloned())
    }

    async fn delete(&self, integration_id: &str, subject: Option<&UserId>) -> Result<bool> {
        let mut rows = lock(&self.rows);
        let before = rows.len();
        rows.retain(|c| !is_row(c, integration_id, subject));
        Ok(rows.len() < before)
    }

    async fn compare_and_swap(
        &self,
        expected_updated_at: DateTime<Utc>,
        credential: &StoredCredential,
    ) -> Result<bool> {
        let mut rows = lock(&self.rows);
        let subject = credential.subject_user_id.as_ref();
        match rows
            .iter_mut()
            .find(|c| is_row(c, &credential.integration_id, subject))
        {
            Some(existing) if existing.updated_at == expected_updated_at => {
                *existing = credential.clone();
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[derive(Default)]
struct State {
    integrations: Vec<Integration>,
    access: Vec<SpaceAccess>,
    policies: HashMap<OrgId, IntegrationPolicy>,
}

/// In-memory [`IntegrationStore`]. Deleting an integration also clears the
/// credentials of its paired [`credential_store`](Self::credential_store).
#[derive(Default)]
pub struct InMemoryIntegrationStore {
    state: Mutex<State>,
    credentials: Arc<InMemoryIntegrationCredentialStore>,
}

impl InMemoryIntegrationStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// The credential store paired with this store.
    pub fn credential_store(&self) -> Arc<InMemoryIntegrationCredentialStore> {
        self.credentials.clone()
    }
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
        let mut state = lock(&self.state);
        if state.integrations.iter().any(|i| i.id == integration.id) {
            anyhow::bail!("integration already exists: {}", integration.id);
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
        state.integrations[pos] = next;
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
        drop(state);
        self.credentials.remove_integration(id);
        Ok(true)
    }

    async fn add_space_access(
        &self,
        integration_id: &str,
        space_id: &SpaceId,
        mode: SpaceAccessMode,
    ) -> Result<()> {
        let mut state = lock(&self.state);
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
