//! SQLite-backed [`CatalogItemStore`] and [`CatalogSubscriptionStore`] implementations.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::catalog::CatalogResourceType;
use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::{
    CatalogItem, CatalogItemStore, CatalogSubscription, CatalogSubscriptionStore,
};

/// SQLite-backed catalog item store.
pub struct SqliteCatalogItemStore {
    pool: SqlitePool,
}

impl SqliteCatalogItemStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn resource_type_from_str(s: &str) -> CatalogResourceType {
    match s {
        "skill" => CatalogResourceType::Skill,
        "template" => CatalogResourceType::Template,
        "interface" => CatalogResourceType::Interface,
        other => {
            tracing::warn!("unrecognized catalog resource_type '{other}', defaulting to Skill");
            CatalogResourceType::Skill
        }
    }
}

fn row_to_item(row: sqlx::sqlite::SqliteRow) -> CatalogItem {
    CatalogItem {
        id: row.get("id"),
        org_id: OrgId(row.get("org_id")),
        resource_type: resource_type_from_str(row.get("resource_type")),
        name: row.get("name"),
        description: row.get("description"),
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl CatalogItemStore for SqliteCatalogItemStore {
    async fn create_item(&self, item: &CatalogItem) -> Result<()> {
        sqlx::query(
            "INSERT INTO catalog_items (id, org_id, resource_type, name, description, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&item.id)
        .bind(&item.org_id.0)
        .bind(item.resource_type.to_string())
        .bind(&item.name)
        .bind(&item.description)
        .bind(item.created_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("creating catalog item: {}", item.id))?;
        Ok(())
    }

    async fn get_item(&self, id: &str) -> Result<Option<CatalogItem>> {
        let row = sqlx::query(
            "SELECT id, org_id, resource_type, name, description, created_at
             FROM catalog_items WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("loading catalog item: {id}"))?;
        Ok(row.map(row_to_item))
    }

    async fn list_items(
        &self,
        org_id: &OrgId,
        resource_type: Option<&CatalogResourceType>,
    ) -> Result<Vec<CatalogItem>> {
        let rows = match resource_type {
            Some(rt) => sqlx::query(
                "SELECT id, org_id, resource_type, name, description, created_at
                     FROM catalog_items WHERE org_id = ? AND resource_type = ? ORDER BY created_at",
            )
            .bind(&org_id.0)
            .bind(rt.to_string())
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("listing catalog items for org {} type {rt}", org_id.0))?,
            None => sqlx::query(
                "SELECT id, org_id, resource_type, name, description, created_at
                     FROM catalog_items WHERE org_id = ? ORDER BY created_at",
            )
            .bind(&org_id.0)
            .fetch_all(&self.pool)
            .await
            .with_context(|| format!("listing catalog items for org {}", org_id.0))?,
        };
        Ok(rows.into_iter().map(row_to_item).collect())
    }

    async fn delete_item(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM catalog_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("deleting catalog item: {id}"))?;
        Ok(result.rows_affected() > 0)
    }
}

// -- CatalogSubscriptionStore ------------------------------------------------

/// SQLite-backed catalog subscription store.
pub struct SqliteCatalogSubscriptionStore {
    pool: SqlitePool,
}

impl SqliteCatalogSubscriptionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_sub(row: sqlx::sqlite::SqliteRow) -> CatalogSubscription {
    CatalogSubscription {
        id: row.get("id"),
        space_id: SpaceId(row.get("space_id")),
        catalog_item_id: row.get("catalog_item_id"),
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl CatalogSubscriptionStore for SqliteCatalogSubscriptionStore {
    async fn create_subscription(&self, sub: &CatalogSubscription) -> Result<()> {
        sqlx::query(
            "INSERT INTO catalog_subscriptions (id, space_id, catalog_item_id, created_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&sub.id)
        .bind(&sub.space_id.0)
        .bind(&sub.catalog_item_id)
        .bind(sub.created_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("creating catalog subscription: {}", sub.id))?;
        Ok(())
    }

    async fn get_subscription(&self, id: &str) -> Result<Option<CatalogSubscription>> {
        let row = sqlx::query(
            "SELECT id, space_id, catalog_item_id, created_at
             FROM catalog_subscriptions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("loading catalog subscription: {id}"))?;
        Ok(row.map(row_to_sub))
    }

    async fn list_subscriptions(&self, space_id: &SpaceId) -> Result<Vec<CatalogSubscription>> {
        let rows = sqlx::query(
            "SELECT id, space_id, catalog_item_id, created_at
             FROM catalog_subscriptions WHERE space_id = ? ORDER BY created_at",
        )
        .bind(&space_id.0)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("listing subscriptions for space: {}", space_id.0))?;
        Ok(rows.into_iter().map(row_to_sub).collect())
    }

    async fn delete_subscription(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM catalog_subscriptions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("deleting catalog subscription: {id}"))?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::org_storage::OrgStorageLayer;
    use chrono::Utc;

    async fn setup() -> OrgStorageLayer {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        // Seed an org and a space for FK constraints.
        sqlx::query("INSERT INTO organizations (id, name, slug) VALUES ('org_1', 'Acme', 'acme')")
            .execute(layer.pool())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_eng', 'org_1', 'Engineering', 'eng')",
        )
        .execute(layer.pool())
        .await
        .unwrap();
        layer
    }

    #[tokio::test]
    async fn catalog_item_crud() {
        let layer = setup().await;
        let store = layer.catalog_item_store();
        let now = Utc::now();

        let item = CatalogItem {
            id: "ci_1".into(),
            org_id: OrgId::from("org_1"),
            resource_type: CatalogResourceType::Skill,
            name: "web-fetch".into(),
            description: "Fetch web pages".into(),
            created_at: now,
        };
        store.create_item(&item).await.unwrap();

        let found = store.get_item("ci_1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "web-fetch");

        let all = store.list_items(&OrgId::from("org_1"), None).await.unwrap();
        assert_eq!(all.len(), 1);

        let skills = store
            .list_items(&OrgId::from("org_1"), Some(&CatalogResourceType::Skill))
            .await
            .unwrap();
        assert_eq!(skills.len(), 1);

        let templates = store
            .list_items(&OrgId::from("org_1"), Some(&CatalogResourceType::Template))
            .await
            .unwrap();
        assert_eq!(templates.len(), 0);

        assert!(store.delete_item("ci_1").await.unwrap());
        assert!(!store.delete_item("ci_1").await.unwrap());
    }

    #[tokio::test]
    async fn catalog_subscription_crud() {
        let layer = setup().await;
        let item_store = layer.catalog_item_store();
        let sub_store = layer.catalog_subscription_store();
        let now = Utc::now();

        // Seed a catalog item first (FK).
        let item = CatalogItem {
            id: "ci_1".into(),
            org_id: OrgId::from("org_1"),
            resource_type: CatalogResourceType::Skill,
            name: "web-fetch".into(),
            description: "".into(),
            created_at: now,
        };
        item_store.create_item(&item).await.unwrap();

        let sub = CatalogSubscription {
            id: "sub_1".into(),
            space_id: SpaceId::from("sp_eng"),
            catalog_item_id: "ci_1".into(),
            created_at: now,
        };
        sub_store.create_subscription(&sub).await.unwrap();

        let subs = sub_store
            .list_subscriptions(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].catalog_item_id, "ci_1");

        assert!(sub_store.delete_subscription("sub_1").await.unwrap());
        assert!(!sub_store.delete_subscription("sub_1").await.unwrap());
    }
}
