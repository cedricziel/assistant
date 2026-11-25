//! SQLite-backed [`IntegrationStore`] and [`IntegrationCredentialStore`].

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use assistant_core::identity::{OrgId, SpaceId, UserId};
use assistant_core::integrations::{
    CatalogMode, CredentialKind, CredentialStatus, Integration, IntegrationCredentialStore,
    IntegrationFilter, IntegrationPolicy, IntegrationStore, IntegrationStoreError, SpaceAccess,
    SpaceAccessMode, StoredCredential,
};

// Macros rather than consts: sqlx accepts only `&'static str` literals.
macro_rules! integration_columns {
    () => {
        "id, org_id, layer, space_id, owner_user_id, catalog_entry_id, connect_url, slug, name, \
         auth_kind, header_name, header_value_prefix, credential_mode, trust, enabled, \
         oauth_client_id_override, created_at, updated_at"
    };
}

macro_rules! credential_columns {
    () => {
        "integration_id, subject_user_id, kind, ciphertext, nonce, key_id, expires_at, status, \
         updated_at"
    };
}

/// SQLite-backed integration store.
pub struct SqliteIntegrationStore {
    pool: SqlitePool,
}

impl SqliteIntegrationStore {
    /// `expected_updated_at`, when set, must equal the stored one.
    async fn update_checked(
        &self,
        i: &Integration,
        expected_updated_at: Option<DateTime<Utc>>,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let current = sqlx::query(
            "SELECT catalog_entry_id, connect_url, updated_at FROM integrations WHERE id = ?",
        )
        .bind(&i.id)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("loading integration {}", i.id))?;
        let Some(current) = current else {
            return Ok(false);
        };
        if let Some(expected) = expected_updated_at
            && current.try_get::<DateTime<Utc>, _>("updated_at")? != expected
        {
            return Err(IntegrationStoreError::Stale { id: i.id.clone() }.into());
        }
        let from_catalog = current
            .try_get::<Option<String>, _>("catalog_entry_id")?
            .is_some();
        let url_changed = current.try_get::<String, _>("connect_url")? != i.connect_url;
        if from_catalog && url_changed {
            return Err(IntegrationStoreError::ConnectUrlImmutable { id: i.id.clone() }.into());
        }

        sqlx::query(
            "UPDATE integrations SET slug = ?, name = ?, connect_url = ?, auth_kind = ?,
                    header_name = ?, header_value_prefix = ?, credential_mode = ?, trust = ?,
                    enabled = ?, oauth_client_id_override = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&i.slug)
        .bind(&i.name)
        .bind(&i.connect_url)
        .bind(i.auth_kind.as_str())
        .bind(&i.header_name)
        .bind(&i.header_value_prefix)
        .bind(i.credential_mode.as_str())
        .bind(i.trust.as_str())
        .bind(i.enabled)
        .bind(&i.oauth_client_id_override)
        .bind(i.updated_at)
        .bind(&i.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_write_error(e, &i.slug, &format!("updating integration {}", i.id)))?;

        if url_changed {
            // Credentials were issued for the old host; never send them to the new one.
            sqlx::query("DELETE FROM integration_credentials WHERE integration_id = ?")
                .bind(&i.id)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("dropping credentials of integration {}", i.id))?;
            sqlx::query("DELETE FROM integration_oauth_states WHERE integration_id = ?")
                .bind(&i.id)
                .execute(&mut *tx)
                .await
                .with_context(|| format!("dropping OAuth state of integration {}", i.id))?;
        }
        tx.commit().await?;
        Ok(true)
    }

    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse<T: std::str::FromStr<Err = anyhow::Error>>(row: &SqliteRow, column: &str) -> Result<T> {
    let text: String = row.try_get(column)?;
    text.parse()
}

