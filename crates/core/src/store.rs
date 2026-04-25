//! Multi-tenant store traits and domain types.
//!
//! Defines backend-agnostic storage interfaces for organizations, users,
//! spaces, and memberships. Implementations live in the `assistant-storage`
//! crate (SQLite) or as in-memory stores for testing.

use std::collections::HashMap;
use std::sync::Mutex;

use anyhow::{Result, bail};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::catalog::CatalogResourceType;
use crate::identity::{OrgId, Role, SpaceId, UserId};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// An organization — the top-level multi-tenant namespace.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrgId,
    pub name: String,
    pub slug: String,
    /// `"password"` or `"oidc"`.
    pub auth_mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A user account within an organization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub org_id: OrgId,
    pub email: String,
    pub name: String,
    /// Argon2id hash. Empty for OIDC-only users.
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// OIDC identity provider issuer URL (if federated).
    pub idp_issuer: Option<String>,
    /// OIDC subject claim (unique ID at the IdP).
    pub idp_subject: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A space — a workspace within an organization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Space {
    pub id: SpaceId,
    pub org_id: OrgId,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A user's membership in a space.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpaceMembership {
    pub user_id: UserId,
    pub space_id: SpaceId,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

/// A resource published to the org-level catalog.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogItem {
    pub id: String,
    pub org_id: OrgId,
    pub resource_type: CatalogResourceType,
    /// Name/slug of the resource (e.g. skill name, template name).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// A space's subscription to a catalog item.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogSubscription {
    pub id: String,
    pub space_id: SpaceId,
    pub catalog_item_id: String,
    pub created_at: DateTime<Utc>,
}

/// A configured interface instance within a space.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InterfaceInstance {
    pub id: String,
    pub org_id: OrgId,
    pub space_id: SpaceId,
    /// Interface type: `"slack"`, `"matrix"`, `"mattermost"`, etc.
    pub interface_type: String,
    /// JSON-encoded configuration (tokens, channels, etc.).
    pub config: String,
    pub created_at: DateTime<Utc>,
}

/// Binds a persona to an interface instance so the persona is reachable
/// through that interface.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonaBinding {
    pub id: String,
    pub space_id: SpaceId,
    pub persona_id: String,
    pub interface_instance_id: String,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Store traits
// ---------------------------------------------------------------------------

/// Organization CRUD operations.
#[async_trait]
pub trait OrgStore: Send + Sync {
    async fn create_org(&self, org: &Organization) -> Result<()>;
    async fn get_org(&self, id: &OrgId) -> Result<Option<Organization>>;
    async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Organization>>;
    async fn update_org(&self, org: &Organization) -> Result<()>;
    async fn list_orgs(&self) -> Result<Vec<Organization>>;
}

/// User CRUD operations.
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn create_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, id: &UserId) -> Result<Option<User>>;
    async fn get_user_by_email(&self, org_id: &OrgId, email: &str) -> Result<Option<User>>;
    async fn get_user_by_idp(&self, issuer: &str, subject: &str) -> Result<Option<User>>;
    async fn list_users(&self, org_id: &OrgId) -> Result<Vec<User>>;
    async fn update_user(&self, user: &User) -> Result<()>;
    async fn delete_user(&self, id: &UserId) -> Result<bool>;
}

/// Space CRUD operations.
#[async_trait]
pub trait SpaceStore: Send + Sync {
    async fn create_space(&self, space: &Space) -> Result<()>;
    async fn get_space(&self, id: &SpaceId) -> Result<Option<Space>>;
    async fn list_spaces(&self, org_id: &OrgId) -> Result<Vec<Space>>;
    async fn update_space(&self, space: &Space) -> Result<()>;
    async fn delete_space(&self, id: &SpaceId) -> Result<bool>;
}

/// Space membership operations.
#[async_trait]
pub trait MembershipStore: Send + Sync {
    async fn add_membership(&self, membership: &SpaceMembership) -> Result<()>;
    async fn remove_membership(&self, user_id: &UserId, space_id: &SpaceId) -> Result<bool>;
    async fn get_memberships_for_user(&self, user_id: &UserId) -> Result<Vec<SpaceMembership>>;
    async fn get_members_of_space(&self, space_id: &SpaceId) -> Result<Vec<SpaceMembership>>;
    /// Build a `HashMap<SpaceId, Role>` for use in [`AuthContext`](crate::auth::AuthContext).
    async fn get_space_roles(&self, user_id: &UserId) -> Result<HashMap<SpaceId, Role>> {
        let memberships = self.get_memberships_for_user(user_id).await?;
        Ok(memberships
            .into_iter()
            .map(|m| (m.space_id, m.role))
            .collect())
    }
}

