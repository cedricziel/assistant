//! SQLite-backed [`SpaceStore`] and [`MembershipStore`] implementations.

use std::collections::HashMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::{OrgId, Role, SpaceId, UserId};
use assistant_core::store::{MembershipStore, Space, SpaceMembership, SpaceStore};

// ---------------------------------------------------------------------------
// Row mapping helpers
// ---------------------------------------------------------------------------

fn row_to_space(row: sqlx::sqlite::SqliteRow) -> Space {
    Space {
        id: SpaceId(row.get("id")),
        org_id: OrgId(row.get("org_id")),
        name: row.get("name"),
        slug: row.get("slug"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_membership(row: sqlx::sqlite::SqliteRow) -> SpaceMembership {
    let role_str: String = row.get("role");
    SpaceMembership {
        user_id: UserId(row.get("user_id")),
        space_id: SpaceId(row.get("space_id")),
        role: role_from_str(&role_str),
        created_at: row.get("created_at"),
    }
}

/// Parse a role string from the database.
fn role_from_str(s: &str) -> Role {
    match s {
        "org_admin" => Role::OrgAdmin,
        "space_admin" => Role::SpaceAdmin,
        "member" => Role::Member,
        "viewer" => Role::Viewer,
        _ => Role::Viewer, // safe fallback
    }
}

// ---------------------------------------------------------------------------
// SqliteSpaceStore
// ---------------------------------------------------------------------------

/// SQLite-backed space store.
pub struct SqliteSpaceStore {
    pool: SqlitePool,
}

impl SqliteSpaceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SpaceStore for SqliteSpaceStore {
    async fn create_space(&self, space: &Space) -> Result<()> {
        sqlx::query(
            "INSERT INTO spaces (id, org_id, name, slug, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&space.id.0)
        .bind(&space.org_id.0)
        .bind(&space.name)
        .bind(&space.slug)
        .bind(space.created_at)
        .bind(space.updated_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("creating space: {}", space.id))?;
        Ok(())
    }

    async fn get_space(&self, id: &SpaceId) -> Result<Option<Space>> {
        let row = sqlx::query(
            "SELECT id, org_id, name, slug, created_at, updated_at
             FROM spaces WHERE id = ?",
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_space))
    }

    async fn list_spaces(&self, org_id: &OrgId) -> Result<Vec<Space>> {
        let rows = sqlx::query(
            "SELECT id, org_id, name, slug, created_at, updated_at
             FROM spaces WHERE org_id = ? ORDER BY created_at",
        )
        .bind(&org_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_space).collect())
    }

    async fn update_space(&self, space: &Space) -> Result<()> {
        sqlx::query("UPDATE spaces SET name = ?, slug = ?, updated_at = ? WHERE id = ?")
            .bind(&space.name)
            .bind(&space.slug)
            .bind(space.updated_at)
            .bind(&space.id.0)
            .execute(&self.pool)
            .await
            .with_context(|| format!("updating space: {}", space.id))?;
        Ok(())
    }

    async fn delete_space(&self, id: &SpaceId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM spaces WHERE id = ?")
            .bind(&id.0)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

// ---------------------------------------------------------------------------
// SqliteMembershipStore
// ---------------------------------------------------------------------------

/// SQLite-backed membership store.
pub struct SqliteMembershipStore {
    pool: SqlitePool,
}

impl SqliteMembershipStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MembershipStore for SqliteMembershipStore {
    async fn add_membership(&self, membership: &SpaceMembership) -> Result<()> {
        sqlx::query(
            "INSERT INTO space_memberships (user_id, space_id, role, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&membership.user_id.0)
        .bind(&membership.space_id.0)
        .bind(membership.role.to_string())
        .bind(membership.created_at)
        .execute(&self.pool)
        .await
        .with_context(|| {
            format!(
                "adding membership: user {} in space {}",
                membership.user_id, membership.space_id
            )
        })?;
        Ok(())
    }

    async fn remove_membership(&self, user_id: &UserId, space_id: &SpaceId) -> Result<bool> {
        let result =
            sqlx::query("DELETE FROM space_memberships WHERE user_id = ? AND space_id = ?")
                .bind(&user_id.0)
                .bind(&space_id.0)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_memberships_for_user(&self, user_id: &UserId) -> Result<Vec<SpaceMembership>> {
        let rows = sqlx::query(
            "SELECT user_id, space_id, role, created_at
             FROM space_memberships WHERE user_id = ?",
        )
        .bind(&user_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_membership).collect())
    }

    async fn get_members_of_space(&self, space_id: &SpaceId) -> Result<Vec<SpaceMembership>> {
        let rows = sqlx::query(
            "SELECT user_id, space_id, role, created_at
             FROM space_memberships WHERE space_id = ?",
        )
        .bind(&space_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_membership).collect())
    }

    async fn get_space_roles(&self, user_id: &UserId) -> Result<HashMap<SpaceId, Role>> {
        let rows = sqlx::query(
            "SELECT user_id, space_id, role, created_at
             FROM space_memberships WHERE user_id = ?",
        )
        .bind(&user_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let role_str: String = r.get("role");
                (SpaceId(r.get("space_id")), role_from_str(&role_str))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_storage::OrgStorageLayer;
    use chrono::Utc;

    use assistant_core::store::{OrgStore, Organization, UserStore};

    async fn setup() -> (OrgStorageLayer, SqliteSpaceStore, SqliteMembershipStore) {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        let now = Utc::now();

        // Create org and users (FK requirements).
        layer
            .org_store()
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

        layer
            .user_store()
            .create_user(&assistant_core::store::User {
                id: UserId::from("usr_alice"),
                org_id: OrgId::from("org_1"),
                email: "alice@acme.com".into(),
                name: "Alice".into(),
                password_hash: String::new(),
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();

        layer
            .user_store()
            .create_user(&assistant_core::store::User {
                id: UserId::from("usr_bob"),
                org_id: OrgId::from("org_1"),
                email: "bob@acme.com".into(),
                name: "Bob".into(),
                password_hash: String::new(),
                idp_issuer: None,
                idp_subject: None,
                created_at: now,
                updated_at: now,
            })
            .await
            .unwrap();

        let space_store = layer.space_store();
        let membership_store = layer.membership_store();
        (layer, space_store, membership_store)
    }

    fn make_space(id: &str, org_id: &str, slug: &str) -> Space {
        let now = Utc::now();
        Space {
            id: SpaceId::from(id),
            org_id: OrgId::from(org_id),
            name: format!("{slug} space"),
            slug: slug.into(),
            created_at: now,
            updated_at: now,
        }
    }

    fn make_membership(user_id: &str, space_id: &str, role: Role) -> SpaceMembership {
        SpaceMembership {
            user_id: UserId::from(user_id),
            space_id: SpaceId::from(space_id),
            role,
            created_at: Utc::now(),
        }
    }

    // -- SpaceStore --

    #[tokio::test]
    async fn space_create_and_get() {
        let (_layer, space_store, _) = setup().await;
        let space = make_space("sp_eng", "org_1", "engineering");
        space_store.create_space(&space).await.unwrap();

        let found = space_store
            .get_space(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().slug, "engineering");
    }

    #[tokio::test]
    async fn space_list_by_org() {
        let (_layer, space_store, _) = setup().await;
        space_store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();
        space_store
            .create_space(&make_space("sp_mktg", "org_1", "marketing"))
            .await
            .unwrap();

        let spaces = space_store
            .list_spaces(&OrgId::from("org_1"))
            .await
            .unwrap();
        assert_eq!(spaces.len(), 2);
    }

    #[tokio::test]
    async fn space_delete() {
        let (_layer, space_store, _) = setup().await;
        space_store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();

        let deleted = space_store
            .delete_space(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert!(deleted);

        let found = space_store
            .get_space(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert!(found.is_none());
    }

    // -- MembershipStore --

    #[tokio::test]
    async fn membership_add_and_query() {
        let (_layer, space_store, membership_store) = setup().await;
        space_store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();

        membership_store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();

        let alice_memberships = membership_store
            .get_memberships_for_user(&UserId::from("usr_alice"))
            .await
            .unwrap();
        assert_eq!(alice_memberships.len(), 1);
        assert_eq!(alice_memberships[0].role, Role::Member);

        let eng_members = membership_store
            .get_members_of_space(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(eng_members.len(), 1);
    }

    #[tokio::test]
    async fn membership_duplicate_rejected() {
        let (_layer, space_store, membership_store) = setup().await;
        space_store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();

        membership_store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();
        let err = membership_store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Viewer))
            .await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn membership_remove() {
        let (_layer, space_store, membership_store) = setup().await;
        space_store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();

        membership_store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();

        let removed = membership_store
            .remove_membership(&UserId::from("usr_alice"), &SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert!(removed);

        let memberships = membership_store
            .get_memberships_for_user(&UserId::from("usr_alice"))
            .await
            .unwrap();
        assert!(memberships.is_empty());
    }

    #[tokio::test]
    async fn membership_space_roles() {
        let (_layer, space_store, membership_store) = setup().await;
        space_store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();
        space_store
            .create_space(&make_space("sp_mktg", "org_1", "marketing"))
            .await
            .unwrap();

        membership_store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::Member))
            .await
            .unwrap();
        membership_store
            .add_membership(&make_membership("usr_alice", "sp_mktg", Role::SpaceAdmin))
            .await
            .unwrap();

        let roles = membership_store
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
        let (_layer, space_store, membership_store) = setup().await;
        space_store
            .create_space(&make_space("sp_eng", "org_1", "engineering"))
            .await
            .unwrap();

        membership_store
            .add_membership(&make_membership("usr_alice", "sp_eng", Role::SpaceAdmin))
            .await
            .unwrap();
        membership_store
            .add_membership(&make_membership("usr_bob", "sp_eng", Role::Member))
            .await
            .unwrap();

        let members = membership_store
            .get_members_of_space(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(members.len(), 2);
    }
}
