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
    ];

    for (name, sql) in migrations {
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
}