/// Org-level catalog item CRUD.
#[async_trait]
pub trait CatalogItemStore: Send + Sync {
    async fn create_item(&self, item: &CatalogItem) -> Result<()>;
    async fn get_item(&self, id: &str) -> Result<Option<CatalogItem>>;
    async fn list_items(
        &self,
        org_id: &OrgId,
        resource_type: Option<&CatalogResourceType>,
    ) -> Result<Vec<CatalogItem>>;
    async fn delete_item(&self, id: &str) -> Result<bool>;
}

/// Space-level catalog subscriptions.
#[async_trait]
pub trait CatalogSubscriptionStore: Send + Sync {
    async fn create_subscription(&self, sub: &CatalogSubscription) -> Result<()>;
    async fn get_subscription(&self, id: &str) -> Result<Option<CatalogSubscription>>;
    async fn list_subscriptions(&self, space_id: &SpaceId) -> Result<Vec<CatalogSubscription>>;
    async fn delete_subscription(&self, id: &str) -> Result<bool>;
}

/// Per-space interface instance management.
#[async_trait]
pub trait InterfaceInstanceStore: Send + Sync {
    async fn create_instance(&self, instance: &InterfaceInstance) -> Result<()>;
    async fn get_instance(&self, id: &str) -> Result<Option<InterfaceInstance>>;
    async fn list_instances(&self, space_id: &SpaceId) -> Result<Vec<InterfaceInstance>>;
    async fn delete_instance(&self, id: &str) -> Result<bool>;
}

/// Persona ↔ interface instance bindings.
#[async_trait]
pub trait BindingStore: Send + Sync {
    async fn create_binding(&self, binding: &PersonaBinding) -> Result<()>;
    async fn get_binding(&self, id: &str) -> Result<Option<PersonaBinding>>;
    async fn list_bindings(&self, space_id: &SpaceId) -> Result<Vec<PersonaBinding>>;
    async fn delete_binding(&self, id: &str) -> Result<bool>;
}

// ---------------------------------------------------------------------------
// In-memory implementations (for tests and single-process deployments)
// ---------------------------------------------------------------------------

/// In-memory organization store.
#[derive(Default)]
pub struct InMemoryOrgStore {
    orgs: Mutex<Vec<Organization>>,
}

#[async_trait]
impl OrgStore for InMemoryOrgStore {
    async fn create_org(&self, org: &Organization) -> Result<()> {
        let mut orgs = self.orgs.lock().unwrap();
        if orgs.iter().any(|o| o.id == org.id) {
            bail!("organization already exists: {}", org.id);
        }
        if orgs.iter().any(|o| o.slug == org.slug) {
            bail!("organization slug already taken: {}", org.slug);
        }
        orgs.push(org.clone());
        Ok(())
    }

    async fn get_org(&self, id: &OrgId) -> Result<Option<Organization>> {
        Ok(self
            .orgs
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.id == *id)
            .cloned())
    }

    async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Organization>> {
        Ok(self
            .orgs
            .lock()
            .unwrap()
            .iter()
            .find(|o| o.slug == slug)
            .cloned())
    }

    async fn update_org(&self, org: &Organization) -> Result<()> {
        let mut orgs = self.orgs.lock().unwrap();
        if let Some(existing) = orgs.iter_mut().find(|o| o.id == org.id) {
            *existing = org.clone();
            Ok(())
        } else {
            bail!("organization not found: {}", org.id)
        }
    }

    async fn list_orgs(&self) -> Result<Vec<Organization>> {
        Ok(self.orgs.lock().unwrap().clone())
    }
}

/// In-memory user store.
#[derive(Default)]
pub struct InMemoryUserStore {
    users: Mutex<Vec<User>>,
}

