//! SQLite-backed [`InterfaceInstanceStore`] implementation.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::{OrgId, SpaceId};
use assistant_core::store::{InterfaceInstance, InterfaceInstanceStore};

/// SQLite-backed interface instance store.
pub struct SqliteInterfaceInstanceStore {
    pool: SqlitePool,
}

impl SqliteInterfaceInstanceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_instance(row: sqlx::sqlite::SqliteRow) -> InterfaceInstance {
    InterfaceInstance {
        id: row.get("id"),
        org_id: OrgId(row.get("org_id")),
        space_id: SpaceId(row.get("space_id")),
        interface_type: row.get("interface_type"),
        config: row.get("config"),
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl InterfaceInstanceStore for SqliteInterfaceInstanceStore {
    async fn create_instance(&self, instance: &InterfaceInstance) -> Result<()> {
        sqlx::query(
            "INSERT INTO interface_instances (id, org_id, space_id, interface_type, config, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&instance.id)
        .bind(&instance.org_id.0)
        .bind(&instance.space_id.0)
        .bind(&instance.interface_type)
        .bind(&instance.config)
        .bind(instance.created_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("creating interface instance: {}", instance.id))?;
        Ok(())
    }

    async fn get_instance(&self, id: &str) -> Result<Option<InterfaceInstance>> {
        let row = sqlx::query(
            "SELECT id, org_id, space_id, interface_type, config, created_at
             FROM interface_instances WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_instance))
    }

    async fn list_instances(&self, space_id: &SpaceId) -> Result<Vec<InterfaceInstance>> {
        let rows = sqlx::query(
            "SELECT id, org_id, space_id, interface_type, config, created_at
             FROM interface_instances WHERE space_id = ? ORDER BY created_at",
        )
        .bind(&space_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_instance).collect())
    }

    async fn delete_instance(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM interface_instances WHERE id = ?")
            .bind(id)
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

    async fn setup() -> OrgStorageLayer {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
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
    async fn interface_instance_crud() {
        let layer = setup().await;
        let store = layer.interface_instance_store();
        let now = Utc::now();

        let instance = InterfaceInstance {
            id: "ii_1".into(),
            org_id: OrgId::from("org_1"),
            space_id: SpaceId::from("sp_eng"),
            interface_type: "slack".into(),
            config: r#"{"token":"xoxb-test"}"#.into(),
            created_at: now,
        };
        store.create_instance(&instance).await.unwrap();

        let found = store.get_instance("ii_1").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().interface_type, "slack");

        let all = store
            .list_instances(&SpaceId::from("sp_eng"))
            .await
            .unwrap();
        assert_eq!(all.len(), 1);

        assert!(store.delete_instance("ii_1").await.unwrap());
        assert!(store.get_instance("ii_1").await.unwrap().is_none());
    }
}