fn row_to_integration(row: &SqliteRow) -> Result<Integration> {
    Ok(Integration {
        id: row.try_get("id")?,
        org_id: OrgId(row.try_get("org_id")?),
        layer: parse(row, "layer")?,
        space_id: row.try_get::<Option<String>, _>("space_id")?.map(SpaceId),
        owner_user_id: row
            .try_get::<Option<String>, _>("owner_user_id")?
            .map(UserId),
        catalog_entry_id: row.try_get("catalog_entry_id")?,
        connect_url: row.try_get("connect_url")?,
        slug: row.try_get("slug")?,
        name: row.try_get("name")?,
        auth_kind: parse(row, "auth_kind")?,
        header_name: row.try_get("header_name")?,
        header_value_prefix: row.try_get("header_value_prefix")?,
        credential_mode: parse(row, "credential_mode")?,
        trust: parse(row, "trust")?,
        enabled: row.try_get("enabled")?,
        oauth_client_id_override: row.try_get("oauth_client_id_override")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn row_to_access(row: &SqliteRow) -> Result<SpaceAccess> {
    Ok(SpaceAccess {
        integration_id: row.try_get("integration_id")?,
        space_id: SpaceId(row.try_get("space_id")?),
        mode: parse(row, "mode")?,
    })
}

/// Map the slug unique index violation to [`IntegrationStoreError::DuplicateSlug`].
fn map_write_error(err: sqlx::Error, slug: &str, what: &str) -> anyhow::Error {
    let is_duplicate_slug = matches!(
        &err,
        sqlx::Error::Database(db) if db.is_unique_violation() && db.message().contains("idx_integrations_slug")
    );
    if is_duplicate_slug {
        IntegrationStoreError::DuplicateSlug { slug: slug.into() }.into()
    } else {
        anyhow::Error::new(err).context(what.to_owned())
    }
}

#[async_trait]
impl IntegrationStore for SqliteIntegrationStore {
    async fn create(&self, i: &Integration) -> Result<()> {
        sqlx::query(concat!(
            "INSERT INTO integrations (",
            integration_columns!(),
            ") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        ))
        .bind(&i.id)
        .bind(&i.org_id.0)
        .bind(i.layer.as_str())
        .bind(i.space_id.as_ref().map(|s| s.0.as_str()))
        .bind(i.owner_user_id.as_ref().map(|u| u.0.as_str()))
        .bind(&i.catalog_entry_id)
        .bind(&i.connect_url)
        .bind(&i.slug)
        .bind(&i.name)
        .bind(i.auth_kind.as_str())
        .bind(&i.header_name)
        .bind(&i.header_value_prefix)
        .bind(i.credential_mode.as_str())
        .bind(i.trust.as_str())
        .bind(i.enabled)
        .bind(&i.oauth_client_id_override)
        .bind(i.created_at)
        .bind(i.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_write_error(e, &i.slug, &format!("creating integration {}", i.id)))?;
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Option<Integration>> {
        let row = sqlx::query(concat!(
            "SELECT ",
            integration_columns!(),
            " FROM integrations WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("loading integration {id}"))?;
        row.as_ref().map(row_to_integration).transpose()
    }

    async fn list(&self, org_id: &OrgId, filter: &IntegrationFilter) -> Result<Vec<Integration>> {
        let layer = filter.layer.map(|l| l.as_str());
        let space = filter.space_id.as_ref().map(|s| s.0.as_str());
        let owner = filter.owner_user_id.as_ref().map(|u| u.0.as_str());
        let rows = sqlx::query(concat!(
            "SELECT ",
            integration_columns!(),
            " FROM integrations
             WHERE org_id = ?
               AND (? IS NULL OR layer = ?)
               AND (? IS NULL OR space_id = ?)
               AND (? IS NULL OR owner_user_id = ?)
             ORDER BY created_at, id"
        ))
        .bind(&org_id.0)
        .bind(layer)
        .bind(layer)
        .bind(space)
        .bind(space)
        .bind(owner)
        .bind(owner)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("listing integrations for org {org_id}"))?;
        rows.iter().map(row_to_integration).collect()
    }

    async fn update(&self, i: &Integration) -> Result<bool> {
        self.update_checked(i, None).await
    }

    async fn update_if_unchanged(
        &self,
        i: &Integration,
        expected_updated_at: DateTime<Utc>,
    ) -> Result<bool> {
        self.update_checked(i, Some(expected_updated_at)).await
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        // Credentials, space access and OAuth state go via ON DELETE CASCADE.
        let result = sqlx::query("DELETE FROM integrations WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .with_context(|| format!("deleting integration {id}"))?;
        Ok(result.rows_affected() > 0)
    }

    async fn add_space_access(
        &self,
        integration_id: &str,
        space_id: &SpaceId,
        mode: SpaceAccessMode,
    ) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO integration_space_access (integration_id, space_id, mode)
             VALUES (?, ?, ?)",
        )
        .bind(integration_id)
        .bind(&space_id.0)
        .bind(mode.as_str())
        .execute(&self.pool)
        .await
        .with_context(|| format!("adding {mode} access of {integration_id} for {space_id}"))?;
        Ok(())
    }

    async fn remove_space_access(
        &self,
        integration_id: &str,
        space_id: &SpaceId,
        mode: SpaceAccessMode,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM integration_space_access
             WHERE integration_id = ? AND space_id = ? AND mode = ?",
        )
        .bind(integration_id)
        .bind(&space_id.0)
        .bind(mode.as_str())
        .execute(&self.pool)
        .await
        .with_context(|| format!("removing {mode} access of {integration_id} for {space_id}"))?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_space_access(&self, integration_id: &str) -> Result<Vec<SpaceAccess>> {
        let rows = sqlx::query(
            "SELECT integration_id, space_id, mode FROM integration_space_access
             WHERE integration_id = ? ORDER BY space_id, mode",
        )
        .bind(integration_id)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("listing space access of {integration_id}"))?;
        rows.iter().map(row_to_access).collect()
    }

    async fn list_org_space_access(&self, org_id: &OrgId) -> Result<Vec<SpaceAccess>> {
        let rows = sqlx::query(
            "SELECT a.integration_id, a.space_id, a.mode
             FROM integration_space_access a
             JOIN integrations i ON i.id = a.integration_id
             WHERE i.org_id = ? ORDER BY a.integration_id, a.space_id, a.mode",
        )
        .bind(&org_id.0)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("listing space access for org {org_id}"))?;
        rows.iter().map(row_to_access).collect()
    }

    async fn get_policy(&self, org_id: &OrgId) -> Result<IntegrationPolicy> {
        let row = sqlx::query(
            "SELECT org_id, catalog_mode, allowed_catalog_ids, allow_custom_urls,
                    allow_user_integrations, updated_at
             FROM integration_policy WHERE org_id = ?",
        )
        .bind(&org_id.0)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("loading integration policy for org {org_id}"))?;
        let Some(row) = row else {
            return Ok(IntegrationPolicy::default_for(org_id.clone()));
        };
        let allowed: String = row.try_get("allowed_catalog_ids")?;
        Ok(IntegrationPolicy {
            org_id: org_id.clone(),
            catalog_mode: parse::<CatalogMode>(&row, "catalog_mode")?,
            allowed_catalog_ids: serde_json::from_str(&allowed)
                .context("parsing allowed_catalog_ids")?,
            allow_custom_urls: row.try_get("allow_custom_urls")?,
            allow_user_integrations: row.try_get("allow_user_integrations")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    async fn put_policy(&self, policy: &IntegrationPolicy) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO integration_policy
               (org_id, catalog_mode, allowed_catalog_ids, allow_custom_urls,
                allow_user_integrations, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&policy.org_id.0)
        .bind(policy.catalog_mode.as_str())
        .bind(serde_json::to_string(&policy.allowed_catalog_ids)?)
        .bind(policy.allow_custom_urls)
        .bind(policy.allow_user_integrations)
        .bind(policy.updated_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("saving integration policy for org {}", policy.org_id))?;
        Ok(())
    }
}

/// SQLite-backed credential store. Holds ciphertext only.
pub struct SqliteIntegrationCredentialStore {
    pool: SqlitePool,
}

impl SqliteIntegrationCredentialStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn row_to_credential(row: &SqliteRow) -> Result<StoredCredential> {
    Ok(StoredCredential {
        integration_id: row.try_get("integration_id")?,
        subject_user_id: row
            .try_get::<Option<String>, _>("subject_user_id")?
            .map(UserId),
        kind: parse::<CredentialKind>(row, "kind")?,
        ciphertext: row.try_get("ciphertext")?,
        nonce: row.try_get("nonce")?,
        key_id: row.try_get("key_id")?,
        expires_at: row.try_get("expires_at")?,
        status: parse::<CredentialStatus>(row, "status")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[async_trait]
impl IntegrationCredentialStore for SqliteIntegrationCredentialStore {
    async fn put(&self, c: &StoredCredential) -> Result<()> {
        sqlx::query(concat!(
            "INSERT OR REPLACE INTO integration_credentials (",
            credential_columns!(),
            ") VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        ))
        .bind(&c.integration_id)
        .bind(c.subject_user_id.as_ref().map(|u| u.0.as_str()))
        .bind(c.kind.as_str())
        .bind(&c.ciphertext)
        .bind(&c.nonce)
        .bind(&c.key_id)
        .bind(c.expires_at)
        .bind(c.status.as_str())
        .bind(c.updated_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("saving credential of integration {}", c.integration_id))?;
        Ok(())
    }

    async fn get(
        &self,
        integration_id: &str,
        subject: Option<&UserId>,
    ) -> Result<Option<StoredCredential>> {
        let row = sqlx::query(concat!(
            "SELECT ",
            credential_columns!(),
            " FROM integration_credentials
             WHERE integration_id = ? AND subject_user_id IS ?"
        ))
        .bind(integration_id)
        .bind(subject.map(|u| u.0.as_str()))
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("loading credential of integration {integration_id}"))?;
        row.as_ref().map(row_to_credential).transpose()
    }

    async fn delete(&self, integration_id: &str, subject: Option<&UserId>) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM integration_credentials
             WHERE integration_id = ? AND subject_user_id IS ?",
        )
        .bind(integration_id)
        .bind(subject.map(|u| u.0.as_str()))
        .execute(&self.pool)
        .await
        .with_context(|| format!("deleting credential of integration {integration_id}"))?;
        Ok(result.rows_affected() > 0)
    }

    async fn compare_and_swap(
        &self,
        expected_updated_at: DateTime<Utc>,
        c: &StoredCredential,
    ) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE integration_credentials
             SET kind = ?, ciphertext = ?, nonce = ?, key_id = ?, expires_at = ?,
                 status = ?, updated_at = ?
             WHERE integration_id = ? AND subject_user_id IS ? AND updated_at = ?",
        )
        .bind(c.kind.as_str())
        .bind(&c.ciphertext)
        .bind(&c.nonce)
        .bind(&c.key_id)
        .bind(c.expires_at)
        .bind(c.status.as_str())
        .bind(c.updated_at)
        .bind(&c.integration_id)
        .bind(c.subject_user_id.as_ref().map(|u| u.0.as_str()))
        .bind(expected_updated_at)
        .execute(&self.pool)
        .await
        .with_context(|| format!("swapping credential of integration {}", c.integration_id))?;
        Ok(result.rows_affected() > 0)
    }
}