#[async_trait]
impl UserStore for InMemoryUserStore {
    async fn create_user(&self, user: &User) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        if users.iter().any(|u| u.id == user.id) {
            bail!("user already exists: {}", user.id);
        }
        if users
            .iter()
            .any(|u| u.org_id == user.org_id && u.email == user.email)
        {
            bail!("email already registered in org: {}", user.email);
        }
        users.push(user.clone());
        Ok(())
    }

    async fn get_user(&self, id: &UserId) -> Result<Option<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == *id)
            .cloned())
    }

    async fn get_user_by_email(&self, org_id: &OrgId, email: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.org_id == *org_id && u.email == email)
            .cloned())
    }

    async fn get_user_by_idp(&self, issuer: &str, subject: &str) -> Result<Option<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .find(|u| {
                u.idp_issuer.as_deref() == Some(issuer) && u.idp_subject.as_deref() == Some(subject)
            })
            .cloned())
    }

    async fn list_users(&self, org_id: &OrgId) -> Result<Vec<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .iter()
            .filter(|u| u.org_id == *org_id)
            .cloned()
            .collect())
    }

    async fn update_user(&self, user: &User) -> Result<()> {
        let mut users = self.users.lock().unwrap();
        if let Some(existing) = users.iter_mut().find(|u| u.id == user.id) {
            *existing = user.clone();
            Ok(())
        } else {
            bail!("user not found: {}", user.id)
        }
    }

    async fn delete_user(&self, id: &UserId) -> Result<bool> {
        let mut users = self.users.lock().unwrap();
        let len_before = users.len();
        users.retain(|u| u.id != *id);
        Ok(users.len() < len_before)
    }
}

/// In-memory space store.
#[derive(Default)]
pub struct InMemorySpaceStore {
    spaces: Mutex<Vec<Space>>,
}

#[async_trait]
impl SpaceStore for InMemorySpaceStore {
    async fn create_space(&self, space: &Space) -> Result<()> {
        let mut spaces = self.spaces.lock().unwrap();
        if spaces.iter().any(|s| s.id == space.id) {
            bail!("space already exists: {}", space.id);
        }
        spaces.push(space.clone());
        Ok(())
    }

    async fn get_space(&self, id: &SpaceId) -> Result<Option<Space>> {
        Ok(self
            .spaces
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id == *id)
            .cloned())
    }

    async fn list_spaces(&self, org_id: &OrgId) -> Result<Vec<Space>> {
        Ok(self
            .spaces
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.org_id == *org_id)
            .cloned()
            .collect())
    }

    async fn update_space(&self, space: &Space) -> Result<()> {
        let mut spaces = self.spaces.lock().unwrap();
        if let Some(existing) = spaces.iter_mut().find(|s| s.id == space.id) {
            *existing = space.clone();
            Ok(())
        } else {
            bail!("space not found: {}", space.id)
        }
    }

    async fn delete_space(&self, id: &SpaceId) -> Result<bool> {
        let mut spaces = self.spaces.lock().unwrap();
        let len_before = spaces.len();
        spaces.retain(|s| s.id != *id);
        Ok(spaces.len() < len_before)
    }
}

/// In-memory membership store.
#[derive(Default)]
pub struct InMemoryMembershipStore {
    memberships: Mutex<Vec<SpaceMembership>>,
}

#[async_trait]
impl MembershipStore for InMemoryMembershipStore {
    async fn add_membership(&self, membership: &SpaceMembership) -> Result<()> {
        let mut memberships = self.memberships.lock().unwrap();
        if memberships
            .iter()
            .any(|m| m.user_id == membership.user_id && m.space_id == membership.space_id)
        {
            bail!(
                "membership already exists: user {} in space {}",
                membership.user_id,
                membership.space_id
            );
        }
        memberships.push(membership.clone());
        Ok(())
    }

    async fn remove_membership(&self, user_id: &UserId, space_id: &SpaceId) -> Result<bool> {
        let mut memberships = self.memberships.lock().unwrap();
        let len_before = memberships.len();
        memberships.retain(|m| !(m.user_id == *user_id && m.space_id == *space_id));
        Ok(memberships.len() < len_before)
    }

