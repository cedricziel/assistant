//! SQLite-backed [`UserStore`] implementation.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::{OrgId, UserId};
use assistant_core::store::{User, UserStore};

/// SQLite-backed user store.
pub struct SqliteUserStore {
    pool: SqlitePool,
}

impl SqliteUserStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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
}
