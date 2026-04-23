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
}