    async fn get_memberships_for_user(&self, user_id: &UserId) -> Result<Vec<SpaceMembership>> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.user_id == *user_id)
            .cloned()
            .collect())
    }

    async fn get_members_of_space(&self, space_id: &SpaceId) -> Result<Vec<SpaceMembership>> {
        Ok(self
            .memberships
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.space_id == *space_id)
            .cloned()
            .collect())
    }
}

/// In-memory catalog item store.
#[derive(Default)]
pub struct InMemoryCatalogItemStore {
    items: Mutex<Vec<CatalogItem>>,
}

#[async_trait]
impl CatalogItemStore for InMemoryCatalogItemStore {
    async fn create_item(&self, item: &CatalogItem) -> Result<()> {
        let mut items = self.items.lock().unwrap();
        if items.iter().any(|i| i.id == item.id) {
            bail!("catalog item already exists: {}", item.id);
        }
        items.push(item.clone());
        Ok(())
    }

    async fn get_item(&self, id: &str) -> Result<Option<CatalogItem>> {
        Ok(self
            .items
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == id)
            .cloned())
    }

    async fn list_items(
        &self,
        org_id: &OrgId,
        resource_type: Option<&CatalogResourceType>,
    ) -> Result<Vec<CatalogItem>> {
        Ok(self
            .items
            .lock()
            .unwrap()
            .iter()
            .filter(|i| {
                i.org_id == *org_id && resource_type.is_none_or(|rt| i.resource_type == *rt)
            })
            .cloned()
            .collect())
    }

    async fn delete_item(&self, id: &str) -> Result<bool> {
        let mut items = self.items.lock().unwrap();
        let len_before = items.len();
        items.retain(|i| i.id != id);
        Ok(items.len() < len_before)
    }
}

/// In-memory catalog subscription store.
#[derive(Default)]
pub struct InMemoryCatalogSubscriptionStore {
    subs: Mutex<Vec<CatalogSubscription>>,
}

#[async_trait]
impl CatalogSubscriptionStore for InMemoryCatalogSubscriptionStore {
    async fn create_subscription(&self, sub: &CatalogSubscription) -> Result<()> {
        let mut subs = self.subs.lock().unwrap();
        if subs.iter().any(|s| s.id == sub.id) {
            bail!("subscription already exists: {}", sub.id);
        }
        if subs
            .iter()
            .any(|s| s.space_id == sub.space_id && s.catalog_item_id == sub.catalog_item_id)
        {
            bail!(
                "duplicate subscription: space={} item={}",
                sub.space_id.0,
                sub.catalog_item_id
            );
        }
        subs.push(sub.clone());
        Ok(())
    }

    async fn get_subscription(&self, id: &str) -> Result<Option<CatalogSubscription>> {
        Ok(self
            .subs
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .cloned())
    }

    async fn list_subscriptions(&self, space_id: &SpaceId) -> Result<Vec<CatalogSubscription>> {
        Ok(self
            .subs
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.space_id == *space_id)
            .cloned()
            .collect())
    }

    async fn delete_subscription(&self, id: &str) -> Result<bool> {
        let mut subs = self.subs.lock().unwrap();
        let len_before = subs.len();
        subs.retain(|s| s.id != id);
        Ok(subs.len() < len_before)
    }
}

/// In-memory interface instance store.
#[derive(Default)]
pub struct InMemoryInterfaceInstanceStore {
    instances: Mutex<Vec<InterfaceInstance>>,
}

#[async_trait]
impl InterfaceInstanceStore for InMemoryInterfaceInstanceStore {
    async fn create_instance(&self, instance: &InterfaceInstance) -> Result<()> {
        let mut instances = self.instances.lock().unwrap();
        if instances.iter().any(|i| i.id == instance.id) {
            bail!("interface instance already exists: {}", instance.id);
        }
        instances.push(instance.clone());
        Ok(())
    }

    async fn get_instance(&self, id: &str) -> Result<Option<InterfaceInstance>> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == id)
            .cloned())
    }

    async fn list_instances(&self, space_id: &SpaceId) -> Result<Vec<InterfaceInstance>> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.space_id == *space_id)
            .cloned()
            .collect())
    }

    async fn delete_instance(&self, id: &str) -> Result<bool> {
        let mut instances = self.instances.lock().unwrap();
        let len_before = instances.len();
        instances.retain(|i| i.id != id);
        Ok(instances.len() < len_before)
    }
}

