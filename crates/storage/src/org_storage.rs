//! Org-level storage layer backed by SQLite.
//!
//! Manages the `org.db` database which holds organizations, users, spaces,
//! memberships, and auth state. Runs its own migration set separate from
//! the existing `assistant.db` migrations.

use std::path::Path;

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use tracing::debug;

/// Storage layer for org-level data (org.db).
///
/// Holds the connection pool and runs org-specific migrations on creation.
#[derive(Clone)]
pub struct OrgStorageLayer {
    pool: SqlitePool,
}

impl OrgStorageLayer {
    /// Open (or create) an org database at the given path and run migrations.
    pub async fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating directory for org.db: {}", parent.display()))?;
        }

        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = SqlitePool::connect(&url)
            .await
            .with_context(|| format!("connecting to org.db: {}", db_path.display()))?;

        run_org_migrations(&pool).await?;

        debug!(path = %db_path.display(), "org.db ready");
        Ok(Self { pool })
    }

    /// Create an in-memory org database (for tests).
    pub async fn new_in_memory() -> Result<Self> {
        let pool = SqlitePool::connect("sqlite::memory:").await?;
        run_org_migrations(&pool).await?;
        Ok(Self { pool })
    }

    /// Get the underlying pool (for building store implementations).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Build a [`SqliteOrgStore`](super::org_store::SqliteOrgStore).
    pub fn org_store(&self) -> super::org_store::SqliteOrgStore {
        super::org_store::SqliteOrgStore::new(self.pool.clone())
    }

    /// Build a [`SqliteOrgOidcConfigStore`](super::org_oidc_config_store::SqliteOrgOidcConfigStore).
    pub fn oidc_config_store(&self) -> super::org_oidc_config_store::SqliteOrgOidcConfigStore {
        super::org_oidc_config_store::SqliteOrgOidcConfigStore::new(self.pool.clone())
    }

    /// Build a [`SqliteUserStore`](super::user_store::SqliteUserStore).
    pub fn user_store(&self) -> super::user_store::SqliteUserStore {
        super::user_store::SqliteUserStore::new(self.pool.clone())
    }

    /// Build a [`SqliteSpaceStore`](super::space_store::SqliteSpaceStore).
    pub fn space_store(&self) -> super::space_store::SqliteSpaceStore {
        super::space_store::SqliteSpaceStore::new(self.pool.clone())
    }

    /// Build a [`SqliteMembershipStore`](super::space_store::SqliteMembershipStore).
    pub fn membership_store(&self) -> super::space_store::SqliteMembershipStore {
        super::space_store::SqliteMembershipStore::new(self.pool.clone())
    }

    /// Build a [`SqliteApiKeyStore`](super::api_key_store::SqliteApiKeyStore).
    pub fn api_key_store(&self) -> super::api_key_store::SqliteApiKeyStore {
        super::api_key_store::SqliteApiKeyStore::new(self.pool.clone())
    }

    /// Build a [`SqliteClientStore`](super::auth_state_store::SqliteClientStore).
    pub fn client_store(&self) -> super::auth_state_store::SqliteClientStore {
        super::auth_state_store::SqliteClientStore::new(self.pool.clone())
    }

    /// Build a [`SqliteAuthCodeStore`](super::auth_state_store::SqliteAuthCodeStore).
    pub fn auth_code_store(&self) -> super::auth_state_store::SqliteAuthCodeStore {
        super::auth_state_store::SqliteAuthCodeStore::new(self.pool.clone())
    }

    /// Build a [`SqliteRefreshTokenStore`](super::auth_state_store::SqliteRefreshTokenStore).
    pub fn refresh_token_store(&self) -> super::auth_state_store::SqliteRefreshTokenStore {
        super::auth_state_store::SqliteRefreshTokenStore::new(self.pool.clone())
    }

    /// Build a [`SqliteDeviceCodeStore`](super::auth_state_store::SqliteDeviceCodeStore).
    pub fn device_code_store(&self) -> super::auth_state_store::SqliteDeviceCodeStore {
        super::auth_state_store::SqliteDeviceCodeStore::new(self.pool.clone())
    }

    /// Build a [`SqliteCatalogItemStore`](super::catalog_item_store::SqliteCatalogItemStore).
    pub fn catalog_item_store(&self) -> super::catalog_item_store::SqliteCatalogItemStore {
        super::catalog_item_store::SqliteCatalogItemStore::new(self.pool.clone())
    }

    /// Build a [`SqliteCatalogSubscriptionStore`](super::catalog_item_store::SqliteCatalogSubscriptionStore).
    pub fn catalog_subscription_store(
        &self,
    ) -> super::catalog_item_store::SqliteCatalogSubscriptionStore {
        super::catalog_item_store::SqliteCatalogSubscriptionStore::new(self.pool.clone())
    }

    /// Build a [`SqliteInterfaceInstanceStore`](super::interface_instance_store::SqliteInterfaceInstanceStore).
    pub fn interface_instance_store(
        &self,
    ) -> super::interface_instance_store::SqliteInterfaceInstanceStore {
        super::interface_instance_store::SqliteInterfaceInstanceStore::new(self.pool.clone())
    }

    /// Build a [`SqliteIntegrationStore`](super::integration_store::SqliteIntegrationStore).
    pub fn integration_store(&self) -> super::integration_store::SqliteIntegrationStore {
        super::integration_store::SqliteIntegrationStore::new(self.pool.clone())
    }

    /// Build a [`SqliteIntegrationCredentialStore`](super::integration_store::SqliteIntegrationCredentialStore).
    pub fn integration_health_store(
        &self,
    ) -> super::integration_store::SqliteIntegrationHealthStore {
        super::integration_store::SqliteIntegrationHealthStore::new(self.pool.clone())
    }

    /// Build a [`SqliteIntegrationCredentialStore`](super::integration_store::SqliteIntegrationCredentialStore).
    pub fn integration_credential_store(
        &self,
    ) -> super::integration_store::SqliteIntegrationCredentialStore {
        super::integration_store::SqliteIntegrationCredentialStore::new(self.pool.clone())
    }

    /// Build a [`SqliteBindingStore`](super::binding_store::SqliteBindingStore).
    pub fn binding_store(&self) -> super::binding_store::SqliteBindingStore {
        super::binding_store::SqliteBindingStore::new(self.pool.clone())
    }
}

