//! SQLite-backed [`UserStore`] implementation.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::{OrgId, UserId};
use assistant_core::store::{SessionState, User, UserStore};

/// SQLite-backed user store.
pub struct SqliteUserStore {
    pool: SqlitePool,
}

impl SqliteUserStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Look up a user in one org by their IdP (issuer, subject) pair.
    ///
    /// Orgs can share an IdP, so the lookup must not cross org boundaries.
    pub async fn get_user_by_idp_in_org(
        &self,
        org_id: &OrgId,
        issuer: &str,
        subject: &str,
    ) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at
             FROM users WHERE org_id = ? AND idp_issuer = ? AND idp_subject = ?",
        )
        .bind(&org_id.0)
        .bind(issuer)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_user))
    }

    /// Look up a user by email across all organizations.
    ///
    /// Used during OAuth login when the caller does not yet know which org
    /// the user belongs to. Returns the first matching user.
    pub async fn get_user_by_email_any(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at
             FROM users WHERE email = ? LIMIT 1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_user))
    }

    /// List the users with this email in every organization.
    ///
    /// Emails are only unique per org, so the sign-in flow uses this to find
    /// out which orgs an email belongs to.
    pub async fn list_users_by_email_any(&self, email: &str) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at
             FROM users WHERE email = ? ORDER BY org_id",
        )
        .bind(email)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_user).collect())
    }
}

