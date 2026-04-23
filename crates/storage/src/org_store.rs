//! SQLite-backed [`OrgStore`] implementation.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::OrgId;
use assistant_core::store::{OrgStore, Organization};

/// SQLite-backed organization store.
pub struct SqliteOrgStore {
    pool: SqlitePool,
}

impl SqliteOrgStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_org(row: sqlx::sqlite::SqliteRow) -> Organization {
    Organization {
        id: OrgId(row.get("id")),
        name: row.get("name"),
        slug: row.get("slug"),
        auth_mode: row.get("auth_mode"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

#[async_trait]
impl OrgStore for SqliteOrgStore {
    async fn create_org(&self, org: &Organization) -> Result<()> {
        sqlx::query(
            "INSERT INTO organizations (id, name, slug, auth_mode, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&org.id.0)
        .bind(&org.name)
        .bind(&org.slug)
        .bind(&org.auth_mode)
        .bind(org.created_at)
        .bind(org.updated_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("creating organization: {}", org.id))?;
        Ok(())
    }

    async fn get_org(&self, id: &OrgId) -> Result<Option<Organization>> {
        let row = sqlx::query(
            "SELECT id, name, slug, auth_mode, created_at, updated_at
             FROM organizations WHERE id = ?",
        )
        .bind(&id.0)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_org))
    }

    async fn get_org_by_slug(&self, slug: &str) -> Result<Option<Organization>> {
        let row = sqlx::query(
            "SELECT id, name, slug, auth_mode, created_at, updated_at
             FROM organizations WHERE slug = ?",
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_org))
    }

    async fn update_org(&self, org: &Organization) -> Result<()> {
        let result = sqlx::query(
            "UPDATE organizations SET name = ?, slug = ?, auth_mode = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&org.name)
        .bind(&org.slug)
        .bind(&org.auth_mode)
        .bind(org.updated_at)
        .bind(&org.id.0)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            anyhow::bail!("organization not found: {}", org.id);
        }
        Ok(())
    }

    async fn list_orgs(&self) -> Result<Vec<Organization>> {
        let rows = sqlx::query(
            "SELECT id, name, slug, auth_mode, created_at, updated_at
             FROM organizations ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_org).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_storage::OrgStorageLayer;
    use chrono::Utc;

    async fn setup() -> (OrgStorageLayer, SqliteOrgStore) {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        let store = layer.org_store();
        (layer, store)
    }

    fn make_org(id: &str, slug: &str) -> Organization {
        let now = Utc::now();
        Organization {
            id: OrgId::from(id),
            name: format!("{slug} Inc."),
            slug: slug.into(),
            auth_mode: "password".into(),
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_and_get() {
        let (_layer, store) = setup().await;
        let org = make_org("org_1", "acme");
        store.create_org(&org).await.unwrap();

        let found = store.get_org(&OrgId::from("org_1")).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.slug, "acme");
        assert_eq!(found.name, "acme Inc.");
    }

    #[tokio::test]
    async fn get_by_slug() {
        let (_layer, store) = setup().await;
        store.create_org(&make_org("org_1", "acme")).await.unwrap();

        let found = store.get_org_by_slug("acme").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, OrgId::from("org_1"));

        let missing = store.get_org_by_slug("nope").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn duplicate_slug_rejected() {
        let (_layer, store) = setup().await;
        store.create_org(&make_org("org_1", "acme")).await.unwrap();
        let err = store.create_org(&make_org("org_2", "acme")).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn update_org() {
        let (_layer, store) = setup().await;
        let mut org = make_org("org_1", "acme");
        store.create_org(&org).await.unwrap();

        org.name = "Acme Corp".into();
        org.updated_at = Utc::now();
        store.update_org(&org).await.unwrap();

        let found = store.get_org(&OrgId::from("org_1")).await.unwrap().unwrap();
        assert_eq!(found.name, "Acme Corp");
    }

    #[tokio::test]
    async fn update_nonexistent_fails() {
        let (_layer, store) = setup().await;
        let org = make_org("org_999", "ghost");
        let err = store.update_org(&org).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn list_orgs() {
        let (_layer, store) = setup().await;
        store.create_org(&make_org("org_1", "acme")).await.unwrap();
        store
            .create_org(&make_org("org_2", "globex"))
            .await
            .unwrap();

        let all = store.list_orgs().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn get_nonexistent_returns_none() {
        let (_layer, store) = setup().await;
        let found = store.get_org(&OrgId::from("no_such")).await.unwrap();
        assert!(found.is_none());
    }
}