/// Run all org.db migrations in order.
async fn run_org_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON;").execute(pool).await?;
    sqlx::query("PRAGMA busy_timeout = 5000;")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (
            name        TEXT PRIMARY KEY,
            applied_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(pool)
    .await?;

    let migrations: &[(&str, &str)] = &[
        (
            "001_org_tables",
            include_str!("../../../migrations/org/001_org_tables.sql"),
        ),
        (
            "002_auth_tables",
            include_str!("../../../migrations/org/002_auth_tables.sql"),
        ),
        (
            "003_api_keys",
            include_str!("../../../migrations/org/003_api_keys.sql"),
        ),
        (
            "004_catalog_interfaces_bindings",
            include_str!("../../../migrations/org/004_catalog_interfaces_bindings.sql"),
        ),
        (
            "006_integrations",
            include_str!("../../../migrations/org/006_integrations.sql"),
        ),
        (
            "007_user_token_version",
            include_str!("../../../migrations/org/007_user_token_version.sql"),
        ),
        (
            "008_integration_health",
            include_str!("../../../migrations/org/008_integration_health.sql"),
        ),
        (
            "009_org_oidc_configs",
            include_str!("../../../migrations/org/009_org_oidc_configs.sql"),
        ),
    ];

    for &(name, sql) in migrations {
        let already_applied: bool =
            sqlx::query_scalar("SELECT COUNT(*) > 0 FROM _migrations WHERE name = ?")
                .bind(name)
                .fetch_one(pool)
                .await?;

        if !already_applied {
            debug!(migration = name, "applying org migration");
            sqlx::raw_sql(sql)
                .execute(pool)
                .await
                .with_context(|| format!("failed to apply org migration: {name}"))?;
            sqlx::query("INSERT INTO _migrations (name) VALUES (?)")
                .bind(name)
                .execute(pool)
                .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn org_storage_layer_creates_in_memory() {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        // Verify tables exist by querying them.
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM organizations")
            .fetch_one(layer.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn org_storage_layer_creates_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_org.db");
        let layer = OrgStorageLayer::new(&db_path).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(layer.pool())
            .await
            .unwrap();
        assert_eq!(count.0, 0);
        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn org_migrations_are_idempotent() {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        // Running migrations again should not fail.
        run_org_migrations(layer.pool()).await.unwrap();
    }

    // -- 006_integrations --

    async fn seeded() -> OrgStorageLayer {
        let layer = OrgStorageLayer::new_in_memory().await.unwrap();
        for sql in [
            "INSERT INTO organizations (id, name, slug) VALUES ('org_1', 'Acme', 'acme')",
            "INSERT INTO users (id, org_id, email) VALUES ('usr_1', 'org_1', 'a@x.io')",
            "INSERT INTO spaces (id, org_id, name, slug) VALUES ('sp_1', 'org_1', 'Eng', 'eng')",
        ] {
            sqlx::query(sql).execute(layer.pool()).await.unwrap();
        }
        layer
    }

    async fn insert_integration(
        layer: &OrgStorageLayer,
        id: &str,
        layer_kind: &str,
        space_id: Option<&str>,
        owner: Option<&str>,
        slug: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO integrations
               (id, org_id, layer, space_id, owner_user_id, connect_url, slug, name,
                auth_kind, credential_mode)
             VALUES (?, 'org_1', ?, ?, ?, 'https://mcp.example.com', ?, 'Example', 'none', 'per_user')",
        )
        .bind(id)
        .bind(layer_kind)
        .bind(space_id)
        .bind(owner)
        .bind(slug)
        .execute(layer.pool())
        .await
        .map(|_| ())
    }

    #[tokio::test]
    async fn integrations_accept_a_valid_row_per_layer() {
        let layer = seeded().await;
        insert_integration(&layer, "int_o", "org", None, None, "linear")
            .await
            .unwrap();
        insert_integration(&layer, "int_s", "space", Some("sp_1"), None, "linear")
            .await
            .unwrap();
        insert_integration(&layer, "int_u", "user", None, Some("usr_1"), "linear")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn integrations_reject_layer_owner_mismatch() {
        let layer = seeded().await;
        insert_integration(&layer, "ok", "org", None, None, "ok")
            .await
            .unwrap();
        let cases = [
            ("space without space_id", "space", None, None),
            ("space with owner", "space", Some("sp_1"), Some("usr_1")),
            ("user without owner", "user", None, None),
            ("user with space", "user", Some("sp_1"), Some("usr_1")),
            ("org with space", "org", Some("sp_1"), None),
            ("org with owner", "org", None, Some("usr_1")),
            ("unknown layer", "team", None, None),
        ];
        for (name, kind, space, owner) in cases {
            let res = insert_integration(&layer, "bad", kind, space, owner, "bad").await;
            assert!(res.is_err(), "{name} must be rejected");
        }
    }

    #[tokio::test]
    async fn integrations_reject_duplicate_slug_within_layer_and_owner() {
        let layer = seeded().await;
        insert_integration(&layer, "a", "org", None, None, "linear")
            .await
            .unwrap();
        assert!(
            insert_integration(&layer, "b", "org", None, None, "linear")
                .await
                .is_err(),
            "duplicate org-layer slug (NULL owners must still collide)"
        );
        insert_integration(&layer, "c", "space", Some("sp_1"), None, "linear")
            .await
            .unwrap();
        assert!(
            insert_integration(&layer, "d", "space", Some("sp_1"), None, "linear")
                .await
                .is_err()
        );
        insert_integration(&layer, "e", "user", None, Some("usr_1"), "linear")
            .await
            .unwrap();
        assert!(
            insert_integration(&layer, "f", "user", None, Some("usr_1"), "linear")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn deleting_an_integration_cascades_to_children() {
        let layer = seeded().await;
        insert_integration(&layer, "int_o", "org", None, None, "linear")
            .await
            .unwrap();
        for sql in [
            "INSERT INTO integration_space_access (integration_id, space_id, mode) VALUES ('int_o', 'sp_1', 'allow')",
            "INSERT INTO integration_credentials (integration_id, kind, ciphertext, nonce, key_id, updated_at)
             VALUES ('int_o', 'header', x'00', x'00', 'k', '2026-01-01T00:00:00Z')",
            "INSERT INTO integration_oauth_states (state_hash, integration_id, pkce_verifier_ct, pkce_verifier_nonce, pkce_verifier_key_id, created_at)
             VALUES ('h', 'int_o', x'00', x'00', 'k', '2026-01-01T00:00:00Z')",
        ] {
            sqlx::query(sql).execute(layer.pool()).await.unwrap();
        }
        sqlx::query("DELETE FROM integrations WHERE id = 'int_o'")
            .execute(layer.pool())
            .await
            .unwrap();
        for sql in [
            "SELECT COUNT(*) FROM integration_space_access",
            "SELECT COUNT(*) FROM integration_credentials",
            "SELECT COUNT(*) FROM integration_oauth_states",
        ] {
            let n: i64 = sqlx::query_scalar(sql)
                .fetch_one(layer.pool())
                .await
                .unwrap();
            assert_eq!(n, 0, "{sql} must be empty after cascade");
        }
    }

    #[tokio::test]
    async fn shared_credential_is_unique_per_integration() {
        let layer = seeded().await;
        insert_integration(&layer, "int_o", "org", None, None, "linear")
            .await
            .unwrap();
        let insert = "INSERT INTO integration_credentials (integration_id, subject_user_id, kind, ciphertext, nonce, key_id, updated_at)
             VALUES ('int_o', ?, 'header', x'00', x'00', 'k', '2026-01-01T00:00:00Z')";
        sqlx::query(insert)
            .bind(None::<&str>)
            .execute(layer.pool())
            .await
            .unwrap();
        assert!(
            sqlx::query(insert)
                .bind(None::<&str>)
                .execute(layer.pool())
                .await
                .is_err(),
            "second shared credential (NULL subject) must collide"
        );
        sqlx::query(insert)
            .bind("usr_1")
            .execute(layer.pool())
            .await
            .unwrap();
    }
}