fn row_to_user(row: sqlx::sqlite::SqliteRow) -> User {
    User {
        id: UserId(row.get("id")),
        org_id: OrgId(row.get("org_id")),
        email: row.get("email"),
        name: row.get("name"),
        password_hash: row.get("password_hash"),
        idp_issuer: row.get("idp_issuer"),
        idp_subject: row.get("idp_subject"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[async_trait]
impl UserStore for SqliteUserStore {
    async fn create_user(&self, user: &User) -> Result<()> {
        sqlx::query(
            "INSERT INTO users (id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&user.id.0)
        .bind(&user.org_id.0)
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(&user.idp_issuer)
        .bind(&user.idp_subject)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("creating user: {}", user.id))?;
        Ok(())
    }

    async fn get_user(&self, id: &UserId) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at
             FROM users WHERE id = ?",
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_user))
    }

    async fn get_user_by_email(&self, org_id: &OrgId, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at
             FROM users WHERE org_id = ? AND email = ?",
        )
        .bind(&org_id.0)
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_user))
    }

    async fn get_user_by_idp(&self, issuer: &str, subject: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at
             FROM users WHERE idp_issuer = ? AND idp_subject = ?",
        )
        .bind(issuer)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_user))
    }

    async fn list_users(&self, org_id: &OrgId) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, org_id, email, name, password_hash, idp_issuer, idp_subject, created_at, updated_at
             FROM users WHERE org_id = ? ORDER BY created_at",
        )
        .bind(&org_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_user).collect())
    }

    async fn update_user(&self, user: &User) -> Result<()> {
        let result = sqlx::query(
            "UPDATE users SET email = ?, name = ?, password_hash = ?, idp_issuer = ?, idp_subject = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.password_hash)
        .bind(&user.idp_issuer)
        .bind(&user.idp_subject)
        .bind(user.updated_at)
        .bind(&user.id.0)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            anyhow::bail!("user not found: {}", user.id);
        }
        Ok(())
    }

    async fn delete_user(&self, id: &UserId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(&id.0)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn session_state(&self, id: &UserId) -> Result<Option<SessionState>> {
        let row = sqlx::query("SELECT org_id, token_version FROM users WHERE id = ?")
            .bind(&id.0)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|row| SessionState {
            org_id: OrgId(row.get("org_id")),
            token_version: row.get::<i64, _>("token_version").max(0) as u64,
        }))
    }

    async fn bump_token_version(&self, id: &UserId) -> Result<bool> {
        let result = sqlx::query("UPDATE users SET token_version = token_version + 1 WHERE id = ?")
            .bind(&id.0)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_storage::OrgStorageLayer;
    use chrono::Utc;

    use assistant_core::store::{OrgStore, Organization};

    async fn setup() -> (OrgStorageLayer, SqliteUserStore) {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        let org_store = layer.org_store();
        let now = Utc::now();
        org_store
            .create_org(&Organization {
                id: OrgId::from("org_1"),
                name: "Acme".into(),
                slug: "acme".into(),
                auth_mode: "password".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        let store = layer.user_store();
        (layer, store)
    }

    fn make_user(id: &str, org_id: &str, email: &str) -> User {
        let now = Utc::now();
        User {
            id: UserId::from(id),
            org_id: OrgId::from(org_id),
            email: email.into(),
            name: id.into(),
            password_hash: String::new(),
            idp_issuer: None,
            idp_subject: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_and_get() {
        let (_layer, store) = setup().await;
        let user = make_user("usr_alice", "org_1", "alice@acme.com");
        store.create_user(&user).await.unwrap();

        let found = store.get_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "alice@acme.com");
    }

    #[tokio::test]
    async fn get_by_email() {
        let (_layer, store) = setup().await;
        store
            .create_user(&make_user("usr_alice", "org_1", "alice@acme.com"))
            .await
            .unwrap();

        let found = store
            .get_user_by_email(&OrgId::from("org_1"), "alice@acme.com")
            .await
            .unwrap();
        assert!(found.is_some());

        let not_found = store
            .get_user_by_email(&OrgId::from("org_2"), "alice@acme.com")
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn list_by_email_any_returns_the_user_of_every_org() {
        let (layer, store) = setup().await;
        let now = Utc::now();
        layer
            .org_store()
            .create_org(&Organization {
                id: OrgId::from("org_2"),
                name: "Beta".into(),
                slug: "beta".into(),
                auth_mode: "password".into(),
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();
        for (id, org, email) in [
            ("usr_a1", "org_1", "alice@acme.com"),
            ("usr_a2", "org_2", "alice@acme.com"),
            ("usr_b1", "org_1", "bob@acme.com"),
        ] {
            store.create_user(&make_user(id, org, email)).await.unwrap();
        }

        let mut orgs: Vec<String> = store
            .list_users_by_email_any("alice@acme.com")
            .await
            .unwrap()
            .into_iter()
            .map(|u| u.org_id.0)
            .collect();
        orgs.sort();
        assert_eq!(orgs, ["org_1", "org_2"]);

        let none = store
            .list_users_by_email_any("nobody@acme.com")
            .await
            .unwrap();
        assert!(none.is_empty(), "unknown email matches nothing");
    }

    #[tokio::test]
    async fn get_by_idp() {
        let (_layer, store) = setup().await;
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
    async fn duplicate_email_in_same_org_rejected() {
        let (_layer, store) = setup().await;
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
    async fn update_user() {
        let (_layer, store) = setup().await;
        let mut user = make_user("usr_alice", "org_1", "alice@acme.com");
        store.create_user(&user).await.unwrap();

        user.name = "Alice Smith".into();
        user.updated_at = Utc::now();
        store.update_user(&user).await.unwrap();

        let found = store
            .get_user(&UserId::from("usr_alice"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "Alice Smith");
    }

    #[tokio::test]
    async fn delete_user() {
        let (_layer, store) = setup().await;
        store
            .create_user(&make_user("usr_alice", "org_1", "alice@acme.com"))
            .await
            .unwrap();

        let deleted = store.delete_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(deleted);

        let found = store.get_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(found.is_none());

        let not_deleted = store.delete_user(&UserId::from("usr_alice")).await.unwrap();
        assert!(!not_deleted);
    }

    #[tokio::test]
    async fn list_by_org() {
        let (_layer, store) = setup().await;
        store
            .create_user(&make_user("usr_1", "org_1", "alice@acme.com"))
            .await
            .unwrap();
        store
            .create_user(&make_user("usr_2", "org_1", "bob@acme.com"))
            .await
            .unwrap();

        let users = store.list_users(&OrgId::from("org_1")).await.unwrap();
        assert_eq!(users.len(), 2);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let (_layer, store) = setup().await;
        let found = store.get_user(&UserId::from("no_such")).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn session_state_starts_at_version_zero() {
        let (_layer, store) = setup().await;
        store
            .create_user(&make_user("usr_1", "org_1", "alice@acme.com"))
            .await
            .unwrap();

        let state = store
            .session_state(&UserId::from("usr_1"))
            .await
            .unwrap()
            .expect("user exists");
        assert_eq!(state.org_id, OrgId::from("org_1"));
        assert_eq!(state.token_version, 0);
    }

    #[tokio::test]
    async fn session_state_is_none_for_deleted_user() {
        let (_layer, store) = setup().await;
        store
            .create_user(&make_user("usr_1", "org_1", "alice@acme.com"))
            .await
            .unwrap();
        store.delete_user(&UserId::from("usr_1")).await.unwrap();

        let state = store.session_state(&UserId::from("usr_1")).await.unwrap();
        assert_eq!(state, None);
    }

    #[tokio::test]
    async fn bump_token_version_persists_and_is_per_user() {
        let (_layer, store) = setup().await;
        for (id, email) in [("usr_1", "alice@acme.com"), ("usr_2", "bob@acme.com")] {
            store
                .create_user(&make_user(id, "org_1", email))
                .await
                .unwrap();
        }

        assert!(
            store
                .bump_token_version(&UserId::from("usr_1"))
                .await
                .unwrap()
        );
        assert!(
            store
                .bump_token_version(&UserId::from("usr_1"))
                .await
                .unwrap()
        );

        let one = store.session_state(&UserId::from("usr_1")).await.unwrap();
        let two = store.session_state(&UserId::from("usr_2")).await.unwrap();
        assert_eq!(one.unwrap().token_version, 2);
        assert_eq!(two.unwrap().token_version, 0);
    }

    #[tokio::test]
    async fn update_user_keeps_token_version() {
        let (_layer, store) = setup().await;
        let mut user = make_user("usr_1", "org_1", "alice@acme.com");
        store.create_user(&user).await.unwrap();
        store.bump_token_version(&user.id).await.unwrap();

        user.name = "Alice".into();
        store.update_user(&user).await.unwrap();

        let state = store.session_state(&user.id).await.unwrap();
        assert_eq!(state.unwrap().token_version, 1);
    }

    #[tokio::test]
    async fn bump_token_version_of_unknown_user_returns_false() {
        let (_layer, store) = setup().await;
        assert!(
            !store
                .bump_token_version(&UserId::from("ghost"))
                .await
                .unwrap()
        );
    }
}