/// In-memory persona-interface binding store.
#[derive(Default)]
pub struct InMemoryBindingStore {
    bindings: Mutex<Vec<PersonaBinding>>,
}

#[async_trait]
impl BindingStore for InMemoryBindingStore {
    async fn create_binding(&self, binding: &PersonaBinding) -> Result<()> {
        let mut bindings = self.bindings.lock().unwrap();
        if bindings.iter().any(|b| b.id == binding.id) {
            bail!("binding already exists: {}", binding.id);
        }
        if bindings.iter().any(|b| {
            b.persona_id == binding.persona_id
                && b.interface_instance_id == binding.interface_instance_id
        }) {
            bail!(
                "binding already exists for persona {} on interface {}",
                binding.persona_id,
                binding.interface_instance_id
            );
        }
        bindings.push(binding.clone());
        Ok(())
    }

    async fn get_binding(&self, id: &str) -> Result<Option<PersonaBinding>> {
        Ok(self
            .bindings
            .lock()
            .unwrap()
            .iter()
            .find(|b| b.id == id)
            .cloned())
    }

    async fn list_bindings(&self, space_id: &SpaceId) -> Result<Vec<PersonaBinding>> {
        Ok(self
            .bindings
            .lock()
            .unwrap()
            .iter()
            .filter(|b| b.space_id == *space_id)
            .cloned()
            .collect())
    }

