//! Bridge between `assistant_auth::oidc::OidcUserStore` and
//! `assistant_storage::OrgStorageLayer`.

use anyhow::{Context, Result};
use assistant_core::clock::{Clock, SystemClock};
use async_trait::async_trait;

use assistant_auth::oidc::{OidcUserInfo, OidcUserStore};
use assistant_core::identity::{OrgId, UserId};
use assistant_core::store::UserStore;
use assistant_storage::OrgStorageLayer;

/// Adapts [`OrgStorageLayer`] to the [`OidcUserStore`] trait so the OIDC
/// callback can provision and look up users.
pub struct OrgOidcUserStore {
    storage: std::sync::Arc<OrgStorageLayer>,
    /// The org that OIDC-provisioned users belong to.
    org_id: OrgId,
}

impl OrgOidcUserStore {
    pub fn new(storage: std::sync::Arc<OrgStorageLayer>, org_id: OrgId) -> Self {
        Self { storage, org_id }
    }
}

#[async_trait]
impl OidcUserStore for OrgOidcUserStore {
    async fn find_by_idp(&self, issuer: &str, subject: &str) -> Result<Option<UserId>> {
        let user = self
            .storage
            .user_store()
            .get_user_by_idp(issuer, subject)
            .await?;
        Ok(user.map(|u| u.id))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserId>> {
        let user = self
            .storage
            .user_store()
            .get_user_by_email_any(email)
            .await?;
        Ok(user.map(|u| u.id))
    }

    async fn create_user(&self, info: &OidcUserInfo) -> Result<UserId> {
        let now = SystemClock.now();
        let user_id = UserId::from(format!("usr_{}", uuid::Uuid::new_v4()));
        let user = assistant_core::store::User {
            id: user_id.clone(),
            org_id: self.org_id.clone(),
            email: info.email.clone().unwrap_or_default(),
            name: info.name.clone().unwrap_or_default(),
            password_hash: String::new(), // OIDC users have no local password.
            idp_issuer: Some(info.idp_issuer.clone()),
            idp_subject: Some(info.idp_subject.clone()),
            created_at: now,
            updated_at: now,
        };
        self.storage
            .user_store()
            .create_user(&user)
            .await
            .context("failed to create OIDC user")?;
        Ok(user_id)
    }

    async fn link_idp(&self, user_id: &UserId, issuer: &str, subject: &str) -> Result<()> {
        let mut user = self
            .storage
            .user_store()
            .get_user(user_id)
            .await?
            .with_context(|| format!("user {user_id} not found"))?;
        user.idp_issuer = Some(issuer.to_owned());
        user.idp_subject = Some(subject.to_owned());
        user.updated_at = SystemClock.now();
        self.storage
            .user_store()
            .update_user(&user)
            .await
            .context("failed to link IdP to user")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use assistant_core::identity::OrgId;
    use assistant_core::store::{OrgStore, Organization};

    async fn setup() -> Arc<OrgStorageLayer> {
        let storage = Arc::new(OrgStorageLayer::new_in_memory().await.unwrap());
        let now = chrono::Utc::now();
        storage
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("org_test"),
                name: "Test Org".into(),
                slug: "test".into(),
                auth_mode: "oidc".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        storage
    }

    #[tokio::test]
    async fn create_and_find_by_idp() {
        let storage = setup().await;
        let store = OrgOidcUserStore::new(storage, OrgId::from("org_test"));

        let info = OidcUserInfo {
            idp_issuer: "https://idp.example.com".into(),
            idp_subject: "sub_123".into(),
            email: Some("alice@example.com".into()),
            name: Some("Alice".into()),
        };

        let user_id = store.create_user(&info).await.unwrap();

        let found = store
            .find_by_idp("https://idp.example.com", "sub_123")
            .await
            .unwrap();
        assert_eq!(found, Some(user_id));
    }

    #[tokio::test]
    async fn find_by_email_and_link() {
        let storage = setup().await;
        let store = OrgOidcUserStore::new(storage.clone(), OrgId::from("org_test"));

        // Create a user without IdP link.
        let now = chrono::Utc::now();
        let user = assistant_core::store::User {
            id: UserId::from("usr_bob"),
            org_id: OrgId::from("org_test"),
            email: "bob@example.com".into(),
            name: "Bob".into(),
            password_hash: String::new(),
            idp_issuer: None,
            idp_subject: None,
            created_at: now,
            updated_at: now,
        };
        storage.user_store().create_user(&user).await.unwrap();

        // Find by email.
        let found = store.find_by_email("bob@example.com").await.unwrap();
        assert_eq!(found, Some(UserId::from("usr_bob")));

        // Link to IdP.
        store
            .link_idp(
                &UserId::from("usr_bob"),
                "https://idp.example.com",
                "sub_bob",
            )
            .await
            .unwrap();

        // Now find by IdP should work.
        let found = store
            .find_by_idp("https://idp.example.com", "sub_bob")
            .await
            .unwrap();
        assert_eq!(found, Some(UserId::from("usr_bob")));
    }

    #[tokio::test]
    async fn find_nonexistent_returns_none() {
        let storage = setup().await;
        let store = OrgOidcUserStore::new(storage, OrgId::from("org_test"));

        let found = store
            .find_by_idp("https://no.such", "nobody")
            .await
            .unwrap();
        assert!(found.is_none());

        let found = store.find_by_email("nobody@example.com").await.unwrap();
        assert!(found.is_none());
    }
}
