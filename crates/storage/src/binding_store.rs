//! SQLite-backed [`BindingStore`] implementation.

use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::SpaceId;
use assistant_core::store::{BindingStore, PersonaBinding};

/// SQLite-backed persona-interface binding store.
pub struct SqliteBindingStore {
    pool: SqlitePool,
}

impl SqliteBindingStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_binding(row: sqlx::sqlite::SqliteRow) -> PersonaBinding {
    PersonaBinding {
        id: row.get("id"),
        space_id: SpaceId(row.get("space_id")),
        persona_id: row.get("persona_id"),
        interface_instance_id: row.get("interface_instance_id"),
        created_at: row.get("created_at"),
    }
}

#[async_trait]
impl BindingStore for SqliteBindingStore {
    async fn create_binding(&self, binding: &PersonaBinding) -> Result<()> {
        sqlx::query(
            "INSERT INTO persona_bindings (id, space_id, persona_id, interface_instance_id, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&binding.id)
        .bind(&binding.space_id.0)
        .bind(&binding.persona_id)
        .bind(&binding.interface_instance_id)
        .bind(binding.created_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("creating persona binding: {}", binding.id))?;
        Ok(())
    }

    async fn list_bindings(&self, space_id: &SpaceId) -> Result<Vec<PersonaBinding>> {
        let rows = sqlx::query(
            "SELECT id, space_id, persona_id, interface_instance_id, created_at
             FROM persona_bindings WHERE space_id = ? ORDER BY created_at",
        )
        .bind(&space_id.0)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_binding).collect())
    }

    async fn delete_binding(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM persona_bindings WHERE id = ?")
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
        // Seed an interface instance for FK constraint.
        sqlx::query(
            "INSERT INTO interface_instances (id, org_id, space_id, interface_type, config)
             VALUES ('ii_1', 'org_1', 'sp_eng', 'slack', '{}')",
        )
        .execute(layer.pool())
        .await
        .unwrap();
        layer
    }

    #[tokio::test]
    async fn binding_crud() {
        let layer = setup().await;
        let store = layer.binding_store();
        let now = Utc::now();

        let binding = PersonaBinding {
            id: "bind_1".into(),
            space_id: SpaceId::from("sp_eng"),
            persona_id: "persona_ops".into(),
            interface_instance_id: "ii_1".into(),
            created_at: now,
        };
        store.create_binding(&binding).await.unwrap();

        let bindings = store.list_bindings(&SpaceId::from("sp_eng")).await.unwrap();
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].persona_id, "persona_ops");

        assert!(store.delete_binding("bind_1").await.unwrap());
        assert!(!store.delete_binding("bind_1").await.unwrap());
    }

    #[tokio::test]
    async fn duplicate_binding_rejected() {
        let layer = setup().await;
        let store = layer.binding_store();
        let now = Utc::now();

        let binding = PersonaBinding {
            id: "bind_1".into(),
            space_id: SpaceId::from("sp_eng"),
            persona_id: "persona_ops".into(),
            interface_instance_id: "ii_1".into(),
            created_at: now,
        };
        store.create_binding(&binding).await.unwrap();

        // Same persona + interface_instance should fail (UNIQUE constraint).
        let dup = PersonaBinding {
            id: "bind_2".into(),
            space_id: SpaceId::from("sp_eng"),
            persona_id: "persona_ops".into(),
            interface_instance_id: "ii_1".into(),
            created_at: now,
        };
        assert!(store.create_binding(&dup).await.is_err());
    }
}