    async fn delete_binding(&self, id: &str) -> Result<bool> {
        let mut bindings = self.bindings.lock().unwrap();
        let len_before = bindings.len();
        bindings.retain(|b| b.id != id);
        Ok(bindings.len() < len_before)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    fn make_org(id: &str, slug: &str) -> Organization {
        Organization {
            id: OrgId::from(id),
            name: format!("{slug} Inc."),
            slug: slug.into(),
            auth_mode: "password".into(),
            created_at: now(),
            updated_at: now(),
        }
    }

    fn make_user(id: &str, org_id: &str, email: &str) -> User {
        User {
            id: UserId::from(id),
            org_id: OrgId::from(org_id),
            email: email.into(),
            name: id.into(),
            password_hash: String::new(),
            idp_issuer: None,
            idp_subject: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn make_space(id: &str, org_id: &str, slug: &str) -> Space {
        Space {
            id: SpaceId::from(id),
            org_id: OrgId::from(org_id),
            name: format!("{slug} space"),
            slug: slug.into(),
            created_at: now(),
            updated_at: now(),
        }
    }

    fn make_membership(user_id: &str, space_id: &str, role: Role) -> SpaceMembership {
        SpaceMembership {
            user_id: UserId::from(user_id),
            space_id: SpaceId::from(space_id),
            role,
            created_at: now(),
        }
    }

    // -- OrgStore --

    #[tokio::test]
    async fn org_create_and_get() {
        let store = InMemoryOrgStore::default();
        let org = make_org("org_1", "acme");
        store.create_org(&org).await.unwrap();

        let found = store.get_org(&OrgId::from("org_1")).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().slug, "acme");
    }

    #[tokio::test]
    async fn org_get_by_slug() {
        let store = InMemoryOrgStore::default();
        store.create_org(&make_org("org_1", "acme")).await.unwrap();

        let found = store.get_org_by_slug("acme").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, OrgId::from("org_1"));

        let missing = store.get_org_by_slug("nope").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn org_duplicate_id_rejected() {
        let store = InMemoryOrgStore::default();
        store.create_org(&make_org("org_1", "acme")).await.unwrap();
        let err = store.create_org(&make_org("org_1", "other")).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn org_duplicate_slug_rejected() {
        let store = InMemoryOrgStore::default();
        store.create_org(&make_org("org_1", "acme")).await.unwrap();
        let err = store.create_org(&make_org("org_2", "acme")).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn org_update() {
        let store = InMemoryOrgStore::default();
        let mut org = make_org("org_1", "acme");
        store.create_org(&org).await.unwrap();

        org.name = "Acme Corp".into();
        store.update_org(&org).await.unwrap();

        let found = store.get_org(&OrgId::from("org_1")).await.unwrap().unwrap();
        assert_eq!(found.name, "Acme Corp");
    }

    #[tokio::test]
    async fn org_list() {
        let store = InMemoryOrgStore::default();
        store.create_org(&make_org("org_1", "acme")).await.unwrap();
        store
            .create_org(&make_org("org_2", "globex"))
            .await
            .unwrap();

        let all = store.list_orgs().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    // -- UserStore --

    #[tokio::test]
    async fn user_create_and_get() {
        let store = InMemoryUserStore::default();
        let user = make_user("usr_alice", "org_1", "alice@acme.com");
        store.create_user(&user).await.unwrap();

        let found = store.get_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "alice@acme.com");
    }

    #[tokio::test]
    async fn user_get_by_email() {
        let store = InMemoryUserStore::default();
        store
            .create_user(&make_user("usr_alice", "org_1", "alice@acme.com"))
            .await
            .unwrap();

        let found = store
            .get_user_by_email(&OrgId::from("org_1"), "alice@acme.com")
            .await
            .unwrap();
        assert!(found.is_some());

        // Different org, same email → not found.
        let not_found = store
            .get_user_by_email(&OrgId::from("org_2"), "alice@acme.com")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn user_get_by_idp() {
        let store = InMemoryUserStore::default();
        let mut user = make_user("usr_alice", "org_1", "alice@acme.com");
        user.idp_issuer = Some("https://idp.acme.com".into());
        user.idp_subject = Some("sub_12345".into());
        store.create_user(&user).await.unwrap();

        let found = store
            .get_user_by_idp("https://idp.acme.com", "sub_12345")
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, UserId::from("usr_alice"));

        let not_found = store
            .get_user_by_idp("https://idp.acme.com", "sub_99999")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn user_duplicate_email_in_same_org_rejected() {
        let store = InMemoryUserStore::default();
        store
            .create_user(&make_user("usr_1", "org_1", "alice@acme.com"))
            .await
            .unwrap();
        let err = store
            .create_user(&make_user("usr_2", "org_1", "alice@acme.com"))
            .await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn user_same_email_different_orgs_allowed() {
        let store = InMemoryUserStore::default();
        store
            .create_user(&make_user("usr_1", "org_1", "alice@acme.com"))
            .await
            .unwrap();
        store
            .create_user(&make_user("usr_2", "org_2", "alice@acme.com"))
            .await
            .unwrap();

        let org1_users = store.list_users(&OrgId::from("org_1")).await.unwrap();
        assert_eq!(org1_users.len(), 1);
    }

    #[tokio::test]
    async fn user_update() {
        let store = InMemoryUserStore::default();
        let mut user = make_user("usr_alice", "org_1", "alice@acme.com");
        store.create_user(&user).await.unwrap();

        user.name = "Alice Smith".into();
        store.update_user(&user).await.unwrap();

        let found = store
            .get_user(&UserId::from("usr_alice"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Alice Smith");
    }

    #[tokio::test]
    async fn user_delete() {
        let store = InMemoryUserStore::default();
        store
            .create_user(&make_user("usr_alice", "org_1", "alice@acme.com"))
            .await
            .unwrap();

        let deleted = store.delete_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(deleted);

        let found = store.get_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(found.is_none());

        // Deleting again returns false.
        let not_deleted = store.delete_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(!not_deleted);
    }

    #[tokio::test]
    async fn user_list_by_org() {
        let store = InMemoryUserStore::default();
        store
            .create_user(&make_user("usr_1", "org_1", "alice@acme.com"))
            .await
            .unwrap();
        store
            .create_user(&make_user("usr_2", "org_1", "bob@acme.com"))
            .await
            .unwrap();
        store
            .create_user(&make_user("usr_3", "org_2", "carol@globex.com"))
            .await
            .unwrap();

        let org1 = store.list_users(&OrgId::from("org_1")).await.unwrap();
        assert_eq!(org1.len(), 2);
        let org2 = store.list_users(&OrgId::from("org_2")).await.unwrap();
        assert_eq!(org2.len(), 1);
    }

    // -- SpaceStore --

    #[tokio::test]
    async fn space_create_and_get() {
        let store = InMemorySpaceStore::default();
        let space = make_space("sp_eng", "org_1", "engineering");
        store.create_space(&space).await.unwrap();

        let found = store.get_space(&SpaceId::from("sp_eng")).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().slug, "engineering");
    }

    #[tokio::test]
    async fn space_duplicate_id_rejected() {
        let store = InMemorySpaceStore::default();
        store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();
        let err = store
            .create_space(&make_space("sp_eng", "org_1", "other"))
            .await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn space_list_by_org() {
        let store = InMemorySpaceStore::default();
        store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();
        store
            .create_space(&make_space("sp_mktg", "org_1", "marketing"))
            .await
            .unwrap();
        store
            .create_space(&make_space("sp_dev", "org_2", "dev"))
            .await
            .unwrap();

        let org1 = store.list_spaces(&OrgId::from("org_1")).await.unwrap();
        assert_eq!(org1.len(), 2);
        let org2 = store.list_spaces(&OrgId::from("org_2")).await.unwrap();
        assert_eq!(org2.len(), 1);
    }

    #[tokio::test]
    async fn space_delete() {
        let store = InMemorySpaceStore::default();
        store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();

        let deleted = store.delete_space(&SpaceId::from("sp_eng")).await.unwrap();
        assert!(deleted);

        let found = store.get_space(&SpaceId::from("sp_eng")).await.unwrap();
        assert!(found.is_none());
    }

    // -- MembershipStore --

    #[tokio::test]
    async fn membership_add_and_query() {
        let store = InMemoryMembershipStore::default();
        store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();
        store
            .add_membership(&make_membership("usr_alice", "sp_mktg", Role::Viewer))
            .await
            .unwrap();

        let alice = store
            .get_memberships_for_user(&UserId::from("usr_alice"))
            .await
            .unwrap();
        assert_eq!(alice.len(), 2);

        let eng = store
            .get_members_of_space(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(eng.len(), 1);
        assert_eq!(eng[0].role, Role::Member);
    }

    #[tokio::test]
    async fn membership_duplicate_rejected() {
        let store = InMemoryMembershipStore::default();
        store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();
        let err = store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Viewer))
            .await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn membership_remove() {
        let store = InMemoryMembershipStore::default();
        store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();

        let removed = store
            .remove_membership(&UserId::from("usr_alice"), &SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert!(removed);

        let alice = store
            .get_memberships_for_user(&UserId::from("usr_alice"))
            .await
            .unwrap();
        assert!(alice.is_empty());
    }

    #[tokio::test]
    async fn membership_space_roles_helper() {
        let store = InMemoryMembershipStore::default();
        store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();
        store
            .add_membership(&make_membership("usr_alice", "sp_mktg", Role::SpaceAdmin))
            .await
            .unwrap();

        let roles = store
            .get_space_roles(&UserId::from("usr_alice"))
            .await
            .unwrap();
        assert_eq!(roles.get(&SpaceId::from("sp_eng")), Some(&Role::Member));
        assert_eq!(
            roles.get(&SpaceId::from("sp_mktg")),
            Some(&Role::SpaceAdmin)
        );
    }

    #[tokio::test]
    async fn membership_multiple_users_in_space() {
        let store = InMemoryMembershipStore::default();
        store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::SpaceAdmin))
            .await
            .unwrap();
        store
            .add_membership(&make_membership("usr_bob", "sp_eng", Role::Member))
            .await
            .unwrap();
        store
            .add_membership(&make_membership("usr_carol", "sp_eng", Role::Viewer))
            .await
            .unwrap();

        let members = store
            .get_members_of_space(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(members.len(), 3);
    }

    // -- CatalogItemStore --

    fn make_catalog_item(
        id: &str,
        org_id: &str,
        rt: CatalogResourceType,
        name: &str,
    ) -> CatalogItem {
        CatalogItem {
            id: id.into(),
            org_id: OrgId::from(org_id),
            resource_type: rt,
            name: name.into(),
            description: format!("{name} description"),
            created_at: now(),
        }
    }

    #[tokio::test]
    async fn catalog_item_create_and_list() {
        let store = InMemoryCatalogItemStore::default();
        store
            .create_item(&make_catalog_item(
                "ci_1",
                "org_1",
                CatalogResourceType::Skill,
                "web-fetch",
            ))
            .await
            .unwrap();
        store
            .create_item(&make_catalog_item(
                "ci_2",
                "org_1",
                CatalogResourceType::Template,
                "chatbot",
            ))
            .await
            .unwrap();

        let all = store.list_items(&OrgId::from("org_1"), None).await.unwrap();
        assert_eq!(all.len(), 2);

        let skills = store
            .list_items(&OrgId::from("org_1"), Some(&CatalogResourceType::Skill))
            .await
            .unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "web-fetch");
    }

    #[tokio::test]
    async fn catalog_item_delete() {
        let store = InMemoryCatalogItemStore::default();
        store
            .create_item(&make_catalog_item(
                "ci_1",
                "org_1",
                CatalogResourceType::Skill,
                "web-fetch",
            ))
            .await
            .unwrap();

        assert!(store.delete_item("ci_1").await.unwrap());
        assert!(!store.delete_item("ci_1").await.unwrap());
    }

    // -- CatalogSubscriptionStore --

    #[tokio::test]
    async fn subscription_create_and_list() {
        let store = InMemoryCatalogSubscriptionStore::default();
        let sub = CatalogSubscription {
            id: "sub_1".into(),
            space_id: SpaceId::from("sp_eng"),
            catalog_item_id: "ci_1".into(),
            created_at: now(),
        };
        store.create_subscription(&sub).await.unwrap();

        let subs = store
            .list_subscriptions(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].catalog_item_id, "ci_1");
    }

    #[tokio::test]
    async fn subscription_delete() {
        let store = InMemoryCatalogSubscriptionStore::default();
        let sub = CatalogSubscription {
            id: "sub_1".into(),
            space_id: SpaceId::from("sp_eng"),
            catalog_item_id: "ci_1".into(),
            created_at: now(),
        };
        store.create_subscription(&sub).await.unwrap();

        assert!(store.delete_subscription("sub_1").await.unwrap());
        assert!(!store.delete_subscription("sub_1").await.unwrap());
    }

    // -- InterfaceInstanceStore --

    fn make_instance(
        id: &str,
        org_id: &str,
        space_id: &str,
        iface_type: &str,
    ) -> InterfaceInstance {
        InterfaceInstance {
            id: id.into(),
            org_id: OrgId::from(org_id),
            space_id: SpaceId::from(space_id),
            interface_type: iface_type.into(),
            config: "{}".into(),
            created_at: now(),
        }
    }

    #[tokio::test]
    async fn interface_instance_create_and_list() {
        let store = InMemoryInterfaceInstanceStore::default();
        store
            .create_instance(&make_instance("ii_1", "org_1", "sp_eng", "slack"))
            .await
            .unwrap();
        store
            .create_instance(&make_instance("ii_2", "org_1", "sp_eng", "matrix"))
            .await
            .unwrap();

        let instances = store
            .list_instances(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(instances.len(), 2);
    }

    #[tokio::test]
    async fn interface_instance_delete() {
        let store = InMemoryInterfaceInstanceStore::default();
        store
            .create_instance(&make_instance("ii_1", "org_1", "sp_eng", "slack"))
            .await
            .unwrap();

        assert!(store.delete_instance("ii_1").await.unwrap());
        assert!(store.get_instance("ii_1").await.unwrap().is_none());
    }

    // -- BindingStore --

    #[tokio::test]
    async fn binding_create_and_list() {
        let store = InMemoryBindingStore::default();
        let binding = PersonaBinding {
            id: "bind_1".into(),
            space_id: SpaceId::from("sp_eng"),
            persona_id: "persona_ops".into(),
            interface_instance_id: "ii_1".into(),
            created_at: now(),
        };
        store.create_binding(&binding).await.unwrap();

        let bindings = store.list_bindings(&SpaceId::from("sp_eng")).await.unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].persona_id, "persona_ops");
    }

    #[tokio::test]
    async fn binding_delete() {
        let store = InMemoryBindingStore::default();
        let binding = PersonaBinding {
            id: "bind_1".into(),
            space_id: SpaceId::from("sp_eng"),
            persona_id: "persona_ops".into(),
            interface_instance_id: "ii_1".into(),
            created_at: now(),
        };
        store.create_binding(&binding).await.unwrap();

        assert!(store.delete_binding("bind_1").await.unwrap());
        assert!(!store.delete_binding("bind_1").await.unwrap());
    }
}
